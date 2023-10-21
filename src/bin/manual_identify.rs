use std::error::Error;
use std::future::{self};
use std::net::IpAddr;
use std::process::ExitCode;
use std::sync::Arc;

use bytes::Bytes;
use clap::Parser;

use log::{debug, error, info, warn};
use serde_derive::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::Duration;

use dali::base::address::{Address, Short};
use dali::base::status::GearStatus;
use dali::defs::common::MASK;
use dali::defs::gear::cmd;
use dali::drivers::command_utils::{send_device_cmd, send_device_level};
use dali::drivers::driver::OpenError;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
use dali::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG, /*PRIORITY_1, PRIORITY_5, SEND_TWICE*/};
use dali::httpd::{self, ServerConfig};
use dali_tools as dali;

type DynResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

async fn get_long_address(driver: &mut dyn DaliDriver, addr: &Short) -> DynResult<u32> {
    let low = match send_device_cmd(driver, addr, cmd::QUERY_RANDOM_ADDRESS_L, EXPECT_ANSWER).await
    {
        DaliSendResult::Answer(s) => s,
        DaliSendResult::Timeout => return Err("Timeout".into()),
        e => return Err(e.into()),
    };
    let mid = match send_device_cmd(driver, addr, cmd::QUERY_RANDOM_ADDRESS_M, EXPECT_ANSWER).await
    {
        DaliSendResult::Answer(s) => s,
        DaliSendResult::Timeout => return Err("Timeout".into()),
        e => return Err(e.into()),
    };

    let high = match send_device_cmd(driver, addr, cmd::QUERY_RANDOM_ADDRESS_H, EXPECT_ANSWER).await
    {
        DaliSendResult::Answer(s) => s,
        DaliSendResult::Timeout => return Err("Timeout".into()),
        e => return Err(e.into()),
    };

    Ok(u32::from(high) << 16 | u32::from(mid) << 8 | u32::from(low))
}

#[derive(Copy, Clone)]
pub struct GearData {
    long: u32,
    old_addr: Option<u8>,
    new_addr: Option<u8>,
}

pub struct IdentificationCtxt {
    current_gear: usize,
    target_gear: usize,
    gears: Vec<GearData>,
    low_level: u8,  // Set to MASK for min level
    high_level: u8, // Set to MASK for max level
}

type SyncDriver = Arc<Mutex<Box<dyn DaliDriver>>>;

async fn find(driver: &SyncDriver, ctxt: &mut IdentificationCtxt) -> DynResult<()> {
    for addr in 1..=64 {
        debug!("Checking {}", addr);
        let driver = &mut **driver.lock().await;
        let status = match send_device_cmd(
            driver,
            &Short::new(addr),
            cmd::QUERY_STATUS,
            EXPECT_ANSWER,
        )
        .await
        {
            DaliSendResult::Answer(s) => Some(GearStatus::new(s)),
            DaliSendResult::Timeout => None,
            e => return Err(e.into()),
        };
        if let Some(status) = status {
            info!("{}: Status: {}\r\n", addr, status);
            match get_long_address(driver, &Short::new(addr)).await {
                Ok(l) => {
                    info!("{}: Long: {:06x}\r\n", addr, l);
                    ctxt.gears.push(GearData {
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

async fn set_low_level(
    driver: &SyncDriver,
    ctxt: &IdentificationCtxt,
    addr: &Address,
) -> DynResult<()> {
    let driver = &mut **driver.lock().await;
    match if ctxt.low_level == MASK {
        send_device_cmd(driver, addr, cmd::RECALL_MIN_LEVEL, NO_FLAG).await
    } else {
        send_device_level(driver, addr, ctxt.low_level, NO_FLAG).await
    } {
        DaliSendResult::Ok => {}
        e => return Err(e.into()),
    }
    Ok(())
}

async fn set_high_level(
    driver: &SyncDriver,
    ctxt: &IdentificationCtxt,
    addr: &Address,
) -> DynResult<()> {
    let driver = &mut **driver.lock().await;
    match if ctxt.high_level == MASK {
        send_device_cmd(driver, addr, cmd::RECALL_MAX_LEVEL, NO_FLAG).await
    } else {
        send_device_level(driver, addr, ctxt.high_level, NO_FLAG).await
    } {
        DaliSendResult::Ok => {}
        e => return Err(e.into()),
    }
    Ok(())
}

fn send_scan_update(
    send: &mut broadcast::Sender<Bytes>,
    ctxt: &mut IdentificationCtxt,
) -> DynResult<()> {
    let index = ctxt.current_gear as u8;
    let length = ctxt.gears.len() as u8;
    let address = if index < length {
        if let Some(addr) = ctxt.gears[usize::from(index)].old_addr {
            addr
        } else {
            0
        }
    } else {
        0
    };
    send.send(Bytes::from(
        serde_json::to_string(&DaliReplies::ScanUpdate {
            address,
            index,
            length,
        })
        .unwrap(),
    ))?;
    Ok(())
}

async fn handle_commands(
    driver: &SyncDriver,
    ctxt: &mut IdentificationCtxt,
    send: &mut broadcast::Sender<Bytes>,
    cmd: DaliCommands,
) -> DynResult<()> {
    match cmd {
        DaliCommands::ScanAddress(addr) => {
            ctxt.target_gear = usize::from(addr);
        }
        DaliCommands::FindAll(_) => {
            ctxt.gears.clear();
            find(driver, ctxt).await?;
            ctxt.current_gear = 0;
            ctxt.target_gear = 0;
            send_scan_update(send, ctxt)?;
            set_low_level(driver, ctxt, &Address::Broadcast).await?;
            if let Some(GearData {
                old_addr: Some(addr),
                ..
            }) = ctxt.gears.get(0)
            {
                set_high_level(driver, ctxt, &Address::Short(Short::new(*addr))).await?;
            }
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
enum DaliCommands {
    ScanAddress(u8),
    FindAll(bool),
}

#[derive(Serialize, Deserialize)]
enum DaliReplies {
    ScanUpdate { address: u8, index: u8, length: u8 },
}

async fn cmd_thread(
    driver: SyncDriver,
    ctxt: Arc<Mutex<IdentificationCtxt>>,
    mut send: broadcast::Sender<Bytes>,
    recv: mpsc::Receiver<Bytes>,
) {
    let mut step_gear = false;
    tokio::pin!(recv);
    loop {
        tokio::select! {
            res = recv.recv() => {
                match res {
                    Some(data) => {
                        match std::str::from_utf8(&data[..]) {
                            Ok(json) => {
                                debug!("JSON: {}", json);
                                match serde_json::from_str::<DaliCommands>(json) {
                                    Ok(cmd) => {
                                        let mut ctxt = ctxt.lock().await;
                                        if let Err(e) = handle_commands(&driver, &mut ctxt, &mut send,cmd).await {
                                            error!("Command failed: {}", e);
                                        }
                                        step_gear = ctxt.current_gear != ctxt.target_gear;

                                    }
                                    Err(e) => {
                                        error!("Failed to parse JSON message: {e}");
                                    }
                                }
                            }
                            Err(e) => error!("Illegal UTF-8 in message from client: {}", e)

                        }
                    }
                    None=> break
                }
            }
            _ = future::ready(()), if step_gear => {

                let mut ctxt = ctxt.lock().await;
                if ctxt.current_gear < ctxt.target_gear {
                    ctxt.current_gear += 1;
                    if let Some(&GearData{old_addr: Some(addr), ..}) = ctxt.gears.get(ctxt.current_gear) {
                        if let Err(e) =
                            set_high_level(&driver,
                                           &mut *ctxt, &Address::Short(Short::new(addr))).await {
                            error!("Failed to set high level: {}",e);
                        }
                    }
                } else {
                    if let Some(&GearData{old_addr: Some(addr), ..}) = ctxt.gears.get(ctxt.current_gear) {
                        if let Err(e) =
                            set_low_level(&driver,
                                           &mut *ctxt, &Address::Short(Short::new(addr))).await {
                                error!("Failed to set high level: {}",e);
                            }
                    }
                    ctxt.current_gear -= 1;
                }
                step_gear = ctxt.current_gear != ctxt.target_gear;
                debug!("Index: {}", ctxt.current_gear);
                if let Err(e) =  send_scan_update(&mut send, &mut *ctxt)
                {
                    error!("Failed to send scan update (WS): {}",e);
                }

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
    let id_ctxt = IdentificationCtxt {
        gears: Vec::new(),
        current_gear: 0,
        target_gear: 0,
        low_level: args.low,
        high_level: args.high,
    };
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
    let id_ctxt = Arc::new(Mutex::new(id_ctxt));
    let (ws_send, dali_recv) = mpsc::channel(10);
    let (dali_send, _) = broadcast::channel(10);
    let cmd_join = tokio::spawn(cmd_thread(
        driver.clone(),
        id_ctxt.clone(),
        dali_send.clone(),
        dali_recv,
    ));
    let mut conf = ServerConfig::new();
    if let Some(addr) = args.http_address {
        conf = conf.bind_addr(addr);
    }
    conf = conf.port(args.http_port);
    conf = conf.web_socket(ws_send, dali_send);
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
