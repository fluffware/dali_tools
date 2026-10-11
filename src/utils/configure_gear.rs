use crate::common::address::{Long, Short};
use crate::utils::device_configuration as conf;
use crate::utils::device_configuration::Found as _;
use std::ops::RangeBounds;
use std::str::FromStr;

pub trait ConfigureGear {
    fn conf_random_address(&mut self, _addr: Long) {}
    fn conf_short_address(&mut self, _addr_or_mask: Option<Short>) {} // None if MASK
    fn conf_last_light_level(&mut self, _level: u8) {}
    fn conf_target_level(&mut self, _level: u8) {}
    fn conf_power_on_level(&mut self, _level: u8) {}
    fn conf_system_failure_level(&mut self, _level: u8) {}
    fn conf_min_level(&mut self, _level: u8) {}
    fn conf_max_level(&mut self, _level: u8) {}
    fn conf_fade_time(&mut self, _time: u8) {}
    fn conf_fade_rate(&mut self, _rate: u8) {}
    fn conf_extended_fade_time_base(&mut self, _base: u8) {}
    fn conf_extended_fade_time_multiplier(&mut self, _multiplier: u8) {}
    fn conf_gear_groups(&mut self, _groups: u16) {}
    fn conf_scenes(&mut self, _map: &[(u8, u8)]) {} // (scene number (0 based), scene level)
    fn conf_memory_bank(&mut self, _bank: u8, _data: &[u8]) {}
    fn conf_device_types(&mut self, _types: &[u8]) {}
}

fn path_err<E, P, R>(error: E, path: P) -> conf::Result<R>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
    P: Into<String>,
{
    Err(conf::Error::from_error(error.into(), path.into()))
}

fn path_error<E, P>(error: E, path: P) -> conf::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
    P: Into<String>,
{
    conf::Error::from_error(error.into(), path.into())
}

fn configure_variable_uint<R, S>(
    conf: &dyn conf::Map,
    name: &str,
    set: S,
    range: R,
    offset: i64,
) -> conf::Result<()>
where
    S: FnOnce(i64),
    R: RangeBounds<i64>,
{
    if let Ok(value) = conf.get(name) {
        let conf_value: i64 = value.as_i64()?;
        if !range.contains(&conf_value) {
            return Err(conf::Error::from_error(
                format!("Value for '{}' is out of range", name).into(),
                value.path(),
            ));
        }
        set(conf_value - offset)
    }
    Ok(())
}

pub fn parse_gear(
    device: &mut dyn ConfigureGear,
    conf: &dyn conf::Map,
    index: usize,
) -> conf::Result<()>
where
{
    configure_variable_uint(
        conf,
        "randomAddress",
        |a| device.conf_random_address(a as u32),
        0..0x1000000,
        0,
    )?;

    let mut addr_step = 1i64;
    configure_variable_uint(conf, "shortAddressStep", |s| addr_step = s, 1..64, 0)?;

    if let Some(addr_value) = conf.get("shortAddress").found()? {
        let addr = if let Ok(addr) = addr_value.as_i64() {
            if !(1..=64).contains(&addr) {
                return path_err(
                    format!("Value for 'shortAddress' is out of range"),
                    addr_value.path(),
                );
            }

            Some(Short::new((addr + addr_step * index as i64 - 1) as u8))
        } else if let Ok(addr) = addr_value.as_string() {
            if addr.eq_ignore_ascii_case("mask") {
                None
            } else {
                return Err(conf::Error::from_error(
                    "Value for 'shortAddress' must be an integer or MASK".into(),
                    addr_value.path(),
                ));
            }
        } else {
            return Err(conf::Error::from_error(
                "Value for 'shortAddress' must be an integer or MASK".into(),
                addr_value.path(),
            ));
        };
        device.conf_short_address(addr);
    }

    configure_variable_uint(
        conf,
        "lastLightLevel",
        |level| device.conf_last_light_level(level as u8),
        0..=255,
        0,
    )?;
    configure_variable_uint(
        conf,
        "targetLevel",
        |level| device.conf_target_level(level as u8),
        0..255,
        0,
    )?;

    configure_variable_uint(
        conf,
        "powerOnLevel",
        |level| device.conf_power_on_level(level as u8),
        0..255,
        0,
    )?;
    configure_variable_uint(
        conf,
        "systemFailureLevel",
        |level| device.conf_system_failure_level(level as u8),
        0..=255,
        0,
    )?;
    configure_variable_uint(
        conf,
        "minLevel",
        |level| device.conf_min_level(level as u8),
        0..=255,
        0,
    )?;
    configure_variable_uint(
        conf,
        "maxLevel",
        |level| device.conf_max_level(level as u8),
        0..=255,
        0,
    )?;
    configure_variable_uint(
        conf,
        "fadeRate",
        |rate| device.conf_fade_rate(rate as u8),
        0..=15,
        0,
    )?;

    configure_variable_uint(
        conf,
        "fadeTime",
        |time| device.conf_fade_time(time as u8),
        0..=15,
        0,
    )?;

    configure_variable_uint(
        conf,
        "extendedFadeTimeBase",
        |time| device.conf_extended_fade_time_base(time as u8),
        0..=15,
        0,
    )?;
    configure_variable_uint(
        conf,
        "extendedFadeTimeMultiplier",
        |time| device.conf_extended_fade_time_multiplier(time as u8),
        0..=15,
        0,
    )?;

    if let Some(dt_value) = conf.get("deviceTypes").found()? {
        let mut device_types = Vec::new();
        dt_value.seq_iterate(&mut |_, dt_value: &conf::Value<'_>| {
            if let Ok(dt) = dt_value.as_i64()
                && (1..=255).contains(&dt)
            {
                device_types.push(dt as u8);
            } else {
                return Err(conf::Error::from_error(
                    "Invalid type number".into(),
                    dt_value.path(),
                ));
            }
            Ok(())
        })?;
        device.conf_device_types(&device_types);
    }
    if let Some(group_value) = conf.get("gearGroups").found()? {
        if let Ok(group_bits) = group_value.as_i64() {
            let gear_groups = u16::try_from(group_bits).map_err(|e| {
                conf::Error::from_error(
                    format!("Invalid group bitmask: {}", e).into(),
                    group_value.path(),
                )
            })?;
            device.conf_gear_groups(gear_groups);
        } else if let Ok(group_seq) = group_value.clone().into_seq() {
            let mut gear_groups = 0;
            group_seq.iterate(&mut |_, g: &conf::Value<'_>| {
                if let Ok(bit) = g.as_i64()
                    && (1..=16).contains(&bit)
                {
                    gear_groups |= 1 << (bit - 1);
                } else {
                    return path_err("Invalid group number", g.path());
                }
                Ok(())
            })?;
            device.conf_gear_groups(gear_groups);
        } else {
            return path_err(
                "'gearGroups' must be a sequence or a number",
                group_value.path(),
            );
        }
    }

    if let Ok(scene_value) = conf.get("scenes") {
        let mut map = Vec::new();
        if let Ok(scenes) = scene_value.clone().into_seq() {
            scenes.iterate(&mut |index, level_val: &conf::Value<'_>| {
                let level = level_val
                    .as_i64()
                    .map_err(|_| path_error("Invalid level", scene_value.path()))?;
                if !(0..=255).contains(&level) {
                    return path_err("Level out of range", level_val.path());
                }
                map.push((index as u8, level as u8));
                Ok(())
            })?;

            if map.len() > 16 {
                return path_err("Too many scenes", scenes.path());
            }
        } else if let Ok(scenes) = scene_value.clone().into_map() {
            scenes.iterate(&mut |key, level_value: &conf::Value<'_>| {
                let index = u64::from_str(key)
                    .map_err(|_| path_error("Invalid scene index", scene_value.path()))?;
                if !(0..16).contains(&index) {
                    return path_err("Scene index out of range", scene_value.path());
                }
                let level = level_value
                    .as_i64()
                    .map_err(|_| path_error("Invalid level", level_value.path()))?;
                if !(0..=255).contains(&level) {
                    return path_err("Level out of range", level_value.path());
                }
                map.push((index as u8, level as u8));
                Ok(())
            })?;
        } else {
            return path_err(
                "'scenes' must be a sequence or a mapping",
                scene_value.path(),
            );
        }
        device.conf_scenes(&map);
    }
    /*
        if let Some(memory_banks) = conf.get("memoryBanks") {
            let Some(banks) = memory_banks.as_mapping() else {
                return Err("'memoryBanks' must be a mapping".into());
            };
            for (bank_value, data_value) in banks {
                let Some(bank) = bank_value.as_i64() else {
                    return Err("'memoryBanks' keys must be numbers".into());
                };
                if bank < 0 || bank > 255 {
                    return Err("Memory bank number out of range".into());
                };
                let bank = bank as u8;
                let Some(data) = data_value.as_sequence() else {
                    return Err("Value for each bank must me a sequence of bytes".into());
                };
                let mut bytes = Vec::new();
                for byte_value in data {
                    let byte = if let Some(byte) = byte_value.as_i64()
                        && byte >= 0
                        && byte <= 255
                    {
                        byte as u8
                    } else {
                        return Err("Illegal byte value for memory bank".into());
                    };
                    bytes.push(byte);
                }
                device.conf_memory_bank(bank, &bytes);
            }
        }
    */
    Ok(())
}

#[cfg(test)]

mod test {
    use super::ConfigureGear;
    use crate::common::address::Long;
    use crate::gear::address::Short;
    use crate::utils::device_configuration as conf;
    use crate::utils::device_configuration::ConfigureDevice;
    use crate::utils::error::DynResult;
    use crate::utils::parse_config as parse;
    use parse::CreateDevice;

    struct Gear {
        name: String,
        gear_type: String,
        random_address: Long,
        short_address: Option<Short>,
        last_light_level: u8,
        target_level: u8,
        power_on_level: u8,
        system_failure_level: u8,
        min_level: u8,
        max_level: u8,
        fade_time: u8,
        fade_rate: u8,
        extended_fade_time_base: u8,
        extended_fade_time_multiplier: u8,
        gear_groups: u16,
        scenes: Vec<(u8, u8)>,
    }

    impl Default for Gear {
        fn default() -> Gear {
            Gear {
                name: String::new(),
                gear_type: String::new(),
                random_address: 0,
                short_address: None,
                last_light_level: 0u8,
                target_level: 0u8,
                power_on_level: 0u8,
                system_failure_level: 0u8,
                min_level: 1u8,
                max_level: 254u8,
                fade_time: 1u8,
                fade_rate: 4u8,
                extended_fade_time_base: 1u8,
                extended_fade_time_multiplier: 1u8,
                gear_groups: 0,
                scenes: Vec::new(),
            }
        }
    }
    impl Gear {
        fn new(name: &str, gear_type: &str) -> Self {
            let mut gear = Self::default();
            gear.name = name.to_string();
            gear.gear_type = gear_type.to_string();
            gear
        }
    }
    impl ConfigureGear for Gear {
        fn conf_random_address(&mut self, addr: Long) {
            self.random_address = addr;
        }
        fn conf_short_address(&mut self, addr_or_mask: Option<Short>) {
            self.short_address = addr_or_mask
        }
        fn conf_last_light_level(&mut self, level: u8) {
            self.last_light_level = level
        }
        fn conf_target_level(&mut self, level: u8) {
            self.target_level = level
        }
        fn conf_power_on_level(&mut self, level: u8) {
            self.power_on_level = level
        }
        fn conf_system_failure_level(&mut self, level: u8) {
            self.system_failure_level = level
        }
        fn conf_min_level(&mut self, level: u8) {
            self.min_level = level
        }
        fn conf_max_level(&mut self, level: u8) {
            self.max_level = level
        }
        fn conf_fade_time(&mut self, time: u8) {
            self.fade_time = time
        }
        fn conf_fade_rate(&mut self, rate: u8) {
            self.fade_rate = rate
        }
        fn conf_extended_fade_time_base(&mut self, base: u8) {
            self.extended_fade_time_base = base
        }
        fn conf_extended_fade_time_multiplier(&mut self, multiplier: u8) {
            self.extended_fade_time_multiplier = multiplier
        }
        fn conf_gear_groups(&mut self, groups: u16) {
            self.gear_groups = groups
        }
        fn conf_scenes(&mut self, map: &[(u8, u8)]) {
            self.scenes = Vec::from(map)
        } // (scene number (0 based), scene level)
    }

    impl ConfigureDevice for Gear {
        fn configure(&mut self, configuration: &dyn conf::Map, index: usize) -> conf::Result<()> {
            super::parse_gear(self, configuration, index)
        }
    }

    struct GearFactory {
        pub gears: Vec<Gear>,
    }

    impl Default for GearFactory {
        fn default() -> Self {
            Self { gears: Vec::new() }
        }
    }
    impl CreateDevice for GearFactory {
        fn new_device(
            &mut self,
            name: &str,
            gear_type: &str,
        ) -> DynResult<&mut (dyn ConfigureDevice + Send + Sync)> {
            let gear = Gear::new(name, gear_type);

            Ok(self.gears.push_mut(gear))
        }
    }

    #[test]
    fn test_parse_config() -> DynResult<()> {
        let mut factory = GearFactory::default();
        parse::parse_config(
            br"
dali:
  gears:
    BAR01:
      nameStep: 2
      type: generic_gear
      count: 8
      shortAddressStep: 3
      shortAddress: 1
      targetLevel: 0
      fadeTime: 1
      fadeRate: 3
      gearGroups: 0xf0
      scenes:
        2: 78
        4: 255
"
            .as_ref(),
            &mut factory,
        )?;
        assert_eq!(factory.gears.len(), 8);
        assert_eq!(&factory.gears[5].gear_type, "generic_gear");
        assert_eq!(&factory.gears[2].name, "BAR05");
        assert_eq!(&factory.gears[3].short_address, &Some(Short::new(9)));
        assert_eq!(&factory.gears[7].short_address, &Some(Short::new(21)));
        assert_eq!(factory.gears[7].gear_groups, 0xf0);
        assert_eq!(factory.gears[7].scenes, &[(2, 78), (4, 255)]);
        Ok(())
    }

    #[test]
    fn test_template() -> DynResult<()> {
        let mut factory = GearFactory::default();
        parse::parse_config(
            br"
dali:
  templates:
    tw:
      gearGroups: [2,3]

  gears:
    BAR01:
      nameStep: 2
      type: generic_gear
      count: 8
      shortAddressStep: 3
      shortAddress: 1
      targetLevel: 0
      fadeTime: 1
      fadeRate: 3
      template: tw
"
            .as_ref(),
            &mut factory,
        )?;

        assert_eq!(factory.gears.len(), 8);
        assert_eq!(&factory.gears[5].gear_type, "generic_gear");
        assert_eq!(&factory.gears[2].name, "BAR05");
        assert_eq!(&factory.gears[3].short_address, &Some(Short::new(9)));
        assert_eq!(factory.gears[7].gear_groups, 0b110);
        Ok(())
    }

    use std::fs::File;
}
