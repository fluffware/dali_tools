pub type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub type DynResultFuture<T> =
    std::pin::Pin<Box<dyn std::future::Future<Output = DynResult<T>> + Send>>;
