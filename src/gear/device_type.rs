use std::fmt;

pub struct DeviceType(u8);
pub mod types {
    pub const FLORESCENT: u8 = 0;
    pub const EMERGENCY: u8 = 1;
    pub const DISCHARGE: u8 = 2;
    pub const LV_HALOGEN: u8 = 3;
    pub const INCANDESCENT: u8 = 4;
    pub const DC_CONTROL: u8 = 5;
    pub const LED: u8 = 6;
    pub const SWITCHING: u8 = 7;
    pub const COLOUR: u8 = 8;
    pub const UNIMPLEMENTED: u8 = 254;
}

impl DeviceType {
    pub fn new(dtype: u8) -> DeviceType {
        DeviceType(dtype)
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
        if type_str.is_empty() {
            write!(f, "Unknown type {}", self.0)
        } else {
            f.write_str(type_str)
        }
    }
}
