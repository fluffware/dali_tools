use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

use dali::drivers::driver::OpenError;
use dali::drivers::send_flags::PRIORITY_1;
use dali::gear::commands_102::Commands102;
use dali::utils::address_assignment::{clear_short_address, program_short_address};
use dali::utils::discover;
use dali_tools as dali;
use dali_tools::common::commands::Commands;
use dali_tools::common::driver_commands::DriverCommands;
use dali_tools::gear::address::Short;
use dali_tools::utils::address_set::AddressSet;

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
        .arg(
            Arg::new("allocate")
                .long("allocate")
                .action(clap::ArgAction::SetTrue)
                .help("Allocate addresses for devices with no address"),
        )
        .get_matches();

    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let clear_conflicts = *matches.get_one::<bool>("clear_conflicts").unwrap();
    let allocate = *matches.get_one::<bool>("allocate").unwrap();
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
    let mut discovered = discover::find_quick::<Commands102>(driver.clone());

    let mut allocated_addrs = AddressSet::new();
    let mut short_conflicts = Vec::new();
    let mut unallocated = Vec::new();
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
                    short_conflicts.push(device.clone());
                }
                if let Some(addr) = device.short {
                    allocated_addrs.insert(addr);
                } else {
                    unallocated.push(device);
                }
            }
            Err(e) => eprintln!("Discovery failed: {}", e),
        }
    }
    let mut driver = driver.lock().await;
    let mut commands = Commands102::from_driver(driver.as_mut(), PRIORITY_1);
    if clear_conflicts && !short_conflicts.is_empty() {
        let _ = commands.initialise_all().await;
        for d in short_conflicts {
            if let Some(long) = d.long
                && let Err(e) = clear_short_address(&mut commands, long).await
            {
                eprintln!(
                    "Failed to clear short address for long address {}: {}",
                    long, e,
                );
            }
        }
        let _ = commands.terminate().await;
    }
    if allocate && !unallocated.is_empty() {
        let _ = commands.initialise_no_addr().await;
        let mut next = 0;
        for device in unallocated {
            while next < 64 && allocated_addrs.contains(Short::new(next)) {
                next += 1;
            }
            if next == 64 {
                eprintln!("No free addresses");
                return;
            }
            if let Some(long) = device.long {
                if let Err(e) = program_short_address(&mut commands, long, Short::new(next)).await {
                    eprintln!(
                        "Failed to program short address for long address {}: {}",
                        long, e,
                    );
                } else {
                    next += 1;
                }
            }
        }
        let _ = commands.terminate().await;
    }
}
