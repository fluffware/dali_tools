use tokio::stream::Stream;
use std::pin::Pin;
use tokio::time::Instant;

#[derive(Debug)]
pub enum DaliBusEventType
{
    Recv8bitFrame(u8),
    Recv16bitFrame([u8;2]),
    Recv24bitFrame([u8;3]),
    RecvFramingError,
    BusPowerOff,
    BusPowerOn
}

#[derive(Debug)]
pub struct DaliBusEvent
{
    pub timestamp: Instant,
    pub event: DaliBusEventType
}
    
pub trait DALImonitor: Send {
    fn monitor_stream(&mut self) -> Option<Pin<Box<dyn Stream<Item = DaliBusEvent>>>>;
}
