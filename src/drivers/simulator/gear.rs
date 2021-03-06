use super::device::DALIsimDevice;
use crate::drivers::driver::DALIcommandError;
use crate::defs::common::MASK;
use crate::defs::gear:: {cmd,status,device_type, light_source};
use crate::drivers::driver;
use std::time::Instant;
use std::time::Duration;

extern crate rand;
use rand::Rng;

#[derive(PartialEq)]
pub enum InitialisationState
{
    ENABLED,
    DISABLED,
    WITHDRAWN
}

#[derive(PartialEq)]
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
    pub phm: u8,

    // Fade endpoints. Scaled for better precision.
    // Scaled by 128
    fade_start_level: i16,
    // Scaled by 128
    fade_end_level: i16,
    
    // Timers            
    fade_start_time: Instant,
    fade_duration: Duration,
    init_start_time: Instant,
}

impl DALIsimGear
{
    pub fn new() -> DALIsimGear
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
            phm: 0,

            fade_start_level: 0,
            // Scaled by 128
            fade_end_level: 0,
            
            fade_start_time: Instant::now(),
            fade_duration: Duration::new(0,0),
            init_start_time: Instant::now(),
                
        }
    }
}

const INIT_TIMEOUT: Duration = Duration::from_secs(15*60);

fn check_timers(dev: &mut DALIsimGear)
{
    if dev.initialisation_state != InitialisationState::DISABLED {
        if dev.init_start_time.elapsed() >= INIT_TIMEOUT {
            dev.initialisation_state = InitialisationState::DISABLED;
        }
    }
    
    if  (dev.status & status::FADE_RUNNING) != 0 {
        let elapsed = dev.fade_start_time.elapsed();
        if elapsed >=  dev.fade_duration {
            dev.actual_level = dev.target_level;
            dev.status &= !status::FADE_RUNNING;
        } else {
            let elapsed_millis = elapsed.as_millis() as i128;
            let duration_millis = dev.fade_duration.as_millis() as i128;
            dev.actual_level = 
                ((dev.fade_start_level 
                 + (((dev.fade_end_level - dev.fade_start_level) as i128 
                     * elapsed_millis + duration_millis/2) 
                                   / duration_millis) as i16) >> 7) as u8;
        }
    }
}

const fn fade_time(n: u8) -> Duration
{
    let n = n as u64;
    let millis = (1u64<<(n/2)) * ((n&1) * 707 + (1-(n&1)) * 500);
    Duration::from_millis(millis)
}

const FADE_TIMES: [Duration;16] = [
    Duration::from_millis(0),
    fade_time(1),
    fade_time(2),
    fade_time(3),
    fade_time(4),
    fade_time(5),
    fade_time(6),
    fade_time(7),
    fade_time(8),
    fade_time(9),
    fade_time(10),
    fade_time(11),
    fade_time(12),
    fade_time(13),
    fade_time(14),
    fade_time(15)
];

const FADE_MULTIPLIER : [Duration;5] = [
    Duration::from_millis(0),
    Duration::from_millis(100),
    Duration::from_secs(1),
    Duration::from_secs(10),
    Duration::from_secs(60)
];
    
fn start_fade_time(dev: &mut DALIsimGear)
{
    if (dev.fade & 0xf0) == 0x00 && (dev.extended_fade_time & 0x70) == 0x00 {
        // No fade, change instantly
        dev.actual_level = dev.target_level;
        return;
    } else {
        if (dev.fade & 0xf0) == 0x0 {
            // Use extended fade times
            if dev.extended_fade_time == 0 || dev.extended_fade_time > 0x4f {
                // Extended fade is zero
                dev.actual_level = dev.target_level;
                return;
            } else {
                // Extended fade time
                dev.fade_duration = 
                    FADE_MULTIPLIER[dev.extended_fade_time as usize>>4] 
                    * ((dev.extended_fade_time & 0x0f) +1) as u32;
            }
        } else {
            // Basic fadetime
            dev.fade_duration = FADE_TIMES[dev.fade as usize >> 4];
        }
    }
    dev.fade_start_time = Instant::now();
    dev.fade_start_level = (dev.actual_level as i16) << 7;
    dev.fade_end_level = (dev.target_level as i16) << 7;
}

fn query_status_flag(dev: &DALIsimGear, flag: u8)
                     ->Result<u8, DALIcommandError>
{
    if (dev.status & flag) != 0 {
        driver::YES
    } else {
        driver::NO
    } 
}

// Status flags that are not dependant on any other state
pub const STORED_STATUS_FLAGS : u8 = 
    status::GEAR_FAILURE 
    | status::LAMP_FAILURE
    | status::LIMIT_ERROR
    | status::FADE_RUNNING
    | status::RESET_STATE
    | status::POWER_CYCLE;

fn update_status(dev: &mut DALIsimGear) 
{
    dev.status = (dev.status & STORED_STATUS_FLAGS) 
        | if dev.actual_level > 0 {status::LAMP_ON} else {0}
        | if dev.short_address == MASK {status::NO_ADDRESS} else {0};
}

fn yes_no(p: bool) -> Result<u8, DALIcommandError>
{
    if p {driver::YES} else {driver::NO}
}

fn device_cmd(dev: &mut DALIsimGear, _addr: u8, cmd: u8, _flags: u16) 
              ->Result<u8, DALIcommandError>
{
    match cmd {
        cmd::QUERY_STATUS => {
            update_status(dev);
            return Ok(dev.status)
        },
        cmd::QUERY_CONTROL_GEAR_PRESENT => 
            return driver::YES,
        cmd::QUERY_CONTROL_GEAR_FAILURE =>
            return query_status_flag(&dev, status::GEAR_FAILURE),
        cmd::QUERY_LAMP_FAILURE =>
            return query_status_flag(&dev, status::LAMP_FAILURE),
        cmd::QUERY_LAMP_POWER_ON =>
            return yes_no(dev.actual_level > 0),
        cmd::QUERY_LIMIT_ERROR =>
            return query_status_flag(&dev, status::LIMIT_ERROR),
        cmd::QUERY_RESET_STATE =>
            return query_status_flag(&dev, status::RESET_STATE),
        cmd::QUERY_MISSING_SHORT_ADDRESS => {
            return yes_no(dev.short_address == MASK)
        },
        cmd::QUERY_VERSION_NUMBER => {
            return Ok(0xff)
        },
        cmd::QUERY_DEVICE_TYPE =>
            return Ok(device_type::LED),
        cmd::QUERY_NEXT_DEVICE_TYPE =>
            return driver::NO,
        cmd::QUERY_PHYSICAL_MINIMUM =>
            return Ok(dev.phm),
        cmd::QUERY_POWER_FAILURE =>
            return query_status_flag(&dev, status::POWER_CYCLE),
        cmd::QUERY_CONTENT_DTR0 =>
            return Ok(dev.dtr0),
        cmd::QUERY_CONTENT_DTR1 =>
            return Ok(dev.dtr1),
        cmd::QUERY_CONTENT_DTR2 =>
            return Ok(dev.dtr2),
        cmd::QUERY_OPERATING_MODE =>
            return Ok(0x00),
        cmd::QUERY_LIGHT_SOURCE_TYPE =>
            return Ok(light_source::LED),
        cmd::QUERY_ACTUAL_LEVEL =>
            return Ok(dev.actual_level),
        cmd::QUERY_MAX_LEVEL =>
            return Ok(dev.max_level),
        cmd::QUERY_MIN_LEVEL =>
            return Ok(dev.min_level),
        cmd::QUERY_POWER_ON_LEVEL =>
            return Ok(dev.power_on_level),
        cmd::QUERY_SYSTEM_FAILURE_LEVEL =>
            return Ok(dev.system_failure_level),
        cmd::QUERY_FADE =>
            return Ok(dev.fade),
        cmd::QUERY_SCENE_LEVEL_0..= cmd::QUERY_SCENE_LEVEL_15 =>
            return Ok(dev.scene[(cmd - cmd::QUERY_SCENE_LEVEL_0) as usize]),
        cmd::QUERY_GROUPS_0_7 =>
            return Ok((dev.gear_groups & 0xff) as u8),
        cmd::QUERY_GROUPS_8_15 =>
            return Ok((dev.gear_groups >> 8) as u8),
        cmd::QUERY_RANDOM_ADDRESS_H =>
            return Ok((dev.random_address >> 16) as u8),
        cmd::QUERY_RANDOM_ADDRESS_M =>
            return Ok(((dev.random_address >> 8) & 0xff) as u8),
        cmd::QUERY_RANDOM_ADDRESS_L =>
            return Ok((dev.random_address & 0xff) as u8),        
        _ => {}
    }
    Err(DALIcommandError::Timeout)
}

fn special_cmd(dev: &mut DALIsimGear, cmd: u8, data: u8, flags: u16) 
              ->Result<u8, DALIcommandError>
{
    //eprintln!("Special cmd: {:02x}", cmd);
    match cmd {
        cmd::TERMINATE => {
            dev.initialisation_state = InitialisationState::DISABLED;
            // TODO stop identification
            driver::NO
        },
        cmd::INITIALISE if flags & driver::SEND_TWICE != 0=> {
            if (((data & 0x81) == 0x01) 
                && (data >> 1) == dev.short_address)
                || (data == 0xff && dev.short_address == MASK)
                || data == 0x00
            {
                // TODO restart initialisation timer 
                dev.initialisation_state = InitialisationState::ENABLED;
            }
            
            driver::NO
        },
        cmd::RANDOMISE if flags & driver::SEND_TWICE != 0=> {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.random_address = rand::thread_rng().gen_range(0, 0xffffff);
            }
            driver::NO
        },
        cmd::COMPARE => {
            if dev.initialisation_state == InitialisationState::ENABLED
                && dev.random_address <= dev.search_address {
                    driver::YES
                } else {
                    driver::NO
                }
        },
        cmd::WITHDRAW => {
             if dev.initialisation_state == InitialisationState::ENABLED
                && dev.random_address == dev.search_address {
                    dev.initialisation_state = InitialisationState::WITHDRAWN; 
                }
            driver::NO
        },
        cmd::SEARCHADDRH => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.search_address =
                    (dev.search_address & 0x00ffff) | ((data as u32) << 16);
            }
            driver::NO
        },
        cmd::SEARCHADDRM => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.search_address =
                    (dev.search_address & 0xff00ff) | ((data as u32) << 8);
            }
            driver::NO
        },
        cmd::SEARCHADDRL => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.search_address =
                    (dev.search_address & 0xffff00) | (data as u32);
            }
            driver::NO
        },
        
        cmd::PROGRAM_SHORT_ADDRESS => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                if (data & 0x81) == 0x01 {
                    dev.short_address = data>>1;
                } else if data == MASK {
                    dev.short_address = MASK;
                }
            }
            driver::NO            
        },
        cmd::QUERY_SHORT_ADDRESS => {
            if dev.initialisation_state != InitialisationState::DISABLED
                && dev.search_address == dev.random_address {
                    eprintln!("Query_Short_Address: {}", dev.short_address);
                    Ok((dev.short_address<<1) | 0x01)
                } else {
                    driver::NO            
                }
        },
        cmd::DTR0 => {
            dev.dtr0 = data;
            driver::NO
        },
        cmd::DTR1 => {
            dev.dtr1 = data;
            driver::NO
        },
        cmd::DTR2 => {
            dev.dtr2 = data;
            driver::NO
        },
        
        _ => {
            driver::NO
        }
    }
}

impl DALIsimDevice for DALIsimGear
{
    fn power(&mut self, on: bool)
    {
        self.powered = on;
    }
    
    fn forward16(&mut self, cmd: &[u8], flags:u16) 
                 ->Result<u8, DALIcommandError>
    {
        /*eprintln!("Gear {} received: {:02x} {:02x}", self.short_address,
                  cmd[0], cmd[1]);*/
        match cmd[0] >> 1 {
            addr @ 0x00..=0x3f => {
                if addr == self.short_address {
                    return device_cmd(self, cmd[0], cmd[1], flags);
                }
            },
            addr @ 0x40..=0x4f => {
                if self.gear_groups & (1<<(addr & 0x0f)) != 0 {
                    return device_cmd(self, cmd[0], cmd[1], flags);
                }
            },
            0x7e => {
                if self.short_address == MASK {
                    return device_cmd(self, cmd[0], cmd[1], flags);
                }
            },
            0x7f => {
                return device_cmd(self, cmd[0], cmd[1], flags);
            },
            _ => {
                return special_cmd(self, cmd[0], cmd[1], flags);
            }
        };
        driver::NO
    }
}

