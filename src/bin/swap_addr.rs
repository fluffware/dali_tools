use dali::base::address::BusAddress;
use dali::base::address::Long;
use dali::base::address::Short;
use dali::gear::cmd_defs as cmd;
use dali::drivers::command_utils::send16;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
use dali::drivers::driver_utils::DaliDriverExt;
use dali::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG, PRIORITY_1, SEND_TWICE};
use dali_tools as dali;

extern crate clap;
use clap::{value_parser, Arg, Command};

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
    let h = send16::device_cmd(
        driver,
        &short_addr,
        cmd::QUERY_RANDOM_ADDRESS_H,
        EXPECT_ANSWER,
    )
    .await
    .check_answer()?;
    let m = send16::device_cmd(
        driver,
        &short_addr,
        cmd::QUERY_RANDOM_ADDRESS_M,
        EXPECT_ANSWER,
    )
    .await
    .check_answer()?;
    let l = send16::device_cmd(
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
    println!("Set {}, got {}", short, a + 1);
    Ok(())
}

async fn swap_addr(
    driver: &mut dyn DaliDriver,
    addr1: Short,
    addr2: Short,
) -> Result<(), DaliSendResult> {
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

#[tokio::main]
async fn main() {
    if let Err(e) = dali::drivers::init() {
        println!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = Command::new("swap_addr")
        .about("Swaps short addresses of two devices. If only one is present then the address of that one is changed.")
        .arg(Arg::new("DEVICE").long("devices").short('d')
             .long("device")
             .default_value("default")
             .help("Select DALI-device"))
        .arg(Arg::new("ADDR1").required(true).value_parser(value_parser!(u8)).help("First address"))
        .arg(Arg::new("ADDR2").required(true).value_parser(value_parser!(u8)).help("Second address"))
        .get_matches();

    let addr1 = match matches.try_get_one::<u8>("ADDR1") {
        Ok(Some(&x)) if x >= 1 && x <= 64 => Short::new(x),
        Ok(Some(_)) => {
            println!("First address out of range");
            return;
        }
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
        Ok(Some(&x)) if x >= 1 && x <= 64 => Short::new(x),
        Ok(Some(_)) => {
            println!("Second address out of range");
            return;
        }
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
    match swap_addr(&mut *driver, addr1, addr2).await {
        Ok(_) => {}
        Err(e) => {
            println!("Failed while scanning for devices: {:?}", e);
        }
    }
}
