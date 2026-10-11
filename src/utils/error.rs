pub type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
