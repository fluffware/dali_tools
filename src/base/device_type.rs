use crate::gear::device_type as types;
use std::fmt;

pub struct DeviceType(u8);

impl DeviceType {
    pub fn new(dtype: u8) -> DeviceType {
        DeviceType { 0: dtype }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let type_str = match self.0 {
            types::FLORESCENT => "Florescent",
            types::EMERGENCY => "Self-contained emergency",
            types::DISCHARGE => "Discharge (HID)",
            types::LV_HALOGEN => "Low-voltage halogen",
            types::INCANDESCENT => "Incandescent",
            types::DC_CONTROL => "Conversion to D.C. voltage",
            types::LED => "LED",
            types::SWITCHING => "Switching",
            types::COLOUR => "Colour",
            types::UNIMPLEMENTED => "Not implemented",
            _ => "",
        };
        if type_str == "" {
            write!(f, "Unknown type {}", self.0)
        } else {
            f.write_str(type_str)
        }
    }
}
