use dali::base::address::Short;
use dali::drivers::driver::OpenError;
use dali::utils::device_info;
use dali::utils::memory_banks;
use dali_tools as dali;
extern crate clap;
use clap::{value_parser, Arg, Command};

#[tokio::main]
async fn main() {
    if let Err(e) = dali::drivers::init() {
        println!("Failed to initialize DALI drivers: {}", e);
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
            Arg::new("memory_banks")
                .short('m')
                .long("memory-banks")
                .value_parser(value_parser!(bool))
                .default_value("false")
                .default_missing_value("true")
                .help("Read information from memory banks"),
        )
        .get_matches();

    let addr: Short = match matches.get_one::<u8>("ADDR") {
        Some(&x) if x >= 1 && x <= 64 => Short::new(x),
        Some(_) => {
            println!("Address out of range");
            return;
        }
        None => {
            println!("Address invalid");
            return;
        }
    };
    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let read_memory = *matches.get_one::<bool>("memory_banks").unwrap();
    let mut driver = match dali::drivers::open(device_name) {
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

    let info = match device_info::read_device_info(&mut *driver, addr).await {
        Ok(i) => i,
        Err(e) => {
            println!("Failed to read device info: {}", e);
            return;
        }
    };
    println!("{}", info);
    if read_memory {
        match memory_banks::read_bank_0(&mut *driver, addr, 0, 0, 0x18).await {
            Ok(data) => println!("{}", data),
            Err(e) => {
                println!("Failed to read memory banks: {}", e);
                return;
            }
        }
    }
}
