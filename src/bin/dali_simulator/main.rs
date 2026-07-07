use clap::{Arg, Command};
use dali::simulator;
use dali_tools as dali;
use dali_tools::simulator::sim_bus::DaliSimBusDevice;
use futures::FutureExt;
use futures::future::FusedFuture;
use log::debug;
use log::error;
use std::fs::File;
use tokio::signal;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "sim_serial")]
mod sim_serial;
#[cfg(not(feature = "sim_serial"))]
mod sim_serial {
    use dali_tools::simulator::sim_bus::DaliSimBusDevice;
    use tokio_util::sync::CancellationToken;

    pub async fn start_serial(
        bus_device: DaliSimBusDevice,
        cancel: CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error>> {
        cancel.cancelled().await;
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = dali::drivers::init() {
        eprintln!("Failed to initialize DALI drivers: {}", e);
    }
    let mut cli_cmd = Command::new("dali_simulator")
        .about("Simulate DALI-devices on a bus ")
        .arg(Arg::new("CONFIG").required(true).help("Configuration file"))
        .arg(
            Arg::new("SERIAL_DEVICE")
                .long("serial-device")
                .default_value("/dev/tnt0")
                .help("Serial device for simulation"),
        );
    let matches = cli_cmd.get_matches();
    let conf_filename = matches.get_one::<String>("CONFIG").unwrap();
    let conf_file = match File::open(conf_filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "Failed to open configuration file '{}': {}",
                conf_filename, e
            );
            return;
        }
    };
    let (bus, mut sched) = match simulator::setup::setup_simulator(conf_file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to start simulator: {}", e);
            return;
        }
    };
    let serial_path = matches.get_one::<String>("SERIAL_DEVICE").unwrap();
    let cancel = CancellationToken::new();
    let serial = sim_serial::start_serial(
        DaliSimBusDevice::new(bus, sched.new_task()),
        &serial_path,
        cancel.clone(),
    )
    .fuse();
    tokio::pin!(serial);
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);
    loop {
        tokio::select! {
                res = &mut serial => {
            if let Err(e) = res {
                error!("Serial pseudo port failed: {}",e);
            }
            break;
            }
                res = &mut ctrl_c => {
            if let Err(_) = res {
                error!("Failed to wait for Ctrl-C");
            }
            break;
                }
        }
    }
    cancel.cancel();
    debug!("Cancelled");
    if !serial.is_terminated() {
        let _ = serial.await;
    }
    debug!("Exiting");
}
