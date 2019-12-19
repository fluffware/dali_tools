extern crate hidapi;
use std::sync::Arc;
use futures::lock::Mutex;
use std::pin::Pin;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::thread::JoinHandle;
use std::thread;
use std::time::Duration;
use futures::future::Future;
use futures::executor::block_on;

use hidapi::HidDevice;
use hidapi::HidDeviceInfo;
//use hidapi::HidError;

use super::super::driver::{self,DALIdriver, DALIcommandError};
use super::super::utils::{DALIreq, DALIcmd, DALIreply};
use std::error::Error;
use std::fmt;

pub struct Helvar510driver
{
    join: Option<JoinHandle<DriverError>>,
    sender: Option<mpsc::SyncSender<Arc<DALIreq>>>
}

#[derive(Debug,Clone)]
enum DriverError
{
    OK,
    OpenFailed,
    NoInterfaceFound,
    CommandError
}

impl Error for DriverError
{
}

impl fmt::Display for DriverError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No error")
    }
}

fn driver_engine(rx: &mut mpsc::Receiver<Arc<DALIreq>>) -> DriverError {
       let api = hidapi::HidApi::new().unwrap();
    // Print out information about all connected devices
    let mut device: Option<HidDevice> = None;
    for info in api.devices() {
        //println!("{:#?}", info);
       
        match  info {
    	    &HidDeviceInfo{product_id:0x0510, vendor_id: 0x16eb, 
	  		   interface_number: 0, ..} => {
                println!("Device: {:04x}:{:04x} Interface: {} Usage: {}/{}", info.vendor_id, info.product_id, info.interface_number, info.usage_page, info.usage);         
		match info.open_device(&api) {
		    Ok(d) => {
			device = Some(d);
		    }
		    
		    Err(e) => {
                        println!("Failed to open HID device: {}",e);
                        return DriverError::OpenFailed;
                    }
                            
		}
		break
	    },
	    _ => {}
        }	  	   
    }
    let device = match device {
        Some(d) => d,
        None => {
            println!("No device found");
            return DriverError::NoInterfaceFound;
        }
    };
    device.set_blocking_mode(true).unwrap();
    
    let send = [2, 0x82, 0x04];
    match device.write(&send) {
        Ok(_s) => {/*println!("Sent {} bytes", send.len())*/},
        Err(e) => println!("Failed to send {} bytes: {}", send.len(), e)
    };
    let mut buf = [0u8; 64];
    loop {
        let res = device.read_timeout(&mut buf[..],1000).unwrap();
        if res > 0 {
            break
        }
    }
    loop {
        match rx.try_recv() {
            Ok(ref req) => {
                let mut cmd = 0x50;
                if (req.cmd.flags & driver::SEND_TWICE) != 0 {
                    cmd |= 0x80;
                }
                if (req.cmd.flags & driver::EXPECT_ANSWER) != 0 {
                    cmd |= 0x04;
                }
                
                let send = [0x3, cmd,req.cmd.data[0], req.cmd.data[1]];
                match device.write(&send) {
                    Ok(_s) => {
                        // println!("Sent {} bytes", send.len());
                    },
                    Err(e) => println!("Failed to send {} bytes: {}", send.len(), e)
                };
                let mut buf = [0u8; 64];
                loop {
                    let res = device.read_timeout(&mut buf[..],1000).unwrap();
                    //println!("Read done: {}",res);
                    if res > 0 {
                        if res == 0x24 {
                            //let msglen:usize = buf[0] as usize;
                            //println!("Read msg: {}", &buf[1..(msglen+1)].iter().map(|x| format!("{:02x}",x)).collect::<Vec<String>>().join(" "));
                            let mut reply = block_on(req.reply.lock());
                            match (&buf[0..2], 
                                   (req.cmd.flags & driver::EXPECT_ANSWER) != 0) {
                                (&[1,0x64], false) => {
                                    reply.err = DALIcommandError::OK;  
                                },
                                (&[2,0x6d], true) => {
                                    reply.err = DALIcommandError::OK;
                                    reply.data[0] = buf[2];
                                },
                                (&[1,0x6c], true) => {
                                    reply.err = DALIcommandError::Framing;  
                                },
                                (&[1, 0x6b], true) => {
                                    reply.err = DALIcommandError::Timeout;
                                },
                                _ => {
                                    let err = Arc::new(DriverError::CommandError);
                                    reply.err = DALIcommandError::DriverError(err);
                                }
                            };
                            let mut waker = block_on(req.waker.lock());
                            match waker.take() {
                                Some(w) => w.wake(),
                                _ => {}
                            }
                            break
                        }
                    }
                }
            }
            Err(TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(100));
            },
            Err(TryRecvError::Disconnected) => break
        };
        
    };
    return DriverError::OK;

}
    
fn driver_thread(mut rx: mpsc::Receiver<Arc<DALIreq>>) -> DriverError
{
    
    let res = driver_engine(&mut rx);
    loop {
        match rx.try_recv() {
            Ok(ref req) => {
                let mut reply = block_on(req.reply.lock());
                reply.err = DALIcommandError::DriverError(Arc::new(res.clone()));
                let mut waker = block_on(req.waker.lock());
                match waker.take() {
                    Some(w) => {w.wake();},
                    _ => {}
                }
            },
            _ => break
        }
    }
    res
}
   

impl Helvar510driver
{
    pub fn new() -> Helvar510driver {
        let (tx, rx) = mpsc::sync_channel::<Arc<DALIreq>>(10);
        let join = thread::spawn(|| {
            driver_thread(rx)
        });
        Helvar510driver{sender: Some(tx),join: Some(join)}
    }
}

impl Drop for Helvar510driver
{
    fn drop(&mut self)
    {
        self.sender = None;
        match self.join.take() {
            Some(join) => {
                join.join().expect("Failed to join driver thread");
            },
            None => {}
        }
    }
}

struct ResultFuture
{
    req: Arc<DALIreq>
}

impl ResultFuture
{
    fn new(req: Arc<DALIreq>) -> ResultFuture {
        ResultFuture{req: req}
    }
}

impl Future for ResultFuture
{
    type Output = Result<u8, DALIcommandError>;
    fn poll(self: Pin<&mut Self>, cx: &mut futures::task::Context)
            ->futures::task::Poll<Self::Output>
    {

        let mut waker = block_on(self.req.waker.lock());
        *waker = Some(cx.waker().clone());
        let reply = match self.req.reply.try_lock() {
            Some(r) => r,
            None => return futures::task::Poll::Pending
        };
        match &reply.err {
            DALIcommandError::Pending => futures::task::Poll::Pending,
            DALIcommandError::OK => {
                futures::task::Poll::Ready(Ok(reply.data[0]))
            },
            err => {
                futures::task::Poll::Ready(Err(err.clone()))
            }
        }
    }
}
                          

impl DALIdriver for Helvar510driver
{
    fn send_command(&mut self, cmd: &[u8;2], flags:u16) -> 
        Pin<Box<dyn Future<Output = Result<u8, DALIcommandError>> + Send>>
    {
        let req = DALIreq{cmd: DALIcmd 
                          {
                              data: [cmd[0], cmd[1], 0],
                              flags: flags
                          },
                          reply:
                          Arc::new(Mutex::new(DALIreply {
                              data: [0,0,0],
                              err: DALIcommandError::Pending
                          })),
                          waker: Arc::new(Mutex::new(None))
        };
        let req_ref = Arc::new(req);
        
        self.sender.as_ref().unwrap().send(req_ref.clone()).expect("Failed to send DALI request");
        Box::pin(ResultFuture::new(req_ref))
        
    }
}
