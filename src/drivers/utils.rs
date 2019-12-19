use futures::lock::Mutex;
use std::sync::Arc;
use super::driver::{DALIcommandError};

pub struct DALIcmd
{
    pub data: [u8;3],
    pub flags: u16,
}

pub struct DALIreply
{
    pub data: [u8;3],
    pub err: DALIcommandError,
}

pub struct DALIreq
{
    pub cmd: DALIcmd,
    pub reply: Arc<Mutex<DALIreply>>,
    pub waker: Arc<Mutex<Option<futures::task::Waker>>>
}
