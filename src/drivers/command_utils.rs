use super::driver::{DaliDriver, DaliSendResult};
use super::driver_utils::DaliDriverExt;
use super::send_flags::Flags;
use crate::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG, SEND_TWICE};
use crate::utils::dyn_future::DynFuture;

pub mod send16 {
    use super::*;
    use crate::gear::address::Address;
    use crate::gear::cmd_defs as cmd;
    use crate::gear::cmd_defs::Command;

    /// Send DALI commands
    ///
    /// # Arguments
    /// * `cmd` - DALI command
    /// * `flags` - Options for transaction
    pub fn cmd<'driver, const T: bool>(
        driver: &'driver mut dyn DaliDriver,
        cmd: Command<false, T>,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame16(&cmd.0, flags | if T { SEND_TWICE } else { NO_FLAG })
    }

    // Make DALI query
    ///
    /// # Arguments
    /// * `cmd` - DALI query
    /// * `flags` - Options for transaction
    pub fn query<'driver>(
        driver: &'driver mut dyn DaliDriver,
        cmd: Command<true, false>,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame16(&cmd.0, flags | EXPECT_ANSWER)
    }

    /// Send DALI DAPC commands
    ///
    /// # Arguments
    /// * `addr` - Address of device(s)
    /// * `level` - Intensity level
    /// * `flags` - Options for transaction

    pub fn device_level<'driver>(
        driver: &'driver mut dyn DaliDriver,
        addr: Address,
        level: u8,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame16(&cmd::DAPC(addr, level).0, flags)
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
        driver.send_frame16(&cmd::DTR0(dtr).0, flags)
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
        driver.send_frame16(&cmd::DTR1(dtr).0, flags)
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
        driver.send_frame16(&cmd::DTR2(dtr).0, flags)
    }
}

pub mod send24 {
    use super::*;
    use crate::control::cmd_defs as cmd;
    use crate::control::cmd_defs::Command;

    /// Send DALI commands
    ///
    /// # Arguments
    /// * `cmd` - DALI command
    /// * `flags` - Options for transaction
    pub fn cmd<'driver, const T: bool>(
        driver: &'driver mut dyn DaliDriver,
        cmd: Command<false, T>,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame24(&cmd.0, flags | if T { SEND_TWICE } else { NO_FLAG })
    }

    // Make DALI query
    ///
    /// # Arguments
    /// * `cmd` - DALI query
    /// * `flags` - Options for transaction
    pub fn query<'driver>(
        driver: &'driver mut dyn DaliDriver,
        cmd: Command<true, false>,
        flags: Flags,
    ) -> DynFuture<'driver, DaliSendResult> {
        driver.send_frame24(&cmd.0, flags | EXPECT_ANSWER)
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
        driver.send_frame24(&cmd::DTR0(dtr).0, flags)
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
        driver.send_frame24(&cmd::DTR1(dtr).0, flags)
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
        driver.send_frame24(&cmd::DTR2(dtr).0, flags)
    }
}
