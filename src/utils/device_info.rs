use crate::common::address::{Long, Short};
use crate::common::defs::MASK;
use crate::control::cmd_defs::Command as Command24;
use crate::control::cmd_defs::{self as ccmd};
use crate::drivers::command_utils::send16;
use crate::drivers::driver::{DaliDriver, DaliSendResult};
use crate::drivers::driver_utils::DaliDriverExt;
use crate::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG};
use crate::gear::cmd_defs as cmd;
use crate::gear::cmd_defs::Command as Command16;
use crate::gear::device_type::DeviceType;
use crate::gear::status::GearStatus;
use std::fmt;

pub struct GearInfo {
    random_addr: Option<Long>,
    short_addr: Option<Short>,
    version: Option<u8>,
    device_types: Vec<DeviceType>,
    light_source_types: Vec<u8>,
    operating_mode: Option<u8>,
    status: Option<GearStatus>,
    groups: Option<u16>,
    scenes: Option<[u8; 16]>,
    physical_min: Option<u8>,
    actual_level: Option<u8>,
    min_level: Option<u8>,
    max_level: Option<u8>,

    power_on_level: Option<u8>,
    failure_level: Option<u8>,
    fade: Option<u8>,
    extended_fade_time: Option<u8>,
}

impl GearInfo {
    fn new() -> GearInfo {
        GearInfo {
            random_addr: None,
            short_addr: None,
            version: None,
            device_types: Vec::new(),
            light_source_types: Vec::new(),
            operating_mode: None,
            status: None,
            groups: None,
            scenes: None,
            physical_min: None,
            actual_level: None,
            min_level: None,
            max_level: None,

            power_on_level: None,
            failure_level: None,
            fade: None,
            extended_fade_time: None,
        }
    }
}
pub fn fmt_groups(f: &mut fmt::Formatter<'_>, groups: u16) -> fmt::Result {
    let mut str = Vec::new();
    let mut bit = 0;
    loop {
        while bit < 16 && ((groups & (1u16 << bit)) == 0) {
            bit += 1
        }
        if bit == 16 {
            break;
        }
        let start = bit;
        bit += 1;
        while bit < 16 && ((groups & (1u16 << bit)) != 0) {
            bit += 1
        }
        if bit == start + 1 {
            str.push(format!("{}", bit));
        } else {
            str.push(format!(" {}-{}", start + 1, bit));
        }
        if bit == 16 {
            break;
        }
    }
    f.write_str(&str.join(", "))
}

pub fn fmt_scenes(f: &mut fmt::Formatter<'_>, scenes: &[u8; 16]) -> fmt::Result {
    let mut str = Vec::new();
    for i in 0..16 {
        if scenes[i] != MASK {
            str.push(format!("{}: {}", i, scenes[i]));
        }
    }
    f.write_str(&str.join(", "))
}

impl fmt::Display for GearInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(long) = self.random_addr {
            writeln!(f, "Random address: {} (0x{:06x})", long, long)?
        }
        if let Some(short) = self.short_addr {
            writeln!(f, "Short address: {} (0x{:02x})", short, short.value())?
        }
        if self.device_types.len() > 0 {
            f.write_str("Device type:")?;
            for t in &self.device_types {
                write!(f, " {} (0x{})", t, t.value())?;
            }
            f.write_str("\n")?;
        }
        if let Some(s) = &self.status {
            writeln!(f, "Status: {} (0x{:02x})", s, s.value())?;
        }

        if let Some(groups) = self.groups {
            f.write_str("Groups: ")?;
            fmt_groups(f, groups)?;
            f.write_str("\n")?;
        }

        if let Some(scenes) = self.scenes {
            f.write_str("Scenes: ")?;
            fmt_scenes(f, &scenes)?;
            f.write_str("\n")?;
        }

        if let Some(v) = &self.version {
            writeln!(f, "Version: {}.{}", v >> 2, v & 3)?;
        }

        if let Some(v) = &self.physical_min {
            writeln!(f, "Physical minimum level: {}", v)?;
        }

        if let Some(v) = &self.actual_level {
            writeln!(f, "Actual level: {}", v)?;
        }
        if let Some(v) = &self.min_level {
            writeln!(f, "Minimum level: {}", v)?;
        }
        if let Some(v) = &self.max_level {
            writeln!(f, "Maximum level: {}", v)?;
        }
        if let Some(v) = &self.power_on_level {
            writeln!(f, "Power on level: {}", v)?;
        }
        if let Some(v) = &self.failure_level {
            writeln!(f, "System failure level: {}", v)?;
        }
        if let Some(v) = self.fade {
            let mut ext: String;
            let t = match v >> 4 {
                0 => {
                    if let Some(v) = self.extended_fade_time {
                        ext = ((v & 0x0f) + 1).to_string();
                        match (v >> 4) & 0x07 {
                            0 => ext = "0 s".to_string(),
                            1 => ext += "00 ms",
                            2 => ext += " s",
                            3 => ext += "0 s",
                            4 => ext += " min",
                            _ => ext = "Invalid".to_string(),
                        };
                    } else {
                        ext = "Invalid".to_string()
                    }
                    &ext
                }
                1 => "0.7 s",
                2 => "1.0 s",
                3 => "1.4 s",
                4 => "2.0 s",
                5 => "2.0 s",
                6 => "4.0 s",
                7 => "5.7 s",
                8 => "8.0 s",
                9 => "11.3 s",
                10 => "16 s",
                11 => "22.6 s",
                12 => "32 s",
                13 => "45.3 s",
                14 => "64 s",
                15 => "90.5 s",
                _ => "",
            };

            let r = match v & 0x0f {
                0 => "358",
                1 => "358",
                2 => "253",
                3 => "179",
                4 => "127",
                5 => "89.4",
                6 => "63.3",
                7 => "44.7",
                8 => "31.6",
                9 => "22.4",
                10 => "15.8",
                11 => "11.2",
                12 => "7.9",
                13 => "5.6",
                14 => "4.0",
                15 => "2.8",
                _ => "2.8",
            };

            writeln!(f, "Fade time: {}", t)?;
            writeln!(f, "Fade rate: {} steps/s", r)?;
        }

        Ok(())
    }
}

async fn send_query(
    d: &mut dyn DaliDriver,
    cmd: Command16<true, false>,
) -> Result<Option<u8>, DaliSendResult> {
    match send16::query(d, cmd, NO_FLAG).await {
        DaliSendResult::Answer(s) => Ok(Some(s)),
        DaliSendResult::Timeout => Ok(None),
        e => return Err(e),
    }
}

pub async fn read_gear_info(
    d: &mut dyn DaliDriver,
    addr: Short,
) -> Result<GearInfo, DaliSendResult> {
    let mut info: GearInfo = GearInfo::new();
    info.short_addr = Some(addr);
    info.status = match send16::query(d, cmd::QUERY_STATUS(addr), NO_FLAG).await {
        DaliSendResult::Answer(s) => Some(GearStatus::new(s)),
        DaliSendResult::Timeout => None,
        e => return Err(e),
    };
    match send16::query(d, cmd::QUERY_DEVICE_TYPE(addr), NO_FLAG).await {
        DaliSendResult::Answer(MASK) => loop {
            match send16::query(d, cmd::QUERY_NEXT_DEVICE_TYPE(addr), NO_FLAG).await {
                DaliSendResult::Answer(MASK) => break,
                DaliSendResult::Answer(t) => info.device_types.push(DeviceType::new(t)),
                DaliSendResult::Timeout => break,
                e => return Err(e),
            };
        },
        DaliSendResult::Answer(t) => info.device_types.push(DeviceType::new(t)),
        DaliSendResult::Timeout => {}
        e => return Err(e),
    };

    info.groups = match (
        send16::query(d, cmd::QUERY_GROUPS_0_7(addr), NO_FLAG).await,
        send16::query(d, cmd::QUERY_GROUPS_8_15(addr), NO_FLAG).await,
    ) {
        (DaliSendResult::Answer(l), DaliSendResult::Answer(h)) => {
            Some(((h as u16) << 8) | (l as u16))
        }
        (DaliSendResult::Timeout, _) => None,
        (_, DaliSendResult::Timeout) => None,
        (e, DaliSendResult::Answer(_)) => return Err(e),
        (_, e) => return Err(e),
    };

    let mut scenes = [MASK; 16];
    let mut scene_count = 0;
    for i in 0..16 {
        scenes[i] = match send16::query(d, cmd::QUERY_SCENE_LEVEL(addr, i as u8), NO_FLAG).await {
            DaliSendResult::Answer(s) => {
                scene_count += 1;
                s
            }
            DaliSendResult::Timeout => MASK,
            e => return Err(e),
        };
    }
    if scene_count > 0 {
        info.scenes = Some(scenes);
    }
    info.physical_min = send_query(d, cmd::QUERY_PHYSICAL_MINIMUM(addr)).await?;
    info.actual_level = send_query(d, cmd::QUERY_ACTUAL_LEVEL(addr)).await?;
    info.min_level = send_query(d, cmd::QUERY_MIN_LEVEL(addr)).await?;
    info.max_level = send_query(d, cmd::QUERY_MAX_LEVEL(addr)).await?;
    info.failure_level = send_query(d, cmd::QUERY_SYSTEM_FAILURE_LEVEL(addr)).await?;
    info.power_on_level = send_query(d, cmd::QUERY_POWER_ON_LEVEL(addr)).await?;
    info.operating_mode = send_query(d, cmd::QUERY_OPERATING_MODE(addr)).await?;
    info.version = send_query(d, cmd::QUERY_VERSION_NUMBER(addr)).await?;
    info.fade = send_query(d, cmd::QUERY_FADE(addr)).await?;
    info.extended_fade_time = send_query(d, cmd::QUERY_EXTENDED_FADE_TIME(addr)).await?;

    match send16::query(d, cmd::QUERY_LIGHT_SOURCE_TYPE(addr), NO_FLAG).await {
        DaliSendResult::Answer(s) => info.light_source_types.push(s),
        DaliSendResult::Timeout => {}
        e => return Err(e),
    };

    Ok(info)
}
pub struct Instance {
    pub instance_type: u8,
    pub resolution: u8,
    pub error: u8,
    pub status: u8,
    pub event_priority: u8,
    pub instance_groups: [u8; 3], // Primary, 1, 2
    pub event_scheme: u8,
    pub input_value: u32,
    pub feature_types: Vec<u8>,
    pub event_filter: u32,
}
pub struct ControlInfo {
    pub random_addr: Long,
    pub short_addr: Short,
    pub version: u8,
    pub device_status: u8,
    pub controller_error: u8,
    pub device_error: u8,
    pub operation_mode: u8,
    pub manufacturer_specific_mode: u8,
    pub device_groups: u32,
    pub device_capabilities: u32,
    pub instances: Vec<Instance>,
}

impl ControlInfo {
    fn new() -> ControlInfo {
        ControlInfo {
            random_addr: 0,
            short_addr: Short::new(1),
            version: 0,
            device_status: 0,
            controller_error: 0,
            device_error: 0,
            operation_mode: 0,
            manufacturer_specific_mode: 0,
            device_groups: 0,
            device_capabilities: 0,
            instances: Vec::new(),
        }
    }
}
impl fmt::Display for ControlInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Random address: {0} (0x{0:06x})", self.random_addr)?;
        writeln!(
            f,
            "Short address: {} (0x{:02x})",
            self.short_addr,
            self.short_addr.value()
        )?;

        writeln!(f, "Version: {}.{}", self.version >> 2, self.version & 3)?;

        Ok(())
    }
}

async fn send_query24(
    d: &mut dyn DaliDriver,
    cmd: Command24<true, false>,
) -> Result<Option<u8>, DaliSendResult> {
    match d.send_frame24(&cmd.0, EXPECT_ANSWER).await {
        DaliSendResult::Answer(s) => Ok(Some(s)),
        DaliSendResult::Timeout => Ok(None),
        e => return Err(e),
    }
}
pub async fn read_control_info(
    d: &mut dyn DaliDriver,
    addr: Short,
) -> Result<ControlInfo, DaliSendResult> {
    let mut info: ControlInfo = ControlInfo::new();
    info.short_addr = addr;
    info.version = send_query24(d, ccmd::QUERY_VERSION_NUMBER(addr))
        .await?
        .unwrap_or(0);
    Ok(info)
}
