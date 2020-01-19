use super::super::common::MASK;

pub const FLORESCENT: u8 = 0;
pub const HID: u8 = 2;
pub const LV_HALOGEN: u8 = 3;
pub const INCANDESCENT: u8 = 4;
pub const LED: u8 = 6;
pub const OLED: u8 = 7;
pub const OTHER: u8 = 252;
pub const UNKNOWN: u8 = 253;
pub const NO_LIGHT_SOURCE: u8 = 254;
pub const MULTIPLE_LIGHT_SOURCES: u8 = MASK;
