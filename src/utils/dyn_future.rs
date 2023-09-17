use std::future::Future;
use std::pin::Pin;

pub type DynFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type DynFutureStatic<T> = DynFuture<'static, T>;
