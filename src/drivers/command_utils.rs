use super::driver::{DaliDriver, DaliSendResult};
use crate::base::address::BusAddress;
use std::pin::Pin;
use std::future::Future;

/// Send addressed DALI commands
///
/// # Arguments
/// * `addr` - Destination address of command 
/// * `cmd` - Second byte of command
/// * `flags` - Options for transaction
pub fn send_device_cmd(driver: &mut dyn DaliDriver, 
		   addr: &dyn BusAddress, cmd: u8, flags:u16) -> 
    Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
{
    driver.send_frame_16(&[addr.bus_address() | 1, cmd], flags)
}

/// Send DALI DAPC commands
///
/// # Arguments
/// * `addr` - Address of device(s) 
/// * `level` - Intensity level
/// * `flags` - Options for transaction

pub fn send_device_level(driver: &mut dyn DaliDriver,
		     addr: &dyn BusAddress, level: u8, flags:u16) ->
    Pin<Box<dyn Future<Output = DaliSendResult> + Send>>
{
    driver.send_frame_16(&[addr.bus_address(), level], flags)
}


	
