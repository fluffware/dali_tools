use std::pin::Pin;
use core::future::Future;
use std::error::Error;
use std::sync::Arc;
use std::fmt;
use std::ops::Deref;
use crate::base::address::BusAddress;

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


pub trait DALIdriver: Send
{
    /// Send raw DALI commands
    ///
    /// # Arguments
    /// * `cmd` - Bytes of command
    /// * `flags` - Options for transaction
    
    fn send_command(&mut self, cmd: &[u8;2], flags:u16) -> 
        Pin<Box<dyn Future<Output = Result<u8, DALIcommandError>> + Send>>;

    /// Send addressed DALI commands
    ///
    /// # Arguments
    /// * `addr` - Destination address of command 
    /// * `cmd` - Second byte of command
    /// * `flags` - Options for transaction
    fn send_device_cmd(&mut self, addr: &dyn BusAddress, cmd: u8, flags:u16) -> 
        Pin<Box<dyn Future<Output = Result<u8, DALIcommandError>> + Send>>
    {
        self.send_command(&[addr.bus_address() | 1, cmd], flags)
    }
    
    /// Send DALI DAPC commands
    ///
    /// # Arguments
    /// * `addr` - Address of device(s) 
    /// * `level` - Intensity level
    /// * `flags` - Options for transaction
    
    fn send_device_level(&mut self, addr: &dyn BusAddress, level: u8,
                         flags:u16) ->
        Pin<Box<dyn Future<Output = Result<u8, DALIcommandError>> + Send>>
    {
          self.send_command(&[addr.bus_address(), level], flags)
    }
}

pub const YES: Result<u8, DALIcommandError> = Ok(0xff);
pub const NO: Result<u8, DALIcommandError> = Err(DALIcommandError::Timeout);
