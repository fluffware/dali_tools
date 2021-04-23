
use std::future::Future;
use std::task::Poll;
use std::task::Context;
use std::pin::Pin;

pub struct IdleFuture<T,O>
    where T: Future<Output = O>
{
    future: Option<T>
}

impl<T,O> IdleFuture<T,O>
    where T: Future<Output = O>
{
    pub fn new() -> IdleFuture<T,O>
    {
        IdleFuture{future: None}
    }

    pub fn set(&mut self, future: T) {
        self.future = Some(future);
    }

    pub fn idle(&mut self) {
        self.future = None;
    }
}

impl<T,O> Future for IdleFuture<T,O>
    where T: Future<Output = O> + Unpin
{
    type Output = O;
    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output>
    {
        
        if let Some(ref mut future) = self.get_mut().future {
            T::poll(Pin::new(future), cx)
        } else {
            Poll::Pending
        }
    }
}
