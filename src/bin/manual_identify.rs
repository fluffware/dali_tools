use std::collections::BTreeMap;
use std::error::Error;
use std::future::{self};
use std::net::IpAddr;
use std::ops::RangeBounds;
use std::pin::Pin;
use std::process::ExitCode;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::cmp;
use std::sync::atomic::{self, AtomicU16};
use std::sync::{Arc, Mutex as BlockingMutex, RwLock};
use std::task::{Context, Poll};

use clap::Parser;
use futures::future::{Fuse, FutureExt};
use log::{debug, error, info};
use std::future::Future;

use hyper::{header, http};
use hyper::{Body, Request, Response};
use serde::Serializer;
use serde_derive::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::Duration;

use dali::common::address::{Long, Short};
use dali::common::defs::MASK;
use dali::drivers::command_utils::send16;
use dali::drivers::driver::OpenError;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
use dali::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG};
use dali::gear::address::Address;
use dali::gear::cmd_defs as cmd;
use dali::gear::status::GearStatus;
use dali::httpd::{self, ServerConfig};
use dali::utils::address_assignment::program_short_addresses;
//use dali::utils::filtered_vec::FilteredVec;
use dali_tools as dali;

type DynResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

async fn query_long_addr(
    driver: &mut dyn DaliDriver,
    short_addr: Short,
) -> Result<Long, DaliSendResult> {
    let h = send16::device_cmd(
        driver,
        &short_addr,
        cmd::QUERY_RANDOM_ADDRESS_H,
        EXPECT_ANSWER,
    )
    .await
    .check_answer()?;
    let m = send16::device_cmd(
        driver,
        &short_addr,
        cmd::QUERY_RANDOM_ADDRESS_M,
        EXPECT_ANSWER,
    )
    .await
    .check_answer()?;
    let l = send16::device_cmd(
        driver,
        &short_addr,
        cmd::QUERY_RANDOM_ADDRESS_L,
        EXPECT_ANSWER,
    )
    .await
    .check_answer()?;
    Ok((h as u32) << 16 | (m as u32) << 8 | (l as u32))
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
pub struct GearData {
    long: u32,
    old_addr: Option<u8>,
    new_addr: Option<u8>,
}

struct GearState {
    current_gear: usize,
    target_gear: usize,
    gears: Vec<GearData>,
}

pub struct IdentificationCtxt {
    state: RwLock<GearState>,
    low_level: AtomicU8,  // Set to MASK for min level
    high_level: AtomicU8, // Set to MASK for max level
}

impl IdentificationCtxt {
    pub fn new() -> IdentificationCtxt {
        let state = GearState {
            current_gear: 0,
            target_gear: 0,
            gears: Vec::new(),
        };
        IdentificationCtxt {
            state: RwLock::new(state),
            low_level: AtomicU8::new(MASK),
            high_level: AtomicU8::new(MASK),
        }
    }

    fn get_state<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&GearState) -> R,
    {
        let state = self.state.read().unwrap();
        f(&*state)
    }

    fn modify_state<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut GearState) -> R,
    {
        let mut state = self.state.write().unwrap();
        f(&mut *state)
    }

    fn get_current_gear<R, F>(&self, f: F) -> R
    where
        F: FnOnce(Option<&GearData>) -> R,
    {
        self.get_state(|state| {
            let gear = state.gears.get(state.current_gear);
            f(gear)
        })
    }
}

type SyncDriver = Arc<Mutex<Box<dyn DaliDriver>>>;

async fn find<F>(driver: &SyncDriver, found: &mut F) -> DynResult<()>
where
    F: FnMut(GearData),
{
    for addr in 0..64 {
        debug!("Checking {}", addr);
        let driver = &mut **driver.lock().await;
        let status =
            match send16::device_cmd(driver, &Short::new(addr), cmd::QUERY_STATUS, EXPECT_ANSWER)
                .await
            {
                DaliSendResult::Answer(s) => Some(GearStatus::new(s)),
                DaliSendResult::Timeout => None,
                e => return Err(e.into()),
            };
        if let Some(status) = status {
            info!("{}: Status: {}\r\n", addr, status);
            match query_long_addr(driver, Short::new(addr)).await {
                Ok(l) => {
                    info!("{}: Long: {:06x}\r\n", addr, l);
                    found(GearData {
                        long: l,
                        old_addr: Some(addr),
                        new_addr: None,
                    });
                }
                Err(_) => {}
            }
        }
    }
    Ok(())
}

async fn set_low_level(driver: &SyncDriver, low_level: u8, addr: &Address) -> DynResult<()> {
    let driver = &mut **driver.lock().await;
    match if low_level == MASK {
        send16::device_cmd(driver, addr, cmd::RECALL_MIN_LEVEL, NO_FLAG).await
    } else {
        send16::device_level(driver, addr, low_level, NO_FLAG).await
    } {
        DaliSendResult::Ok => {}
        e => return Err(e.into()),
    }
    Ok(())
}

async fn set_high_level(driver: &SyncDriver, high_level: u8, addr: &Address) -> DynResult<()> {
    let driver = &mut **driver.lock().await;
    match if high_level == MASK {
        send16::device_cmd(driver, addr, cmd::RECALL_MAX_LEVEL, NO_FLAG).await
    } else {
        send16::device_level(driver, addr, high_level, NO_FLAG).await
    } {
        DaliSendResult::Ok => {}
        e => return Err(e.into()),
    }
    Ok(())
}

fn reply_scan_update(ctxt: &IdentificationCtxt) -> String {
    ctxt.get_state(|state| {
        let index = state.current_gear as u8;
        let gears = &state.gears;
        let length = gears.len() as u8;
        let mut current_address = MASK;
        let mut new_address = MASK;
        if index < length {
            if let Some(&GearData {
                old_addr, new_addr, ..
            }) = gears.get(usize::from(index))
            {
                current_address = old_addr.unwrap_or(MASK);
                new_address = new_addr.unwrap_or(MASK);
            }
        }
        serde_json::to_string(&ScanUpdate {
            current_address,
            new_address,
            index,
            length,
        })
        .unwrap()
    })
}
async fn clear_scan(driver: &SyncDriver, ctxt: &Arc<IdentificationCtxt>) -> DynResult<()> {
    set_low_level(
        driver,
        ctxt.low_level.load(Ordering::Relaxed),
        &Address::Broadcast,
    )
    .await?;

    let addr: Option<u8> = ctxt.get_current_gear(|gear| gear.and_then(|g| g.old_addr));
    if let Some(addr) = addr {
        set_high_level(
            driver,
            ctxt.high_level.load(Ordering::Relaxed),
            &Address::Short(Short::new(addr)),
        )
        .await?;
    }
    Ok(())
}
async fn handle_commands(
    driver: &SyncDriver,
    ctxt: &Arc<IdentificationCtxt>,
    cmd: DaliCommands,
    current_high: &mut bool,
) -> DynResult<DaliCommandStatus> {
    match cmd {
        DaliCommands::NoCommand => {}
        DaliCommands::ScanAddress { index } => {
            ctxt.modify_state(|state| {
                state.target_gear = usize::from(index);
            });
        }
        DaliCommands::FindAll => {
            ctxt.modify_state(|s| {
                s.gears.clear();
                s.target_gear = 0;
                s.current_gear = 0;
            });
            find(driver, &mut |gd| {
                ctxt.modify_state(|s| {
                    s.gears.push(gd);
                })
            })
            .await?;
            clear_scan(driver, ctxt).await?;
            *current_high = true;
        }
        /*
            DaliCommands::RequestScanUpdate => {
                reply_scan_update(cmd_req.reply, ctxt)?;
        }
        */
        DaliCommands::NewAddress { address, index } => {
            ctxt.modify_state(|state| {
                let gears = &mut state.gears;
                if let Some(gear) = gears.get_mut(usize::from(index)).as_mut() {
                    if (0..64).contains(&address) {
                        debug!("new_addr = {}", address);
                        gear.new_addr = Some(address);
                    }
                }
            });
        }
        DaliCommands::ChangeAddresses => {
            let driver = &mut **driver.lock().await;
            let mut swaps = Vec::new();
            ctxt.modify_state(|state| {
                let gears = &mut state.gears;
                for g in gears.iter_mut() {
                    if let (Some(old_addr), Some(new_addr)) = (g.old_addr, g.new_addr) {
                        swaps.push((Short::new(old_addr), Short::new(new_addr)));
                        g.old_addr = g.new_addr.take();
                    }
                }
            });
            program_short_addresses(driver, &swaps).await?;
        }
        DaliCommands::Sort => {
            ctxt.modify_state(|state| {
                let gears = &mut state.gears;
                state.target_gear = 0;
                state.current_gear = 0;
                // First unallocated sorted by long address
                // Second not remappped sorted by short address
                // Last remapped sorted by new short address
                gears.sort_by(|a, b| match (a.new_addr, b.new_addr) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => cmp::Ordering::Greater,
                    (None, Some(_)) => cmp::Ordering::Less,
                    (None, None) => match (a.old_addr, b.old_addr) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => cmp::Ordering::Greater,
                        (None, Some(_)) => cmp::Ordering::Less,
                        (None, None) => a.long.cmp(&b.long),
                    },
                });
            });
            clear_scan(driver, ctxt).await?;
            *current_high = true;
        } /*
                       DaliCommands::RequestGearList => {
                           let gears = ctxt.gears.lock().unwrap();
                           let list: Vec<GearData> = gears.iter().cloned().collect();
                           let _ = cmd_req.reply.send(DaliReplies::GearList(list));
                   }
                              program_short_addresses(driver, &swaps).await?;
          */
    }
    Ok(DaliCommandStatus::Done)
}

#[derive(Serialize, PartialEq)]
enum DaliCommandStatus {
    Executing,
    Done,
    Failed,
}

#[derive(Serialize)]
struct CmdLogEntry {
    id: u32,
    cmd: DaliCommands,
    status: DaliCommandStatus,
    #[serde(skip)]
    notify: Option<oneshot::Receiver<DaliCommandStatus>>,
}

#[derive(Clone)]
struct CmdLog(Arc<BlockingMutex<Vec<CmdLogEntry>>>);

struct CmdLogNotify(CmdLog);

impl Future for CmdLogNotify {
    type Output = u32;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.lock(|log| {
            for entry in log.iter_mut() {
                if let Some(ref mut notify) = entry.notify {
                    tokio::pin!(notify);
                    match notify.poll(cx) {
                        Poll::Ready(reply) => {
                            entry.status = reply.unwrap_or(DaliCommandStatus::Failed);
                            entry.notify = None;
                            return Poll::Ready(entry.id);
                        }
                        Poll::Pending => {}
                    }
                }
            }
            Poll::Pending
        })
    }
}

impl CmdLog {
    pub fn new() -> Self {
        CmdLog(Arc::new(BlockingMutex::new(Vec::new())))
    }

    pub fn push(&self, entry: CmdLogEntry) {
        self.lock(|log| log.push(entry));
    }

    pub fn notify(&self) -> impl Future<Output = u32> {
        CmdLogNotify(self.clone())
    }

    pub fn lock<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut Vec<CmdLogEntry>) -> T,
    {
        f(&mut *self.0.lock().unwrap())
    }
}

#[derive(Debug)]
struct ResponseError(String);

impl Error for ResponseError {}

impl std::fmt::Display for ResponseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

fn bad_request(msg: &str) -> DynResult<Response<Body>> {
    Response::builder()
        .status(http::StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(msg.to_owned()))
        .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

fn get_int_arg<'a, B>(
    args: &BTreeMap<&str, &str>,
    name: &str,
    bounds: B,
) -> Result<i32, ResponseError>
where
    B: RangeBounds<i32>,
{
    let Some(cmd_str) = args.get(name) else {
        return Err(ResponseError(format!("Argument '{name}' missing")));
    };
    let Ok(v) = cmd_str.parse::<i32>() else {
        return Err(ResponseError(format!(
            "Value for '{name}' is not an integer"
        )));
    };
    if !bounds.contains(&v) {
        return Err(ResponseError(format!(
            "Value for '{name}' is out of bounds"
        )));
    }
    Ok(v)
}

static NEXT_LOG_ID: AtomicU16 = AtomicU16::new(0);

fn decode_get_request(
    req: Request<Body>,
    cmd_req: mpsc::Sender<DaliCommandRequest>,
    ctxt: &Arc<IdentificationCtxt>,
    log: CmdLog,
) -> DynResult<Response<Body>> {
    if req.uri().path() == "/dyn/dali" {
        let mut args = BTreeMap::new();
        if let Some(query) = req.uri().query() {
            let mut query_parts = query.split('&');
            if let Some(kv) = query_parts.next() {
                let Some((k, v)) = kv.split_once('=') else {
                    return bad_request("Missing '='");
                };
                args.insert(k, v);
                while let Some(kv) = query_parts.next() {
                    let Some((k, v)) = kv.split_once('=') else {
                        return bad_request("Missing '='");
                    };
                    args.insert(k, v);
                }
            }
        }
        let Some(cmd_arg) = args.get("cmd").and_then(|s| s.parse::<i32>().ok()) else {
            return bad_request("Missing or invalid 'cmd'");
        };
        let cmd = match u32::try_from(cmd_arg).unwrap() {
            DaliCommands::SCAN_ADDRESS => {
                let index = get_int_arg(&args, "index", 0..=63)? as u8;
                DaliCommands::ScanAddress { index }
            }
            DaliCommands::FIND_ALL => DaliCommands::FindAll,
            DaliCommands::NEW_ADDRESS => {
                let address = get_int_arg(&args, "address", 0..=63)? as u8;
                let index = get_int_arg(&args, "index", 0..=63)? as u8;
                DaliCommands::NewAddress { address, index }
            }
            DaliCommands::CHANGE_ADDRESSES => DaliCommands::ChangeAddresses,
            DaliCommands::SORT => DaliCommands::Sort,
            _ => return bad_request("Unknown command number"),
        };
        /*
         Err(err) => {
                    warn!("Invalid command syntax: {err}");
                    bad_request("Invalid command syntax")
        }
         */

        println!("Command: {:?}", cmd);
        let (reply, rx) = oneshot::channel();
        if let Err(err) = cmd_req.try_send(DaliCommandRequest {
            cmd: cmd.clone(),
            reply,
        }) {
            error!("Failed to queue command: {err}");
            return bad_request("Failed to queue command");
        }
        let id = NEXT_LOG_ID.fetch_add(1, atomic::Ordering::Relaxed);
        let log_item = CmdLogEntry {
            id: u32::from(id),
            cmd: cmd,
            status: DaliCommandStatus::Executing,
            notify: Some(rx),
        };
        log.push(log_item);

        Response::builder()
            .status(http::StatusCode::ACCEPTED)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from(format!("{{\"id\":{id}}}")))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    } else if req.uri().path() == "/dyn/cmd_status" {
        let entry = log.lock(|log| {
            if let Some(index) = log
                .iter()
                .position(|entry| entry.status != DaliCommandStatus::Executing)
            {
                serde_json::to_string(&log.remove(index)).unwrap()
            } else {
                if log.len() > 0 {
                    serde_json::to_string(&log[0]).unwrap()
                } else {
                    serde_json::to_string(&CmdLogEntry {
                        id: 0,
                        cmd: DaliCommands::NoCommand,
                        status: DaliCommandStatus::Done,
                        notify: None,
                    })
                    .unwrap()
                }
            }
        });
        Response::builder()
            .status(http::StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(entry))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    } else if req.uri().path() == "/dyn/scan_state" {
        Response::builder()
            .status(http::StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(reply_scan_update(ctxt)))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    } else {
        Response::builder()
            .status(http::StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from(format!("No such command")))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

#[derive(Serialize, Deserialize)]
enum DaliSetIntensity {
    Intensity(u8),
    Low(bool),
    High(bool),
}

#[derive(Clone, Debug)]
enum DaliCommands {
    NoCommand,
    ScanAddress { index: u8 },
    FindAll,
    NewAddress { address: u8, index: u8 },
    ChangeAddresses,
    Sort,
}

impl DaliCommands {
    pub const NO_COMMAND: u32 = 0;
    pub const SCAN_ADDRESS: u32 = 1;
    pub const FIND_ALL: u32 = 2;
    pub const NEW_ADDRESS: u32 = 3;
    pub const CHANGE_ADDRESSES: u32 = 4;
    pub const SORT: u32 = 5;
}

impl serde::Serialize for DaliCommands {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use DaliCommands::*;
        serializer.serialize_u32(match self {
            NoCommand => Self::NO_COMMAND,
            ScanAddress { .. } => Self::SCAN_ADDRESS,
            FindAll => Self::FIND_ALL,
            NewAddress { .. } => Self::NEW_ADDRESS,
            ChangeAddresses { .. } => Self::CHANGE_ADDRESSES,
            Sort { .. } => Self::SORT,
        })
    }
}

/*
  #[serde(alias = "3")]
    RequestScanUpdate,
 #[serde(alias = "4")]
RequestGearList,
 */
/*
#[derive(Serialize, Deserialize, Clone, Debug)]
enum DaliReplies {
    ScanUpdate {
        current_address: u8,
        new_address: u8,
        index: u8,
        length: u8,
    },
    Found {
        count: u8,
    },
    GearList(Vec<GearData>),
    GearData(GearData),
}
 */

#[derive(Serialize, Debug)]
struct ScanUpdate {
    current_address: u8,
    new_address: u8,
    index: u8,
    length: u8,
}

struct DaliCommandRequest {
    pub cmd: DaliCommands,
    pub reply: oneshot::Sender<DaliCommandStatus>,
}

async fn cmd_thread(
    driver: SyncDriver,
    ctxt: Arc<IdentificationCtxt>,
    mut cmd_req: mpsc::Receiver<DaliCommandRequest>,
    cmd_log: CmdLog,
) {
    let mut step_gear = false;
    let mut current_high = false;
    let start_blink = Fuse::terminated();
    tokio::pin!(start_blink);
    let tick_blink = Fuse::terminated();
    tokio::pin!(tick_blink);
    loop {
        tokio::select! {
            res = cmd_req.recv() => {
                match res {
                    Some(cmd) => {
                        match handle_commands(&driver, &ctxt, cmd.cmd, &mut current_high).await {
                            Err(e) => {
                                error!("Command failed: {}", e);
                                let _ = cmd.reply.send(DaliCommandStatus::Failed);
                            }
                            Ok(status) => {
                                let _ = cmd.reply.send(status);
                            }
                        }
                        step_gear = ctxt.modify_state(|state| state.current_gear != state.target_gear);

                    }
                    None => break
                }
            }
            res = cmd_log.notify() =>
            {
                debug!("Reply for id {}", res);
            }
            _ = &mut start_blink => {
                tick_blink.set(tokio::time::sleep(Duration::from_millis(500)).fuse());
            }
            _ = &mut tick_blink => {
                tick_blink.set(tokio::time::sleep(Duration::from_millis(300)).fuse());

                let addr: Option<u8> = ctxt.get_current_gear(|gear| gear.and_then(|g| g.old_addr));
                if let Some(addr) = addr  {
                    if let Err(e) = {
                        if current_high {
                            set_low_level(&driver,
                                          ctxt.low_level.load(Ordering::Relaxed),
                                          &Address::Short(Short::new(addr))).await
                        } else {
                            set_high_level(&driver,
                       ctxt.high_level.load(Ordering::Relaxed),
                                           &Address::Short(Short::new(addr))).await
                        }
                    } {
                        error!("Failed to blink: {}",e);
                    }
                }
            }
            _ = future::ready(()), if step_gear => {

                start_blink.set(Fuse::terminated());
                tick_blink.set(Fuse::terminated());

                if current_high {
            if let Some(addr) = ctxt.get_current_gear(|gear| gear.and_then(|g| g.old_addr)) {
                        if let Err(e) =
                            set_high_level(&driver,
                                           ctxt.high_level.load(Ordering::Relaxed),
                                           &Address::Short(Short::new(addr))).await {
                                error!("Failed to set high level: {}",e);
                            }
                    }
                }
                let (step_up,addr)  = ctxt.modify_state(|state| {
                    if state.current_gear < state.target_gear {
                        state.current_gear += 1;

                        (true, state.gears.get(state.current_gear).and_then(|g| g.old_addr))
                    } else {
                        state.current_gear -= 1;
                        (true, state.gears.get(state.current_gear+1).and_then(|g| g.old_addr))
                    }});
                if let Some(addr) = addr {
                    if step_up {
                        if let Err(e) =
                            set_high_level(&driver,
                                           ctxt.high_level.load(Ordering::Relaxed), &Address::Short(Short::new(addr))).await {
                                error!("Failed to set high level: {}",e);
                            }
                    } else {
                        if let Err(e) =
                            set_low_level(&driver,
                                          ctxt.low_level.load(Ordering::Relaxed), &Address::Short(Short::new(addr))).await {
                                error!("Failed to set high level: {}",e);
                            }
                    }
                }
                step_gear = ctxt.get_state(|s| s.current_gear != s.target_gear);

                start_blink.set(tokio::time::sleep(Duration::from_millis(1000)).fuse());
            }
        }
    }
}

#[derive(Parser, Debug)]
// Identify DALI gear
struct CmdArgs {
    // Select DALI-device
    #[arg(short = 'd', long, default_value = "default")]
    device: String,
    // Low DALI level
    #[arg(long, default_value_t = MASK)]
    low: u8,
    // Low DALI level
    #[arg(long, default_value_t = MASK)]
    high: u8,

    /// Bind HTTP-server to this address
    #[arg(long)]
    http_address: Option<IpAddr>,
    /// HTTP port
    #[arg(long, default_value_t = 0)]
    http_port: u16,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    if let Err(e) = dali::drivers::init() {
        error!("Failed to initialize DALI drivers: {}", e);
    }
    let args = CmdArgs::parse();

    if args.low > MASK {
        error!("Low level out of range");
        return ExitCode::FAILURE;
    }
    if args.high > MASK {
        error!("High level out of range");
        return ExitCode::FAILURE;
    }

    debug!("Low: {} High: {}", args.low, args.high);
    let id_ctxt = IdentificationCtxt::new();
    let driver = match dali::drivers::open(&args.device) {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to open DAIL device: {}", e);
            if let OpenError::NotFound = e {
                info!("Available drivers:");
                for name in dali::drivers::driver_names() {
                    info!("  {}", name);
                }
            }
            return ExitCode::FAILURE;
        }
    };
    let driver = Arc::new(Mutex::new(driver));
    let id_ctxt = Arc::new(id_ctxt);
    let cmd_log = CmdLog::new();

    let (cmd_req_tx, cmd_req_rx) = mpsc::channel(10);
    let cmd_join = tokio::spawn(cmd_thread(
        driver.clone(),
        id_ctxt.clone(),
        cmd_req_rx,
        cmd_log.clone(),
    ));
    let mut conf = ServerConfig::new();
    if let Some(addr) = args.http_address {
        conf = conf.bind_addr(addr);
    }
    conf = conf.port(args.http_port);

    conf = conf.build_page(Box::new(move |req| {
        decode_get_request(req, cmd_req_tx.clone(), &id_ctxt, cmd_log.clone())
    }));
    let (server, addr, port) = httpd::start(conf).unwrap();
    let url = format!("http://{}:{}", addr, port);
    info!("Started server at {}", url);
    tokio::select! {
        res = server => {
            if let Err(e) = res {
                error!("server error: {e}");
                return ExitCode::FAILURE;

            }
        }
    }
    cmd_join.await.unwrap();
    debug!("main done");
    ExitCode::SUCCESS
}
