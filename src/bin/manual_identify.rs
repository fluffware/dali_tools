use std::error::Error;
use std::future::{self};
use std::net::IpAddr;
use std::process::ExitCode;
use std::sync::{Arc, Mutex as BlockingMutex};

use bytes::Bytes;
use clap::Parser;
use futures::future::{Fuse, FutureExt};
use log::{debug, error, info};

use serde_derive::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::Duration;

use dali::base::address::{Address, Long, Short,BusAddress};
use dali::base::status::GearStatus;
use dali::defs::common::MASK;
use dali::defs::gear::cmd;
use dali::drivers::command_utils::{send_device_cmd, send_device_level};
use dali::drivers::driver::OpenError;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
use dali::drivers::driver_utils::DaliDriverExt;
use dali::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG, PRIORITY_1, /*PRIORITY_5,*/ SEND_TWICE};
use dali::httpd::{self, ServerConfig};
//use dali::utils::filtered_vec::FilteredVec;
use dali_tools as dali;

type DynResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

async fn set_search_addr(driver: &mut dyn DaliDriver, addr: Long) -> Result<u8, DaliSendResult> {
    let res = driver.send_frame16(&[cmd::SEARCHADDRH, (addr >> 16 & 0xff) as u8], PRIORITY_1);
    res.await.check_send()?;

    let res = driver.send_frame16(&[cmd::SEARCHADDRM, (addr >> 8 & 0xff) as u8], PRIORITY_1);
    res.await.check_send()?;
    let res = driver.send_frame16(&[cmd::SEARCHADDRL, (addr & 0xff) as u8], PRIORITY_1);
    res.await.check_send()?;
    Ok(0)
}

async fn query_long_addr(
    driver: &mut dyn DaliDriver,
    short_addr: Short,
) -> Result<Long, DaliSendResult> {
    let h = send_device_cmd(
        driver,
        &short_addr,
        cmd::QUERY_RANDOM_ADDRESS_H,
        EXPECT_ANSWER,
    )
    .await
    .check_answer()?;
    let m = send_device_cmd(
        driver,
        &short_addr,
        cmd::QUERY_RANDOM_ADDRESS_M,
        EXPECT_ANSWER,
    )
    .await
    .check_answer()?;
    let l = send_device_cmd(
        driver,
        &short_addr,
        cmd::QUERY_RANDOM_ADDRESS_L,
        EXPECT_ANSWER,
    )
    .await
    .check_answer()?;
    Ok((h as u32) << 16 | (m as u32) << 8 | (l as u32))
}

async fn program_short_address(
    driver: &mut dyn DaliDriver,
    long: Long,
    short: Short,
) -> Result<(), DaliSendResult> {
    set_search_addr(driver, long).await?;
    driver
        .send_frame16(
            &[cmd::PROGRAM_SHORT_ADDRESS, short.bus_address() | 1],
            NO_FLAG,
        )
        .await
        .check_send()?;
    let a = driver
        .send_frame16(&[cmd::QUERY_SHORT_ADDRESS, 0x00], EXPECT_ANSWER)
        .await
        .check_answer()?;
    println!("Set {}, got {}", short, a>>1 + 1);
    Ok(())
}

async fn swap_addr(
    driver: &SyncDriver,
    addr1: Short,
    addr2: Short,
) -> Result<(), DaliSendResult> {
    let driver = &mut **driver.lock().await;
    let long1 = match query_long_addr(driver, addr1).await {
        Ok(a) => Some(a),
        Err(DaliSendResult::Timeout) => None,
        Err(e) => return Err(e),
    };
    println!("{}: 0x{:?}", addr1, long1);
    let long2 = match query_long_addr(driver, addr2).await {
        Ok(a) => Some(a),
        Err(DaliSendResult::Timeout) => None,
        Err(e) => return Err(e),
    };
    println!("{}: 0x{:?}", addr2, long2);
    driver
        .send_frame16(&[cmd::INITIALISE, cmd::INITIALISE_ALL], SEND_TWICE)
        .await
        .check_send()?;
    if let Some(l) = long1 {
        program_short_address(driver, l, addr2).await?;
    }
    if let Some(l) = long2 {
        program_short_address(driver, l, addr1).await?;
    }
    driver
        .send_frame16(&[cmd::TERMINATE, 0], NO_FLAG)
        .await
        .check_send()?;
    Ok(())
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
    gears: BlockingMutex<Vec<GearData>>,
    low_level: u8,  // Set to MASK for min level
    high_level: u8, // Set to MASK for max level
    current_high: bool,
}

type SyncDriver = Arc<Mutex<Box<dyn DaliDriver>>>;

async fn find<F>(driver: &SyncDriver, found: &mut F) -> DynResult<()>
where
    F: FnMut(GearData),
{
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
    let gears = ctxt.gears.lock().unwrap();
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
    };

    send.send(Bytes::from(
        serde_json::to_string(&DaliReplies::ScanUpdate {
            current_address,
            new_address,
            index,
            length,
        })
        .unwrap(),
    ))?;
    Ok(())
}

fn send_gear(send: &mut broadcast::Sender<Bytes>, gear: &GearData) -> DynResult<()> {
    send.send(Bytes::from(
        serde_json::to_string(&DaliReplies::GearAdded {
            long_address: gear.long,
            current_address: gear.old_addr,
            new_address: gear.new_addr,
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
            {
                let mut gears = ctxt.gears.lock().unwrap();
                gears.clear();
            }
            find(driver, &mut |gd| {
                ctxt.gears.lock().unwrap().push(gd);
                send_gear(send, &gd).unwrap();
            })
            .await?;
            ctxt.current_gear = 0;
            ctxt.target_gear = 0;
            send_scan_update(send, ctxt)?;
            set_low_level(driver, ctxt, &Address::Broadcast).await?;
            let addr = if let Some(&GearData {
                old_addr: Some(addr),
                ..
            }) = ctxt.gears.lock().unwrap().get(0)
            {
                Some(addr)
            } else {
                None
            };
            if let Some(addr) = addr {
                set_high_level(driver, ctxt, &Address::Short(Short::new(addr))).await?;
                ctxt.current_high = true;
            }
        }
        DaliCommands::RequestScanUpdate(_) => {
            send_scan_update(send, ctxt)?;
        }
        DaliCommands::NewAddress { address, index } => {
            {
                let mut gears = ctxt.gears.lock().unwrap();
                if let Some(gear) = gears.get_mut(usize::from(index)).as_mut() {
                    if address >= 1 && address <= 64 {
                        gear.new_addr = Some(address);
                    }
                }
            }
            send_scan_update(send, ctxt)?;
        }
        DaliCommands::ChangeAddresses(_) => {
	    let mut swaps = Vec::new();
	    {
		let mut gears = ctxt.gears.lock().unwrap();
		for g in gears.iter_mut() {
		    if let (Some(old_addr), Some(new_addr)) = (g.old_addr, g.new_addr) {
			swaps.push((old_addr,new_addr));
			g.old_addr = g.new_addr.take();
		}
		}
	    }
	    for swap in swaps {
		swap_addr(driver, Short::new(swap.0), Short::new(swap.1)).await?;
	    }
        }
        DaliCommands::RequestGearList(_) => {
            let gears = ctxt.gears.lock().unwrap();
            for g in gears.iter() {
                send_gear(send, g)?;
            }
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize)]
enum DaliSetIntensity {
    Intensity(u8),
    Low(bool),
    High(bool),
}

#[derive(Serialize, Deserialize)]
enum DaliCommands {
    ScanAddress(u8),
    FindAll(bool),
    RequestScanUpdate(bool),
    RequestGearList(bool),
    NewAddress { address: u8, index: u8 },
    ChangeAddresses(bool),
}

#[derive(Serialize, Deserialize)]
enum DaliReplies {
    ScanUpdate {
        current_address: u8,
        new_address: u8,
        index: u8,
        length: u8,
    },
    GearAdded {
        long_address: u32,
        current_address: Option<u8>,
        new_address: Option<u8>,
    },
    GearRemoved {
        long_address: u32,
    },
}

fn gear_old_addr(ctxt: &IdentificationCtxt, index: usize) -> Option<Address> {
    if let Some(&GearData {
        old_addr: Some(addr),
        ..
    }) = ctxt.gears.lock().unwrap().get(index)
    {
        Some(Address::Short(Short::new(addr)))
    } else {
        None
    }
}

async fn cmd_thread(
    driver: SyncDriver,
    ctxt: Arc<Mutex<IdentificationCtxt>>,
    mut send: broadcast::Sender<Bytes>,
    recv: mpsc::Receiver<Bytes>,
) {
    let mut step_gear = false;
    tokio::pin!(recv);
    let start_blink = Fuse::terminated();
    tokio::pin!(start_blink);
    let tick_blink = Fuse::terminated();
    tokio::pin!(tick_blink);
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
            _ = &mut start_blink => {
                tick_blink.set(tokio::time::sleep(Duration::from_millis(500)).fuse());
            }
            _ = &mut tick_blink => {
                let mut ctxt = ctxt.lock().await;
                tick_blink.set(tokio::time::sleep(Duration::from_millis(300)).fuse());
                let addr = if let Some(&GearData{old_addr: Some(addr), ..})
                    = ctxt.gears.lock().unwrap().get(ctxt.current_gear) {
                        Some(addr)
                    }else {
                        None

                    };

                if let Some(addr) = addr  {
                    if let Err(e) = {
                            if ctxt.current_high {
                                ctxt.current_high = false;
                                set_low_level(&driver,
                                              &mut *ctxt,
                                              &Address::Short(Short::new(addr))).await
                            } else {
                                ctxt.current_high = true;
                                set_high_level(&driver,
                                               &mut *ctxt,
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
                let mut ctxt = ctxt.lock().await;
                if !ctxt.current_high {
                    if let Some(addr) = gear_old_addr(&ctxt, ctxt.current_gear) {
                        if let Err(e) =
                            set_high_level(&driver,
                                           &mut *ctxt,
                                           &addr).await {
                                error!("Failed to set high level: {}",e);
                            }
                    }
                }
                if ctxt.current_gear < ctxt.target_gear {
                    ctxt.current_gear += 1;
                    if let Some(addr) = gear_old_addr(&ctxt, ctxt.current_gear) {
                        if let Err(e) =
                            set_high_level(&driver,
                                           &mut *ctxt, &addr).await {
                            error!("Failed to set high level: {}",e);
                        }
                    }
                } else {
                    if let Some(addr) = gear_old_addr(&ctxt, ctxt.current_gear) {
                        if let Err(e) =
                            set_low_level(&driver,
                                           &mut *ctxt, &addr).await {
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
    let id_ctxt = IdentificationCtxt {
        gears: BlockingMutex::new(Vec::new()),
        current_gear: 0,
        target_gear: 0,
        low_level: args.low,
        high_level: args.high,
        current_high: false,
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
