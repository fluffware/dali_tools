use super::driver::{DaliDriver, DaliSendResult};
use super::driver_utils::DaliDriverExt;
use super::send_flags::Flags;
use crate::common::address::BusAddress;
use crate::gear::cmd_defs as cmd;
use crate::utils::dyn_future::DynFuture;

pub mod send16 {
    use super::*;
    /// Send addressed DALI commands
    ///
    /// # Arguments
    /// * `addr` - Destination address of command
    /// * `cmd` - Second byte of command
    /// * `flags` - Options for transaction
    pub fn device_cmd<'driver>(
        driver: &'driver mut dyn DaliDriver,
        addr: &dyn BusAddress,
        cmd: u8,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame16(&[addr.bus_address() | 1, cmd], flags)
    }

    /// Send DALI DAPC commands
    ///
    /// # Arguments
    /// * `addr` - Address of device(s)
    /// * `level` - Intensity level
    /// * `flags` - Options for transaction

    pub fn device_level<'driver>(
        driver: &'driver mut dyn DaliDriver,
        addr: &dyn BusAddress,
        level: u8,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame16(&[addr.bus_address(), level], flags)
    }

    /// Set value of DTR0
    ///
    /// # Arguments
    /// * `dtr` - Value to store in DTR0

    pub fn set_dtr0<'driver>(
        driver: &'driver mut dyn DaliDriver,
        dtr: u8,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame16(&[cmd::DTR0, dtr], flags)
    }

    /// Set value of DTR1
    ///
    /// # Arguments
    /// * `dtr` - Value to store in DTR1

    pub fn set_dtr1<'driver>(
        driver: &'driver mut dyn DaliDriver,
        dtr: u8,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame16(&[cmd::DTR1, dtr], flags)
    }

    /// Set value of DTR2
    ///
    /// # Arguments
    /// * `dtr` - Value to store in DTR2

    pub fn set_dtr2<'driver>(
        driver: &'driver mut dyn DaliDriver,
        dtr: u8,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame16(&[cmd::DTR2, dtr], flags)
    }
}
