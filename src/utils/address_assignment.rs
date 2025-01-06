use crate as dali;
use crate::drivers::driver_utils::DaliDriverExt;
use crate::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG};
use crate::utils::long_address::{query_long_addr, set_search_addr};
use dali::base::address::{BusAddress, Long, Short};
use dali::defs::gear::cmd;
use dali::drivers::driver::{DaliDriver, DaliSendResult};

pub enum Error {
    Send(DaliSendResult),
    AddressValidation,
}

impl From<DaliSendResult> for Error {
    fn from(result: DaliSendResult) -> Error {
        Self::Send(result)
    }
}

pub async fn program_short_address(
    driver: &mut dyn DaliDriver,
    long: Long,
    short: Short,
) -> Result<(), Error> {
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
    if short.bus_address() != (a & 0xfe) {
        return Err(Error::AddressValidation);
    }
    //println!("Set {}, got {}", short, (a>>1)+1);
    Ok(())
}

pub async fn clear_short_address(driver: &mut dyn DaliDriver, long: Long) -> Result<(), Error> {
    set_search_addr(driver, long).await?;
    driver
        .send_frame16(&[cmd::PROGRAM_SHORT_ADDRESS, 0xff], NO_FLAG)
        .await
        .check_send()?;
    let a = driver
        .send_frame16(&[cmd::QUERY_SHORT_ADDRESS, 0x00], EXPECT_ANSWER)
        .await
        .check_answer()?;
    if a != 0xff {
        return Err(Error::AddressValidation);
    }
    Ok(())
}

pub async fn program_short_addresses(
    driver: &mut dyn DaliDriver,
    map: &[(Short, Short)],
) -> Result<(), Error> {
    for (_old, new) in map {
        let long = query_long_addr(driver, new).await?;
        clear_short_address(driver, long).await?;
    }
    for (old, new) in map {
        let long = query_long_addr(driver, old).await?;
        program_short_address(driver, long, *new).await?;
    }
    Ok(())
}
