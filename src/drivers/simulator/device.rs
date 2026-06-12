use crate::drivers::driver::DaliBusEventType;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub struct DaliSimBusEvent {
    pub source_id: u32,
    pub timestamp: Instant, // Time of first transition of frame
    pub duration: Option<Duration>,
    pub event_type: DaliBusEventType,
}

pub trait DaliSimHost: Send {
    fn send_event(
        &self,
        event: DaliSimBusEvent,
    ) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>;
    fn current_time(&self) -> Instant;
    fn wait_until(&self, when: Instant) -> Pin<Box<dyn Future<Output = ()>>>;
    fn real_time(&self) -> bool;
    fn next_source_id(&self) -> u32;
}

pub trait DaliSimDevice: Send {
    /// Called when the device is connected to a host
    fn start(
        &mut self,
        host: Box<dyn DaliSimHost>,
    ) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>;
    /// Called when disconnected from the host
    fn stop(&mut self) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>;
    /// A new event has been dispatched on the bus
    fn event(&mut self, event: &DaliSimBusEvent) -> Option<DaliSimBusEvent>;
}
