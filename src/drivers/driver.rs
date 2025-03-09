use super::send_flags::Flags;
use crate::utils::dyn_future::DynFuture;
use core::convert::TryFrom;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug)]
pub enum DaliSendResult {
    Ok,         // Frame sent without errors
    Answer(u8), // Recieved an answer
    Timeout,    // An answer didn't arrive in time
    Framing,
    DriverError(Box<dyn Error + Sync + Send + 'static>),
    Pending,
}

impl fmt::Display for DaliSendResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DaliSendResult::Ok => write!(f, "No error"),
            DaliSendResult::Answer(r) => write!(f, "Answer: 0x{:02x}", r),
            DaliSendResult::Timeout => write!(f, "Command timed out"),
            DaliSendResult::Framing => write!(f, "Invalid framing"),
            DaliSendResult::DriverError(e) => write!(f, "Drive error: {}", e.to_string()),
            DaliSendResult::Pending => write!(f, "Pending"),
        }
    }
}

impl Error for DaliSendResult {}
impl DaliSendResult {
    pub fn check_send(self) -> Result<(), DaliSendResult> {
        match self {
            DaliSendResult::Ok => Ok(()),
            e => Err(e),
        }
    }

    pub fn check_answer(self) -> Result<u8, DaliSendResult> {
        match self {
            DaliSendResult::Answer(r) => Ok(r),
            e => Err(e),
        }
    }
 
}

#[derive(Debug)]
pub struct FromDaliBusEventTypeError {}
impl Error for FromDaliBusEventTypeError {}
impl fmt::Display for FromDaliBusEventTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Only events containing frames can be converted")
    }
}

#[derive(Debug, Clone)]
pub enum DaliFrame {
    Frame8(u8),
    Frame16([u8; 2]),
    Frame24([u8; 3]),
    Frame25([u8; 4]),
}

impl DaliFrame {
    pub fn bit_length(&self) -> u32 {
        use DaliFrame::*;
        match self {
            Frame8(_) => 8,
            Frame16(_) => 16,
            Frame24(_) => 24,
            Frame25(_) => 25,
        }
    }
}

impl TryFrom<&DaliBusEventType> for DaliFrame {
    type Error = FromDaliBusEventTypeError;
    fn try_from(t: &DaliBusEventType) -> Result<Self, Self::Error> {
        match t {
            DaliBusEventType::Frame8(f) => Ok(Self::Frame8(*f)),
            DaliBusEventType::Frame16(f) => Ok(Self::Frame16(*f)),
            DaliBusEventType::Frame24(f) => Ok(Self::Frame24(*f)),
            DaliBusEventType::Frame25(f) => Ok(Self::Frame25(*f)),
            _ => Err(FromDaliBusEventTypeError {}),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DaliBusEventType {
    Frame8(u8),
    Frame16([u8; 2]),
    Frame24([u8; 3]),
    Frame25([u8; 4]),
    FramingError,
    BusPowerOff,
    BusPowerOn,
    Overrun, // The previous event wasn't read before the next one arrived
}

impl From<DaliFrame> for DaliBusEventType {
    fn from(t: DaliFrame) -> Self {
        match t {
            DaliFrame::Frame8(f) => Self::Frame8(f),
            DaliFrame::Frame16(f) => Self::Frame16(f),
            DaliFrame::Frame24(f) => Self::Frame24(f),
            DaliFrame::Frame25(f) => Self::Frame25(f),
        }
    }
}

#[derive(Debug)]
pub struct DaliBusEvent {
    // For reception this is the time when the frame was accepted. This is the time of the last
    // transition + 2.4ms for stop condition.
    pub timestamp: Instant,
    pub event_type: DaliBusEventType,
}

pub type DaliBusEventResult = Result<DaliBusEvent, Box<dyn Error + Sync + Send>>;
pub trait DaliDriver: Send {
    /// Send a raw DALI frame
    ///
    /// # Arguments
    /// * `cmd` - Bytes of command
    /// * `flags` - Options for transaction

    fn send_frame(&mut self, cmd: DaliFrame, flags: Flags) -> DynFuture<DaliSendResult>;

    fn next_bus_event(&mut self) -> DynFuture<DaliBusEventResult>;

    fn current_timestamp(&self) -> Instant;

    fn wait_until(&self, end: Instant) -> DynFuture<()>;
}

pub const YES: DaliSendResult = DaliSendResult::Answer(0xff);
pub const NO: DaliSendResult = DaliSendResult::Timeout;
pub const MULTIPLE: DaliSendResult = DaliSendResult::Framing;

#[derive(Debug)]
pub enum OpenError {
    NotFound,
    ParameterError(String),
    DriverError(Box<dyn std::error::Error + Send + Sync>),
}

impl Error for OpenError {}

impl fmt::Display for OpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpenError::NotFound => write!(f, "Driver not found"),
            OpenError::ParameterError(e) => write!(f, "Parameter error: {}", e),
            OpenError::DriverError(e) => write!(f, "Driver error: {}", e),
        }
    }
}

#[derive(Debug)]
pub struct DriverInfo {
    // Name of the driver
    pub name: String,
    // A text decribing the driver in some detail
    pub description: String,
    // Open a driver instance using the supplied parameters
    pub open: fn(params: HashMap<String, String>) -> Result<Box<dyn DaliDriver>, OpenError>,
}

lazy_static! {
    pub static ref DRIVERS: Mutex<Vec<DriverInfo>> = Mutex::new(Vec::new());
}

/// Opens a driver instance with the given name and parameters
///
/// # Arguments
///
/// * `name_params` - A string on the form <NAME> [':' <PARAM>=<VALUE> [',' <PARAM>=<VALUE>] ...]
pub fn open(name_params: &str) -> Result<Box<dyn DaliDriver>, OpenError> {
    let mut param_map = HashMap::<String, String>::new();
    let name = if let Some((n, params)) = name_params.split_once(':') {
        let params = params.trim();
        if !params.is_empty() {
            for param in params.split(',') {
                if let Some((par_name, value)) = param.split_once('=') {
                    param_map.insert(par_name.trim().to_string(), value.trim().to_string());
                } else {
                    return Err(OpenError::ParameterError(
                        "Parameter delimit name and value with '='".to_string(),
                    ));
                }
            }
        }
        n.trim()
    } else {
        name_params
    };

    let locked = DRIVERS.lock().unwrap();
    for d in locked.iter() {
        if name == d.name {
            return (d.open)(param_map);
        }
    }
    Err(OpenError::NotFound)
}

pub fn add_driver(info: DriverInfo) {
    DRIVERS.lock().unwrap().push(info);
}

pub fn driver_names() -> Vec<String> {
    let mut names = Vec::new();
    let locked = DRIVERS.lock().unwrap();
    for d in locked.iter() {
        names.push(d.name.clone());
    }
    names
}

#[cfg(test)]
fn abc_open_callback(_params: HashMap<String, String>) -> Result<Box<dyn DaliDriver>, OpenError> {
    Err(OpenError::DriverError("abc".into()))
}

#[cfg(test)]
fn def_open_callback(params: HashMap<String, String>) -> Result<Box<dyn DaliDriver>, OpenError> {
    match params.get("one") {
        Some(s) if s == "1" => {}
        Some(_) => panic!("Incorrect value for parameter one"),
        None => panic!("Missing parameter one"),
    };
    match params.get("two") {
        Some(s) if s == "2" => {}
        Some(_) => panic!("Incorrect value for parameter two"),
        None => panic!("Missing parameter two"),
    };

    Err(OpenError::DriverError("def".into()))
}

#[test]
fn register_driver_test() {
    add_driver(DriverInfo {
        name: "abc".to_string(),
        description: "abc driver".to_string(),
        open: abc_open_callback,
    });

    add_driver(DriverInfo {
        name: "def".to_string(),
        description: "def driver".to_string(),
        open: def_open_callback,
    });
    match open("foobar") {
        Err(OpenError::NotFound) => {}
        _ => panic!("Unexpected return from open_driver"),
    }

    match open("abc") {
        Err(OpenError::DriverError(_)) => {}
        _ => panic!("Unexpected return from open_driver"),
    }

    match open("def: three=a, two = 2, one= 1") {
        Err(OpenError::DriverError(_)) => {}
        _ => panic!("Unexpected return from open_driver"),
    }
}
