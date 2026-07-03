use super::device::{DALI_SIMULATOR_DEVICES, DaliSimDevice, DaliSimDeviceEntry};
use super::sim_bus::{DaliSimBusDevice, DaliSimBusDeviceEvent, DaliSimBusEvent};
use super::timing::{
    FRAME_8_DURATION, FRAME_16_DURATION, INIT_TIMEOUT, REPLY_DELAY, SEND_TWICE_DURATION,
};
use crate::common::defs::MASK;
use crate::drivers::driver::DaliBusEventType;
use crate::drivers::send_flags::Flags;
use crate::drivers::simulator::gear::rand::RngExt;
use crate::gear::cmd_defs;
use crate::gear::{device_type, fade, light_source, status};
use linkme::distributed_slice;
use log::debug;
use std::future;
use std::pin::Pin;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::time::Instant;
use tokio::task::JoinHandle;

extern crate rand;

type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(PartialEq)]
pub enum InitialisationState {
    ENABLED,
    DISABLED,
    WITHDRAWN,
}

#[derive(PartialEq)]
pub enum WriteEnableState {
    ENABLED,
    DISABLED,
}

pub struct GearState {
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
    pub scene: [u8; 16],
    pub dtr0: u8,
    pub dtr1: u8,
    pub dtr2: u8,
    pub physical_minimum_level: u8,

    // Fade endpoints. Scaled for better precision.
    // Scaled by 128
    fade_start_level: i16,
    // Scaled by 128
    fade_end_level: i16,

    // Timers
    fade_start_time: Instant,
    fade_duration: Duration,
    init_end_time: Instant,
}

#[allow(dead_code)]
pub struct DaliSimGear {
    state: Arc<RwLock<GearState>>,
    thread: Option<JoinHandle<()>>,
}

impl DaliSimGear {
    pub fn new() -> DaliSimGear {
        let mut rng = rand::rng();
        let phm = 0x01;
        let now = Instant::now();
        let state = GearState {
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
            random_address: rng.random_range(0..0x1000000),
            operating_mode: 0,
            initialisation_state: InitialisationState::DISABLED,
            write_enable_state: WriteEnableState::DISABLED,
            status: 0x00,
            gear_groups: 0x0000,
            scene: [MASK; 16],
            dtr0: 0,
            dtr1: 0,
            dtr2: 0,
            physical_minimum_level: phm,

            fade_start_level: 0,
            // Scaled by 128
            fade_end_level: 0,

            fade_start_time: now,
            fade_duration: Duration::new(0, 0),
            init_end_time: now + INIT_TIMEOUT,
        };
        DaliSimGear {
            state: Arc::new(RwLock::new(state)),
            thread: None,
        }
    }
}

fn check_timers(dev: &mut GearState) {
    if dev.initialisation_state != InitialisationState::DISABLED {
        if dev.init_end_time.elapsed() >= INIT_TIMEOUT {
            dev.initialisation_state = InitialisationState::DISABLED;
        }
    }

    if (dev.status & status::flag::FADE_RUNNING) != 0 {
        let elapsed = dev.fade_start_time.elapsed();
        if elapsed >= dev.fade_duration {
            dev.actual_level = dev.target_level;
            dev.status &= !status::flag::FADE_RUNNING;
        } else {
            let elapsed_millis = elapsed.as_millis() as i128;
            let duration_millis = dev.fade_duration.as_millis() as i128;
            dev.actual_level = ((dev.fade_start_level
                + (((dev.fade_end_level - dev.fade_start_level) as i128 * elapsed_millis
                    + duration_millis / 2)
                    / duration_millis) as i16)
                >> 7) as u8;
        }
    }
}

const fn fade_time(n: u8) -> Duration {
    let n = n as u64;
    let millis = (1u64 << (n / 2)) * ((n & 1) * 707 + (1 - (n & 1)) * 500);
    Duration::from_millis(millis)
}

const FADE_TIMES: [Duration; 16] = [
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
    fade_time(15),
];

const FADE_MULTIPLIER: [Duration; 5] = [
    Duration::from_millis(0),
    Duration::from_millis(100),
    Duration::from_secs(1),
    Duration::from_secs(10),
    Duration::from_secs(60),
];

fn start_fade_time(dev: &mut GearState) {
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
                dev.fade_duration = FADE_MULTIPLIER[dev.extended_fade_time as usize >> 4]
                    * ((dev.extended_fade_time & 0x0f) + 1) as u32;
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

fn query_status_flag(dev: &GearState, flag: u8) -> Option<DaliBusEventType> {
    if (dev.status & flag) != 0 {
        YES_REPLY
    } else {
        NO_REPLY
    }
}

// Status flags that are not dependant on any other state
pub const STORED_STATUS_FLAGS: u8 = status::flag::GEAR_FAILURE
    | status::flag::LAMP_FAILURE
    | status::flag::LIMIT_ERROR
    | status::flag::FADE_RUNNING
    | status::flag::RESET_STATE
    | status::flag::POWER_CYCLE;

fn update_status(dev: &mut GearState) {
    dev.status = (dev.status & STORED_STATUS_FLAGS)
        | if dev.actual_level > 0 {
            status::flag::LAMP_ON
        } else {
            0
        }
        | if dev.short_address == MASK {
            status::flag::NO_ADDRESS
        } else {
            0
        };
}

fn yes_no(p: bool) -> Option<DaliBusEventType> {
    if p { YES_REPLY } else { NO_REPLY }
}

const YES_REPLY: Option<DaliBusEventType> = Some(DaliBusEventType::Frame8(MASK));
const NO_REPLY: Option<DaliBusEventType> = None;

fn device_cmd(dev: &mut GearState, _addr: u8, cmd: u8, _flags: Flags) -> Option<DaliBusEventType> {
    match cmd {
        cmd_defs::QUERY_STATUS_OPCODE_BYTE => {
            update_status(dev);
            return Some(DaliBusEventType::Frame8(dev.status));
        }
        cmd_defs::QUERY_CONTROL_GEAR_PRESENT_OPCODE_BYTE => return YES_REPLY,
        cmd_defs::QUERY_CONTROL_GEAR_FAILURE_OPCODE_BYTE => {
            return query_status_flag(&dev, status::flag::GEAR_FAILURE);
        }
        cmd_defs::QUERY_LAMP_FAILURE_OPCODE_BYTE => {
            return query_status_flag(&dev, status::flag::LAMP_FAILURE);
        }
        cmd_defs::QUERY_LAMP_POWER_ON_OPCODE_BYTE => return yes_no(dev.actual_level > 0),
        cmd_defs::QUERY_LIMIT_ERROR_OPCODE_BYTE => {
            return query_status_flag(&dev, status::flag::LIMIT_ERROR);
        }
        cmd_defs::QUERY_RESET_STATE_OPCODE_BYTE => {
            return query_status_flag(&dev, status::flag::RESET_STATE);
        }
        cmd_defs::QUERY_MISSING_SHORT_ADDRESS_OPCODE_BYTE => {
            return yes_no(dev.short_address == MASK);
        }
        cmd_defs::QUERY_VERSION_NUMBER_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(2 << 2 + 0)); // 2.0
        }
        cmd_defs::QUERY_DEVICE_TYPE_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(device_type::types::LED));
        }
        cmd_defs::QUERY_NEXT_DEVICE_TYPE_OPCODE_BYTE => return NO_REPLY,
        cmd_defs::QUERY_PHYSICAL_MINIMUM_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.physical_minimum_level));
        }
        cmd_defs::QUERY_POWER_FAILURE_OPCODE_BYTE => {
            return query_status_flag(&dev, status::flag::POWER_CYCLE);
        }
        cmd_defs::QUERY_CONTENT_DTR0_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.dtr0));
        }
        cmd_defs::QUERY_CONTENT_DTR1_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.dtr1));
        }
        cmd_defs::QUERY_CONTENT_DTR2_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.dtr2));
        }
        cmd_defs::QUERY_OPERATING_MODE_OPCODE_BYTE => return Some(DaliBusEventType::Frame8(0x00)),
        cmd_defs::QUERY_LIGHT_SOURCE_TYPE_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(light_source::LED));
        }
        cmd_defs::QUERY_ACTUAL_LEVEL_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.actual_level));
        }
        cmd_defs::QUERY_MAX_LEVEL_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.max_level));
        }
        cmd_defs::QUERY_MIN_LEVEL_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.min_level));
        }
        cmd_defs::QUERY_POWER_ON_LEVEL_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.power_on_level));
        }
        cmd_defs::QUERY_SYSTEM_FAILURE_LEVEL_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.system_failure_level));
        }
        cmd_defs::QUERY_FADE_OPCODE_BYTE => return Some(DaliBusEventType::Frame8(dev.fade)),
        cmd_defs::QUERY_SCENE_LEVEL_FIRST_OPCODE_BYTE
            ..=cmd_defs::QUERY_SCENE_LEVEL_LAST_OPCODE_BYTE => {
            let level = dev.scene[(cmd - cmd_defs::QUERY_SCENE_LEVEL_FIRST_OPCODE_BYTE) as usize];
            return Some(DaliBusEventType::Frame8(level));
        }
        cmd_defs::QUERY_GROUPS_0_7_OPCODE_BYTE => {
            let groups = (dev.gear_groups & 0xff) as u8;
            return Some(DaliBusEventType::Frame8(groups));
        }
        cmd_defs::QUERY_GROUPS_8_15_OPCODE_BYTE => {
            let groups = (dev.gear_groups >> 8) as u8;
            return Some(DaliBusEventType::Frame8(groups));
        }
        cmd_defs::QUERY_RANDOM_ADDRESS_H_OPCODE_BYTE => {
            let addr = (dev.random_address >> 16) as u8;
            return Some(DaliBusEventType::Frame8(addr));
        }
        cmd_defs::QUERY_RANDOM_ADDRESS_M_OPCODE_BYTE => {
            let addr = ((dev.random_address >> 8) & 0xff) as u8;
            return Some(DaliBusEventType::Frame8(addr));
        }
        cmd_defs::QUERY_RANDOM_ADDRESS_L_OPCODE_BYTE => {
            let addr = (dev.random_address & 0xff) as u8;
            return Some(DaliBusEventType::Frame8(addr));
        }
        _ => {}
    }
    None
}

fn special_cmd(dev: &mut GearState, cmd: u8, data: u8, flags: Flags) -> Option<DaliBusEventType> {
    //eprintln!("Special cmd: {:02x}", cmd);
    match cmd {
        cmd_defs::TERMINATE_ADDRESS_BYTE => {
            dev.initialisation_state = InitialisationState::DISABLED;
            // TODO stop identification
            NO_REPLY
        }
        cmd_defs::INITIALISE_ADDRESS_BYTE if flags.send_twice() => {
            if (((data & 0x81) == 0x01) && (data >> 1) == dev.short_address)
                || (data == cmd_defs::INITIALISE_NO_ADDR_OPCODE_BYTE && dev.short_address == MASK)
                || data == cmd_defs::INITIALISE_ALL_OPCODE_BYTE
            {
                println!("Initialised");
                // TODO restart initialisation timer
                dev.initialisation_state = InitialisationState::ENABLED;
            }

            NO_REPLY
        }
        cmd_defs::RANDOMISE_ADDRESS_BYTE if flags.send_twice() => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                let mut rng = rand::rng();
                dev.random_address = rng.random_range(0..=0xffffff);
            }
            NO_REPLY
        }
        cmd_defs::COMPARE_ADDRESS_BYTE => {
            println!(
                "Comparing: 0x{:06x} <=  0x{:06x}",
                dev.random_address, dev.search_address
            );
            if dev.initialisation_state == InitialisationState::ENABLED
                && dev.random_address <= dev.search_address
            {
                YES_REPLY
            } else {
                NO_REPLY
            }
        }
        cmd_defs::WITHDRAW_ADDRESS_BYTE => {
            if dev.initialisation_state == InitialisationState::ENABLED
                && dev.random_address == dev.search_address
            {
                dev.initialisation_state = InitialisationState::WITHDRAWN;
            }
            NO_REPLY
        }
        cmd_defs::SEARCHADDRH_ADDRESS_BYTE => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.search_address = (dev.search_address & 0x00ffff) | ((data as u32) << 16);
            }
            NO_REPLY
        }
        cmd_defs::SEARCHADDRM_ADDRESS_BYTE => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.search_address = (dev.search_address & 0xff00ff) | ((data as u32) << 8);
            }
            NO_REPLY
        }
        cmd_defs::SEARCHADDRL_ADDRESS_BYTE => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                dev.search_address = (dev.search_address & 0xffff00) | (data as u32);
            }
            NO_REPLY
        }

        cmd_defs::PROGRAM_SHORT_ADDRESS_ADDRESS_BYTE => {
            if dev.initialisation_state != InitialisationState::DISABLED {
                if (data & 0x81) == 0x01 {
                    dev.short_address = data >> 1;
                } else if data == MASK {
                    dev.short_address = MASK;
                }
            }
            NO_REPLY
        }
        cmd_defs::QUERY_SHORT_ADDRESS_ADDRESS_BYTE => {
            if dev.initialisation_state != InitialisationState::DISABLED
                && dev.search_address == dev.random_address
            {
                debug!("Query_Short_Address: {}", dev.short_address);
                Some(DaliBusEventType::Frame8((dev.short_address << 1) | 0x01))
            } else {
                NO_REPLY
            }
        }
        cmd_defs::DTR0_ADDRESS_BYTE => {
            dev.dtr0 = data;
            NO_REPLY
        }
        cmd_defs::DTR1_ADDRESS_BYTE => {
            dev.dtr1 = data;
            NO_REPLY
        }
        cmd_defs::DTR2_ADDRESS_BYTE => {
            dev.dtr2 = data;
            NO_REPLY
        }

        _ => NO_REPLY,
    }
}
fn device_init() -> Box<dyn DaliSimDevice> {
    Box::new(DaliSimGear::new())
}

#[distributed_slice(DALI_SIMULATOR_DEVICES)]
static DALI_SIMULATOR_DEVICE: DaliSimDeviceEntry = DaliSimDeviceEntry {
    name: "generic_gear",
    init: device_init,
};

impl DaliSimDevice for DaliSimGear {
    fn configure(&mut self, conf: &yaml_serde::value::Mapping) -> DynResult<()> {
        Ok(())
    }

    fn start(&mut self, bus_device: DaliSimBusDevice) -> DynResult<()> {
        self.thread = Some(tokio::spawn(device_thread(bus_device, self.state.clone())));
        Ok(())
    }

    fn stop(&mut self) -> DynResult<()> {
        Ok(())
    }
}

async fn device_thread(bus: DaliSimBusDevice, state: Arc<RwLock<GearState>>) {
    let mut last_event = None;
    loop {
        match bus.wait().await {
            DaliSimBusDeviceEvent::Timeout => {}
            DaliSimBusDeviceEvent::Shutdown => break,
            DaliSimBusDeviceEvent::Message(msg) => {
                let mut flags = Flags::Empty;
                match (&msg, &last_event) {
                    (
                        DaliSimBusEvent {
                            timestamp: ts,
                            event_type: DaliBusEventType::Frame16(cmd),
                            ..
                        },
                        Some(DaliSimBusEvent {
                            timestamp: last_ts,
                            event_type: DaliBusEventType::Frame16(last_cmd),
                            ..
                        }),
                    ) => {
                        if ts.duration_since(*last_ts) < FRAME_16_DURATION + SEND_TWICE_DURATION
                            && cmd == last_cmd
                        {
                            flags = flags | Flags::SendTwice(true);
                        }
                    }
                    _ => {}
                }

                last_event = Some(msg.clone());
                let event_type = match msg.event_type {
                    DaliBusEventType::Frame16(cmd) => {
                        let mut state = state.write().unwrap();
                        let short_address = state.short_address;
                        debug!(
                            "Gear {} received: {:02x} {:02x}",
                            short_address, cmd[0], cmd[1]
                        );
                        match cmd[0] >> 1 {
                            addr @ 0x00..=0x3f if addr == short_address => {
                                device_cmd(&mut *state, cmd[0], cmd[1], flags)
                            }
                            addr @ 0x40..=0x4f if state.gear_groups & (1 << (addr & 0x0f)) != 0 => {
                                device_cmd(&mut *state, cmd[0], cmd[1], flags)
                            }
                            0x7e if state.short_address == MASK => {
                                device_cmd(&mut *state, cmd[0], cmd[1], flags)
                            }
                            0x7f => device_cmd(&mut *state, cmd[0], cmd[1], flags),
                            _ => special_cmd(&mut *state, cmd[0], cmd[1], flags),
                        }
                    }
                    _ => None,
                };
                if let Some(event_type) = event_type {
                    let now = bus.current_time();
                    bus.add_event(
                        event_type,
                        now + FRAME_8_DURATION + REPLY_DELAY,
                        Some(now + REPLY_DELAY),
                    );
                }
            }
        }
    }
}
