use dali_tools as dali;
use dali::drivers::driver::{self,DALIdriver,DALIcommandError};
use dali::drivers::helvar::helvar510::Helvar510driver;
use dali::defs::gear::cmd;
use futures::executor::block_on;

#[macro_use]
extern crate clap;

async fn set_search_addr(driver: &mut dyn DALIdriver, addr: u32)
                         -> Result<u8, DALIcommandError>
{
    let res = driver.send_command(&[cmd::SEARCHADDRH,
                                    (addr>>16 & 0xff) as u8],
                                      driver::PRIORITY_1);
    res.await?;
    let res = driver.send_command(&[cmd::SEARCHADDRM,
                                    (addr>>8 & 0xff) as u8], 
                                  driver::PRIORITY_1);
    res.await?;
    let res = driver.send_command(&[cmd::SEARCHADDRL, (addr & 0xff) as u8],
                                  driver::PRIORITY_1);
    res.await?;
    Ok(0)
}

async fn query_long_addr(driver: &mut dyn DALIdriver, short_addr: u8)
    -> Result<u32, DALIcommandError>
{
    let hq = driver.send_command(&[short_addr<<1 | 1, 
                                   cmd::QUERY_RANDOM_ADDRESS_H],
                                   driver::EXPECT_ANSWER);
    let mq = driver.send_command(&[short_addr<<1 | 1, 
                                   cmd::QUERY_RANDOM_ADDRESS_M],
                                 driver::EXPECT_ANSWER);
    let lq = driver.send_command(&[short_addr<<1 | 1, 
                                   cmd::QUERY_RANDOM_ADDRESS_L],
                                 driver::EXPECT_ANSWER);
    let h = hq.await?;
    let m = mq.await?;
    let l = lq.await?;
    Ok((h as u32)<<16 | (m as u32)<<8 | (l as u32))
}

async fn program_short_address(driver: &mut dyn DALIdriver, long: u32, short: u8)
    -> Result<(), DALIcommandError>
{
    set_search_addr(driver, long).await?;
    driver.send_command(&[cmd::PROGRAM_SHORT_ADDRESS,
                          (short <<1) | 1], 0).await?;
    let a = driver.send_command(&[cmd::QUERY_SHORT_ADDRESS,0x00], 
                                driver::EXPECT_ANSWER).await?;
    println!("Set {}, got {}", short, a>>1);
    Ok(())
}

async fn swap_addr(driver: &mut dyn DALIdriver, addr1:u8, addr2:u8)
    -> Result<(), DALIcommandError>
{
    let long1 = match query_long_addr(driver, addr1).await {
        Ok(a) => Some(a),
        Err(DALIcommandError::Timeout) => None,
        Err(e) => return Err(e)
    };
    println!("{}: 0x{:?}", addr1, long1);
    let long2 = match query_long_addr(driver, addr2).await {
        Ok(a) => Some(a),
        Err(DALIcommandError::Timeout) => None,
        Err(e) => return Err(e)
    };
    println!("{}: 0x{:?}", addr2, long2);
    driver.send_command(&[cmd::INITIALISE, cmd::INITIALISE_ALL], 
                        driver::SEND_TWICE).await?;
    if let Some(l) = long1 {
        program_short_address(driver, l, addr2).await;
    }
    if let Some(l) = long2 {
        program_short_address(driver, l, addr1).await;
    }
    driver.send_command(&[cmd::TERMINATE, 0], 0).await?;
    Ok(())
}

fn main() {
    let matches = clap_app!(swap_addr =>
                         (about: "Swaps short addresses of two devices. If only one is present then the address of that one is changed.")
                         (@arg ADDR1: +required "First address")
                         (@arg ADDR2: +required "Second address")
    ).get_matches();
    
    let addr1 = match u8::from_str_radix(matches.value_of("ADDR1").unwrap(),10){
        Ok(x) if x >= 1 && x <= 64 => x-1,
        Ok(_) => {
            println!("First address out of range");
            return
        }
        Err(e) => {
            println!("First address invalid: {}",e);
            return
        }
    };
    
    let addr2 = match u8::from_str_radix(matches.value_of("ADDR2").unwrap(),10){
        Ok(x) if x >= 1 && x <= 64 => x-1,
        Ok(_) => {
            println!("First address out of range");
            return
        }
        Err(e) => {
            println!("First address invalid: {}",e);
            return
        }
    };

    let mut driver = Helvar510driver::new();
    match block_on(swap_addr(&mut driver, addr1, addr2)) {
        Ok(_) => {},
        Err(e) => {
            println!("Failed while scanning for devices: {:?}",e);
        }
    }

    
}