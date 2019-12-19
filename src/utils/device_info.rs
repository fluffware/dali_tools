use crate::base::status::GearStatus;
use crate::base::address::{Short,Long,BusAddress};
use crate::base::device_type::DeviceType;
use crate::defs::gear::cmd;
use crate::defs::common::MASK;
use crate::drivers::driver::{self, DALIdriver, DALIcommandError};
use std::fmt;
use futures::future;

pub struct DeviceInfo
{
    random_addr: Option<Long>,
    short_addr: Option<Short>,
    version: Option<u8>,
    device_types: Vec<DeviceType>,
    light_source_types: Vec<u8>,
    operating_mode: Option<u8>,
    status: Option<GearStatus>,
    groups: Option<u16>,
    scenes: Option<[u8;16]>,
    physical_min: Option<u8>,
    actual_level: Option<u8>,
    min_level: Option<u8>,
    max_level: Option<u8>,
    
    powere_on_level: Option<u8>,
    failure_level: Option<u8>,
    fade: Option<u8>,
    extended_fade_time: Option<u8>,
}

impl DeviceInfo {
    fn new() -> DeviceInfo 
    {
        DeviceInfo {
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
            
            powere_on_level: None,
            failure_level: None,
            fade: None,
            extended_fade_time: None,
            
        }
    }
}
pub fn fmt_groups(f: &mut fmt::Formatter<'_>, groups: u16) -> fmt::Result
{
    let mut str = Vec::new();
    let mut bit = 0;
    loop {
        while bit < 16 && ((groups & (1u16 << bit)) == 0) {bit += 1}
        if bit == 16 {break}
        let start = bit;
        bit += 1;
        while bit < 16 && ((groups & (1u16 << bit)) != 0) {bit += 1}
        if bit == start + 1 {
            str.push(format!("{}", bit));
        } else {
            str.push(format!(" {}-{}", start+1, bit));
        }
        if bit == 16 {break}                
    }
    f.write_str(&str.join(", "))
}

pub fn fmt_scenes(f: &mut fmt::Formatter<'_>, scenes: &[u8;16]) -> fmt::Result
{
    let mut str = Vec::new();
    for i in 0..16 {
        if scenes[i] != MASK {
            str.push(format!("{}: {}", i, scenes[i]));
        }
    }
    f.write_str(&str.join(", "))
}

impl fmt::Display for DeviceInfo
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result 
    {
        if let Some(long) = self.random_addr {
            writeln!(f, "Random address: {} (0x{:06x})", long,long)?
        }
        if let Some(short) = self.short_addr {
            writeln!(f, "Short address: {} (0x{:02x})",
                     short,short.bus_address())?
        }
        if self.device_types.len() > 0 {
            f.write_str("Device type:")?;
            for t in &self.device_types {
                write!(f," {} (0x{})", t, t.value())?;
            }
            f.write_str("\n")?;
        }
        if let Some(s) = &self.status {
            writeln!(f, "Status: {} (0x{:02x})", s,s.value())?;
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
            writeln!(f, "Version: {}.{}", v>>2, v&3)?;
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
        if let Some(v) = &self.powere_on_level {
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
                        ext = ((v & 0x0f ) + 1).to_string();
                        match (v >> 4) & 0x07 {
                            0 => ext = "0 s".to_string(),
                            1 => ext += "00 ms",
                            2 => ext += " s",
                            3 => ext += "0 s",
                            4 => ext += " min",
                            _ => ext = "Invalid".to_string()
                        };
                    } else {
                        ext = "Invalid".to_string()
                    }
                    &ext
                },
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
                _ => ""
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
                _ => "2.8"
            };
                
            writeln!(f, "Fade time: {}", t)?;
            writeln!(f, "Fade rate: {} steps/s", r)?;
        }

        Ok(())
    }

}

async fn send_query(d: &mut dyn DALIdriver, addr: Short, cmd: u8)
              -> Result<Option<u8>, DALIcommandError>
{
    match d.send_device_cmd(&addr, cmd, 
                            driver::EXPECT_ANSWER).await {
        Ok(s) => Ok(Some(s)),
        Err(DALIcommandError::Timeout) => Ok(None),
        Err(e) => return Err(e)
    }
}

pub async fn read_device_info(d: &mut dyn DALIdriver, addr: Short)
                          -> Result<DeviceInfo, DALIcommandError>
{
    let mut info: DeviceInfo = DeviceInfo::new();
    info.short_addr = Some(addr);
    info.status = match d.send_device_cmd(&addr, cmd::QUERY_STATUS, 
                                          driver::EXPECT_ANSWER).await {
        Ok(s) => Some(GearStatus::new(s)),
        Err(DALIcommandError::Timeout) => None,
        Err(e) => return Err(e)
    };
    match d.send_device_cmd(&addr, cmd::QUERY_DEVICE_TYPE, 
                            driver::EXPECT_ANSWER).await {
        Ok(t) if t == MASK => {
            loop {
                match d.send_device_cmd(&addr,
                                        cmd::QUERY_NEXT_DEVICE_TYPE, 
                                        driver::EXPECT_ANSWER).await {
                    Ok(MASK) => break,
                    Ok(t) => info.device_types.push(DeviceType::new(t)),
                    Err(DALIcommandError::Timeout) => break,
                    Err(e) => return Err(e)
                };
            }
        },
        Ok(t) => info.device_types.push(DeviceType::new(t)),
        Err(DALIcommandError::Timeout) => {},
        Err(e) => return Err(e)
    };

    info.groups =
        match future::join(d.send_device_cmd(&addr, cmd::QUERY_GROUPS_0_7, 
                                             driver::EXPECT_ANSWER),
                           d.send_device_cmd(&addr, cmd::QUERY_GROUPS_8_15, 
                                             driver::EXPECT_ANSWER)).await {
            (Ok(l), Ok(h)) => Some(((h as u16) << 8) | (l as u16)),
            (Err(DALIcommandError::Timeout), _) => None,
            (_, Err(DALIcommandError::Timeout)) => None,
            (Err(e),_) | (_, Err(e)) => return Err(e)
        };

    let mut scenes = [MASK;16];
    let mut scene_count = 0;
    for i in 0..16 {
        scenes[i] = match d.send_device_cmd(&addr, 
                                            cmd::QUERY_SCENE_LEVEL_0+(i as u8), 
                                            driver::EXPECT_ANSWER).await {
            Ok(s) => {scene_count += 1; s},
            Err(DALIcommandError::Timeout) => MASK,
            Err(e) => return Err(e)
        };
    }
    if scene_count > 0 {
        info.scenes = Some(scenes);
    }
    info.physical_min = send_query(d, addr,
                                   cmd::QUERY_PHYSICAL_MINIMUM).await?;
    info.actual_level = send_query(d, addr,
                                   cmd::QUERY_ACTUAL_LEVEL).await?;
    info.min_level = send_query(d, addr,
                                   cmd::QUERY_MIN_LEVEL).await?;
    info.max_level = send_query(d, addr,
                                cmd::QUERY_MAX_LEVEL).await?;
    info.failure_level = send_query(d, addr,
                                    cmd::QUERY_SYSTEM_FAILURE_LEVEL).await?;
    info.powere_on_level = send_query(d, addr,
                                      cmd::QUERY_POWER_ON_LEVEL).await?;
    info.operating_mode = send_query(d, addr,
                                     cmd::QUERY_OPERATING_MODE).await?;
    info.version = send_query(d, addr,
                              cmd::QUERY_VERSION_NUMBER).await?;
    info.fade = send_query(d, addr,
                           cmd::QUERY_FADE).await?;
    info.extended_fade_time = send_query(d, addr,
                                         cmd::QUERY_EXTENDED_FADE_TIME).await?;

    match d.send_device_cmd(&addr, cmd::QUERY_LIGHT_SOURCE_TYPE, 
                            driver::EXPECT_ANSWER).await {
        Ok(s) => info.light_source_types.push(s),
        Err(DALIcommandError::Timeout) => {},
        Err(e) => return Err(e)
    };
    
    Ok(info)
}
        
