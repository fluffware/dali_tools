use std::pin::Pin;
use futures::future::Future;


pub const PRIORITY_1:u16 = 0x00;
pub const PRIORITY_2:u16 = 0x01;
pub const PRIORITY_3:u16 = 0x02;
pub const PRIORITY_4:u16 = 0x03;
    
pub const SEND_TWICE:u16 = 0x04;
pub const EXPECT_ANSWER:u16 = 0x08; // Expect an answer

#[derive(Copy,Clone,Debug)]
pub enum DALIcommandError
{
    OK,
    Timeout,
    Framing,
    DriverError(u32),
    Pending
}


pub trait DALIdriver
{
    fn send_command(&mut self, cmd: &[u8;2], flags:u16) -> 
        Pin<Box<dyn Future<Output = Result<u8, DALIcommandError>> +Unpin>>;
}
