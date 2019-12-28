use std::sync::Mutex;
use std::sync::Arc;
use futures::future::Future;
use std::pin::Pin;
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

pub struct DALIResultFuture
{
    req: Arc<DALIreq>
}

impl DALIResultFuture
{
    pub fn new(req: Arc<DALIreq>) -> DALIResultFuture {
        DALIResultFuture{req: req}
    }
}

impl Future for DALIResultFuture
{
    type Output = Result<u8, DALIcommandError>;
    fn poll(self: Pin<&mut Self>, cx: &mut futures::task::Context)
            ->futures::task::Poll<Self::Output>
    {

        let mut waker = self.req.waker.lock().unwrap();
        *waker = Some(cx.waker().clone());

        let reply = self.req.reply.lock().unwrap();
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
