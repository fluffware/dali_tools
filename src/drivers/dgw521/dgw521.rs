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
use futures::executor::block_on;
use log::warn;
use std::collections::VecDeque;
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::mpsc::{self, error::TryRecvError};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::time::timeout;
use tokio_modbus::client::rtu;
use tokio_modbus::prelude::*;
use tokio_modbus::slave::Slave;
use tokio_serial::{Parity, SerialStream};

#[allow(dead_code)]
mod mb {
    pub const PROTOCOL: u16 = 256;
    pub const PROTOCOL_MODBUS: bool = false;
    pub const PROTOCOL_DCON: bool = true;
    pub const FIND: u16 = 258;
    pub const FIND_START: bool = true;
    pub const FIND_BUSY: bool = true;
    pub const FIND_DONE: bool = false;

    pub const WATCHDOG: u16 = 269;
    pub const WATCHDOG_ENABLE: bool = true;
    pub const WATCHDOG_DISABLE: bool = false;

    pub const RESET_STATUS: u16 = 272;

    pub const CMD_STATUS_1: u16 = 0;
    pub const CMD_STATUS_2: u16 = 1;
    pub const CMD_STATUS_3: u16 = 2;
    pub const CMD_STATUS_4: u16 = 3;
    pub const CMD_STATUS_5: u16 = 4;
    pub const CMD_STATUS_6: u16 = 5;
    pub const CMD_STATUS_7: u16 = 6;
    pub const CMD_STATUS_8: u16 = 7;

    pub const CMD_STATUS_IDLE: u16 = 0;
    pub const CMD_STATUS_PENDING: u16 = 1;
    pub const CMD_STATUS_EXECUTING: u16 = 2;
    pub const CMD_STATUS_NO_ANSWER: u16 = 3;
    pub const CMD_STATUS_TIMEOUT: u16 = 4;
    pub const CMD_STATUS_ANSWER: u16 = 5;
    pub const CMD_STATUS_INVALID_DATA: u16 = 6;
    pub const CMD_STATUS_EARLY: u16 = 7;

    pub const DALI_CMD_1: u16 = 32;
    pub const DALI_CMD_2: u16 = 33;
    pub const DALI_CMD_3: u16 = 34;
    pub const DALI_CMD_4: u16 = 35;
    pub const DALI_CMD_5: u16 = 36;
    pub const DALI_CMD_6: u16 = 37;
    pub const DALI_CMD_7: u16 = 38;
    pub const DALI_CMD_8: u16 = 39;

    pub const CMD_STATUS_MASK: u16 = 256;
}

#[derive(Debug)]
enum DriverError {
    #[allow(dead_code)]
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

fn send_driver_error<E>(req: DALIreq, error: E)
where
    E: Error + Send + Sync + 'static,
{
    req.reply
        .send(DaliSendResult::DriverError(error.into()))
        .unwrap();
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

const MB_TIMEOUT: Duration = Duration::from_millis(1000);
async fn driver_thread(
    serial: SerialStream,
    mut recv: mpsc::Receiver<DALIreq>,
) -> Result<(), DriverError> {
    let mut ctxt = rtu::attach_slave(serial, Slave::from(1));
    let mut queue = VecDeque::<DALIreq>::new();
    let mut oldest_slot = 0;
    let mut next_slot = 0;
    'outer: loop {
        let mask = match timeout(
            MB_TIMEOUT,
            ctxt.read_input_registers(mb::CMD_STATUS_MASK, 1),
        )
        .await
        {
            Ok(Ok(regs)) => regs[0],
            Ok(Err(e)) => {
                if let Some(req) = queue.pop_front() {
                    oldest_slot = (oldest_slot + 1) & 7;
                    send_driver_error(req, e);
                }
                continue 'outer;
            }

            Err(e) => {
                if let Some(req) = queue.pop_front() {
                    oldest_slot = (oldest_slot + 1) & 7;
                    send_driver_error(req, e);
                }
                warn!("Modbus timeout");
                continue 'outer;
            }
        };
        //println!("{:08b} {} - {}", mask, oldest_slot, next_slot);
        while oldest_slot != next_slot && ((1 << oldest_slot) & mask) != 0 {
            let req = queue.pop_front().unwrap();
            let slot = oldest_slot;
            oldest_slot = (oldest_slot + 1) & 7;
            let res = match timeout(
                MB_TIMEOUT,
                ctxt.read_input_registers(mb::CMD_STATUS_1 + slot, 1),
            )
            .await
            {
                Ok(Ok(regs)) => regs[0],
                Ok(Err(e)) => {
                    send_driver_error(req, e);
                    continue 'outer;
                }
                Err(e) => {
                    send_driver_error(req, e);
                    continue 'outer;
                }
            };
            //println!("Status {:04x} @{}", res, slot);
            req.reply
                .send(match res & 0xff {
                    mb::CMD_STATUS_EXECUTING | mb::CMD_STATUS_PENDING => {
                        DaliSendResult::DriverError("Command not finished".into())
                    }
                    mb::CMD_STATUS_NO_ANSWER => {
                        if req.cmd.flags.expect_answer() {
                            DaliSendResult::Answer((res >> 8) as u8)
                        } else {
                            DaliSendResult::Ok
                        }
                    }
                    mb::CMD_STATUS_INVALID_DATA => DaliSendResult::Framing,
                    mb::CMD_STATUS_EARLY | mb::CMD_STATUS_TIMEOUT => DaliSendResult::Timeout,
                    mb::CMD_STATUS_ANSWER => DaliSendResult::Answer((res >> 8) as u8),
                    _ => DaliSendResult::DriverError("Unknown status".into()),
                })
                .unwrap();
        }
        // Try filling empty slots with commands
        'add: while ((1 << next_slot) & mask) != 0 {
            let req = match recv.try_recv() {
                Ok(req) => req,
                Err(e) => match e {
                    TryRecvError::Empty => break 'add,
                    TryRecvError::Disconnected => break 'outer,
                },
            };
            if let DaliFrame::Frame16(frame) = req.cmd.data {
                match timeout(
                    MB_TIMEOUT,
                    ctxt.write_single_register(
                        mb::DALI_CMD_1 + next_slot,
                        u16::from_be_bytes(frame),
                    ),
                )
                .await
                {
                    Ok(Ok(())) => {}
                    Ok(Err(e)) => {
                        send_driver_error(req, e);
                        continue 'outer;
                    }
                    Err(e) => {
                        send_driver_error(req, e);
                        continue 'outer;
                    }
                };
                queue.push_back(req);
                //println!("Sent {}", next_slot);
            } else {
                req.reply
                    .send(DaliSendResult::DriverError(
                        "Only 16 bit frames supported".into(),
                    ))
                    .unwrap();
            }
            next_slot = (next_slot + 1) & 7;
        }
    }
    println!("Driver exited");
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

impl Drop for Dgw521Driver {
    fn drop(&mut self) {
        if self.send_cmd.take().is_some() {
            if let Some(join) = self.join.take() {
                let _ = block_on(join);
            }
        }
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
        Some(_) | None => Parity::Even,
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
