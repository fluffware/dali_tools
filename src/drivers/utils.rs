use super::driver::{DaliSendResult};
use std::pin::Pin;
use tokio::sync::oneshot;
use tokio::sync::mpsc;
use std::future::Future;

#[derive(Debug)]
pub struct DALIcmd
{
    pub data: [u8;4],
    pub flags: u16,
}

#[derive(Debug)]
pub struct DALIreq
{
    pub cmd: DALIcmd,
    pub reply: oneshot::Sender<DaliSendResult>
}

pub fn send_frame(req_tx: &mut mpsc::Sender<DALIreq>, cmd: &[u8;4], flags:u16) -> 
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
