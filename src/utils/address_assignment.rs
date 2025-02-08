use crate as dali;
use crate::drivers::driver_utils::DaliDriverExt;
use crate::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG, SEND_TWICE};
use crate::utils::long_address::{query_long_addr, set_search_addr};
use dali::base::address::{BusAddress, Long, Short};
use dali::gear::cmd_defs as cmd;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use log::debug;

#[derive(Debug)]
pub enum Error {
    Send(DaliSendResult),
    AddressValidation,
    AddressCollision,
}

impl From<DaliSendResult> for Error {
    fn from(result: DaliSendResult) -> Error {
        Self::Send(result)
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Send(res) => res.fmt(f),
            Error::AddressValidation => {
                write!(f, "Failed to set short address")
            }
            Error::AddressCollision => {
                write!(f, "Duplicate short addresses")
            }
        }
    }
}

pub async fn program_short_address(
    driver: &mut dyn DaliDriver,
    long: Long,
    short: Short,
) -> Result<(), Error> {
    debug!("{} set address {}", long, short);
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
    debug!("Clearing {}",long);
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
    // Keep track of unused addresses
    let mut old_set = BTreeSet::new();
    // All long addresses before remapping
    let mut old_map = BTreeMap::new();

    // Gather all long addresses
    for (old, new) in map {
	debug!("Map {} -> {}", old,new);
        if !old_set.insert(old) {
            return Err(Error::AddressCollision);
        }
        if !old_map.contains_key(old) {
            let long_old = query_long_addr(driver, old).await?;
            old_map.insert(old, long_old);
        }
        if !old_map.contains_key(new) {
            if let Ok(long_new) = query_long_addr(driver, new).await {
		old_map.insert(new, long_new);
	    }
        }
    }
    driver
        .send_frame16(&[cmd::INITIALISE, cmd::INITIALISE_ALL], SEND_TWICE)
        .await
        .check_send()?;
    // Remap according to list
    for (old, new) in map {
        if let Some(long) = old_map.get(new) {
            clear_short_address(driver, *long).await?;
        }
        old_set.remove(new);
        if let Some(long) = old_map.remove(old) {
            program_short_address(driver, long, *new).await?;
        }
    }

    // Assing unused addresses
    for (long, new) in std::iter::zip(old_map.values(), old_set) {
        program_short_address(driver, *long, *new).await?;
    }
    driver
        .send_frame16(&[cmd::TERMINATE, 0], NO_FLAG)
        .await
        .check_send()?;
    Ok(())
}

