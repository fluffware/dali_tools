use super::driver::{DaliSendResult};
use tokio::sync::oneshot;


pub struct DALIcmd
{
    pub data: [u8;4],
    pub flags: u16,
}

pub struct DALIreq
{
    pub cmd: DALIcmd,
    pub reply: oneshot::Sender<DaliSendResult>
}
