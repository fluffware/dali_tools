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
use crate::drivers;
use drivers::driver::{DaliDriver, DaliSendResult, DriverInfo, 
		      OpenError, 
		      DaliBusEvent, DaliBusEventType,
		      DaliBusEventResult, DaliFrame};
use drivers::utils as driver_utils;
use drivers::send_flags::Flags;
use driver_utils::{DALIreq};
use super::dali_msg::DaliMsg;
use super::dali_msg;
use std::thread;
use nix::poll;
use std::os::unix::io::AsRawFd;

struct PruDriver
{
    req_tx: Option<mpsc::Sender<DALIreq>>,
    monitor_tx: mpsc::Sender<oneshot::Sender<DaliBusEventResult>>,
    read_thread_join: Option<thread::JoinHandle<()>>	
}

impl DaliDriver for PruDriver
{
    fn send_frame(&mut self, cmd: DaliFrame, flags:Flags) -> 
        Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
    {
	if let Some(req_tx) = &mut self.req_tx {
	    driver_utils::send_frame(req_tx,&cmd, flags)
	} else {
	    Box::pin(std::future::ready(
		DaliSendResult::DriverError("No command queue".into())))
	}
    }
    
    fn next_bus_event(&mut self) ->
	Pin<Box<dyn Future<Output = DaliBusEventResult>>>
    {
	driver_utils::next_bus_event(&mut self.monitor_tx)
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

fn reply_to_send_result(reply: &DaliMsg) -> DaliSendResult
{
    return match reply.result() {
	dali_msg::DALI_SEND_DONE =>
	    DaliSendResult::OK,
        dali_msg::DALI_RECV_FRAME=> {
            if reply.bit_length() == 8 {
                // eprintln!("Answer: {}", reply.frame_data()[0]);
                DaliSendResult::Answer(reply.frame_data()[0])
            } else {
                DaliSendResult::DriverError("Answer must be 8 bits"
					    .into())
            }
        },
	dali_msg::DALI_ERR_FRAMING =>
	    DaliSendResult::Framing,
	dali_msg::DALI_NO_REPLY =>
	    DaliSendResult::Timeout,
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
		format!("Device result: {}", reply.result()).into())
    }
}

async fn send_frame<W>(dev_write: &mut W,
                       dev_read: &mut mpsc::Receiver<DaliMsg>, 
                       req: &DALIreq, seq_no: &mut u8) 
		       -> DaliSendResult
    where W: Write
{
    let mut msg;
    // Create a frame of correct length
    match req.cmd.data {
	DaliFrame::Frame8(f) => {
	    msg = DaliMsg::frame8(*seq_no, &[f]);
	    msg.set_ignore_collisions(true);
	    msg.set_priority(dali_msg::DALI_FLAGS_PRIORITY_0);
	},
	DaliFrame::Frame16(f) => {
	    msg = DaliMsg::frame16(*seq_no, &f);
	},
	DaliFrame::Frame24(f) => {
	    msg = DaliMsg::frame24(*seq_no, &f);
	},
	DaliFrame::Frame25(f) => {
	    msg = DaliMsg::frame25(*seq_no, &f);
	},
    };
    
    if *seq_no == 255 {
	*seq_no = 1;
    } else {
	*seq_no += 1;
    }
    
    let priority = req.cmd.flags.priority();

    if let DaliFrame::Frame8(_) = req.cmd.data {
	msg.set_priority(dali_msg::DALI_FLAGS_PRIORITY_0);
    } else if !(1..=5).contains(&priority) {
	msg.set_priority(priority);
    }	
    msg.set_send_twice(req.cmd.flags.send_twice());
    
    msg.set_expect_answer(req.cmd.flags.expect_answer());

    msg.set_retry(true);

    //eprintln!("Sending: {:x?}", msg);
    let send_seq = msg.seq();
    let expect_answer = msg.expect_answer();
    let block = unsafe {
	std::mem::transmute::<DaliMsg, [u8;8]>(msg)
    };
    if let Err(e) = dev_write.write_all(&block) {
	eprintln!("Send failed: {}", e);
	return DaliSendResult::DriverError(e.into());
    }
    if let Err(e) = dev_write.flush() {
	eprintln!("Flush failed: {}", e);
	return DaliSendResult::DriverError(e.into());
    }

    loop {
        // Wait for reply
        let reply_msg = dev_read.recv().await;
        match reply_msg {
	    Some(reply) => {
                if reply.seq() == send_seq {
                    if !(expect_answer 
		         && reply.result() == dali_msg::DALI_SEND_DONE) {
                        return reply_to_send_result(&reply);
                    }
                } else {
                    return DaliSendResult::DriverError(
		        format!("Unexpected sequence number in reply: \
                                 Got {}, expected",
                                reply.seq()).into())
                }
            },
	    None => 
                return DaliSendResult::DriverError("Read channel closed".into())
        }
    }
}

#[derive(Debug)]
enum DriverError
{
    UnhandledFrameLength,
    DeviceError,
    TimingError,
    DeviceResult(u8),
    SequenceError{expected: u8, got: u8},
}

impl std::error::Error for DriverError {}

use DriverError::*;
impl std::fmt::Display for DriverError
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
	    UnhandledFrameLength => write!(f, "Unhandled frame length"),
	    DeviceError => write!(f, "Device error"),
	    TimingError => write!(f, "Internal timing error"),
	    DeviceResult(res) => write!(f, "Device result: {}", res),
	    SequenceError{expected, got} => {
		write!(f, "Expected {} as sequence number, got {}",
		       expected, got)
	    }
	}
    }
}

fn handle_block(msg: &DaliMsg,
		mrtx: &mut Option<oneshot::Sender<DaliBusEventResult>>)
		-> Result<(), DriverError>
{
    let event_type;
    if msg.seq() == 0 {
        event_type = match msg.result() {
	    dali_msg::DALI_RECV_FRAME => {
		let frame = msg.frame_data();
		match msg.bit_length() {
		    8 => DaliBusEventType::Frame8(frame[0]),
		    16 => DaliBusEventType::Frame16(
			[frame[0], frame[1]]),
		    24 => DaliBusEventType::Frame24(
			[frame[0], frame[1], frame[2]]),
		    _ => return Err(UnhandledFrameLength)
		}
	    },
	    dali_msg::DALI_ERR_FRAMING =>
		DaliBusEventType::FramingError,
	    dali_msg::DALI_ERR_BUS_LOW =>
		DaliBusEventType::BusPowerOff,
	    dali_msg::DALI_INFO_BUS_HIGH =>
		    DaliBusEventType::BusPowerOn,
	    
	    dali_msg::DALI_ERR_DRIVER =>
		return Err(DeviceError),
	    dali_msg::DALI_ERR_TIMING =>
		return Err(TimingError),
	    _ => return Err(DeviceResult(msg.result()))
	};
    } else {
	return Err(SequenceError{expected: 0, got: msg.seq()});
    }
    let event = DaliBusEvent {
	timestamp: Instant::now(),
	event_type
    };
    if let Some(mrtx) = mrtx.take() {
	mrtx.send(Ok(event)).unwrap_or(());
    }
    Ok(())
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
                       mut monitor: mpsc::Receiver<oneshot::Sender<DaliBusEventResult>>)
{

    
    let mut seq_no = 1;
    let mut mrtx: Option<oneshot::Sender<DaliBusEventResult>> = None;
    loop {
	tokio::select!{
	    msg = dev_read.recv() => {
		match msg {
		    Some(msg) => {
			if let Err(err) = handle_block(&msg, &mut mrtx) {
			     if let Some(mrtx) = mrtx.take() {
				 mrtx.send(Err(Box::new(err))).unwrap_or(());
			     }
			}
		    },
		    None => break
		}
	    },
	    req = rx.recv() => {
		match req {
		    Some(req) => {
			//eprintln!("Got {:?}",req);
			let tx = 
			    send_frame(&mut dev_write, &mut dev_read, &req, &mut seq_no).await;
			req.reply.send(tx).unwrap_or(());
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
    let (mtx, mrx) = mpsc::channel::<oneshot::Sender<DaliBusEventResult>>(10);
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
