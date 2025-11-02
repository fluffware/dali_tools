use dali::common::address::DisplayValue;
use dali::common::address::Short;
use dali::control::commands_103::Commands103;
use dali::gear::commands_102::Commands102;
use dali::drivers::driver::OpenError;
use dali::utils::device_info;
use dali::utils::memory_banks;
use dali_tools as dali;
use dali_tools::common::commands::Commands;

extern crate clap;
use clap::{Arg, Command, value_parser};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = dali::drivers::init() {
        eprintln!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = Command::new("query_device")
        .about("Query one or more DALI gears for various information.")
        .arg(
            Arg::new("DEVICE")
                .short('d')
                .long("device")
                .default_value("default")
                .help("Select DALI-device"),
        )
        .arg(
            Arg::new("ADDR")
                .required(true)
                .value_parser(value_parser!(u8))
                .help("Address"),
        )
        .arg(
            Arg::new("END_ADDR")
                .required(false)
                .value_parser(value_parser!(u8))
                .help("End address"),
        )
        .arg(
            Arg::new("memory_banks")
                .short('m')
                .long("memory-banks")
                .value_parser(value_parser!(bool))
                .default_value("false")
                .default_missing_value("true")
                .help("Read information from memory banks"),
        )
        .arg(
            Arg::new("control")
                .short('c')
                .long("control")
                .action(clap::ArgAction::SetTrue)
                .help("Read info from control devices"),
        )
	.arg(
            Arg::new("try-all")
                .long("try-all")
                .action(clap::ArgAction::SetTrue)
                .help("Try reading parameters even from devices that doesn't respond woth a long address"),
        )
        .get_matches();

    let mut addr: Short = match matches.get_one::<u8>("ADDR") {
        Some(&x) => match Short::from_display_value(x) {
            Ok(a) => a,
            Err(_) => {
                eprintln!("Address out of range");
                return;
            }
        },

        None => {
            eprintln!("Address invalid");
            return;
        }
    };
    let end_addr: Short = match matches.get_one::<u8>("END_ADDR") {
        Some(&x) => match Short::from_display_value(x) {
            Ok(a) => a,
            Err(_) => {
                eprintln!("Address out of range");
                return;
            }
        },
        None => addr,
    };
    if end_addr < addr {
        eprintln!("End address must be greater or equal to start address");
        return;
    }
    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let read_memory = *matches.get_one::<bool>("memory_banks").unwrap();
    let control_device = *matches.get_one::<bool>("control").unwrap();
    let try_all = *matches.get_one::<bool>("try-all").unwrap();
    let mut driver = match dali::drivers::open(device_name) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to open DALI device: {}", e);
            if let OpenError::NotFound = e {
                eprintln!("Available drivers:");
                for name in dali::drivers::driver_names() {
                    eprintln!("  {}", name);
                }
            }
            return;
        }
    };

    loop {
        if control_device {
            let mut commands = Commands103::new(&mut *driver);
            let long = commands.query_random_address(addr).await;
            if let Ok(long) = long {
                println!("Long address: 0x{:06x}", long);
            }
            if try_all || long.is_ok() {
                let info = match device_info::read_control_info(&mut *driver, addr).await {
                    Ok(i) => i,
                    Err(e) => {
                        eprintln!("Failed to read device info: {}", e);
                        return;
                    }
                };
                println!("{}", info);
            }
        } else {
	    let mut commands = Commands102::new(&mut *driver);
            let long = commands.query_random_address(addr).await;
            if let Ok(long) = long {
                println!("Long address: 0x{:06x}", long);
            }
            if try_all || long.is_ok() {
                let info = match device_info::read_gear_info(&mut *driver, addr).await {
                    Ok(i) => i,
                    Err(e) => {
                        eprintln!("Failed to read device info: {}", e);
                        return;
                    }
                };
                println!("{}", info);
                if read_memory {
                    match memory_banks::read_bank_0(&mut *driver, addr, 0, 0, 0x18).await {
                        Ok(data) => println!("{}", data),
                        Err(e) => {
                            eprintln!("Failed to read memory banks: {}", e);
                            return;
                        }
                    }
                }
            }
        }
        if addr == end_addr {
            break;
        }
        addr = match addr.try_add(1) {
            Ok(a) => a,
            Err(_) => break,
        };
    }
}
