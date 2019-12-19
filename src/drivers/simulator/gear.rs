use super::device::DALIsimDevice;
use crate::drivers::driver::DALIcommandError;
use crate::defs::common::MASK;    
pub enum InitialisationState
{
    ENABLED,
    DISABLED,
    WITHDRAWN
}

pub enum WriteEnableState
{
    ENABLED,
    DISABLED
}

pub struct DALIsimGear
{
    pub powered: bool,
    
    pub actual_level: u8,
    pub target_level: u8,
    pub last_active_level: u8,
    pub last_light_level: u8,
    pub power_on_level: u8,
    pub system_failure_level: u8,
    pub min_level: u8,
    pub max_level: u8,
    pub fade: u8, // bit 0-3: fade rate, bit 4-7: fade time
    pub extended_fade_time: u8,
    pub short_address: u8,
    pub search_address: u32,
    pub random_address: u32,
    pub operating_mode: u8,
    pub initialisation_state: InitialisationState,
    pub write_enable_state: WriteEnableState,
    pub status: u8, 
    pub gear_groups: u16,
    pub scene: [u8;16],
    pub dtr0: u8,
    pub dtr1: u8,
    pub dtr2: u8,
    pub phm: u8
}

impl DALIsimGear
{
    fn new() -> DALIsimGear
    {
        let phm = 0x01;
        DALIsimGear{
            powered: true,
            
            actual_level: 0xfe,
            target_level: 0xfe,
            last_active_level: 0xfe,
            last_light_level: 0xfe,
            power_on_level: 0xfe,
            system_failure_level: 0xfe,
            min_level: phm,
            max_level: 0xfe,
            fade: 0x07,
            extended_fade_time: 0x00,
            short_address: MASK,
            search_address: 0xffffff,
            random_address: 0xffffff,
            operating_mode: 0,
            initialisation_state: InitialisationState::DISABLED,
            write_enable_state: WriteEnableState::DISABLED,
            status: 0x00,
            gear_groups: 0x0000,
            scene:  [MASK;16],
            dtr0: 0,
            dtr1: 0,
            dtr2: 0,
            phm: 0
        }
    }
}

impl DALIsimDevice for DALIsimGear
{
    fn power(&mut self, on: bool)
    {
        self.powered = on;
    }
    
    fn forward16(&mut self, cmd: [u8;2], _flags:u16) 
                 ->Result<u8, DALIcommandError>
    {
        let addr = cmd[0] >> 1;
        let addr_match;
        if addr < 64 {
            addr_match = addr == self.short_address;
        } else if addr >= 0x40 && addr < 0x50 {
            addr_match = self.gear_groups & (1<<(addr & 0x0f)) != 0;
        } else if addr == 0x7f {
            addr_match = true;
        } else {
            addr_match = false;
        }
        let _ = addr_match;
        Err(DALIcommandError::Timeout)
    }

    
}
