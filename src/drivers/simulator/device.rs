use crate::drivers::driver::DaliBusEventType;
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

type DynResult<T> =  Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct DaliSimEvent
{
    pub source_id: u32,
    pub timestamp: Instant, // Time of first transition of frame
    pub event_type: DaliBusEventType
}

pub trait DaliSimHost: Send
{
    fn send_event(&mut self, event: DaliSimEvent) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>;
    fn current_time(&self) -> Instant;
    fn real_time(&self) -> bool;
    fn next_source_id(&mut self) -> u32;
    fn clone_box(&self) -> Box<dyn DaliSimHost>;
}

pub trait DaliSimDevice: Send
{
    /// Called when the device is connected to a host
    fn start(&mut self, host: Box<dyn DaliSimHost>)
		-> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>;
    /// Called when disconnected from the host
    fn stop(&mut self) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>;
    /// A new event has been dispatched on the bus
    fn event(&mut self,event: &DaliSimEvent) ->Option<DaliSimEvent>;
}
