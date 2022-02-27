use super::device::{DaliSimHost, DaliSimEvent, DaliSimDevice};
use crate::defs::common::MASK;
use crate::defs::gear:: {cmd,status,device_type, light_source};
use crate::drivers::driver::{DaliBusEventType};
use crate::drivers::send_flags::Flags;
use std::time::Instant;
use std::time::Duration;
use std::future;
use std::future::Future;
use std::pin::Pin;
use super::timing::{FRAME_8_DURATION, FRAME_16_DURATION,
		    REPLY_DELAY, SEND_TWICE_DURATION, INIT_TIMEOUT};

extern crate rand;
use rand::Rng;

type DynResult<T> =  Result<T, Box<dyn std::error::Error + Send + Sync>>;

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

#[allow(dead_code)]
pub struct DaliSimGear
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

    last_event: DaliSimEvent, // Previous event, used for detecting send twice
    source_id: u32,
    host: Option<Box<dyn DaliSimHost>>,
}

impl DaliSimGear
{
    pub fn new() -> DaliSimGear
    {
        let phm = 0x01;
	let now = Instant::now();
        DaliSimGear{
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
            
            fade_start_time: now,
            fade_duration: Duration::new(0,0),
            init_start_time: now,
            last_event: DaliSimEvent{
		source_id: 0,
		timestamp: now,
		event_type: DaliBusEventType::BusPowerOff},
	    source_id: 0,
	    host: None,
        }
    }
}


fn check_timers(dev: &mut DaliSimGear)
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
    
fn start_fade_time(dev: &mut DaliSimGear)
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

fn query_status_flag(dev: &DaliSimGear, flag: u8)
                     -> Option<DaliBusEventType>
{
    if (dev.status & flag) != 0 {
        YES_REPLY
    } else {
        NO_REPLY
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

fn update_status(dev: &mut DaliSimGear) 
{
    dev.status = (dev.status & STORED_STATUS_FLAGS) 
        | if dev.actual_level > 0 {status::LAMP_ON} else {0}
        | if dev.short_address == MASK {status::NO_ADDRESS} else {0};
}

fn yes_no(p: bool) -> Option<DaliBusEventType>
{
    if p {YES_REPLY} else {NO_REPLY}
}

const YES_REPLY: Option<DaliBusEventType>= Some(DaliBusEventType::Frame8(MASK));
const NO_REPLY: Option<DaliBusEventType> = None;

fn device_cmd(dev: &mut DaliSimGear, _addr: u8, cmd: u8, _flags: Flags) 
              -> Option<DaliBusEventType>
{
    match cmd {
        cmd::QUERY_STATUS => {
            update_status(dev);
            return Some(DaliBusEventType::Frame8(dev.status))
        },
        cmd::QUERY_CONTROL_GEAR_PRESENT => 
            return YES_REPLY,
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
            return Some(DaliBusEventType::Frame8(2<<2 + 0)) // 2.0
        },
        cmd::QUERY_DEVICE_TYPE =>
            return Some(DaliBusEventType::Frame8(device_type::LED)),
        cmd::QUERY_NEXT_DEVICE_TYPE =>
            return NO_REPLY,
        cmd::QUERY_PHYSICAL_MINIMUM =>
            return Some(DaliBusEventType::Frame8(dev.phm)),
        cmd::QUERY_POWER_FAILURE =>
            return query_status_flag(&dev, status::POWER_CYCLE),
        cmd::QUERY_CONTENT_DTR0 =>
            return Some(DaliBusEventType::Frame8(dev.dtr0)),
        cmd::QUERY_CONTENT_DTR1 =>
            return Some(DaliBusEventType::Frame8(dev.dtr1)),
        cmd::QUERY_CONTENT_DTR2 =>
            return Some(DaliBusEventType::Frame8(dev.dtr2)),
        cmd::QUERY_OPERATING_MODE =>
            return Some(DaliBusEventType::Frame8(0x00)),
        cmd::QUERY_LIGHT_SOURCE_TYPE =>
            return Some(DaliBusEventType::Frame8(light_source::LED)),
        cmd::QUERY_ACTUAL_LEVEL =>
            return Some(DaliBusEventType::Frame8(dev.actual_level)),
        cmd::QUERY_MAX_LEVEL =>
            return Some(DaliBusEventType::Frame8(dev.max_level)),
        cmd::QUERY_MIN_LEVEL =>
            return Some(DaliBusEventType::Frame8(dev.min_level)),
        cmd::QUERY_POWER_ON_LEVEL =>
            return Some(DaliBusEventType::Frame8(dev.power_on_level)),
        cmd::QUERY_SYSTEM_FAILURE_LEVEL =>
            return Some(DaliBusEventType::Frame8(dev.system_failure_level)),
        cmd::QUERY_FADE =>
            return Some(DaliBusEventType::Frame8(dev.fade)),
        cmd::QUERY_SCENE_LEVEL_0..= cmd::QUERY_SCENE_LEVEL_15 => {
	    let level = dev.scene[(cmd - cmd::QUERY_SCENE_LEVEL_0) as usize];
            return Some(DaliBusEventType::Frame8(level))
	},
        cmd::QUERY_GROUPS_0_7 => {
	    let groups = (dev.gear_groups & 0xff) as u8;
            return Some(DaliBusEventType::Frame8(groups))
	},
        cmd::QUERY_GROUPS_8_15 => {
	    let groups = (dev.gear_groups >> 8) as u8;
            return Some(DaliBusEventType::Frame8(groups))
	},
        cmd::QUERY_RANDOM_ADDRESS_H => {
	    let addr = (dev.random_address >> 16) as u8;
            return Some(DaliBusEventType::Frame8(addr))
	},
        cmd::QUERY_RANDOM_ADDRESS_M => {
	    let addr = ((dev.random_address >> 8) & 0xff) as u8;
            return Some(DaliBusEventType::Frame8(addr));
	},
        cmd::QUERY_RANDOM_ADDRESS_L => {
	    let addr = (dev.random_address & 0xff) as u8; 
            return Some(DaliBusEventType::Frame8(addr))
	},
        _ => {}
    }
    None
}

fn special_cmd(dev: &mut DaliSimGear, cmd: u8, data: u8, flags: Flags) 
              -> Option<DaliBusEventType>
{
    //eprintln!("Special cmd: {:02x}", cmd);
    match cmd {
        cmd::TERMINATE => {
            dev.initialisation_state = InitialisationState::DISABLED;
            // TODO stop identification
            NO_REPLY
        },
        cmd::INITIALISE if flags.send_twice() => {
            if (((data & 0x81) == 0x01) 
                && (data >> 1) == dev.short_address)
                || (data == 0xff && dev.short_address == MASK)
                || data == 0x00
            {
		println!("Initialised"); 
                // TODO restart initialisation timer 
                dev.initialisation_state = InitialisationState::ENABLED;
            }
            
            NO_REPLY
        },
        cmd::RANDOMISE if flags.send_twice() => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.random_address = rand::thread_rng().gen_range(0..=0xffffff);
            }
            NO_REPLY
        },
        cmd::COMPARE => {
	    println!("Comparing: 0x{:06x} <=  0x{:06x}", 
		    dev.random_address,  dev.search_address);
            if dev.initialisation_state == InitialisationState::ENABLED
                && dev.random_address <= dev.search_address {
                    YES_REPLY
                } else {
                    NO_REPLY
                }
        },
        cmd::WITHDRAW => {
             if dev.initialisation_state == InitialisationState::ENABLED
                && dev.random_address == dev.search_address {
                    dev.initialisation_state = InitialisationState::WITHDRAWN; 
                }
            NO_REPLY
        },
        cmd::SEARCHADDRH => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.search_address =
                    (dev.search_address & 0x00ffff) | ((data as u32) << 16);
            }
            NO_REPLY
        },
        cmd::SEARCHADDRM => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.search_address =
                    (dev.search_address & 0xff00ff) | ((data as u32) << 8);
            }
            NO_REPLY
        },
        cmd::SEARCHADDRL => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.search_address =
                    (dev.search_address & 0xffff00) | (data as u32);
            }
            NO_REPLY
        },
        
        cmd::PROGRAM_SHORT_ADDRESS => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                if (data & 0x81) == 0x01 {
                    dev.short_address = data>>1;
                } else if data == MASK {
                    dev.short_address = MASK;
                }
            }
            NO_REPLY         
        },
        cmd::QUERY_SHORT_ADDRESS => {
            if dev.initialisation_state != InitialisationState::DISABLED
                && dev.search_address == dev.random_address {
                    eprintln!("Query_Short_Address: {}", dev.short_address);
                    Some(DaliBusEventType::Frame8((dev.short_address<<1) |0x01))
                } else {
                    NO_REPLY
                }
        },
        cmd::DTR0 => {
            dev.dtr0 = data;
            NO_REPLY
        },
        cmd::DTR1 => {
            dev.dtr1 = data;
            NO_REPLY
        },
        cmd::DTR2 => {
            dev.dtr2 = data;
            NO_REPLY
        },
        
        _ => {
            NO_REPLY
        }
    }
}



impl DaliSimDevice for DaliSimGear
{
    fn start(&mut self, mut host: Box<dyn DaliSimHost>)
	     -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>
    {
	self.source_id = host.next_source_id();
	self.host = Some(host);
	Box::pin(future::ready(Ok(())))
    }
    
    fn stop(&mut self) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>
    {
	Box::pin(future::ready(Ok(())))
    }
    
    fn event(&mut self,event: &DaliSimEvent) 
             -> Option<DaliSimEvent>
    {
	let mut flags = Flags::Empty;
	match (event, &self.last_event) {
	    (DaliSimEvent{timestamp: ts, 
			  event_type: DaliBusEventType::Frame16(cmd), ..},
	     DaliSimEvent{timestamp: last_ts, 
			  event_type: DaliBusEventType::Frame16(last_cmd),
			  ..}) => {
		if ts.duration_since(*last_ts) 
		    < FRAME_16_DURATION + SEND_TWICE_DURATION 
		    && cmd == last_cmd
		{
		    flags = flags | Flags::SendTwice(true);
		}
	    },
	    _ => {}
	}
	self.last_event = event.clone();
	let event_type = match event.event_type {
	    DaliBusEventType::Frame16(cmd) => {
		eprintln!("Gear {} received: {:02x} {:02x}", 
			   self.short_address,
			   cmd[0], cmd[1]);
		match cmd[0] >> 1 {
		    addr @ 0x00..=0x3f if addr == self.short_address => {
			device_cmd(self, cmd[0], cmd[1], flags)
		    },
		    addr @ 0x40..=0x4f 
			if self.gear_groups & (1<<(addr & 0x0f)) != 0 => 
		    {
			device_cmd(self, cmd[0], cmd[1], flags)
		    },
		    0x7e if self.short_address == MASK => {
			device_cmd(self, cmd[0], cmd[1], flags)
		    },
		    0x7f => {
			device_cmd(self, cmd[0], cmd[1], flags)
		    },
		    _ => {
			special_cmd(self, cmd[0], cmd[1], flags)
		    }
		}
	    },
	    _ => None
	};
	if let Some(event_type) = event_type {
	    Some(DaliSimEvent{
		source_id: self.source_id,
		timestamp: Instant::now() +FRAME_8_DURATION+REPLY_DELAY,
		event_type})
	} else {
	    None
	}
    }
}

