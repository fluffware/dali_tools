use crate::drivers;
use crate::utils::dyn_future::DynFuture;
use drivers::driver::{
    DaliBusEvent, DaliBusEventResult, DaliBusEventType, DaliDriver, DaliFrame, DaliSendResult,
    DriverInfo, OpenError,
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
use crate::futures::FutureExt;
use futures::executor::block_on;
use log::{debug, warn};
use std::ops::{AddAssign, Sub};
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::mpsc::{self, error::TryRecvError};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio::time::timeout;
use tokio_modbus::client::{Context, rtu};
use tokio_modbus::prelude::*;
use tokio_modbus::slave::Slave;
use tokio_serial::{Parity, SerialStream};
use std::time::Instant as StdInstant;

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

    pub const MONITOR_BUFFER_INDEX: u16 = 322;
    pub const MONITOR_BUFFER_START: u16 = 1024;
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

#[derive(Debug, Copy, Clone)]

struct SlotIndex(u16);

impl SlotIndex {
    pub fn new(s: u16) -> Self {
        SlotIndex(s)
    }

    pub fn slot(&self) -> usize {
        (self.0 as usize) & 7
    }

    pub fn mask(&self) -> u16 {
        1 << (self.0 & 7)
    }
}

impl AddAssign<u16> for SlotIndex {
    fn add_assign(&mut self, b: u16) {
        self.0 = self.0.wrapping_add(b);
    }
}

impl Sub<SlotIndex> for SlotIndex {
    type Output = u16;
    fn sub(self, b: SlotIndex) -> u16 {
        self.0.wrapping_sub(b.0)
    }
}

struct SendState {
    pending: [Option<DALIreq>; 8],
    oldest_slot: SlotIndex,
    next_slot: SlotIndex,
}
impl SendState {
    pub fn new() -> Self {
        SendState {
            pending: [None, None, None, None, None, None, None, None],
            oldest_slot: SlotIndex::new(0),
            next_slot: SlotIndex::new(0),
        }
    }

    async fn send(&mut self, recv: &mut mpsc::Receiver<DALIreq>, ctxt: &mut Context, req: DALIreq) {
        let mut pending = Some(req);
        'sending: loop {
            let mask = match timeout(
                MB_TIMEOUT,
                ctxt.read_input_registers(mb::CMD_STATUS_MASK, 1),
            )
            .await
            {
                Ok(Ok(regs)) => regs[0],
                Ok(Err(_e)) => {
                    continue 'sending;
                }

                Err(_e) => {
                    warn!("Modbus timeout");
                    continue 'sending;
                }
            };
            /*            println!(
                            "Mask: {:08b}, Oldest slot: {:?}, Next slot: {:?}",
                            mask, self.oldest_slot, self.next_slot
                        );
            */
            // Handle slots that are finished
            while self.next_slot - self.oldest_slot != 0 && (self.oldest_slot.mask() & mask) != 0 {
                let slot = self.oldest_slot.slot();
                let Some(req) = self.pending[slot].take() else {
                    warn!("No request for slot");
                    self.oldest_slot += 1;
                    continue;
                };
                self.oldest_slot += 1;
                let res = match timeout(
                    MB_TIMEOUT,
                    ctxt.read_input_registers(mb::CMD_STATUS_1 + slot as u16, 1),
                )
                .await
                {
                    Ok(Ok(regs)) => regs[0],
                    Ok(Err(e)) => {
                        send_driver_error(req, e);
                        continue 'sending;
                    }
                    Err(e) => {
                        send_driver_error(req, e);
                        warn!("Modbus timeout");
                        continue 'sending;
                    }
                };
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
            while self.next_slot - self.oldest_slot < 8
                && (self.next_slot.mask() & mask) != 0
                && let Some(req) = pending.take()
            {
                if let DaliFrame::Frame16(frame) = req.cmd.data {
                    match timeout(
                        MB_TIMEOUT,
                        ctxt.write_single_register(
                            mb::DALI_CMD_1 + self.next_slot.slot() as u16,
                            u16::from_be_bytes(frame),
                        ),
                    )
                    .await
                    {
                        Ok(Ok(())) => {}
                        Ok(Err(e)) => {
                            send_driver_error(req, e);
                            continue 'sending;
                        }
                        Err(e) => {
                            send_driver_error(req, e);
                            continue 'sending;
                        }
                    };
                    self.pending[self.next_slot.slot()] = Some(req);
                    //println!("Sent {}", next_slot);
                } else {
                    req.reply
                        .send(DaliSendResult::DriverError(
                            "Only 16 bit frames supported".into(),
                        ))
                        .unwrap();
                }
                self.next_slot += 1;

                pending = match recv.try_recv() {
                    Ok(req) => Some(req),
                    Err(e) => match e {
                        TryRecvError::Empty => None,
                        TryRecvError::Disconnected => None,
                    },
                };
            }
            if self.next_slot - self.oldest_slot == 0 {
                break 'sending;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        debug!("Send done");
    }
}

struct MonitorState {
    last_index: u16,
    last_ts: StdInstant,
}

impl MonitorState {
    pub fn new() -> Self {
        MonitorState {
            last_index: 0,
            last_ts: StdInstant::now(),
        }
    }
}

impl MonitorState {
    async fn check_events(&mut self, notify: &mut mpsc::Sender<DaliBusEvent>, ctxt: &mut Context) {
        let index = match timeout(
            MB_TIMEOUT,
            ctxt.read_input_registers(mb::MONITOR_BUFFER_INDEX, 1),
        )
        .await
        {
            Ok(Ok(regs)) => regs[0],
            Ok(Err(e)) => {
                warn!("Modbus error: {}", e);
                return;
            }

            Err(_e) => {
                warn!("Modbus timeout");
                return;
            }
        }
        .wrapping_add(1);

        let mut read_len = index.wrapping_sub(self.last_index);
        if read_len > 0 {
            if read_len > 31 {
                read_len = 31;
                self.last_index = index.wrapping_sub(31);
            }
            let read_index = self.last_index % 32;
            if read_len > 32 - read_index {
                read_len = 32 - read_index;
            }
            match timeout(
                MB_TIMEOUT,
                ctxt.read_input_registers(mb::MONITOR_BUFFER_START + 2 * read_index, 2 * read_len),
            )
            .await
            {
                Ok(Ok(regs)) => {
                    for i in 0..read_len {
                        let info = regs[(i * 2 + 1) as usize];
                        let rel_ts = info >> 6;
                        if rel_ts >= 999 {
                            self.last_ts = StdInstant::now();
                        } else {
                            self.last_ts += Duration::from_millis(rel_ts as u64);
                        }

                        let event_type = if info & 0x06 != 0 {
                            DaliBusEventType::FramingError
                        } else if info & 0x08 != 0 {
                            DaliBusEventType::Frame16(u16::to_be_bytes(regs[(i * 2) as usize]))
                        } else {
                            DaliBusEventType::Frame8((regs[(i * 2) as usize] & 0xff) as u8)
                        };
                        let _ = notify.try_send(DaliBusEvent {
                            timestamp: self.last_ts,
                            event_type,
                        });
                    }
                }
                Ok(Err(e)) => {
                    warn!("Modbus error: {}", e);
                    return;
                }

                Err(_e) => {
                    warn!("Modbus timeout");
                    return;
                }
            };

            self.last_index = self.last_index.wrapping_add(read_len);

        }
    }
}

const MB_TIMEOUT: Duration = Duration::from_millis(1000);
const POLL_INTERVAL: Duration = Duration::from_millis(200);
async fn driver_thread(
    serial: SerialStream,
    mut recv: mpsc::Receiver<DALIreq>,
    mut monitor: mpsc::Sender<DaliBusEvent>,
) -> Result<(), DriverError> {
    debug!("driver_thread");
    let mut ctxt = rtu::attach_slave(serial, Slave::from(1));
    let mut send_state = SendState::new();
    let mut monitor_state = MonitorState::new();
    loop {
        match timeout(POLL_INTERVAL, recv.recv()).await {
            Ok(Some(req)) => {
                send_state.send(&mut recv, &mut ctxt, req).await;
            }
            Ok(None) => break,
            Err(_) => {
                monitor_state.check_events(&mut monitor, &mut ctxt).await;
            }
        }
    }
    debug!("Driver exited");
    Ok(())
}

pub struct Dgw521Driver {
    join: Option<JoinHandle<Result<(), DriverError>>>,
    // Needs to be an option so that it can be dropped to signal the receiver
    send_cmd: Option<mpsc::Sender<DALIreq>>,
    recv_monitor: Option<mpsc::Receiver<DaliBusEvent>>,
}
impl Dgw521Driver {
    fn new(port: &str, baud_rate: u32, parity: Parity) -> Result<Dgw521Driver, DriverError> {
        let (tx_cmd, rx_cmd) = mpsc::channel::<DALIreq>(10);
        let (tx_monitor, rx_monitor) = mpsc::channel::<DaliBusEvent>(10);
        let serial = match SerialStream::open(&tokio_serial::new(port, baud_rate).parity(parity)) {
            Ok(s) => s,
            Err(e) => return Err(DriverError::SerialError(e)),
        };
        let join = tokio::spawn(driver_thread(serial, rx_cmd, tx_monitor));
        let driver = Dgw521Driver {
            join: Some(join),
            send_cmd: Some(tx_cmd),
            recv_monitor: Some(rx_monitor),
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
                flags,
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

    fn next_bus_event(&mut self) -> DynFuture<'_, DaliBusEventResult> {
        let Some(recv) = &mut self.recv_monitor else {
            return Box::pin(std::future::ready(Err("No queue".into())));
        };
        Box::pin(
            recv.recv()
                .map(|r| r.ok_or_else(|| "Channel closed".into())),
        )
    }

    fn current_timestamp(&self) -> std::time::Instant {
        Instant::now().into_std()
    }

    fn wait_until(&self, end: std::time::Instant) -> DynFuture<'_, ()> {
        Box::pin(tokio::time::sleep_until(Instant::from(end)))
    }
}

impl Drop for Dgw521Driver {
    fn drop(&mut self) {
        if self.send_cmd.take().is_some() && let Some(join) = self.join.take() {
            let _ = block_on(join);
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
