use super::driver::{DaliDriver, DaliSendResult};
use super::send_flags::Flags;
use crate::base::address::BusAddress;
use std::pin::Pin;
use std::future::Future;
use crate::defs::gear::cmd;

/// Send addressed DALI commands
///
/// # Arguments
/// * `addr` - Destination address of command 
/// * `cmd` - Second byte of command
/// * `flags` - Options for transaction
pub fn send_device_cmd(driver: &mut dyn DaliDriver, 
		   addr: &dyn BusAddress, cmd: u8, flags: Flags) -> 
    Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
{
    driver.send_frame16(&[addr.bus_address() | 1, cmd], flags)
}

/// Send DALI DAPC commands
///
/// # Arguments
/// * `addr` - Address of device(s) 
/// * `level` - Intensity level
/// * `flags` - Options for transaction

pub fn send_device_level(driver: &mut dyn DaliDriver,
		     addr: &dyn BusAddress, level: u8, flags: Flags) ->
    Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
{
    driver.send_frame16(&[addr.bus_address(), level], flags)
}

/// Set value of DTR0
///
/// # Arguments
/// * `dtr` - Value to store in DTR0

pub fn send_set_dtr0(driver: &mut dyn DaliDriver, dtr: u8, flags: Flags) ->
    Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
{
    driver.send_frame16(&[cmd::DTR0, dtr], flags)
}

/// Set value of DTR1
///
/// # Arguments
/// * `dtr` - Value to store in DTR1

pub fn send_set_dtr1(driver: &mut dyn DaliDriver, dtr: u8, flags: Flags) ->
    Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
{
    driver.send_frame16(&[cmd::DTR1, dtr], flags)
}

/// Set value of DTR2
///
/// # Arguments
/// * `dtr` - Value to store in DTR2

pub fn send_set_dtr2(driver: &mut dyn DaliDriver, dtr: u8, flags: Flags) ->
    Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
{
    driver.send_frame16(&[cmd::DTR2, dtr], flags)
}


	
