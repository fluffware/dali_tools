use super::driver::{DaliSendResult,
		    DaliBusEventResult,
		    DaliFrame};
use super::send_flags::Flags;

use std::pin::Pin;
use tokio::sync::oneshot;
use tokio::sync::mpsc;
use std::future::Future;

#[derive(Debug)]
pub struct DALIcmd
{
    pub data: DaliFrame,
    pub flags: Flags
}

#[derive(Debug)]
pub struct DALIreq
{
    pub cmd: DALIcmd,
    pub reply: oneshot::Sender<DaliSendResult>
}

pub fn send_frame(req_tx: &mut mpsc::Sender<DALIreq>,
		  cmd: &DaliFrame, flags:Flags) -> 
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

    match req_tx.try_send(req) {
        Ok(()) => {
            Box::pin(async {
                match rx.await {
                    Ok(r) => r,
                    Err(e) => DaliSendResult::DriverError(Box::new(e))
                }
            })
        },
        Err(_) => {
            Box::pin(std::future::ready(
                    DaliSendResult::DriverError(
                        "Failed to queue command".into())
            ))
	}
    }
}
pub fn next_bus_event(monitor_tx: &mut mpsc::Sender<oneshot::Sender<DaliBusEventResult>>)
		   -> Pin<Box<dyn Future<Output = DaliBusEventResult> + Send>>
    {
	let (tx, rx) = oneshot::channel();
	
	match monitor_tx.try_send(tx) {
            Ok(()) => {
		Box::pin(async {
                    match rx.await {
			Ok(r) => r,
			
			Err(e) => Err(e.into())
                    }
            })
            },
            Err(_) => {
		Box::pin(std::future::ready(
		    Err("Failed to queue monitor request".into())
		))
	    }
	}
    }
