use crate::drivers::simulator::sim_bus::DaliSimBusDevice;
use linkme::distributed_slice;
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};
use yaml_serde::value::Mapping;

type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct DaliSimDeviceEntry {
    pub name: &'static str,
    pub init: fn() -> Box<dyn DaliSimDevice>,
}
#[distributed_slice]
pub static DALI_SIMULATOR_DEVICES: [DaliSimDeviceEntry];

pub trait DaliSimDevice: Send {
    fn configure(&mut self, conf: &Mapping) -> DynResult<()>;
    /// Called when the device is connected to a bus
    fn start(&mut self, bus_device: DaliSimBusDevice) -> DynResult<()>;
    /// Called when disconnected from the host
    fn stop(&mut self) -> DynResult<()>;
}
