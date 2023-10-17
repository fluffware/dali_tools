use std::error::Error;
use std::future::Future;
use std::io::Write;
use std::sync::Arc;

use clap::{value_parser, Arg, Command};
use crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use futures_util::future::{select, Either};
use log::{debug, error, info, warn};
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::time::Duration;
use tokio_stream::StreamExt;

use dali::base::address::{Address, Short};
use dali::base::status::GearStatus;
use dali::defs::gear::cmd;
use dali::drivers::command_utils::send_device_cmd;
use dali::drivers::driver::OpenError;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
use dali::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG, PRIORITY_1, PRIORITY_5, SEND_TWICE};
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
    new_addr: Option<u8>,
}

#[derive(Copy, Clone)]
pub struct IdentificationCtxt {
    gears: [Option<GearData>; 64],
}

type SyncDriver = Arc<Mutex<Box<dyn DaliDriver>>>;

async fn scan<F>(
    driver: &SyncDriver,
    ctxt: &Arc<Mutex<IdentificationCtxt>>,
    cancel: F,
) -> DynResult<()>
where
    F: Future,
{
    let ctxt = ctxt.lock().await;
    let driver = &mut **driver.lock().await;
    tokio::pin!(cancel);
    'scan: loop {
        let delay = tokio::time::sleep(Duration::from_millis(500));
        tokio::pin!(delay);
        match select(delay, cancel).await {
            Either::Left((_, c)) => {
                cancel = c;
            }
            Either::Right((_, _)) => break,
        }
        match send_device_cmd(driver, &Address::Broadcast, cmd::RECALL_MIN_LEVEL, NO_FLAG).await {
            DaliSendResult::Ok => {}
            e => return Err(e.into()),
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
        debug!("Start!");
        for (i, gear) in ctxt.gears.iter().enumerate() {
            if gear.is_some() {
                {
                    match send_device_cmd(
                        driver,
                        &Short::new((i + 1) as u8),
                        cmd::RECALL_MAX_LEVEL,
                        NO_FLAG,
                    )
                    .await
                    {
                        DaliSendResult::Ok => {}
                        e => return Err(e.into()),
                    }
                }
                let delay = tokio::time::sleep(Duration::from_millis(100));
                tokio::pin!(delay);
                match select(delay, cancel).await {
                    Either::Left((_, c)) => {
                        cancel = c;
                    }
                    Either::Right((_, _)) => break 'scan,
                }
            }
        }
    }
    Ok(())
}

async fn find<F>(
    driver: &SyncDriver,
    ctxt: &Arc<Mutex<IdentificationCtxt>>,
    cancel: F,
) -> Result<(), Box<dyn Error>>
where
    F: Future,
{
    let mut stdout = std::io::stdout();
    let mut ctxt = ctxt.lock().await;
    for addr in 1..=64 {
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
            write!(&mut stdout, "{}: Status: {}\r\n", addr, status).unwrap();
            match get_long_address(driver, &Short::new(addr)).await {
                Ok(l) => {
                    write!(stdout, "{}: Long: {:06x}\r\n", addr, l).unwrap();
                    ctxt.gears[usize::from(addr - 1)] = Some(GearData {
                        long: l,
                        new_addr: None,
                    });
                }
                Err(_) => {}
            }
        }
    }
    Ok(())
}

#[derive(Clone)]
enum CmdState {
    Stop,
    Find,
    ScanFastForward,
    ScanSlowForward,
    Exit,
}

async fn cmd_thread(
    driver: SyncDriver,
    ctxt: Arc<Mutex<IdentificationCtxt>>,
    mut cmd_state: watch::Receiver<CmdState>,
) {
    loop {
        let cmd = cmd_state.borrow_and_update().clone();
        match cmd {
            CmdState::Find => match find(&driver, &ctxt, cmd_state.changed()).await {
                Ok(()) => {}
                Err(e) => {
                    error!("Find failed: {}", e);
                }
            },
            CmdState::ScanFastForward => match scan(&driver, &ctxt, cmd_state.changed()).await {
                Ok(()) => {}
                Err(e) => {
                    error!("Scan failed: {}", e);
                }
            },
            CmdState::Stop => {
                let _ = cmd_state.changed().await;
            }
            CmdState::Exit => {
                break;
            }
            _ => {
                warn!("Unknown command");
                let _ = cmd_state.changed().await;
            }
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = dali::drivers::init() {
        error!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = Command::new("identify")
        .about("Identify DALI gear")
        .arg(
            Arg::new("DEVICE")
                .short('d')
                .long("device")
                .default_value("default")
                .help("Select DALI-device"),
        )
        .arg(
            Arg::new("LOW")
                .long("low")
                .default_value("100")
                .value_parser(value_parser!(u8))
                .help("Idle level"),
        )
        .arg(
            Arg::new("HIGH")
                .long("high")
                .default_value("200")
                .value_parser(value_parser!(u8))
                .help("Mark level"),
        )
        .get_matches();

    let low = match matches.try_get_one::<u8>("LOW") {
        Ok(Some(&x)) if x <= 254 => x,
        Ok(Some(_)) => {
            error!("Low level out of range");
            return;
        }
        Ok(None) => {
            error!("Low level missing");
            return;
        }

        Err(e) => {
            error!("Low level invalid: {}", e);
            return;
        }
    };

    let high = match matches.try_get_one::<u8>("HIGH") {
        Ok(Some(&x)) if x <= 254 => x,
        Ok(Some(_)) => {
            error!("High level out of range");
            return;
        }
        Ok(None) => {
            error!("High level missing");
            return;
        }

        Err(e) => {
            error!("High level invalid: {}", e);
            return;
        }
    };
    debug!("Low: {} High: {}", low, high);
    let id_ctxt = IdentificationCtxt { gears: [None; 64] };
    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let driver = match dali::drivers::open(device_name) {
        Ok(d) => d,
        Err(e) => {
            error!("Failed to open DAIL device: {}", e);
            if let OpenError::NotFound = e {
                info!("Available drivers:");
                for name in dali::drivers::driver_names() {
                    info!("  {}", name);
                }
            }
            return;
        }
    };
    let driver = Arc::new(Mutex::new(driver));
    let id_ctxt = Arc::new(Mutex::new(id_ctxt));
    enable_raw_mode().unwrap();
    let mut events = EventStream::new();
    let (cmd_send, cmd_recv) = watch::channel(CmdState::Stop);
    let cmd_join = tokio::spawn(cmd_thread(driver.clone(), id_ctxt.clone(), cmd_recv));
    loop {
        tokio::select! {
            res = events.next() => {
                match res {
                    Some(Ok(event)) => {
                        match event {
                            Event::Key(KeyEvent{code: KeyCode::Char('c'),
                                                modifiers: KeyModifiers::CONTROL, ..}) => {
                                break;
                            }
                            Event::Key(KeyEvent{code: KeyCode::Char('q'), ..}) => {
                                break;
                            }
                            Event::Key(KeyEvent{code: KeyCode::Char('f'), ..}) => {
                                if cmd_send.send(CmdState::Find).is_err() {
                                    warn!("Failed to send find cmd");
                                }
                            }
                            Event::Key(KeyEvent{code: KeyCode::Char('s'), ..}) => {
                                if cmd_send.send(CmdState::ScanFastForward).is_err() {
                                    warn!("Failed to send scan cmd");
                                }
                            }
                            Event::Key(KeyEvent{code: KeyCode::Char(' '), ..}) => {
                                if cmd_send.send(CmdState::Stop).is_err() {
                                    warn!("Failed to send stop cmd");
                                }
                            }
                            _ => {
                                debug!("Event: {:?}", event);
                            }
                        }
                    }
                    Some(Err(e)) => {
                        error!("Error: {}", e);
                        break;
                    }
                    None => {
                        break;
                    }
                }
            }
        }
    }
    disable_raw_mode().unwrap();
    cmd_send.send(CmdState::Exit).unwrap();
    cmd_join.await.unwrap();
    debug!("main done");
}
