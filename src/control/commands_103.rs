use crate::common::commands::{Commands, YesNo};
use crate::common::driver_commands::DriverCommands;
use crate::control::address::{Address, Short};
use crate::control::cmd_defs::*;
use crate::drivers::command_utils::send24;
use crate::drivers::driver::{DaliDriver, DaliSendResult};
use crate::drivers::send_flags::{Flags, PRIORITY_DEFAULT};

pub struct Commands103<'a> {
    driver: &'a mut dyn DaliDriver,
    flags: Flags,
}

impl<'a> Commands103<'a> {
    pub fn new(driver: &'a mut dyn DaliDriver) -> Self {
        Commands103 {
            driver,
            flags: PRIORITY_DEFAULT,
        }
    }

    async fn cmd<const TWICE: bool>(
        &mut self,
        cmd: Command<false, TWICE>,
    ) -> Result<(), DaliSendResult> {
        send24::cmd(self.driver, cmd, self.flags.clone())
            .await
            .check_send()
    }

    async fn query(&mut self, cmd: Command<true, false>) -> Result<u8, DaliSendResult> {
        send24::query(self.driver, cmd, self.flags.clone())
            .await
            .check_answer()
    }
    async fn query_yes_no(&mut self, cmd: Command<true, false>) -> Result<YesNo, DaliSendResult> {
        match send24::query(self.driver, cmd, self.flags.clone()).await {
            DaliSendResult::Answer(v) => Ok(if v == 0xff {
                YesNo::Yes
            } else {
                YesNo::Multiple
            }),
            DaliSendResult::Timeout => Ok(YesNo::No),
            DaliSendResult::Framing => Ok(YesNo::Multiple),
            e => Err(e),
        }
    }
}

impl DriverCommands for Commands103<'_> {
    type Output<'a> = Commands103<'a>;
    fn from_driver<'a>(driver: &'a mut dyn DaliDriver, flags: Flags) -> Self::Output<'a> {
        Commands103 { driver, flags }
    }
}

impl<'a> Commands for Commands103<'a> {
    type Address = Address;
    type Error = DaliSendResult;
    async fn initialise_all(&mut self) -> Result<(), Self::Error> {
        self.cmd(INITIALISE_ALL()).await
    }

    async fn initialise_no_addr(&mut self) -> Result<(), Self::Error> {
        self.cmd(INITIALISE_NO_ADDR()).await
    }

    async fn initialise_addr(&mut self, addr: Short) -> Result<(), Self::Error> {
        self.cmd(INITIALISE_ADDR(addr)).await
    }

    async fn terminate(&mut self) -> Result<(), Self::Error> {
        self.cmd(TERMINATE()).await
    }

    async fn randomize(&mut self) -> Result<(), Self::Error> {
        self.cmd(RANDOMISE()).await
    }

    async fn compare(&mut self) -> Result<YesNo, Self::Error> {
        self.query_yes_no(COMPARE()).await
    }

    async fn withdraw(&mut self) -> Result<(), Self::Error> {
        self.cmd(WITHDRAW()).await
    }
    async fn searchaddr_h(&mut self, h: u8) -> Result<(), Self::Error> {
        self.cmd(SEARCHADDRH(h)).await
    }
    async fn searchaddr_m(&mut self, m: u8) -> Result<(), Self::Error> {
        self.cmd(SEARCHADDRM(m)).await
    }

    async fn searchaddr_l(&mut self, l: u8) -> Result<(), Self::Error> {
        self.cmd(SEARCHADDRL(l)).await
    }

    async fn program_short_address(&mut self, addr: Option<Short>) -> Result<(), Self::Error> {
        self.cmd(PROGRAM_SHORT_ADDRESS(addr)).await
    }
    async fn verify_short_address(&mut self, addr: Short) -> Result<YesNo, Self::Error> {
        self.query_yes_no(VERIFY_SHORT_ADDRESS(addr)).await
    }
    async fn query_short_address(&mut self) -> Result<Option<Short>, Self::Error> {
        let raw = self.query(QUERY_SHORT_ADDRESS()).await?;
        Ok(if raw == 0xff {
            None
        } else {
            Some(Short::new(raw))
        })
    }
    async fn dtr0(&mut self, data: u8) -> Result<(), Self::Error> {
        self.cmd(DTR0(data)).await
    }
    async fn dtr1(&mut self, data: u8) -> Result<(), Self::Error> {
        self.cmd(DTR1(data)).await
    }
    async fn dtr2(&mut self, data: u8) -> Result<(), Self::Error> {
        self.cmd(DTR2(data)).await
    }
    async fn write_memory_location(&mut self, data: u8) -> Result<u8, Self::Error> {
        self.query(WRITE_MEMORY_LOCATION(data)).await
    }

    async fn write_memory_location_no_reply(&mut self, data: u8) -> Result<(), Self::Error> {
        self.cmd(WRITE_MEMORY_LOCATION_NO_REPLY(data)).await
    }

    async fn query_random_address(&mut self, dev_addr: Short) -> Result<u32, Self::Error> {
        let h = self.query(QUERY_RANDOM_ADDRESS_H(dev_addr)).await?;
        let m = self.query(QUERY_RANDOM_ADDRESS_M(dev_addr)).await?;
        let l = self.query(QUERY_RANDOM_ADDRESS_L(dev_addr)).await?;
        Ok((u32::from(h) << 8) | u32::from(m) << 8 | u32::from(l))
    }
    async fn read_memory_location(&mut self, device: Short) -> Result<u8, Self::Error> {
        self.query(READ_MEMORY_LOCATION(device)).await
    }

    async fn identify_device(&mut self, device: Address) -> Result<(), Self::Error> {
        self.cmd(IDENTIFY_DEVICE(device)).await
    }
}
