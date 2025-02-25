use crate::common::address::BusAddress;
use crate::common::commands::Commands;
use crate::control::address::{Address, Short};
use crate::control::cmd_defs::*;
use crate::drivers::driver::{DaliDriver, DaliFrame, DaliSendResult};
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

    async fn send_frame<const TWICE: bool>(
        &mut self,
        cmd: &Command<false, TWICE>,
    ) -> Result<(), DaliSendResult> {
        self.driver
            .send_frame(
                DaliFrame::Frame24(cmd.0),
                self.flags.clone() | Flags::SendTwice(TWICE),
            )
            .await
            .check_send()
    }

    async fn request_answer<const TWICE: bool>(
        &mut self,
        cmd: &Command<true, TWICE>,
    ) -> Result<u8, DaliSendResult> {
        self.driver
            .send_frame(
                DaliFrame::Frame24(cmd.0),
                self.flags.clone() | Flags::SendTwice(TWICE) | Flags::ExpectAnswer(true),
            )
            .await
            .check_answer()
    }
    async fn request_yes_no<const TWICE: bool>(
        &mut self,
        cmd: &Command<true, TWICE>,
    ) -> Result<bool, DaliSendResult> {
        self.driver
            .send_frame(
                DaliFrame::Frame24(cmd.0),
                self.flags.clone() | Flags::SendTwice(TWICE) | Flags::ExpectAnswer(true),
            )
            .await
            .check_yes_no()
    }
}

impl<'a> Commands for Commands103<'a> {
    type Address = Address;
    type Short = Short;
    type Error = DaliSendResult;
    async fn initialize(&mut self, device: u8) -> Result<(), Self::Error> {
        self.send_frame(&INITIALISE.cmd(device)).await
    }

    async fn terminate(&mut self) -> Result<(), Self::Error> {
        self.send_frame(&TERMINATE).await
    }

    async fn randomize(&mut self) -> Result<(), Self::Error> {
        self.send_frame(&RANDOMISE).await
    }

    async fn compare(&mut self) -> Result<bool, Self::Error> {
        self.request_yes_no(&COMPARE).await
    }

    async fn withdraw(&mut self) -> Result<(), Self::Error> {
        self.send_frame(&WITHDRAW).await
    }
    async fn searchaddr_h(&mut self, h: u8) -> Result<(), Self::Error> {
        self.send_frame(&SEARCHADDRH.cmd(h)).await
    }
    async fn searchaddr_m(&mut self, m: u8) -> Result<(), Self::Error> {
        self.send_frame(&SEARCHADDRM.cmd(m)).await
    }

    async fn searchaddr_l(&mut self, l: u8) -> Result<(), Self::Error> {
        self.send_frame(&SEARCHADDRL.cmd(l)).await
    }

    async fn program_short_address(&mut self, addr: Self::Short) -> Result<(), Self::Error> {
        self.send_frame(&PROGRAM_SHORT_ADDRESS.cmd(addr.bus_address() | 1))
            .await
    }
    async fn verify_short_address(&mut self, addr: Self::Short) -> Result<bool, Self::Error> {
        self.request_yes_no(&VERIFY_SHORT_ADDRESS.cmd(addr.bus_address() | 1))
            .await
    }
    async fn query_short_address(&mut self) -> Result<Self::Short, Self::Error> {
        let raw = self.request_answer(&QUERY_SHORT_ADDRESS).await?;
        Ok(Short::new(raw >> 1))
    }
    async fn dtr0(&mut self, data: u8) -> Result<(), Self::Error> {
        self.send_frame(&DTR0.cmd(data)).await
    }
    async fn dtr1(&mut self, data: u8) -> Result<(), Self::Error> {
        self.send_frame(&DTR1.cmd(data)).await
    }
    async fn dtr2(&mut self, data: u8) -> Result<(), Self::Error> {
        self.send_frame(&DTR2.cmd(data)).await
    }
    async fn write_memory_location(&mut self, data: u8) -> Result<u8, Self::Error> {
        self.request_answer(&WRITE_MEMORY_LOCATION.cmd(data)).await
    }

    async fn write_memory_location_no_reply(&mut self, data: u8) -> Result<(), Self::Error> {
        self.send_frame(&WRITE_MEMORY_LOCATION_NO_REPLY.cmd(data))
            .await
    }

    async fn query_random_address(&mut self, device: Address) -> Result<u32, Self::Error> {
        let dev_addr = device.bus_address();
        let h = self
            .request_answer(&QUERY_RANDOM_ADDRESS_H.cmd(dev_addr))
            .await?;
        let m = self
            .request_answer(&QUERY_RANDOM_ADDRESS_M.cmd(dev_addr))
            .await?;
        let l = self
            .request_answer(&QUERY_RANDOM_ADDRESS_L.cmd(dev_addr))
            .await?;
        Ok((u32::from(h) << 8) | u32::from(m) << 8 | u32::from(l))
    }
    async fn read_memory_location(&mut self, device: Address) -> Result<u8, Self::Error> {
        self.request_answer(&WRITE_MEMORY_LOCATION.cmd(device.bus_address()))
            .await
    }

    async fn identify_device(&mut self, device: Address) -> Result<(), Self::Error> {
        self.send_frame(&IDENTIFY_DEVICE.cmd(device.bus_address()))
            .await
    }
}
