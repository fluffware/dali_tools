use crate::drivers;
use crate::utils::dyn_future::DynFuture;
use drivers::driver::{
    DaliBusEventResult, DaliDriver, DaliFrame, DaliSendResult, DriverInfo, OpenError,
};
use drivers::send_flags::Flags;
use drivers::utils::{DALIcmd, DALIreq};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
//use std::sync::Arc;
//use std::sync::Mutex;
use std::io::{Read, Write};
use std::str::FromStr;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_modbus::client::{rtu, Context};
use tokio_modbus::slave::Slave;
use tokio_serial::{Parity, SerialStream};

#[derive(Debug)]
enum DriverError {
    OK,
    CommandError,
    SerialError(tokio_serial::Error),
    IoError(std::io::Error),
}

impl Error for DriverError {}

impl From<tokio_serial::Error> for DriverError {
    fn from(err: tokio_serial::Error) -> DriverError {
        DriverError::SerialError(err)
    }
}

impl From<std::io::Error> for DriverError {
    fn from(err: std::io::Error) -> DriverError {
        DriverError::IoError(err)
    }
}
impl fmt::Display for DriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DriverError::OK => write!(f, "No error"),
            DriverError::CommandError => write!(f, "Command error"),
            DriverError::SerialError(err) => write!(f, "{}", err),
            DriverError::IoError(err) => write!(f, "{}", err),
        }
    }
}

async fn driver_thread(
    serial: SerialStream,
    mut recv: mpsc::Receiver<DALIreq>,
) -> Result<(), DriverError> {
    let mut ctxt = rtu::connect_slave(serial, Slave::from(1)).await?;
    loop {
	match recv.recv().await {
	    Some(req) => {
		println!("{:?}", req);
		req.reply.send(DaliSendResult::Ok).unwrap();
	    }
	    None => break
	}
    }
    Ok(())
}

pub struct Dgw521Driver {
    join: Option<JoinHandle<Result<(), DriverError>>>,
    // Needs to be an option so that it can be dropped to signal the receiver
    send_cmd: Option<mpsc::Sender<DALIreq>>,
    //    _send_monitor: Arc<Mutex<Option<mpsc::Sender<DaliBusEvent>>>>,
}
impl Dgw521Driver {
    fn new(port: &str, baud_rate: u32, parity: Parity) -> Result<Dgw521Driver, DriverError> {
        let (tx, rx) = mpsc::channel::<DALIreq>(10);
        let serial = match SerialStream::open(&tokio_serial::new(port, baud_rate).parity(parity)) {
            Ok(s) => s,
            Err(e) => return Err(DriverError::SerialError(e)),
        };
        let join = tokio::spawn(driver_thread(serial, rx));
        let driver = Dgw521Driver {
            join: Some(join),
            send_cmd: Some(tx),
            //_send_monitor: monitor,
        };
        Ok(driver)
    }
}

impl DaliDriver for Dgw521Driver {
    fn send_frame(
        &mut self,
        cmd: DaliFrame,
        flags: Flags,
    ) -> Pin<Box<dyn Future<Output = DaliSendResult> + Send>> {
        if !matches!(cmd, DaliFrame::Frame16(_)) {
            return Box::pin(std::future::ready(DaliSendResult::DriverError(
                "Only 16-bit frames supported when sending".into(),
            )));
        }
        let (tx, rx) = oneshot::channel();
        let req = DALIreq {
            cmd: DALIcmd {
                data: cmd.clone(),
                flags: flags,
            },
            reply: tx,
        };

        match self.send_cmd.as_mut().unwrap().try_send(req) {
            Ok(()) => Box::pin(async {
                match rx.await {
                    Ok(r) => r,
                    Err(e) => DaliSendResult::DriverError(Box::new(e)),
                }
            }),
            Err(_) => {
                Box::pin(async { DaliSendResult::DriverError(Box::new(DriverError::CommandError)) })
            }
        }
    }

    fn next_bus_event(&mut self) -> DynFuture<DaliBusEventResult> {
        Box::pin(std::future::ready(Err("Not implemented".into())))
    }

    fn current_timestamp(&self) -> std::time::Instant {
        Instant::now().into_std()
    }

    fn wait_until(&self, end: std::time::Instant) -> DynFuture<()> {
        Box::pin(tokio::time::sleep_until(Instant::from(end)))
    }
}

fn driver_open(params: HashMap<String, String>) -> Result<Box<dyn DaliDriver>, OpenError> {
    let port = params
        .get("port")
        .map(|s| s.as_str())
        .unwrap_or("/dev/ttyACM0");
    let baud_rate = match params.get("baud_rate") {
        None => 9600,
        Some(s) => u32::from_str(s)
            .map_err(|_| OpenError::ParameterError("baud_rate has invalid value".to_string()))?,
    };
    let parity = match params.get("parity") {
        Some(p) if p.len() >= 1 => match &p[..1] {
            "E" | "e" => Parity::Even,
            "O" | "o" => Parity::Odd,
            "N" | "n" => Parity::None,
            _ => {
                return Err(OpenError::ParameterError(
                    "parity has invalid value".to_string(),
                ));
            }
        },
        Some(_) | None => Parity::None,
    };
    match Dgw521Driver::new(port, baud_rate, parity) {
        Err(e) => Err(OpenError::DriverError(Box::new(e))),
        Ok(d) => Ok(Box::new(d)),
    }
}

pub fn driver_info() -> DriverInfo {
    DriverInfo {
        name: "DGW521".to_string(),
        description: "Driver for ICP DAS DGW 521 DALI-adapter".to_string(),
        open: driver_open,
    }
}
