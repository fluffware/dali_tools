use super::driver::{DALIcommandError};
use tokio::sync::oneshot;


pub struct DALIcmd
{
    pub data: [u8;3],
    pub flags: u16,
}

pub struct DALIreq
{
    pub cmd: DALIcmd,
    pub reply: oneshot::Sender<Result<u8, DALIcommandError>>
}
