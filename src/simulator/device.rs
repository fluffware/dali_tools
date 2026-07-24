use crate::simulator::sim_bus::DaliSimBusDevice;
use linkme::distributed_slice;
use yaml_serde::value::Mapping;

type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct DaliSimDeviceEntry {
    pub name: &'static str,
    pub init: fn() -> Box<dyn DaliSimDevice>,
}
#[distributed_slice]
pub static DALI_SIMULATOR_DEVICES: [DaliSimDeviceEntry];

#[derive(Debug)]
pub enum ParameterError {
    NotFound,
    InvalidValue,
}

impl std::error::Error for ParameterError {}

impl std::fmt::Display for ParameterError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::NotFound => f.write_str("Parameter not found"),
            Self::InvalidValue => f.write_str("Invalid value for parameter"),
        }
    }
}

pub trait DaliSimDevice: Send {
    /* If index > 0 then a single configuration is repeated for more than one
    device. Adjust parameters (e.g. shorAddress) accordingly.
    */
    fn configure(&mut self, conf: &Mapping, index: usize) -> DynResult<()>;
    /// Called when the device is connected to a bus
    fn start(&mut self, bus_device: DaliSimBusDevice) -> DynResult<()>;
    /// Called when disconnected from the bus
    fn stop(&mut self) -> DynResult<()>;
    // Get a named device parameter as a JSON string
    fn get_parameter(&self, name: &str) -> Result<String, ParameterError>;
    // Set a named device parameter from a JSON string
    fn set_parameter(&self, name: &str, value: &str) -> Result<(), ParameterError>;
}
