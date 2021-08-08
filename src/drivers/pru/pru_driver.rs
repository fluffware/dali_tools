use std::fs::OpenOptions;
use std::fs::File;
use std::io::{Write, Read};
use std::pin::Pin;
use std::future::Future;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio::sync::mpsc::error::TrySendError;
use tokio::sync::oneshot;
use std::collections::HashMap;
use super::super::driver::{self,DaliDriver, DaliSendResult, DriverInfo, 
			   OpenError, 
			   DaliBusEvent, DaliBusEventType};
use super::super::utils as driver_utils;
use driver_utils::{DALIreq};
use super::dali_msg::DaliMsg;
use super::dali_msg;
use std::thread;
use nix::poll;
use std::os::unix::io::AsRawFd;

struct PruDriver
{
    req_tx: Option<mpsc::Sender<DALIreq>>,
    monitor_tx: mpsc::Sender<oneshot::Sender<DaliBusEvent>>,
    read_thread_join: Option<thread::JoinHandle<()>>	
}

impl DaliDriver for PruDriver
{
    fn send_frame(&mut self, cmd: &[u8;4], flags:u16) -> 
        Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
    {
	if let Some(req_tx) = &mut self.req_tx {
	    driver_utils::send_frame(req_tx,cmd, flags)
	} else {
	    Box::pin(std::future::ready(
		DaliSendResult::DriverError("No command queue".into())))
	}
    }
    
    fn next_bus_event(&mut self) -> Pin<Box<dyn Future<Output = DaliBusEvent> + Send>>
    {
	let (tx, rx) = oneshot::channel();
	
	match self.monitor_tx.try_send(tx) {
            Ok(()) => {
		Box::pin(async {
                    match rx.await {
			Ok(r) => r,
			
			Err(e) => DaliBusEvent{
			    timestamp: Instant::now(),
			    event: DaliBusEventType::DriverError(Box::new(e))
			}
                    }
            })
            },
            Err(_) => {
		Box::pin(std::future::ready(
		    DaliBusEvent{
			timestamp: Instant::now(),
			event: DaliBusEventType::DriverError(
                            "Failed to queue monitor request".into())
		    }
		))
	    }
	}
    }
}

impl Drop for PruDriver
{
    fn drop(&mut self )
    {
	self.req_tx = None;
	if let Some(handle) = self.read_thread_join.take() {
	    handle.join().unwrap_or(());
	}
    }
}

async fn send_frame<W>(device: &mut W, req: &DALIreq, seq_no: &mut u8) 
		       -> Option<DaliSendResult>
    where W: Write
{
    let mut msg;
    match req.cmd.flags & 0x0700 {
	driver::LENGTH_8 => {
	    msg = DaliMsg::frame8(*seq_no, &req.cmd.data);
	    msg.set_ignore_collisions(true);
	    msg.set_priority(dali_msg::DALI_FLAGS_PRIORITY_0);
	},
	driver::LENGTH_16 => {
	    msg = DaliMsg::frame16(*seq_no, &req.cmd.data);
	},
	driver::LENGTH_24 => {
	    msg = DaliMsg::frame24(*seq_no, &req.cmd.data);
	},
	driver::LENGTH_25 => {
	    msg = DaliMsg::frame25(*seq_no, &req.cmd.data);
	},
	_ => {
	    return Some(DaliSendResult::DriverError(
		"Illegal frame length".into()))
	}
    };
    
    if *seq_no == 255 {
	*seq_no = 1;
    } else {
	*seq_no += 1;
    }
    
    let priority = req.cmd.flags & 0x0007;
    
    if req.cmd.flags & 0x0700 == driver::LENGTH_8 {
	msg.set_priority(dali_msg::DALI_FLAGS_PRIORITY_0);
    } else if !(1..=5).contains(&priority) {
	msg.set_priority(priority);
    }	
    msg.set_send_twice(req.cmd.flags & driver::SEND_TWICE != 0);
    
    msg.set_expect_answer(req.cmd.flags & driver::EXPECT_ANSWER != 0);

    msg.set_retry(true);

    let block = unsafe {
	std::mem::transmute::<DaliMsg, [u8;8]>(msg)
    };
    if let Err(e) = device.write_all(&block) {
	eprintln!("Send failed: {}", e);
	return Some(DaliSendResult::DriverError(e.into()));
    }
    if let Err(e) = device.flush() {
	eprintln!("Flush failed: {}", e);
	return Some(DaliSendResult::DriverError(e.into()));
    }
    return None;
}

fn handle_block(msg: &DaliMsg,
		reply: &mut Option<oneshot::Sender<DaliSendResult>>,
		mrtx: &mut Option<oneshot::Sender<DaliBusEvent>>,
		wait_seq: &mut u8)
{
    if *wait_seq > 0 && *wait_seq == msg.seq() {
	if msg.result() == dali_msg::DALI_SEND_DONE && msg.expect_answer() {
	    /* Need to wait for answer */
	} else {
	    if let Some(reply) = reply.take() {
		let result = match msg.result() {
		    dali_msg::DALI_SEND_DONE =>
			DaliSendResult::OK,
		    dali_msg::DALI_ERR_FRAMING =>
			DaliSendResult::Framing,
		    dali_msg::DALI_NO_REPLY =>
			DaliSendResult::Timeout,
		    dali_msg::DALI_ERR_BUS_LOW =>
			DaliSendResult::DriverError("Bus has no power".into()),
		    dali_msg::DALI_INFO_BUS_HIGH =>
		    DaliSendResult::DriverError("Bus power restored".into()),
		    dali_msg::DALI_ERR_BUS_BUSY =>
			DaliSendResult::DriverError(
			    "Bus busy, frame not sent".into()),
		    dali_msg::DALI_ERR_DRIVER =>
		    DaliSendResult::DriverError("Device error".into()),
		    dali_msg::DALI_ERR_TIMING =>
			DaliSendResult::DriverError("Internal timing error"
						    .into()),
		    _ =>
			DaliSendResult::DriverError(
			    format!("Device result: {}", msg.result()) .into())
		};
		
		reply.send(result).unwrap_or(());
		
	    }
	}
    } else if msg.seq() == 0 {
	let event_type = match msg.result() {
	    dali_msg::DALI_RECV_FRAME => {
		let frame = msg.frame_data();
		match msg.bit_length() {
		    8 => DaliBusEventType::Recv8bitFrame(frame[0]),
		    16 => DaliBusEventType::Recv16bitFrame(
			[frame[0], frame[1]]),
		    24 => DaliBusEventType::Recv24bitFrame(
			[frame[0], frame[1], frame[2]]),
		    _ => DaliBusEventType::DriverError(
			"Unhandled frame length".into())
		}
	    },
	    dali_msg::DALI_ERR_FRAMING =>
		DaliBusEventType::RecvFramingError,
	    dali_msg::DALI_ERR_BUS_LOW =>
		DaliBusEventType::BusPowerOff,
	    dali_msg::DALI_INFO_BUS_HIGH =>
		    DaliBusEventType::BusPowerOn,
	    
	    dali_msg::DALI_ERR_DRIVER =>
		DaliBusEventType::DriverError("Device error".into()),
	    dali_msg::DALI_ERR_TIMING =>
		DaliBusEventType::DriverError("Internal timing error".into()),
	    _ =>
		DaliBusEventType::DriverError(
		    format!("Device result: {}", msg.result()) .into())
	};
	let event = DaliBusEvent {
	    timestamp: Instant::now(),
	    event: event_type
	};
	if let Some(mrtx) = mrtx.take() {
	    mrtx.send(event).unwrap_or(());
	}
    }
}

fn read_thread(mut device: File, read_tx:  mpsc::Sender<DaliMsg>)
{
    //eprintln!("Read started");
    let mut fds =[ poll::PollFd::new(device.as_raw_fd(), poll::PollFlags::POLLIN)];
    
    let mut data = [0u8;8];
    loop {
	match poll::poll(&mut fds, 1000) {
	    Ok(_s) => {
		if let Some(revents) = fds[0].revents() {
		    if revents.contains(poll::PollFlags::POLLIN) {
			if let Err(e) = device.read(&mut data) {
			    eprintln!("Read from device failed: {}", e);
			}
			let msg = unsafe {
			    std::mem::transmute::<[u8;8], DaliMsg>(data.clone())
			};
			if let Err(e) = read_tx.try_send(msg) {
			    match e {
				TrySendError::Closed(_) => break,
				TrySendError::Full(_) => {}
			    };
			}
		    }
		}
	    },
	    Err(_) => {
		break
	    }
	}
	if read_tx.is_closed() {
	    break
	}
	
    }
    //eprintln!("Read done");
}

async fn driver_thread(mut dev_write: File, 
		       mut dev_read: mpsc::Receiver<DaliMsg>, 
		       mut rx: mpsc::Receiver<DALIreq>,
                       mut monitor: mpsc::Receiver<oneshot::Sender<DaliBusEvent>>)
{

    
    let mut seq_no = 1;
    let mut wait_seq = 0;
    let mut reply = None;
    let mut mrtx: Option<oneshot::Sender<DaliBusEvent>> = None;
    loop {
	tokio::select!{
	    msg = dev_read.recv() => {
		match msg {
		    Some(msg) => handle_block(&msg, &mut reply, &mut mrtx,
					      &mut wait_seq),
		    None => break
		}
	    },
	    req = rx.recv() => {
		match req {
		    Some(req) => {
			//eprintln!("Got {:?}",req);
			wait_seq = seq_no;
			if let Some(tx) = 
			    send_frame(&mut dev_write, &req, &mut seq_no).await 
			{
			    req.reply.send(tx).unwrap_or(());
			    reply = None;
			} else {
			    reply = Some(req.reply);
			}
		    },
		    None => break
		}
	    },
	    req = monitor.recv() => {
		match req {
		    Some(req) => {
			mrtx = Some(req);
		    },
		    None => break
		}
	    }
	}
    }
    drop(dev_write);
    //println!("Driver stopped");
}

fn driver_open(_params: HashMap<String, String>)
		     -> Result<Box<dyn DaliDriver>, OpenError>
{
    let dev = OpenOptions::new()
	.read(true).write(true)
	.open("/dev/dali-pru0").map_err(
	    |e| OpenError::DriverError(e.into())
	)?;
    let (req_tx, req_rx) = mpsc::channel::<DALIreq>(10);
    let (read_tx, read_rx) = mpsc::channel::<DaliMsg>(10);
    let (mtx, mrx) = mpsc::channel::<oneshot::Sender<DaliBusEvent>>(10);
    let read_dev = dev.try_clone().map_err(
	|e| OpenError::DriverError(e.into())
    )?;
    let read_thread_join = 
	thread::spawn(move || read_thread(read_dev, read_tx));
    tokio::spawn(driver_thread(dev, read_rx, req_rx, mrx));
    Ok(Box::new(PruDriver{req_tx: Some(req_tx),
			  monitor_tx: mtx,
			  read_thread_join: Some(read_thread_join)}))
}

pub fn driver_info() -> DriverInfo
{
    DriverInfo{name: "PRU".to_string(), 
	       description: 
	       "Driver for DALI using the PRU on TI processors.".to_string(),
	       open: driver_open
    }
}
