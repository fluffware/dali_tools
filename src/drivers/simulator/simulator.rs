use super::super::driver::{self,DALIdriver, DALIcommandError};
use super::super::utils::{DALIreq, DALIcmd};
use core::future::{Future};
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use std::error::Error;
use std::fmt;
use super::device::DALIsimDevice;

#[derive(Debug,Clone)]
pub enum SimDriverError
{
    OK,
    QueuingFailed,
    ReplyingFailed,
    ThreadError
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
    join: Option<JoinHandle<Result<(),SimDriverError>>>,
    sender: Option<mpsc::Sender<DALIreq>>
}


impl DALIsim
{
    pub fn new() -> DALIsim
    { 
        let (tx, rx) = mpsc::channel::<DALIreq>(10);
        let ctxt = Arc::new(Mutex::new(DALIsimCtxt{
            devices: Vec::new()
        }));
        let thread_ctxt = ctxt.clone();
        let join = tokio::spawn(sim_thread(rx, thread_ctxt));

        

        DALIsim{ctxt: ctxt,
                sender: Some(tx),join: Some(join)}
    }

    pub fn add_device(&self, dev: Box<dyn DALIsimDevice + Send>)
    {
        let mut ctxt = self.ctxt.lock().unwrap();

        ctxt.devices.push(dev);
    }

    pub async fn stop(&mut self) -> Result<(), SimDriverError> {
        self.sender = None;
        match self.join.take() {
            Some(join) => {
                join.await.map_err(|_e| SimDriverError::ThreadError)??;
            },
            None => {}
        }
        Ok(())
    }
}

async fn sim_engine(rx: &mut mpsc::Receiver<DALIreq>, 
              ctxt: Arc<Mutex<DALIsimCtxt>>) -> Result<(),SimDriverError>
{
    loop {
        match rx.recv().await {
            Some(req) => {
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
                    match status {
                        Ok(r) => {
                            req.reply.send(Ok(r))
                                .map_err(|_e| SimDriverError::ReplyingFailed)?;
                        },
                        Err(DALIcommandError::Timeout) 
                            if req.cmd.flags & driver::EXPECT_ANSWER == 0 => {
                                req.reply.send(Ok(0))
                                    .map_err(|_e| SimDriverError::ReplyingFailed)?;
                        },
                        Err(e) => {req.reply.send(Err(e))
                                   .map_err(|_e| SimDriverError::ReplyingFailed)?}
                    }
                }
            },
            None => {
                break
            }
        };
        
    };
    return Ok(());
}

async fn sim_thread(mut rx: mpsc::Receiver<DALIreq>,
              ctxt: Arc<Mutex<DALIsimCtxt>>) -> Result<(),SimDriverError>
{
    let res = sim_engine(&mut rx, ctxt).await;
    if let Err(err) = &res {
        loop {
            match rx.try_recv() {
            Ok(req) => {
                req.reply.send(Err(DALIcommandError::DriverError(
                    Arc::new(err.clone()))))
                    .map_err(|_e| SimDriverError::ReplyingFailed)?;
            },
                _ => break
            }
        }
    }
    res    
}


impl DALIdriver for DALIsim
{
    fn send_command(&mut self, cmd: &[u8;2], flags:u16) -> 
        Pin<Box<dyn Future<Output = Result<u8, DALIcommandError>> + Send>>
    {
        let (tx, rx) = oneshot::channel();
        let req = DALIreq{cmd: DALIcmd 
                          {
                              data: [cmd[0], cmd[1], 0],
                              flags: flags
                          },
                          reply: tx
        };
        match self.sender.as_mut().unwrap().try_send(req) {
            Ok(()) => {
                Box::pin(async {
                    match rx.await {
                            Ok(r) => r,
                        Err(e) => Err(DALIcommandError::DriverError(Arc::new(e)))
                    }
                    })
            },
            Err(_) => {
                Box::pin(async {
                    Err(DALIcommandError::DriverError(
                        Arc::new(SimDriverError::QueuingFailed)))
                })
            }
        }
        
    }
}

