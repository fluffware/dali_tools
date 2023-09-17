use crate::defs::gear::status as flag;
use std::fmt;

pub struct GearStatus(u8);

impl GearStatus {
    pub fn new(status: u8) -> GearStatus {
        GearStatus { 0: status }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl fmt::Display for GearStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut strs = Vec::<&'static str>::new();
        if self.0 & flag::GEAR_FAILURE != 0 {
            strs.push("gear failure");
        }
        if self.0 & flag::LAMP_FAILURE != 0 {
            strs.push("lamp failure");
        }
        if self.0 & flag::LAMP_ON != 0 {
            strs.push("lamp on");
        }
        if self.0 & flag::LIMIT_ERROR != 0 {
            strs.push("limit error");
        }
        if self.0 & flag::FADE_RUNNING != 0 {
            strs.push("fade running");
        }
        if self.0 & flag::RESET_STATE != 0 {
            strs.push("reset state");
        }
        if self.0 & flag::NO_ADDRESS != 0 {
            strs.push("no address");
        }
        if self.0 & flag::POWER_CYCLE != 0 {
            strs.push("power cycle");
        }
        f.write_str(&strs.join(", "))
    }
}
