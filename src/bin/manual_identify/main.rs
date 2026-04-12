use std::collections::BTreeMap;
use std::error::Error;
use std::fs::File;
use std::future::{self};
use std::net::IpAddr;
use std::ops::RangeBounds;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::ExitCode;
use std::str::FromStr;
use std::sync::atomic::{self, AtomicU16};
use std::sync::{Arc, Mutex as BlockingMutex, RwLock};
use std::task::{Context, Poll};

use clap::Parser;
use futures::future::{Fuse, FutureExt};
use log::{debug, error, info};
use std::future::Future;

use hyper::{Body, Request, Response};
use hyper::{header, http};
use serde::Serialize;
use serde::Serializer;
use serde_derive::Serialize;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::time::Duration;

use dali::common::defs::MASK;
use dali::drivers::driver::OpenError;
use dali::httpd::{self, ServerConfig};
//use dali::utils::filtered_vec::FilteredVec;
use dali_tools as dali;
mod configuration;
mod dali_ident;
use configuration::{
    ConfigurationDriver, ConfigurationId, ConfigurationInfo, GearConfiguration, GearId,
};
use dali_ident::DaliConfigurationDriver;

type DynResult<T> = Result<T, Box<dyn Error + Send + Sync>>;
//type DynResultFuture<T> = dyn Future<Output = Result<T, Box<dyn Error + Send + Sync>>>;

type SyncConfigurationDriver = Arc<dyn ConfigurationDriver>;

#[derive(Clone, Debug, Serialize)]
pub enum ConfigurationState {
    Unavailable,
    Unconfigured,                 // Present but no known configuration
    CurrentConf(ConfigurationId), // The gear has been automatically matched to this configuration
    NewConf(ConfigurationId),     // Change to this configuration when commited
}

impl Serialize for GearId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(Into::into(self.clone()))
    }
}
impl Serialize for ConfigurationId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u16(Into::into(self.clone()))
    }
}
#[derive(Clone, Debug, Serialize)]
struct GearData {
    id: GearId,
    label: String,
    conf: ConfigurationState,
}

struct GearState {
    current_gear: usize,           // Index of currently selected gear
    target_gear: usize,            // Index of gear to move selection to
    lowest_configured_gear: usize, // All gears at this index and up is configured
    gears: Vec<GearData>,
    configurations: Vec<ConfigurationInfo>,
}

pub struct IdentificationCtxt {
    state: RwLock<GearState>,
}

impl IdentificationCtxt {
    pub fn new() -> IdentificationCtxt {
        Self::default()
    }

    fn get_state<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&GearState) -> R,
    {
        let state = self.state.read().unwrap();
        f(&state)
    }

    fn modify_state<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut GearState) -> R,
    {
        let mut state = self.state.write().unwrap();
        f(&mut state)
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

impl Default for IdentificationCtxt {
    fn default() -> Self {
        let state = GearState {
            current_gear: 0,
            target_gear: 0,
            lowest_configured_gear: 0,
            gears: Vec::new(),
            configurations: Vec::new(),
        };
        IdentificationCtxt {
            state: RwLock::new(state),
        }
    }
}

fn reply_scan_update(ctxt: &IdentificationCtxt) -> String {
    ctxt.get_state(|state| {
        let index = state.current_gear;
        let gears = &state.gears;
        let length = gears.len();
        let gear_id;
        let new_conf;
        let gear_id_label: String;
        let new_conf_label: String;
        if index < length
            && let Some(GearData {
                id, conf, label, ..
            }) = gears.get(usize::from(index))
        {
            gear_id = Some(id.clone());
            new_conf = conf.clone();
            gear_id_label = label.clone();
            match conf {
                ConfigurationState::CurrentConf(id) | ConfigurationState::NewConf(id) => {
                    if let Some(info) = state.configurations.iter().find(|c| c.id == *id) {
                        new_conf_label = info.label.clone();
                    } else {
                        new_conf_label = "-".to_string();
                    }
                }
                _ => {
                    new_conf_label = "-".to_string();
                }
            }
        } else {
            gear_id = None;
            new_conf = ConfigurationState::Unconfigured;
            gear_id_label = "-".to_string();
            new_conf_label = "-".to_string();
        }
        serde_json::to_string(&ScanUpdate {
            gear_id: gear_id.map(|i| Into::<u16>::into(i)),
            gear_id_label,
            new_conf,
            new_conf_label,
            index: index as u16,
            length: gears.len() as u16,
        })
        .unwrap()
    })
}

fn reply_configuration(ctxt: &IdentificationCtxt, index: u16) -> DynResult<String> {
    let (conf_info, length) = ctxt.get_state(|state| {
        (
            state.configurations.get(index as usize).cloned(),
            state.configurations.len(),
        )
    });
    let Some(conf_info) = conf_info else {
        return Err("Configuration index out of bounds".into());
    };
    serde_json::to_string(&ConfigurationInfoUpdate {
        conf_id: conf_info.id,
        conf_label: conf_info.label,
        index: index as u16,
        length: length as u16,
    })
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
}

async fn clear_scan(
    driver: &SyncConfigurationDriver,
    ctxt: &Arc<IdentificationCtxt>,
) -> DynResult<()> {
    driver.set_all_low().await?;

    if let Some(id) = ctxt.get_current_gear(|gear| gear.and_then(|g| Some(g.id.clone()))) {
        driver.set_high(id).await?;
    }
    Ok(())
}

async fn swap_gears(
    driver: &SyncConfigurationDriver,
    ctxt: &Arc<IdentificationCtxt>,
    gear_index1: usize,
    gear_index2: usize,
) -> DynResult<()> {
    let (current_gear, id1, id2) = ctxt.modify_state(|s| {
        (s.gears[gear_index1], s.gears[gear_index2]) =
            (s.gears[gear_index2].clone(), s.gears[gear_index1].clone());
        (
            s.current_gear,
            s.gears[gear_index1].id.clone(),
            s.gears[gear_index2].id.clone(),
        )
    });
    if gear_index1 > current_gear {
        driver.set_low(id1).await?;
    } else {
        driver.set_high(id1).await?;
    }
    if gear_index2 > current_gear {
        driver.set_low(id2).await?;
    } else {
        driver.set_high(id2).await?;
    }

    Ok(())
}

async fn handle_commands(
    driver: &SyncConfigurationDriver,
    ctxt: &Arc<IdentificationCtxt>,
    cmd: DaliCommands,
    current_high: &mut bool,
) -> DynResult<DaliCommandStatus> {
    match cmd {
        DaliCommands::NoCommand => {}
        DaliCommands::ScanIndex { index } => {
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
            let cb_ctxt = ctxt.clone();
            driver
                .find_all(Box::new(move |gi| {
                    let gd = GearData {
                        id: gi.id,
                        label: gi.label,
                        conf: match gi.conf {
                            Some(id) => ConfigurationState::CurrentConf(id),
                            None => ConfigurationState::Unconfigured,
                        },
                    };
                    cb_ctxt.modify_state(|s| {
                        s.gears.push(gd);
                        s.lowest_configured_gear = s.gears.len();
                    })
                }))
                .await?;
            clear_scan(driver, ctxt).await?;
            let configurations = driver.configurations();
            ctxt.modify_state(|s| {
                s.configurations = configurations;
            });
            *current_high = true;
        }
        /*
            DaliCommands::RequestScanUpdate => {
                reply_scan_update(cmd_req.reply, ctxt)?;
        }
        */
        DaliCommands::NewConfiguration { index, conf_id } => {
            let (id_high, id_low) = ctxt.modify_state(|state| {
                let gears = &mut state.gears;
                let index = usize::from(index);
                if index < gears.len() {
                    debug!("new_conf = {:?}", conf_id);
                    gears[index].conf = ConfigurationState::NewConf(conf_id);
                    if index + 1 < state.lowest_configured_gear {
                        let replaced = gears[state.lowest_configured_gear - 1].clone();
                        state.lowest_configured_gear -= 1;
                        gears[state.lowest_configured_gear] = gears[index].clone();
                        gears[index] = replaced.clone();
                        state.target_gear = state.lowest_configured_gear - 1;
                        return (
                            Some(replaced.id),
                            Some(gears[state.lowest_configured_gear].id.clone()),
                        );
                    }
                }
                (None, None)
            });
	    if let Some(id) = id_low {
		driver.set_low(id).await?;
	    }
	    if let Some(id) = id_high {
		driver.set_high(id).await?;
	    }
        }
        DaliCommands::CommitChanges => {
            let gears = ctxt.get_state(|state| state.gears.clone());
            let mut gear_conf = Vec::new();
            for gear in gears {
                match gear.conf {
                    ConfigurationState::NewConf(conf) => {
                        let id = gear.id;
                        gear_conf.push(GearConfiguration { id, conf });
                    }
                    _ => {}
                }
            }
            driver.commit(gear_conf).await?;
        } /*
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
          }*/ /*
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
        f(&mut self.0.lock().unwrap())
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

fn get_int_arg<B>(args: &BTreeMap<&str, &str>, name: &str, bounds: B) -> Result<i32, ResponseError>
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
                for kv in query_parts {
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
            DaliCommands::SCAN_INDEX => {
                let index = get_int_arg(&args, "index", 0..=63)? as u16;
                DaliCommands::ScanIndex { index }
            }
            DaliCommands::FIND_ALL => DaliCommands::FindAll,
            DaliCommands::NEW_CONFIGURATION => {
                let conf_id =
                    ConfigurationId::try_from(get_int_arg(&args, "id", 1..)? as u16).unwrap();
                let index = get_int_arg(&args, "index", 0..)? as u16;
                DaliCommands::NewConfiguration { conf_id, index }
            }
            DaliCommands::COMMIT_CHANGES => DaliCommands::CommitChanges,
            //DaliCommands::SORT => DaliCommands::Sort,
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
            cmd,
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
            } else if !log.is_empty() {
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
    } else if req.uri().path() == "/dyn/configuration" {
        if let Ok(index) = u16::from_str(req.uri().query().unwrap_or(""))
            && let Ok(reply) = reply_configuration(ctxt, index)
        {
            Response::builder()
                .status(http::StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(reply))
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        } else {
            Response::builder()
                .status(http::StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Invalid configuration index"))
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        }
    } else {
        Response::builder()
            .status(http::StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(Body::from("No such command".to_string()))
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }
}

/*
#[derive(Serialize, Deserialize)]
enum DaliSetIntensity {
    Intensity(u8),
    Low(bool),
    High(bool),
}
 */

#[derive(Clone, Debug)]
enum DaliCommands {
    NoCommand,
    ScanIndex {
        index: u16,
    },
    FindAll,
    NewConfiguration {
        index: u16,
        conf_id: ConfigurationId,
    },
    CommitChanges,
    //Sort,
}

impl DaliCommands {
    pub const NO_COMMAND: u32 = 0;
    pub const SCAN_INDEX: u32 = 1;
    pub const FIND_ALL: u32 = 2;
    pub const NEW_CONFIGURATION: u32 = 3;
    pub const COMMIT_CHANGES: u32 = 4;
    //pub const SORT: u32 = 5;
}

impl serde::Serialize for DaliCommands {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use DaliCommands::*;
        serializer.serialize_u32(match self {
            NoCommand => Self::NO_COMMAND,
            ScanIndex { .. } => Self::SCAN_INDEX,
            FindAll => Self::FIND_ALL,
            NewConfiguration { .. } => Self::NEW_CONFIGURATION,
            CommitChanges => Self::COMMIT_CHANGES,
            //Sort => Self::SORT,
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
    gear_id: Option<u16>,
    gear_id_label: String,
    new_conf: ConfigurationState,
    new_conf_label: String,
    index: u16,
    length: u16,
}

#[derive(Serialize, Debug)]
struct ConfigurationInfoUpdate {
    conf_id: ConfigurationId,
    conf_label: String,
    index: u16,
    length: u16,
}

struct DaliCommandRequest {
    pub cmd: DaliCommands,
    pub reply: oneshot::Sender<DaliCommandStatus>,
}

async fn cmd_thread(
    driver: SyncConfigurationDriver,
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

                let gear_id: Option<GearId> = ctxt.get_current_gear(|gear| gear.and_then(|g| Some(g.id.clone())));
                if let Some(gear_id) = gear_id  {
                    if current_high {
                        current_high = false;
            if let Err(e) = driver.set_low(gear_id).await {
                error!("Failed to set low level while blinking: {e}");
            }
                    } else {
                        current_high = true;
                        if let Err(e) = driver.set_high(gear_id).await {
                error!("Failed to set low level while blinking: {e}");
            }
                    }
                } else {
                    error!("Failed to blink");
                }
            }
            _ = future::ready(()), if step_gear => {

                start_blink.set(Fuse::terminated());
                tick_blink.set(Fuse::terminated());

                if current_high
            && let Some(gear_id) = ctxt.get_current_gear(|gear| gear.and_then(|g| Some(g.id.clone()))) {
            if let Err(e) =
                driver.set_high(gear_id).await {
                error!("Failed to set high level: {}",e);
                }
            current_high = true;
            }

                let (step_up,gear_id)  = ctxt.modify_state(|state| {
                    if state.current_gear < state.target_gear {
                        state.current_gear += 1;

                        (true, state.gears.get(state.current_gear as usize).and_then(|g| Some(g.id.clone())))
                    } else {
                        state.current_gear -= 1;
                        (false, state.gears.get(state.current_gear as usize +1).and_then(|g| Some(g.id.clone())))
                    }});
                if let Some(gear_id) = gear_id {
                    if step_up {
                        if let Err(e) =
                            driver.set_high(gear_id).await {
                error!("Failed to set high level: {}",e);
                }
            current_high = true;
            } else if let Err(e) =
            driver.set_low(gear_id).await {
                error!("Failed to set high level: {}",e);
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

    #[arg(long, short = 'c')]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt::init();
    if let Err(e) = dali::drivers::init() {
        error!("Failed to initialize DALI drivers: {}", e);
    }
    let args = CmdArgs::parse();

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
    let mut driver = DaliConfigurationDriver::new(driver);
    if let Some(filename) = args.config {
        let conf_file = match File::open(&filename) {
            Ok(f) => f,
            Err(e) => {
                error!("Failed to open {}: {}", filename.to_string_lossy(), e);
                return ExitCode::FAILURE;
            }
        };
        if let Err(e) = driver.read_config(conf_file) {
            error!("Failed to openread {}: {}", filename.to_string_lossy(), e);
            return ExitCode::FAILURE;
        }
    }
    let driver = Arc::new(driver);
    let id_ctxt = Arc::new(id_ctxt);
    let cmd_log = CmdLog::new();

    if let Err(e) = driver.start_configuration().await {
        error!("Failed to start configuration driver: {e}");
        return ExitCode::FAILURE;
    }

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
    if let Err(e) = driver.end_configuration().await {
        error!("Failed to stop configuration driver: {e}");
        return ExitCode::FAILURE;
    }

    cmd_join.await.unwrap();
    debug!("main done");
    ExitCode::SUCCESS
}
