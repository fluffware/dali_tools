use super::device::{DALI_SIMULATOR_DEVICES, DaliSimDevice, DaliSimDeviceEntry};
use super::sim_bus::{DaliSimBusDevice, DaliSimBusDeviceEvent, DaliSimBusEvent};
use super::timing::{
    FRAME_8_DURATION, FRAME_16_DURATION, INIT_TIMEOUT, REPLY_DELAY, SEND_TWICE_DURATION,
};
use crate::common::defs::MASK;
use crate::drivers::driver::DaliBusEventType;
use crate::drivers::send_flags::Flags;
use crate::gear::cmd_defs;
use crate::gear::{device_type, light_source, status};
use crate::simulator::device::ParameterError;
use field_access_json::{Error as FieldError, FieldAccessJson};
use field_access_json_derive::FieldAccessJson;
use linkme::distributed_slice;
use log::debug;
use rand::RngExt;
use serde_derive::Serialize;
use std::cmp::{max, min};
use std::convert::TryFrom;
use std::ops::Range;
use std::ops::RangeBounds;
use std::ops::Sub;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::time::Instant;
use tokio::task::JoinHandle;

extern crate rand;

type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
fn boxed_err<E>(e: E) -> Box<dyn std::error::Error + Send + Sync>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    Into::<Box<dyn std::error::Error + Send + Sync>>::into(e)
}

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

const LEVEL_HIRES_SHIFT: i32 = 7;
const LEVEL_HIRES_SCALE: i16 = 1i16 << 7;
#[derive(Serialize, FieldAccessJson)]
#[serde(rename_all = "camelCase")]
pub struct GearState {
    pub name: String,

    pub powered: bool,

    #[serde(rename = "targetLevel")]
    pub target_level: i16, // Scaled by 128
    #[serde(rename = "lastActiveLevel")]
    pub last_active_level: u8,
    #[serde(rename = "lastLightLevel")]
    pub last_light_level: u8,
    #[serde(rename = "powerOnLevel")]
    pub power_on_level: u8,
    #[serde(rename = "systemFailureLevel")]
    pub system_failure_level: u8,
    #[serde(rename = "minLevel")]
    pub min_level: u8,
    #[serde(rename = "maxLevel")]
    pub max_level: u8,
    pub fade: u8,               // bit 0-3: fade rate, bit 4-7: fade time
    pub extended_fade_time: u8, // bit 0-3: base, bit 4-6: multiplier
    pub short_address: u8,
    #[serde(rename = "searchAddress")]
    pub search_address: u32,
    #[serde(rename = "randomAddress")]
    pub random_address: u32,
    pub operating_mode: u8,
    #[serde(skip)]
    pub initialisation_state: InitialisationState,
    #[serde(skip)]
    pub write_enable_state: WriteEnableState,
    /*
    0 - controlGearFailure
    1 - lampFailure
    2 - lampOn
    3 - limitError
    4 - fadeRunning
    5 - resetState
    6 - shortAddress is MASK
    7 - powerCycleSeen
    */
    pub status: u8,
    pub gear_groups: u16,
    pub scenes: [u8; 16],
    #[serde(rename = "DTR0")]
    pub dtr0: u8,
    #[serde(rename = "DTR1")]
    pub dtr1: u8,
    #[serde(rename = "DTR2")]
    pub dtr2: u8,
    #[serde(rename = "PHM")]
    pub physical_minimum_level: u8,

    // Slope of actualLevel towards targetLevel, in steps/ms
    #[serde(skip)]
    fade_slope: i32, // Fixed point, slope*(1<<29)
    #[serde(skip)]
    fade_end_time: Instant,

    // Init timer
    #[serde(skip)]
    init_end_time: Instant,

    #[serde(skip)]
    current_time: Instant, // Time for evaluating, timers, fades, etc.
}

const FADE_SLOPE_SHIFT: i32 = 29;

impl GearState {
    pub fn new(name: String, random_address: u32, now: Instant) -> GearState {
        let phm = 0x01;
        GearState {
            name,
            powered: true,
            target_level: 0xfe * LEVEL_HIRES_SCALE,
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
            random_address,
            operating_mode: 0,
            initialisation_state: InitialisationState::DISABLED,
            write_enable_state: WriteEnableState::DISABLED,
            status: 0x00,
            gear_groups: 0x0000,
            scenes: [MASK; 16],
            dtr0: 0,
            dtr1: 0,
            dtr2: 0,
            physical_minimum_level: phm,

            fade_slope: 0,
            fade_end_time: now,

            init_end_time: now + INIT_TIMEOUT,

            current_time: now,
        }
    }

    // Can return negative value due to rounding errors
    pub fn actual_level_hires_at(&self, time: Instant) -> i16 {
        if self.status & status::flag::FADE_RUNNING != 0 {
            if time > self.fade_end_time {
                i16::from(self.target_level)
            } else {
                let millis_left = (self.fade_end_time - time).as_millis() as i64;
                i16::try_from(
                    (self.target_level as i64)
                        - (self.fade_slope as i64 * millis_left
                            + (1 << (FADE_SLOPE_SHIFT - LEVEL_HIRES_SHIFT - 1))
                            >> (FADE_SLOPE_SHIFT - LEVEL_HIRES_SHIFT)),
                )
                .unwrap()
            }
        } else {
            i16::from(self.target_level)
        }
    }
    pub fn actual_level_at(&self, time: Instant) -> u8 {
        ((self.actual_level_hires_at(time) + 64) >> 7) as u8
    }
    pub fn actual_level(&self) -> u8 {
        self.actual_level_at(self.current_time)
    }

    /// Immediately change actual_level and target_level
    pub fn set_actual_level(&mut self, level: u8) {
        self.status &= !status::flag::FADE_RUNNING;
        self.target_level = i16::from(level) * LEVEL_HIRES_SCALE;
    }

    pub fn reset(&mut self) {
        self.target_level = 0xfe * LEVEL_HIRES_SCALE;
        self.last_active_level = 0xfe;
        self.last_light_level = 0xfe;
        self.power_on_level = 0xfe;
        self.system_failure_level = 0xfe;
        self.min_level = self.physical_minimum_level;
        self.max_level = 0xfe;
        self.fade = 0x07;
        self.extended_fade_time = 0x00;
        self.search_address = 0xffffff;
        self.random_address = 0xffffff;
        self.write_enable_state = WriteEnableState::DISABLED;
        self.status &=
            !(status::flag::LIMIT_ERROR | status::flag::FADE_RUNNING | status::flag::POWER_CYCLE);
        self.status |= status::flag::RESET_STATE;

        self.gear_groups = 0x0000;
        self.scenes = [MASK; 16];
        self.dtr0 = 0;
        self.dtr1 = 0;
        self.dtr2 = 0;
    }
}

#[allow(dead_code)]
pub struct DaliSimGear {
    state: Arc<RwLock<GearState>>,
    thread: Option<JoinHandle<()>>,
}

impl DaliSimGear {
    pub fn new(name: String) -> DaliSimGear {
        let mut rng = rand::rng();
        let now = Instant::now();
        let state = GearState::new(name, rng.random_range(0..0x1000000), now);

        DaliSimGear {
            state: Arc::new(RwLock::new(state)),
            thread: None,
        }
    }
}

fn check_timers(dev: &mut GearState) {
    let now = dev.current_time;
    if dev.initialisation_state != InitialisationState::DISABLED {
        if dev.init_end_time <= now {
            dev.initialisation_state = InitialisationState::DISABLED;
        }
    }

    if (dev.status & status::flag::FADE_RUNNING) != 0 {
        if now >= dev.fade_end_time {
            dev.status &= !status::flag::FADE_RUNNING;
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

fn start_fade(dev: &mut GearState, new_target_level: u8, now: Instant, fade_duration: Duration) {
    // Calulate actual position shifted up by FADE_SLOPE_SHIFT
    let actual_level = if dev.status & status::flag::FADE_RUNNING != 0 {
        if now > dev.fade_end_time {
            i64::from(dev.target_level) << (FADE_SLOPE_SHIFT - LEVEL_HIRES_SHIFT)
        } else {
            let millis_left = (dev.fade_end_time - now).as_millis() as i64;
            ((dev.target_level as i64) << (FADE_SLOPE_SHIFT - LEVEL_HIRES_SHIFT))
                - (dev.fade_slope as i64) * millis_left
        }
    } else {
        i64::from(dev.target_level) << (FADE_SLOPE_SHIFT - LEVEL_HIRES_SHIFT)
    };

    let duration = fade_duration.as_millis() as i64;
    dev.fade_slope = i32::try_from(
        (((new_target_level as i64) << FADE_SLOPE_SHIFT) - actual_level + duration / 2) / duration,
    )
    .unwrap();
    dev.fade_end_time = now + fade_duration;
    dev.target_level = i16::from(new_target_level) << LEVEL_HIRES_SHIFT;
    dev.status |= status::flag::FADE_RUNNING;
}

fn stop_fade(dev: &mut GearState) {
    if dev.status & status::flag::FADE_RUNNING != 0 {
        dev.target_level = dev.actual_level_hires_at(dev.current_time);
        dev.status &= !status::flag::FADE_RUNNING;
    }
}
fn start_fade_time(dev: &mut GearState, new_target_level: u8) {
    debug!("Start fade to {new_target_level}");
    if new_target_level == MASK {
        stop_fade(dev);
        return;
    }
    let now = dev.current_time;

    let fade_duration = if (dev.fade & 0xf0) == 0x0 {
        // Use extended fade times
        if (dev.extended_fade_time & 0x70) == 0 || dev.extended_fade_time > 0x4f {
            // Extended fade is zero
            dev.set_actual_level(new_target_level);
            return;
        } else {
            // Extended fade time
            FADE_MULTIPLIER[dev.extended_fade_time as usize >> 4]
                * ((dev.extended_fade_time & 0x0f) + 1) as u32
        }
    } else {
        // Basic fadetime
        FADE_TIMES[dev.fade as usize >> 4]
    };
    start_fade(dev, new_target_level, now, fade_duration);
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
        | if dev.actual_level() > 0 {
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

fn query_cmd(dev: &mut GearState, _addr: u8, cmd: u8, _flags: Flags) -> Option<DaliBusEventType> {
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
        cmd_defs::QUERY_LAMP_POWER_ON_OPCODE_BYTE => return yes_no(dev.actual_level() > 0),
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
        cmd_defs::QUERY_EXTENDED_VERSION_NUMBER_OPCODE_BYTE => return NO_REPLY,
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
            return Some(DaliBusEventType::Frame8(dev.actual_level()));
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
        cmd_defs::QUERY_EXTENDED_FADE_TIME_OPCODE_BYTE => {
            return Some(DaliBusEventType::Frame8(dev.extended_fade_time));
        }
        cmd_defs::QUERY_SCENE_LEVEL_FIRST_OPCODE_BYTE
            ..=cmd_defs::QUERY_SCENE_LEVEL_LAST_OPCODE_BYTE => {
            let level = dev.scenes[(cmd - cmd_defs::QUERY_SCENE_LEVEL_FIRST_OPCODE_BYTE) as usize];
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

fn set_target_level(dev: &mut GearState, level: u8) {
    let new_target = if level <= dev.min_level {
        dev.min_level
    } else if level >= dev.max_level {
        dev.max_level
    } else {
        level
    };
    start_fade_time(dev, new_target);
}
fn level_cmd(dev: &mut GearState, cmd: u8, _flags: Flags) -> Option<DaliBusEventType> {
    match cmd {
        cmd_defs::OFF_OPCODE_BYTE => {
            set_target_level(dev, 0);
        }
        cmd_defs::UP_OPCODE_BYTE => {
            todo!()
        }
        cmd_defs::DOWN_OPCODE_BYTE => {
            todo!()
        }
        cmd_defs::STEP_UP_OPCODE_BYTE => {
            todo!()
        }
        cmd_defs::STEP_DOWN_OPCODE_BYTE => {
            todo!()
        }
        cmd_defs::RECALL_MAX_LEVEL_OPCODE_BYTE => {
            if dev.initialisation_state == InitialisationState::DISABLED {
                start_fade_time(dev, dev.max_level);
            } else {
                set_target_level(dev, dev.max_level);
            }
        }
        cmd_defs::RECALL_MIN_LEVEL_OPCODE_BYTE => {
            if dev.initialisation_state == InitialisationState::DISABLED {
                start_fade_time(dev, dev.min_level);
            } else {
                set_target_level(dev, dev.min_level);
            }
        }
        cmd_defs::STEP_DOWN_AND_OFF_OPCODE_BYTE => {
            todo!()
        }
        cmd_defs::ON_AND_STEP_UP_OPCODE_BYTE => {
            todo!()
        }
        cmd_defs::ENABLE_DAPC_SEQUENCE_OPCODE_BYTE => {
            todo!()
        }
        _ => {}
    }
    NO_REPLY
}
fn goto_scene_cmd(
    dev: &mut GearState,
    _addr: u8,
    cmd: u8,
    _flags: Flags,
) -> Option<DaliBusEventType> {
    if (cmd_defs::GOTO_SCENE_FIRST_OPCODE_BYTE..=cmd_defs::GOTO_SCENE_LAST_OPCODE_BYTE)
        .contains(&cmd)
    {
        let scene = usize::from(cmd - cmd_defs::GOTO_SCENE_FIRST_OPCODE_BYTE);
        start_fade_time(dev, dev.scenes[scene]);
    }
    NO_REPLY
}
fn set_scene_cmd(
    dev: &mut GearState,
    _addr: u8,
    cmd: u8,
    flags: Flags,
) -> Option<DaliBusEventType> {
    if flags.send_twice() {
        if (cmd_defs::SET_SCENE_FIRST_OPCODE_BYTE..=cmd_defs::SET_SCENE_LAST_OPCODE_BYTE)
            .contains(&cmd)
        {
            let scene = usize::from(cmd - cmd_defs::SET_SCENE_FIRST_OPCODE_BYTE);
            dev.scenes[scene] = dev.dtr0;
        }
    }
    NO_REPLY
}
fn remove_from_scene_cmd(
    dev: &mut GearState,
    _addr: u8,
    cmd: u8,
    flags: Flags,
) -> Option<DaliBusEventType> {
    if flags.send_twice() {
        if (cmd_defs::REMOVE_FROM_SCENE_FIRST_OPCODE_BYTE
            ..=cmd_defs::REMOVE_FROM_SCENE_LAST_OPCODE_BYTE)
            .contains(&cmd)
        {
            let scene = usize::from(cmd - cmd_defs::REMOVE_FROM_SCENE_FIRST_OPCODE_BYTE);
            dev.scenes[scene] = MASK;
        }
    }
    NO_REPLY
}

fn device_control_cmd(dev: &mut GearState, cmd: u8, flags: Flags) -> Option<DaliBusEventType> {
    if flags.send_twice() {
        match cmd {
            cmd_defs::RESET_OPCODE_BYTE => {
                dev.reset();
            }
            cmd_defs::STORE_ACTUAL_LEVEL_IN_DTR0_OPCODE_BYTE => {
                dev.dtr0 = dev.actual_level();
            }
            cmd_defs::SAVE_PERSISTENT_VARIABLES_OPCODE_BYTE => {
                // NOP
            }
            cmd_defs::SET_OPERATING_MODE_OPCODE_BYTE => {
                // NOP
            }
            cmd_defs::RESET_MEMORY_BANK_OPCODE_BYTE => {
                todo! {}
            }
            cmd_defs::IDENTIFY_DEVICE_OPCODE_BYTE => {
                todo! {}
            }

            cmd_defs::SET_SHORT_ADDRESS_OPCODE_BYTE => {
                dev.short_address = dev.dtr0;
            }
            _ => {}
        }
    }
    NO_REPLY
}
fn set_param_cmd(dev: &mut GearState, cmd: u8, flags: Flags) -> Option<DaliBusEventType> {
    if flags.send_twice() {
        match cmd {
            cmd_defs::SET_MAX_LEVEL_OPCODE_BYTE => {
                // 11.4.8
                dev.max_level = if dev.min_level > dev.dtr0 {
                    dev.min_level
                } else if dev.dtr0 == MASK {
                    0xfe
                } else {
                    dev.dtr0
                };
                if dev.actual_level() > dev.max_level {
                    dev.set_actual_level(dev.max_level);
                } else if dev.target_level > i16::from(dev.max_level) * LEVEL_HIRES_SCALE
                    && dev.fade_end_time > dev.current_time
                {
                    start_fade(
                        dev,
                        dev.max_level,
                        dev.current_time,
                        dev.fade_end_time - dev.current_time,
                    );
                }
                None
            }
            cmd_defs::SET_MIN_LEVEL_OPCODE_BYTE => {
                // 11.4.9
                dev.min_level = if dev.dtr0 <= dev.physical_minimum_level {
                    dev.physical_minimum_level
                } else if dev.max_level < dev.dtr0 || dev.dtr0 == MASK {
                    dev.max_level
                } else {
                    dev.dtr0
                };
                let actual_level = dev.actual_level();
                if actual_level > 0 && actual_level < dev.min_level {
                    dev.set_actual_level(dev.max_level);
                } else if dev.target_level < i16::from(dev.min_level) * LEVEL_HIRES_SCALE
                    && dev.fade_end_time > dev.current_time
                {
                    start_fade(
                        dev,
                        dev.min_level,
                        dev.current_time,
                        dev.fade_end_time - dev.current_time,
                    );
                }
                None
            }
            cmd_defs::SET_SYSTEM_FAILURE_LEVEL_OPCODE_BYTE => {
                dev.system_failure_level = dev.dtr0;
                None
            }
            cmd_defs::SET_POWER_ON_LEVEL_OPCODE_BYTE => {
                dev.power_on_level = dev.dtr0;
                None
            }
            cmd_defs::SET_FADE_TIME_OPCODE_BYTE => {
                let fade_time = min(dev.dtr0, 15);
                dev.fade = (dev.fade & 0x0f) | (fade_time << 4);
                None
            }
            cmd_defs::SET_FADE_RATE_OPCODE_BYTE => {
                let fade_rate = max(1, min(dev.dtr0, 15));
                dev.fade = (dev.fade & 0xf0) | fade_rate;
                None
            }
            cmd_defs::SET_EXTENDED_FADE_TIME_OPCODE_BYTE => {
                dev.extended_fade_time = if dev.dtr0 > 0x4f { 0 } else { dev.dtr0 };
                None
            }
            _ => None,
        }
    } else {
        None
    }
}
fn add_to_group_cmd(
    dev: &mut GearState,
    _addr: u8,
    cmd: u8,
    flags: Flags,
) -> Option<DaliBusEventType> {
    if flags.send_twice() {
        if (cmd_defs::ADD_TO_GROUP_FIRST_OPCODE_BYTE..=cmd_defs::ADD_TO_GROUP_LAST_OPCODE_BYTE)
            .contains(&cmd)
        {
            dev.gear_groups |= 1 << (cmd - cmd_defs::ADD_TO_GROUP_FIRST_OPCODE_BYTE);
        }
    }
    NO_REPLY
}
fn remove_from_group_cmd(
    dev: &mut GearState,
    _addr: u8,
    cmd: u8,
    flags: Flags,
) -> Option<DaliBusEventType> {
    if flags.send_twice() {
        if (cmd_defs::REMOVE_FROM_GROUP_FIRST_OPCODE_BYTE
            ..=cmd_defs::REMOVE_FROM_GROUP_LAST_OPCODE_BYTE)
            .contains(&cmd)
        {
            dev.gear_groups &= !(1 << (cmd - cmd_defs::REMOVE_FROM_GROUP_FIRST_OPCODE_BYTE));
        }
    }
    NO_REPLY
}
fn memory_cmd(
    _dev: &mut GearState,
    _addr: u8,
    _cmd: u8,
    _flags: Flags,
) -> Option<DaliBusEventType> {
    NO_REPLY
}
fn application_extended_cmd(
    _dev: &mut GearState,
    _addr: u8,
    _cmd: u8,
    _flags: Flags,
) -> Option<DaliBusEventType> {
    NO_REPLY
}

fn device_cmd(dev: &mut GearState, addr: u8, cmd: u8, flags: Flags) -> Option<DaliBusEventType> {
    if (addr & 1) == 1 {
        match cmd {
            0x00..=0x0a => level_cmd(dev, cmd, flags),
            0x10..=0x1f => goto_scene_cmd(dev, addr, cmd, flags),
            0x20..=0x25 => device_control_cmd(dev, cmd, flags),
            0x2a..=0x30 => set_param_cmd(dev, cmd, flags),
            0x40..=0x4f => set_scene_cmd(dev, addr, cmd, flags),
            0x50..=0x5f => remove_from_scene_cmd(dev, addr, cmd, flags),
            0x60..=0x6f => add_to_group_cmd(dev, addr, cmd, flags),
            0x70..=0x7f => remove_from_group_cmd(dev, addr, cmd, flags),
            0x80 => device_control_cmd(dev, cmd, flags),
            0x81 => memory_cmd(dev, addr, cmd, flags),
            0x90..=0xc4 => query_cmd(dev, addr, cmd, flags),
            0xc5 => memory_cmd(dev, addr, cmd, flags),
            0xe0..=0xfe => application_extended_cmd(dev, addr, cmd, flags),
            0xff => query_cmd(dev, addr, cmd, flags),
            _ => None,
        }
    } else {
        start_fade_time(dev, cmd);
        None
    }
}
fn special_cmd(dev: &mut GearState, cmd: u8, data: u8, flags: Flags) -> Option<DaliBusEventType> {
    //eprintln!("Special cmd: {:02x}", cmd);
    match cmd {
        cmd_defs::TERMINATE_ADDRESS_BYTE => {
            dev.initialisation_state = InitialisationState::DISABLED;
            NO_REPLY
        }
        cmd_defs::INITIALISE_ADDRESS_BYTE if flags.send_twice() => {
            if (((data & 0x81) == 0x01) && (data >> 1) == dev.short_address)
                || (data == cmd_defs::INITIALISE_NO_ADDR_OPCODE_BYTE && dev.short_address == MASK)
                || data == cmd_defs::INITIALISE_ALL_OPCODE_BYTE
            {
                debug!("Initialised");
                dev.init_end_time = dev.current_time + INIT_TIMEOUT;
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
            debug!(
                "Comparing: 0x{:06x} <=  0x{:06x}",
                dev.random_address, dev.search_address
            );
            if dev.initialisation_state == InitialisationState::ENABLED
                && dev.random_address <= dev.search_address
            {
                debug!("Compare success");
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
            if dev.initialisation_state != InitialisationState::DISABLED
                && dev.random_address == dev.search_address
            {
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
fn device_init(name: String) -> Box<dyn DaliSimDevice> {
    Box::new(DaliSimGear::new(name))
}

#[distributed_slice(DALI_SIMULATOR_DEVICES)]
static DALI_SIMULATOR_DEVICE: DaliSimDeviceEntry = DaliSimDeviceEntry {
    name: "generic_gear",
    init: device_init,
};

fn configure_variable_uint<T, R>(
    conf: &yaml_serde::value::Mapping,
    name: &str,
    var: &mut T,
    range: R,
    offset: T,
) -> DynResult<()>
where
    T: Copy + TryFrom<u64> + Sub<Output = T>,
    R: RangeBounds<u64>,
    <T as TryFrom<u64>>::Error: std::error::Error + Send + Sync + 'static,
{
    if let Some(value) = conf.get(name) {
        let conf_value: u64 = value.as_u64().ok_or_else(|| {
            boxed_err(format!("Value for '{}' is not an unsigned integer", name).as_str())
        })?;
        if !range.contains(&conf_value) {
            return Err(boxed_err(format!("Value for '{}' is out of range", name)));
        }
        let var_value: T = conf_value.try_into()?;
        *var = var_value - offset;
    }
    Ok(())
}

fn set_bit_range(value: &mut u8, new_value: u8, bits: Range<usize>) {
    let mask = ((1 << (bits.end - bits.start)) - 1) << bits.start;

    *value = (*value & !mask) | ((new_value << bits.start) & mask);
}

fn set_bit(bits: &mut u8, bit: u8, value: bool) {
    let mask = 1 << bit;
    if value {
        *bits |= mask;
    } else {
        *bits &= !mask;
    }
}

impl DaliSimDevice for DaliSimGear {
    fn configure(&mut self, conf: &yaml_serde::value::Mapping, index: usize) -> DynResult<()> {
        let mut state = self.state.write().unwrap();
        configure_variable_uint(
            conf,
            "randomAddress",
            &mut state.random_address,
            0..=0xffffff,
            0u32,
        )?;
        let mut step = 1u8;
        configure_variable_uint(conf, "shortAddressStep", &mut step, 1..=64, 0)?;
        let mut short_address = 0u8;
        configure_variable_uint(conf, "shortAddress", &mut short_address, 1..=64, 1)?;
        short_address += step * index as u8;
        if !((0..64).contains(&(short_address))) {
            return Err("End address out of bounds".into());
        }
        state.short_address = short_address;
        configure_variable_uint(
            conf,
            "lastLightLevel",
            &mut state.last_light_level,
            0..=255,
            0u8,
        )?;
        let mut target_level = 0;
        configure_variable_uint(conf, "targetLevel", &mut target_level, 0..255, 0)?;
        set_target_level(&mut *state, target_level);

        configure_variable_uint(conf, "powerOnLevel", &mut state.power_on_level, 0..255, 0)?;
        configure_variable_uint(
            conf,
            "systemFailureLevel",
            &mut state.system_failure_level,
            0..=255,
            0,
        )?;
        configure_variable_uint(conf, "minLevel", &mut state.min_level, 0..=255, 0)?;
        configure_variable_uint(conf, "maxLevel", &mut state.max_level, 0..=255, 0)?;
        if let Some(value) = conf.get("fadeRate").and_then(|v| v.as_u64()) {
            state.fade = (state.fade & 0xf0) | (value as u8 & 0x0f);
        }
        if let Some(value) = conf.get("fadeTime").and_then(|v| v.as_u64()) {
            state.fade = (state.fade & 0x0f) | (value as u8 & 0x0f) << 4;
        }
        if let Some(value) = conf.get("extendedFadeTimeBase").and_then(|v| v.as_u64()) {
            state.extended_fade_time = (state.extended_fade_time & 0xf0) | (value as u8 & 0x0f);
        }
        if let Some(value) = conf
            .get("extendedFadeTimeMultiplier")
            .and_then(|v| v.as_u64())
        {
            state.extended_fade_time =
                (state.extended_fade_time & 0x0f) | (value as u8 & 0x07) << 4;
        }
        match conf.get("gearGroups") {
            Some(yaml_serde::Value::Sequence(groups)) => {
                for group in groups {
                    let bit = group
                        .as_u64()
                        .ok_or_else(|| boxed_err("Invalid group number"))?;
                    if !(1..=16).contains(&bit) {
                        return Err("Invalid group number".into());
                    }
                    state.gear_groups |= 1 << (bit - 1);
                }
            }
            Some(yaml_serde::Value::Number(groups)) if let Some(g) = groups.as_u64() => {
                state.gear_groups =
                    u16::try_from(g).map_err(|e| format!("Illegal group bitmask: {}", e))?;
            }
            Some(_) => return Err("'gearGroups' must be a sequence or a number".into()),
            None => {}
        }
        match conf.get("scenes") {
            Some(yaml_serde::Value::Sequence(scenes)) => {
                if scenes.len() > 16 {
                    return Err("too many scenes".into());
                }
                for (index, level_val) in scenes.iter().enumerate() {
                    let level = level_val
                        .as_u64()
                        .ok_or_else(|| boxed_err("Invalid level"))?;
                    if !(0..=255).contains(&level) {
                        return Err("Level out of range".into());
                    }
                    state.scenes[index] = level.try_into().map_err(|e| boxed_err(e))?;
                }
            }
            Some(yaml_serde::Value::Mapping(scenes)) => {
                for (index_val, level_val) in scenes.iter() {
                    let index = index_val
                        .as_u64()
                        .ok_or_else(|| boxed_err("Invalid scene index"))?;
                    if !(0..16).contains(&index) {
                        return Err("Scene index out of range".into());
                    }
                    let level = level_val
                        .as_u64()
                        .ok_or_else(|| boxed_err("Invalid level"))?;
                    if !(0..=255).contains(&level) {
                        return Err("Level out of range".into());
                    }
                    state.scenes[index as usize] = level.try_into().map_err(|e| boxed_err(e))?;
                }
            }
            Some(_) => return Err("'scenes' must be a sequence or a mapping".into()),
            None => {}
        }
        Ok(())
    }

    fn start(&mut self, bus_device: DaliSimBusDevice) -> DynResult<()> {
        self.state.write().unwrap().init_end_time = bus_device.current_time() + INIT_TIMEOUT;
        self.thread = Some(tokio::spawn(device_thread(bus_device, self.state.clone())));
        Ok(())
    }

    fn stop(&mut self) -> DynResult<()> {
        Ok(())
    }

    fn get_parameter(&self, name: &str) -> Result<String, ParameterError> {
        let state = self.state.read().unwrap();
        match name {
            "actualLevel" => {
                Ok(serde_json::to_string(&state.actual_level_at(Instant::now())).unwrap())
            }
            "targetLevel" => Ok(serde_json::to_string(&((state.target_level + 64) / 128)).unwrap()),
            "shortAddress" => Ok(serde_json::to_string(&(state.short_address + 1)).unwrap()),
            "fadeTime" => Ok(serde_json::to_string(&(state.fade >> 4)).unwrap()),
            "fadeRate" => Ok(serde_json::to_string(&(state.fade & 0x0f)).unwrap()),
            "extendedFadeTimeBase" => {
                Ok(serde_json::to_string(&(state.extended_fade_time & 0x0f)).unwrap())
            }
            "extendedFadeTimeMultiplier" => {
                Ok(serde_json::to_string(&((state.extended_fade_time >> 4) & 0x07)).unwrap())
            }
            "initialisationState" => Ok(match state.initialisation_state {
                InitialisationState::ENABLED => "ENABLED",
                InitialisationState::DISABLED => "DISABLED",
                InitialisationState::WITHDRAWN => "WITHDRAWN",
            }
            .to_string()),
            "writeEnableState" => Ok(match state.write_enable_state {
                WriteEnableState::ENABLED => "ENABLED",
                WriteEnableState::DISABLED => "DISABLED",
            }
            .to_string()),

            "controlGearFailure" => {
                Ok(serde_json::to_string(&((state.status & 0x01) != 0)).unwrap())
            }
            "lampFailure" => Ok(serde_json::to_string(&((state.status & 0x02) != 0)).unwrap()),
            "lampOn" => Ok(serde_json::to_string(&((state.status & 0x04) != 0)).unwrap()),
            "limitError" => Ok(serde_json::to_string(&((state.status & 0x08) != 0)).unwrap()),
            "fadeRunning" => Ok(serde_json::to_string(&((state.status & 0x10) != 0)).unwrap()),
            "resetState" => Ok(serde_json::to_string(&((state.status & 0x20) != 0)).unwrap()),
            "powerCycleSeen" => Ok(serde_json::to_string(&((state.status & 0x80) != 0)).unwrap()),

            n => match state.get_field(n) {
                Ok(value) => Ok(value),
                Err(FieldError::NotFound) => Err(ParameterError::NotFound),
                Err(FieldError::ConversionError(_)) => Err(ParameterError::InvalidValue),
            },
        }
    }

    fn set_parameter(&self, name: &str, value: &str) -> Result<(), ParameterError> {
        let mut state = self.state.write().unwrap();
        match name {
            "shortAddress" => match serde_json::from_str::<u8>(value) {
                Ok(value) => {
                    if !(1..64).contains(&value) && value != MASK {
                        return Err(ParameterError::InvalidValue);
                    }
                    state.short_address = value - 1;
                    Ok(())
                }
                Err(_) => Err(ParameterError::InvalidValue),
            },
            "fadeTime" => {
                set_bit_range(
                    &mut state.fade,
                    serde_json::from_str(value).map_err(|_| ParameterError::InvalidValue)?,
                    4..7,
                );
                Ok(())
            }
            "fadeRate" => {
                set_bit_range(
                    &mut state.fade,
                    serde_json::from_str(value).map_err(|_| ParameterError::InvalidValue)?,
                    0..4,
                );
                Ok(())
            }
            "extendedFadeTimeMultiplier" => {
                set_bit_range(
                    &mut state.extended_fade_time,
                    serde_json::from_str(value).map_err(|_| ParameterError::InvalidValue)?,
                    4..6,
                );
                Ok(())
            }

            "extendedFadeTimeBase" => {
                set_bit_range(
                    &mut state.extended_fade_time,
                    serde_json::from_str(value).map_err(|_| ParameterError::InvalidValue)?,
                    0..4,
                );
                Ok(())
            }
            "controlGearFailure" => {
                set_bit(
                    &mut state.status,
                    0,
                    serde_json::from_str::<bool>(value)
                        .map_err(|_| ParameterError::InvalidValue)?,
                );
                Ok(())
            }
            "lampFailure" => {
                set_bit(
                    &mut state.status,
                    1,
                    serde_json::from_str::<bool>(value)
                        .map_err(|_| ParameterError::InvalidValue)?,
                );
                Ok(())
            }
            "limitError" => {
                set_bit(
                    &mut state.status,
                    4,
                    serde_json::from_str::<bool>(value)
                        .map_err(|_| ParameterError::InvalidValue)?,
                );
                Ok(())
            }
            "resetState" => {
                set_bit(
                    &mut state.status,
                    5,
                    serde_json::from_str::<bool>(value)
                        .map_err(|_| ParameterError::InvalidValue)?,
                );
                Ok(())
            }
            "powerCycleSeen" => {
                set_bit(
                    &mut state.status,
                    7,
                    serde_json::from_str::<bool>(value)
                        .map_err(|_| ParameterError::InvalidValue)?,
                );
                Ok(())
            }
            n => match state.set_field(n, value.to_string()) {
                Ok(()) => Ok(()),
                Err(FieldError::NotFound) => Err(ParameterError::NotFound),
                Err(FieldError::ConversionError(_)) => Err(ParameterError::InvalidValue),
            },
        }
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
                        state.current_time = bus.current_time();
                        check_timers(&mut state);
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
#[cfg(test)]
fn check_fade(state: &mut GearState, target_level: u8, now: Instant, duration: Duration) {
    let start_level = state.actual_level_hires_at(now);
    start_fade(state, target_level, now, duration);
    // Allow a differece due to rounding
    if (state.actual_level_hires_at(now) - start_level).abs() > 1 {
        panic!("{} != {}", state.actual_level_hires_at(now), start_level);
    }
    // End of fade should always be exact
    assert_eq!(
        state.actual_level_hires_at(now + duration),
        i16::from(target_level) << LEVEL_HIRES_SHIFT
    );
}

#[test]
fn test_fade() {
    let mut now = Instant::now();
    let mut state = GearState::new("BAR01".to_string(), 0xffffff, now);
    state.target_level = 78 * LEVEL_HIRES_SCALE;
    check_fade(&mut state, 254, now, Duration::from_millis(100));

    now += Duration::from_millis(43);
    check_fade(&mut state, 0, now, Duration::from_secs(15 * 60));

    now += Duration::from_secs(15 * 60);
    check_fade(&mut state, 254, now, Duration::from_secs(15 * 60));

    now += Duration::from_secs(14 * 60);
    check_fade(&mut state, 238, now, Duration::from_secs(15 * 60));
}
