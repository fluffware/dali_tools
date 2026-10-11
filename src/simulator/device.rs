use crate::simulator::sim_bus::DaliSimBusDevice;
use crate::utils::device_configuration::ConfigureDevice;
use linkme::distributed_slice;

type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub struct DaliSimDeviceEntry {
    pub name: &'static str,
    pub init: fn(name: String) -> Box<dyn DaliSimDevice + Sync + Send>,
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

pub trait DaliSimDevice: ConfigureDevice + Send {
    /// Parent class instance
    fn parent(&self) -> &dyn DaliSimDevice;
    fn parent_mut(&mut self) -> &mut dyn DaliSimDevice;

    /// Called when the device is connected to a bus
    fn start(&mut self, bus_device: DaliSimBusDevice) -> DynResult<()> {
        self.parent_mut().start(bus_device)
    }
    /// Called when disconnected from the bus
    fn stop(&mut self) -> DynResult<()> {
        self.parent_mut().stop()
    }
    // Get a named device parameter as a JSON string
    fn get_parameter(&self, name: &str) -> Result<String, ParameterError> {
        self.parent().get_parameter(name)
    }
    // Set a named device parameter from a JSON string
    fn set_parameter(&self, name: &str, value: &str) -> Result<(), ParameterError> {
        self.parent().set_parameter(name, value)
    }
}
