use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use dali::drivers::driver::OpenError;
use dali::drivers::driver_utils::DaliDriverExt;
use dali::drivers::send_flags::SEND_TWICE;
use dali::gear::cmd_defs as cmd;
use dali::utils::address_assignment::clear_short_address;
use dali::utils::discover;

use dali_tools as dali;
extern crate clap;
use clap::{Arg, Command};

#[tokio::main]
async fn main() {
    if let Err(e) = dali::drivers::init() {
        println!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = Command::new("discover")
        .about("Discover all devices on a DALI bus")
        .arg(
            Arg::new("DEVICE")
                .short('d')
                .long("device")
                .default_value("default")
                .help("Select DALI-device"),
        )
        .arg(
            Arg::new("clear_conflicts")
                .long("clear-conflicts")
                .action(clap::ArgAction::SetTrue)
                .help("Clear the short address for devices with duplicate addresses"),
        )
        .get_matches();

    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let clear_conflicts = *matches.get_one::<bool>("clear_conflicts").unwrap();
    let driver = match dali::drivers::open(device_name) {
        Ok(d) => d,
        Err(e) => {
            println!("Failed to open DALI device: {}", e);
            if let OpenError::NotFound = e {
                println!("Available drivers:");
                for name in dali::drivers::driver_names() {
                    println!("  {}", name);
                }
            }
            return;
        }
    };

    let driver = Arc::new(Mutex::new(driver));
    let mut discovered = discover::find_quick(driver.clone());

    let mut short_conflicts = Vec::new();
    while let Some(res) = discovered.next().await {
        match res {
            Ok(device) => {
                println!(
                    "Long: {}, Short: {} {}{}",
                    if let Some(long) = device.long {
                        long.to_string()
                    } else {
                        "None".to_string()
                    },
                    if let Some(short) = device.short {
                        short.to_string()
                    } else {
                        "None".to_string()
                    },
                    if device.short_conflict {
                        ", Short address conflict"
                    } else {
                        ""
                    },
                    if device.long_conflict {
                        ", Long address conflict"
                    } else {
                        ""
                    },
                );
                if device.short_conflict {
                    short_conflicts.push(device);
                }
            }
            Err(e) => println!("Discovery failed: {}", e),
        }
    }
    if clear_conflicts {
        let mut driver = driver.lock().await;
        (*driver)
            .send_frame16(&[cmd::INITIALISE, cmd::INITIALISE_ALL], SEND_TWICE)
            .await;
        for d in short_conflicts {
            if let Some(long) = d.long {
                if let Err(e) = clear_short_address(driver.as_mut(), long).await {
                    println!(
                        "Failed to clear short address for long address {}: {}",
                        long, e,
                    );
                }
            }
        }
    }
}
