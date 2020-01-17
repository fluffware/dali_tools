use super::super::driver::{self, DALIdriver, DALIcommandError};
use super::super::utils::{DALIreq, DALIcmd, DALIreply,DALIResultFuture};
use futures::future::{Future};
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::thread::JoinHandle;
use std::error::Error;
use std::fmt;
use super::device::DALIsimDevice;

#[derive(Debug,Clone)]
enum SimDriverError
{
    OK
}

impl Error for SimDriverError
{
}

impl fmt::Display for SimDriverError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "No error")
    }
}

struct DALIsimCtxt
{
    devices: Vec<Box<dyn DALIsimDevice + Send>>
}

pub struct DALIsim
{
    ctxt: Arc<Mutex<DALIsimCtxt>>,
    join: Option<JoinHandle<SimDriverError>>,
    sender: Option<mpsc::SyncSender<Arc<DALIreq>>>
}


impl DALIsim
{
    pub fn new() -> DALIsim
    { 
        let (tx, rx) = mpsc::sync_channel::<Arc<DALIreq>>(10);
        let ctxt = Arc::new(Mutex::new(DALIsimCtxt{
            devices: Vec::new()
        }));
        let thread_ctxt = ctxt.clone();
        let join = thread::spawn(|| {
            sim_thread(rx, thread_ctxt)

        });

        DALIsim{ctxt: ctxt,
                sender: Some(tx),join: Some(join)}
    }

    pub fn add_device(&self, dev: Box<dyn DALIsimDevice + Send>)
    {
        let mut ctxt = self.ctxt.lock().unwrap();

        ctxt.devices.push(dev);
    }
}

impl Drop for DALIsim
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

fn sim_engine(rx: &mut mpsc::Receiver<Arc<DALIreq>>, 
              ctxt: Arc<Mutex<DALIsimCtxt>>) -> SimDriverError
{
    loop {
        match rx.recv() {
            Ok(ref req) => {
                {
                    let mut ctxt = ctxt.lock().unwrap();
                    // Return Timeout if no device returns a reply
                    let mut status = Err(DALIcommandError::Timeout);
                    for d in &mut ctxt.devices {
                        match d.forward16(&req.cmd.data[0..2], req.cmd.flags) {
                            Ok(r) => {
                                status = match status {
                                    Err(DALIcommandError::Timeout) =>
                                        Ok(r),
                                    Ok(_) | Err(DALIcommandError::Framing) =>
                                        Err(DALIcommandError::Framing),
                                    _ => status
                                };
                            },
                            Err(DALIcommandError::Timeout) => {},
                            Err(DALIcommandError::Framing) => {
                                status = Err(DALIcommandError::Framing);
                            },
                            e => status = e
                        }
                                
                    }
                    let mut reply = req.reply.lock().unwrap();
                    match status {
                        Ok(r) => {
                            reply.data[0] = r;
                            reply.err = DALIcommandError::OK;
                        },
                        Err(DALIcommandError::Timeout) 
                            if req.cmd.flags & driver::EXPECT_ANSWER == 0 => {
                                reply.data[0] = 0;
                                reply.err = DALIcommandError::OK;
                        },
                        Err(e) => {reply.err = e;}
                    }
                }
                let mut waker = req.waker.lock().unwrap();
                match waker.take() {
                    Some(w) =>  {
                        w.wake();
                    },
                    _ => {}
                }
            },
            Err(_) => {
                break
            }
        };
        
    };
    return SimDriverError::OK;
}

fn sim_thread(mut rx: mpsc::Receiver<Arc<DALIreq>>,
              ctxt: Arc<Mutex<DALIsimCtxt>>) -> SimDriverError
{
    let res = sim_engine(&mut rx, ctxt);
    loop {
        match rx.try_recv() {
            Ok(ref req) => {
                let mut reply = req.reply.lock().unwrap();
                reply.err = DALIcommandError::DriverError(Arc::new(res.clone()));
                let mut waker = req.waker.lock().unwrap();
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


impl DALIdriver for DALIsim
{
    fn send_command(&mut self, cmd: &[u8;2], flags:u16) -> 
        Pin<Box<dyn Future<Output = Result<u8, DALIcommandError>> + Send>>
    {
        let req = DALIreq{cmd: DALIcmd 
                          {
                              data: [cmd[0], cmd[1], 0],
                              flags: flags
                          },
                          reply: Arc::new(Mutex::new(DALIreply {
                              data: [0,0,0],
                              err: DALIcommandError::Pending
                          })),
                          waker: Arc::new(Mutex::new(None))
        };
        let req_ref = Arc::new(req);
        
        self.sender.as_ref().unwrap().send(req_ref.clone()).expect("Failed to send DALI request");
        Box::pin(DALIResultFuture::new(req_ref))
        
    }
}

