use dali::common::address::DisplayValue;
use dali::common::address::Short;
use dali::common::commands::Commands;
use dali::drivers::driver::DaliSendResult;
use dali::drivers::send_flags::PRIORITY_1;
use dali::gear::commands_102::Commands102;
use dali::utils::address_assignment;
use dali_tools as dali;
use dali_tools::common::driver_commands::DriverCommands;
use log::debug;

extern crate clap;
use clap::{Arg, Command as ClapCommand, value_parser};

async fn swap_addr<C>(
    commands: &mut C,
    addr1: Short,
    addr2: Short,
) -> Result<(), address_assignment::Error<DaliSendResult>>
where
    C: Commands<Error = DaliSendResult>,
{
    let long1 = match commands.query_random_address(addr1).await {
        Ok(a) => Some(a),
        Err(DaliSendResult::Timeout) => None,
        Err(e) => return Err(e.into()),
    };
    println!(
        "{}: {}",
        addr1,
        long1
            .map(|x| { format!("0x{x:06x}") })
            .unwrap_or_else(|| "-".to_string())
    );
    let long2 = match commands.query_random_address(addr2).await {
        Ok(a) => Some(a),
        Err(DaliSendResult::Timeout) => None,
        Err(e) => return Err(e.into()),
    };
    println!(
        "{}: {}",
        addr2,
        long2
            .map(|x| { format!("0x{x:06x}") })
            .unwrap_or_else(|| "-".to_string())
    );
    commands.initialise_all().await?;
    debug!("initialise_all done");
    if let Some(l) = long1 {
        address_assignment::program_short_address(commands, l, addr2).await?;
    }
    if let Some(l) = long2 {
        address_assignment::program_short_address(commands, l, addr1).await?;
    }
    commands.terminate().await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = dali::drivers::init() {
        println!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = ClapCommand::new("swap_addr")
        .about("Swaps short addresses of two devices. If only one is present then the address of that one is changed.")
        .arg(Arg::new("DEVICE").long("devices").short('d')
             .long("device")
             .default_value("default")
             .help("Select DALI-device"))
        .arg(Arg::new("ADDR1").required(true).value_parser(value_parser!(u8)).help("First address"))
        .arg(Arg::new("ADDR2").required(true).value_parser(value_parser!(u8)).help("Second address"))
        .get_matches();

    let addr1 = match matches.try_get_one::<u8>("ADDR1") {
        Ok(Some(&x)) => match Short::from_display_value(x) {
            Ok(a) => a,
            Err(_) => {
                println!("First address out of range");
                return;
            }
        },
        Ok(None) => {
            println!("First address missing");
            return;
        }
        Err(e) => {
            println!("First address invalid: {}", e);
            return;
        }
    };

    let addr2 = match matches.try_get_one::<u8>("ADDR2") {
        Ok(Some(&x)) => match Short::from_display_value(x) {
            Ok(a) => a,
            Err(_) => {
                println!("FirstSecond address out of range");
                return;
            }
        },
        Ok(None) => {
            println!("Second address missing");
            return;
        }
        Err(e) => {
            println!("Second address invalid: {}", e);
            return;
        }
    };
    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let mut driver = match dali::drivers::open(device_name) {
        Ok(d) => d,
        Err(e) => {
            println!("Failed to open DALI device: {}", e);
            return;
        }
    };
    let mut commands = Commands102::from_driver(driver.as_mut(), PRIORITY_1);
    match swap_addr(&mut commands, addr1, addr2).await {
        Ok(_) => {}
        Err(e) => {
            println!("Failed while scanning for devices: {:?}", e);
        }
    }
}
