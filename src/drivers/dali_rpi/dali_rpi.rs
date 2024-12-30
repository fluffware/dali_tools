use crate::drivers;
use crate::tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::utils::dyn_future::DynFuture;
use drivers::driver::{
    DaliBusEvent, DaliBusEventResult, DaliBusEventType, DaliDriver, DaliFrame, DaliSendResult,
    DriverInfo, OpenError,
};
use drivers::send_flags::Flags;
use drivers::utils::{DALIcmd, DALIreq};
use futures::executor::block_on;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::str::FromStr;
use std::time::Instant;
use tokio::select;
use tokio::sync::mpsc::{self};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::Duration;
use tokio_serial::{Parity, SerialStream};

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
fn bytes_to_event(bytes: &[u8]) -> Option<DaliBusEvent> {
    let event_type = match bytes[1] {
        4 => match bytes[3] {
            8 => Some(DaliBusEventType::Frame8(bytes[4])),
            16 => Some(DaliBusEventType::Frame16([bytes[4], bytes[5]])),
            24 => Some(DaliBusEventType::Frame24([bytes[4], bytes[5], bytes[6]])),
	    _ => Some(DaliBusEventType::FramingError),
        },
        6 => Some(DaliBusEventType::FramingError),
        7 => Some(DaliBusEventType::BusPowerOff),
        8 => Some(DaliBusEventType::BusPowerOn),
        _ => None,
    };
    if let Some(event_type) = event_type {
        Some(DaliBusEvent {
            timestamp: Instant::now(),
            event_type,
        })
    } else {
        None
    }
}

async fn driver_thread(
    mut serial: SerialStream,
    mut recv: mpsc::Receiver<DALIreq>,
    mut monitor: mpsc::Sender<DaliBusEvent>,
) -> Result<(), DriverError> {
    let mut req_timeout = None;
    let mut ser_rx_buf = [0u8; 16];
    let mut ser_rx_pos = 0;
    let mut last_rx_time = Instant::now();
    let mut next_seq = 1u8;
    let mut current_req = None;
    loop {
        select! {
            req = recv.recv(), if current_req.is_none() => {
                //println!("Req: {:?}",req);
                match req {
                    Some(req) => {
                        let mut bytes = [next_seq,
                                     (if req.cmd.flags.expect_answer() {0b1} else {0})
                                         | (if req.cmd.flags.send_twice() {0b10} else {0}),
                                         req.cmd.flags.priority() as u8 | (2 << 3),
                                         req.cmd.data.bit_length() as u8,
                                         0,0,0,0];
                        match  req.cmd.data {
                            DaliFrame::Frame8(d) => bytes[4] = d,
                            DaliFrame::Frame16(d) => {
                                bytes[4] = d[0];
                                bytes[5] = d[1];
                            }
                            DaliFrame::Frame24(d) => {
                                bytes[4] = d[0];
                                bytes[5] = d[1];
                                bytes[6] = d[2];
                            }
                            DaliFrame::Frame25(_) => {
                                req.reply.send(DaliSendResult:: DriverError("25-bit frames not supported".into())).unwrap();
                                continue;
                            }
                        }
                        if let Err(e) = serial.write(&bytes).await {
                            req.reply.send(DaliSendResult:: DriverError(
                                format!("Failed to write to serial device: {}",e).into())).unwrap();
                            continue;

                        }
                        current_req = Some((next_seq,req));
                        next_seq = if next_seq < 0xff {
                            next_seq + 1
                        } else {
                            1u8
                        };
                        req_timeout = Some(Box::pin(tokio::time::sleep(
                            Duration::from_millis(1000))));
                    }
                    None => break
                }
            },
            _ = async {
                if let Some(ref mut timeout) = req_timeout {
                    timeout.await;
                }

            }, if req_timeout.is_some() => {
                if let Some((_seq, req)) = current_req.take() {
                    req.reply.send(DaliSendResult::Timeout).unwrap();
                }
                req_timeout = None;
            },
            r = serial.read(&mut ser_rx_buf[ser_rx_pos..]) => {
                match &r {
                    Ok(ref n) => {
                        let now = Instant::now();
                        // Skip buffered data if it's too old
                        if now - last_rx_time > Duration::from_millis(200) {
                            ser_rx_buf.copy_within(ser_rx_pos.., 0);
                            ser_rx_pos = 0;
                        }
                        ser_rx_pos += n;
                        if ser_rx_pos >= 8 {
                            //println!("Reply: {:?}", &ser_rx_buf[0..8]);
                            if let Some((seqno, _)) = &current_req {
                                if *seqno == ser_rx_buf[0] {
                                    let (_,req) = current_req.take().unwrap();
                                    let result = match ser_rx_buf[1] {
                                        2 => Some(DaliSendResult::Ok),
                                        3 => Some(DaliSendResult::Answer(ser_rx_buf[4])),
                                        10 => Some(DaliSendResult::Timeout),
                                        _ => None,
                                    };
                                    if let Some(result) = result {
                                        req.reply.send(result).unwrap();
                                    }
                                }
                            }
			    if ser_rx_buf[0] == 0 {
				if let Some(event) = bytes_to_event(&ser_rx_buf) {
				     let _ = monitor.send(event).await;
				}
                            }
                            ser_rx_buf.copy_within(8.., 0);
                            ser_rx_pos -= 8;
                        }
            }
                    Err(e) => {

                    }
                }
            }
        }
    }
    Ok(())
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
    match DaliRpiDriver::new(port, baud_rate, parity) {
        Err(e) => Err(OpenError::DriverError(Box::new(e))),
        Ok(d) => Ok(Box::new(d)),
    }
}

pub struct DaliRpiDriver {
    join: Option<JoinHandle<Result<(), DriverError>>>,
    // Needs to be an option so that it can be dropped to signal the receiver
    send_cmd: Option<mpsc::Sender<DALIreq>>,
    rx_monitor: mpsc::Receiver<DaliBusEvent>,
}

impl DaliRpiDriver {
    fn new(port: &str, baud_rate: u32, parity: Parity) -> Result<DaliRpiDriver, DriverError> {
        let (tx, rx) = mpsc::channel::<DALIreq>(10);
        let (tx_monitor, rx_monitor) = mpsc::channel::<DaliBusEvent>(10);
        let serial = match SerialStream::open(&tokio_serial::new(port, baud_rate).parity(parity)) {
            Ok(s) => s,
            Err(e) => return Err(DriverError::SerialError(e)),
        };
        let join = tokio::spawn(driver_thread(serial, rx, tx_monitor));
        let driver = DaliRpiDriver {
            join: Some(join),
            send_cmd: Some(tx),
            rx_monitor,
        };
        Ok(driver)
    }
}

impl DaliDriver for DaliRpiDriver {
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
	Box::pin(async{self.rx_monitor.recv().await.ok_or("Event source close".into())})
    }

    fn current_timestamp(&self) -> std::time::Instant {
        Instant::now()
    }

    fn wait_until(&self, end: std::time::Instant) -> DynFuture<()> {
        Box::pin(tokio::time::sleep_until(end.into()))
    }
}

impl Drop for DaliRpiDriver {
    fn drop(&mut self) {
        if self.send_cmd.take().is_some() {
            if let Some(join) = self.join.take() {
                let _ = block_on(join);
            }
        }
    }
}
pub fn driver_info() -> DriverInfo {
    DriverInfo {
        name: "DALI_RPI".to_string(),
        description: "Driver for DALI on Raspberry Pi Pico".to_string(),
        open: driver_open,
    }
}
