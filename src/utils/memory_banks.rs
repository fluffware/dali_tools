use crate::base::address::Short;
use crate::defs::gear::cmd;
use crate::drivers::command_utils::send16;
use crate::drivers::driver::{DaliDriver, DaliSendResult};
use crate::drivers::driver_utils::DaliDriverExt;
use crate::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG};
use std::convert::TryInto;
use std::error::Error;
use std::fmt;
pub enum MemoryError {
    LengthMismatch,
    InvalidMemoryArea,
}

impl Error for MemoryError {}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::LengthMismatch => write!(f, "DTR0 doesn't match read length"),
            MemoryError::InvalidMemoryArea => {
                write!(f, "Trying to read an unimplemented memory area")
            }
        }
    }
}

impl fmt::Debug for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[derive(Debug)]
pub struct MemoryBank0Info {
    pub gtin: u64,
    pub firmware_version: u16,
    pub id_number: u64,
    pub hardware_version: u16,
    pub version_101: u8,
    pub version_102: u8,
    pub version_103: u8,
    pub n_control_devices: u8,
    pub n_control_gears: u8,
    pub control_gear_index: u8,
}

impl MemoryBank0Info {
    pub fn new() -> MemoryBank0Info {
        MemoryBank0Info {
            gtin: 0,
            firmware_version: 0,
            id_number: 0,
            hardware_version: 0,
            version_101: 0xff,
            version_102: 0xff,
            version_103: 0xff,
            n_control_devices: 0,
            n_control_gears: 0,
            control_gear_index: 0,
        }
    }
}

fn version_str(ver: u8) -> String {
    if ver == 0xff {
        return String::from("-");
    } else {
        return u8::to_string(&(ver >> 2)) + "." + &u8::to_string(&(ver & 3));
    }
}

impl fmt::Display for MemoryBank0Info {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "GTIN: {}", self.gtin)?;
        writeln!(
            f,
            "Firmware version: {}.{}",
            self.firmware_version >> 8,
            self.firmware_version & 0xff
        )?;
        writeln!(f, "Identification number: {}", self.id_number)?;
        writeln!(
            f,
            "Hardware version: {}.{}",
            self.hardware_version >> 8,
            self.hardware_version & 0xff
        )?;
        writeln!(f, "101 version number: {}", version_str(self.version_101))?;
        writeln!(f, "102 version number: {}", version_str(self.version_102))?;
        writeln!(f, "103 version number: {}", version_str(self.version_103))?;
        writeln!(
            f,
            "Number of logical control device units: {}",
            self.n_control_devices
        )?;
        writeln!(
            f,
            "Number of logical control gear units: {}",
            self.n_control_gears
        )?;
        writeln!(
            f,
            "Index of this logical control gear unit: {}",
            self.control_gear_index
        )?;

        Ok(())
    }
}

pub async fn read_range(
    d: &mut dyn DaliDriver,
    addr: Short,
    bank: u8,
    start: u8,
    length: u8,
) -> Result<Vec<u8>, Box<dyn Error>> {
    d.send_frame16(&[cmd::DTR1, bank], NO_FLAG)
        .await
        .check_send()?;
    d.send_frame16(&[cmd::DTR0, start], NO_FLAG)
        .await
        .check_send()?;
    let mut data = Vec::new();
    for _ in 0..length {
        match send16::device_cmd(d, &addr, cmd::READ_MEMORY_LOCATION, EXPECT_ANSWER).await {
            DaliSendResult::Answer(d) => data.push(d),
            DaliSendResult::Timeout => break,
            e => return Err(Box::new(e)),
        }
    }

    let dtr = send16::device_cmd(d, &addr, cmd::QUERY_CONTENT_DTR0, EXPECT_ANSWER)
        .await
        .check_answer()?;
    if length as usize == data.len() {
        if dtr != length + start {
            return Err(Box::new(MemoryError::LengthMismatch));
        }
    } else {
        if dtr != data.len() as u8 + 1 + start {
            return Err(Box::new(MemoryError::LengthMismatch));
        }
    }
    Ok(data)
}

pub async fn read_bank_0(
    d: &mut dyn DaliDriver,
    addr: Short,
    _bank: u8,
    _start: u8,
    _length: u8,
) -> Result<MemoryBank0Info, Box<dyn Error>> {
    let mut bank0 = [0u8; 0x1b];
    let mut info = MemoryBank0Info::new();
    let bytes = read_range(d, addr, 0, 2, 0x19).await?;
    if bytes.len() != 0x19 {
        return Err(Box::new(MemoryError::InvalidMemoryArea));
    }
    bank0[0x02..=0x1a].copy_from_slice(&bytes);
    let mut gtin_bytes = [0u8; 8];
    gtin_bytes[2..8].copy_from_slice(&bank0[0x03..=0x08]);
    info.gtin = u64::from_be_bytes(gtin_bytes);
    info.firmware_version = u16::from_be_bytes((bank0[0x09..=0x0a]).try_into().unwrap());
    info.id_number = u64::from_be_bytes((bank0[0x0b..=0x12]).try_into().unwrap());
    info.hardware_version = u16::from_be_bytes((bank0[0x13..=0x14]).try_into().unwrap());
    info.version_101 = bank0[0x15];
    info.version_102 = bank0[0x16];
    info.version_103 = bank0[0x17];
    info.n_control_devices = bank0[0x18];
    info.n_control_gears = bank0[0x19];
    info.control_gear_index = bank0[0x1a];

    Ok(info)
}
