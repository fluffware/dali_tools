extern crate libusb_async;
use std::sync::Arc;
use std::sync::Mutex;
use std::pin::Pin;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use core::future::Future;
use tokio::time::{Instant};
use super::idle_future::IdleFuture;
use std::collections::HashMap;
use libusb_async::{Context,DeviceHandle};

use super::super::driver::{self,DaliDriver, DaliSendResult, DriverInfo, 
			   OpenError, 
			   DaliBusEvent, DaliBusEventType};
use super::super::utils::{DALIreq, DALIcmd};
use std::error::Error;
use std::fmt;
use std::convert::TryInto;

pub struct Helvar510driver
{
    join: Option<JoinHandle<DriverError>>,
    // Needs to be an option so that it can be dropped to signal the receiver
    send_cmd: Option<mpsc::Sender<DALIreq>>,
    send_monitor:  Arc<Mutex<Option<mpsc::Sender<DaliBusEvent>>>>
    
}

#[derive(Debug,Clone)]
enum DriverError
{
    OK,
    OpenFailed,
    NoInterfaceFound,
    CommandError,
    UsbError,
    ReplyingFailed
}

impl Error for DriverError
{
}

impl From<libusb_async::Error> for DriverError
{
    fn from(_err: libusb_async::Error) -> DriverError
    {
        DriverError::UsbError
    }
}
impl fmt::Display for DriverError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
	match self {
	    DriverError::OK => write!(f, "No error"),
	    DriverError::OpenFailed => write!(f, "Open failed"),
	    DriverError::NoInterfaceFound => write!(f, "No interface found"),
	    DriverError::CommandError => write!(f, "Copmmand error"),
	    DriverError::UsbError => write!(f, "USB error"),
	    DriverError::ReplyingFailed => write!(f, "Replying failed"),
	}
    }
}
fn send_hid_report(dev: &DeviceHandle, data: &[u8])
                   -> libusb_async::TransferFuture
{
    let mut trans = dev.alloc_transfer(0).unwrap();
    trans.fill_control_write(0x21, 0x09, 0x0203, 0, data);
    trans.submit()
}

fn read_hid_report(dev: &DeviceHandle)
                   -> libusb_async::TransferFuture
{
    let mut trans = dev.alloc_transfer(0).unwrap();
    trans.fill_interrupt_read(0x81,128);
    trans.submit()
}

fn setup_usb() -> Result<DeviceHandle, DriverError>
{
    let usb_ctxt = Context::new()?;
    // Print out information about all connected devices
    let mut device: Option<DeviceHandle> = None;
    for dev in usb_ctxt.devices()?.iter() {
        //println!("{:#?}", info);
        let dev_descr = dev.device_descriptor()?;
        let product_id = dev_descr.product_id();
        let vendor_id = dev_descr.vendor_id();
        //let serial_idx = dev_descr.serial_number_string_index();
        
        match  (product_id, vendor_id) {
    	    (0x0510, 0x16eb) => {
                println!("Device: {:04x}:{:04x}", 
                         vendor_id, product_id);         
		match dev.open() {
		    Ok(d) => {
			device = Some(d);
		    }
		    
		    Err(e) => {
                        println!("Failed to open device: {}",e);
                        return Err(DriverError::OpenFailed);
                    }
                            
		}
		break
	    },
	    _ => {}
        }	  	   
    }
    let mut device = match device {
        Some(d) => d,
        None => {
            //println!("No device found");
            return Err(DriverError::NoInterfaceFound);
        }
    };
    if device.kernel_driver_active(0).unwrap_or(false) {
        device.detach_kernel_driver(0)?;
    }
    device.claim_interface(0)?;
    Ok(device)
}

async fn driver_engine(device: DeviceHandle, rx: &mut mpsc::Receiver<DALIreq>, 
                       monitor: Arc<Mutex<Option<mpsc::Sender<DaliBusEvent>>>>) 
                 -> Result<(),DriverError> {
    
    let send = [2, 0x82, 0x04];
    match send_hid_report(&device, &send).await {
        Ok(_) => {println!("Sent {} bytes", send.len());},
        Err(e) => {
            println!("Failed to send {} bytes: {}", send.len(), e);
        }
    };

    let mut read_reply = read_hid_report(&device);
    let mut pending_req: Option<DALIreq> = None;
    let mut write_cmd = IdleFuture::new();
    
    loop {
        let mut recv = Box::pin(rx.recv());
        tokio::select! {
            Ok(r) = &mut read_reply => {
                let buf = r.get_buffer();
                let buf_len =  buf.len();
		//println!("Received len: {}",buf_len);
                if buf_len > 3 {
                    let res = 
                        match buf[1] {
                            0x6d => Some(DaliSendResult::Answer(buf[2])),
                            0x6c => Some(DaliSendResult::Framing),
                            0x64 => Some(DaliSendResult::OK),
                            0x6b => Some(DaliSendResult::Timeout),
                            _ => None
                        };
                    if let Some(res) = res {
                        if let Some(req) =  pending_req.take() {
                            match req.reply.send(res) {
                                // Ignore any errors 
                                _ => {}
                            }
                        }
                    } else {
                        let res = 
                            match buf[1] {
                                0x50 | 0x54 if buf_len >= 4 => {
                                    Some(DaliBusEvent{
                                        timestamp: Instant::now().into_std(),
                                        event: DaliBusEventType::Recv16bitFrame(buf[2..4].try_into().unwrap())})
                                        
                                },
                                0x65 | 0x66 => {
                                    Some(DaliBusEvent{
                                        timestamp: Instant::now().into_std(),
                                        event: DaliBusEventType::Recv8bitFrame(buf[2])}) 
                                },
                                0x30 if buf_len >= 5 => {
                                    Some(DaliBusEvent{
                                        timestamp: Instant::now().into_std(),
                                        event: DaliBusEventType::Recv24bitFrame(buf[2..5].try_into().unwrap())})
                                },
                                _ => None
                            };
                        if let Some(res) = res {
                            if let Some(ref mut monitor) 
                                = monitor.lock().unwrap().as_mut() 
                            {
                                match monitor.try_send(res) {
                                    Ok(_) =>{},
                                    Err(e) => {
                                        println!("Failed to send event: {}",e)
                                    }
                                }
                            }
                        } else {
                            println!("{}", buf[1..usize::from(buf[0])+1].iter().map(|x| format!("{:02x}", x)).collect::<Vec<String>>().join(" "));                            
                        }
                    }
                }
                read_reply = read_hid_report(&device);
            },
            req = &mut recv, if pending_req.is_none() => {
                match req {
                    Some(req) => {
                        println!("Got cmd: {:02x} {:02x}", 
                                 req.cmd.data[0], req.cmd.data[1]);
                        let mut cmd = 0x50;
                        if (req.cmd.flags & driver::SEND_TWICE) != 0 {
                            cmd |= 0x80;
                        }
                        if (req.cmd.flags & driver::EXPECT_ANSWER) != 0 {
                            cmd |= 0x04;
                        }
                        
                        let send = [0x3, cmd,req.cmd.data[0], req.cmd.data[1]];
                        write_cmd.set(send_hid_report(&device, &send));
                        pending_req = Some(req);
                        
                    }, 
                    None => {
                        println!("No clients");
                        break
                    }
                }
            },
            res = &mut write_cmd => {
                match res {
                    Ok(_) => {},
                    Err(e) => println!("Send Failed {}", e)
                }
                write_cmd.idle();
            }
        }
    }
    
    return Ok(());

}
    
async fn driver_thread(device: DeviceHandle, mut rx: mpsc::Receiver<DALIreq>,
                       monitor: Arc<Mutex<Option<mpsc::Sender<DaliBusEvent>>>>)
                       -> DriverError
{
    let res = driver_engine(device, &mut rx, monitor).await.map_or_else(|e| e, |_r| DriverError::OK);
    loop {
        match rx.recv().await {
            Some(req) => {
                match req.reply.send(
                    DaliSendResult::DriverError(Box::new(res.clone()))) {
                    _ => {} // Ignore any errors
                }
            },
            None => break
        }
    }
    println!("Driver stopped");
    res
}
   

impl Helvar510driver
{
    fn new() -> Result<Helvar510driver, DriverError> {
        let (tx, rx) = mpsc::channel::<DALIreq>(10);
        let monitor = Arc::new(Mutex::new(None));
	let device = match setup_usb() {
	    Ok(d) => d,
	    Err(_e) => return Err(DriverError::UsbError)
	};

        let join = tokio::spawn(driver_thread(device, rx, monitor.clone()));
        let driver = Helvar510driver{send_cmd: Some(tx),
				     join: Some(join),
				     send_monitor: monitor};
	Ok(driver)
    }

    pub async fn stop(&mut self) {
        self.send_cmd = None;  // Tell the receiving task to stop
        match self.join.take() {
            Some(join) => {
                join.await.expect("Failed to join driver thread");
            },
            None => {}
        }
    }
}

                          

impl DaliDriver for Helvar510driver
{
    fn send_frame(&mut self, cmd: &[u8;4], flags:u16) -> 
        Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
    {
        let (tx, rx) = oneshot::channel();
        let req = DALIreq{cmd: DALIcmd 
                          {
                              data: cmd.clone(),
                              flags: flags
                          },
                          reply: tx
        };

        match self.send_cmd.as_mut().unwrap().try_send(req) {
            Ok(()) => {
                Box::pin(async {
                    match rx.await {
                            Ok(r) => r,
                        Err(e) => DaliSendResult::DriverError(Box::new(e))
                    }
                    })
            },
            Err(_) => {
                Box::pin(async {
                    DaliSendResult::DriverError(
                        Box::new(DriverError::CommandError))
                })
            }
        }
    }
    
    fn next_bus_event(&mut self) -> Pin<Box<dyn Future<Output = DaliBusEvent> + Send>>
    {
	Box::pin(std::future::ready(DaliBusEvent{
	    timestamp: Instant::now().into_std(),
	    event: DaliBusEventType::NotImplemented}))
    }
}
/*
impl DALImonitor for Helvar510driver
{
    fn monitor_stream(&mut self) 
                      -> Option<Pin<Box<dyn Stream<Item = DaliBusEvent>>>>
    {
        let mut monitor = match self.send_monitor.lock() {
            Ok(m) => m,
            Err(_) => return None
        };
        if monitor.is_some() {
            return None;
        }
        let (tx, rx) = mpsc::channel::<DaliBusEvent>(10);
        *monitor = Some(tx);
        return Some(Box::pin(rx));
    }
}
*/
fn driver_open(_params: HashMap<String, String>)
		     -> Result<Box<dyn DaliDriver>, OpenError>
{
    match Helvar510driver::new() {
	Err(e) => Err(OpenError::DriverError(Box::new(e))),
	Ok(d) => Ok(Box::new(d))
    }
}

pub fn driver_info() -> DriverInfo
{
    DriverInfo{name: "Helvar510".to_string(), 
	       description: 
	       "Driver for Helvar 510 USB DALI-adapter".to_string(),
	       open: driver_open
    }
}
