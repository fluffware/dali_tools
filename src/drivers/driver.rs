use std::pin::Pin;
use futures::future::Future;
use std::error::Error;
use std::sync::Arc;
use std::fmt;
use std::ops::Deref;

pub const PRIORITY_1:u16 = 0x00;
pub const PRIORITY_2:u16 = 0x01;
pub const PRIORITY_3:u16 = 0x02;
pub const PRIORITY_4:u16 = 0x03;
    
pub const SEND_TWICE:u16 = 0x04;
pub const EXPECT_ANSWER:u16 = 0x08; // Expect an answer

#[derive(Debug,Clone)]
pub enum DALIcommandError
{
    OK,
    Timeout,
    Framing,
    DriverError(Arc<dyn Error + Sync + Send + 'static>),
    Pending
}

impl Error for DALIcommandError
{
    fn source(&self) -> Option<&(dyn Error + 'static)>
    {
        match self {
            DALIcommandError::DriverError(x) => Some(x.deref()),
            _ => None
        }
    }
}

impl fmt::Display for DALIcommandError
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DALIcommandError::OK => write!(f, "No error"),
            DALIcommandError::Timeout => write!(f, "Command timed out"),
            DALIcommandError::Framing => write!(f, "Invalid framing"),
            DALIcommandError::DriverError(e) =>write!(f, "Drive error {}", e.to_string()),
            DALIcommandError::Pending => write!(f,"Pending")
        }
    }
}


pub trait DALIdriver
{
    fn send_command(&mut self, cmd: &[u8;2], flags:u16) -> 
        Pin<Box<dyn Future<Output = Result<u8, DALIcommandError>> + Send>>;
}
