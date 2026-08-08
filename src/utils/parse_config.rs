use crate::common::address::{Long, Short};
use std::ops::RangeBounds;
use std::str::FromStr;
use yaml_serde::Value;

pub type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn boxed_err<E>(e: E) -> Box<dyn std::error::Error + Send + Sync>
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    Into::<Box<dyn std::error::Error + Send + Sync>>::into(e)
}

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
}

fn configure_variable_uint<R, S>(
    conf: &yaml_serde::value::Mapping,
    name: &str,
    set: S,
    range: R,
    offset: u64,
) -> DynResult<()>
where
    S: FnOnce(u64),
    R: RangeBounds<u64>,
{
    if let Some(value) = conf.get(name) {
        let conf_value: u64 = value.as_u64().ok_or_else(|| {
            boxed_err(format!("Value for '{}' is not an unsigned integer", name).as_str())
        })?;
        if !range.contains(&conf_value) {
            return Err(boxed_err(format!("Value for '{}' is out of range", name)));
        }
        set(conf_value - offset)
    }
    Ok(())
}

fn parse_gear(
    device: &mut dyn ConfigureGear,
    conf: &yaml_serde::value::Mapping,
    index: usize,
) -> DynResult<()>
where
{
    configure_variable_uint(
        conf,
        "randomAddress",
        |a| device.conf_random_address(a as u32),
        0..=0xffffff,
        0,
    )?;

    let mut addr_step = 1u64;
    configure_variable_uint(conf, "shortAddressStep", |s| addr_step = s, 1..=63, 0)?;

    if let Some(addr_value) = conf.get("shortAddress") {
        let addr = match addr_value {
            Value::String(s) if s.eq_ignore_ascii_case("mask") => None,
            Value::Number(a) if a.is_u64() => {
                let addr = a.as_u64().unwrap();
                if !(1..64).contains(&addr) {
                    return Err(boxed_err(format!(
                        "Value for 'shortAddress' is out of range"
                    )));
                }
                Some(Short::new((addr + addr_step * index as u64 - 1) as u8))
            }
            _ => {
                return Err(boxed_err(format!(
                    "Value for 'shortAddress' must be an integer or MASK"
                )));
            }
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

    match conf.get("gearGroups") {
        Some(yaml_serde::Value::Sequence(groups)) => {
            let mut gear_groups = 0;
            for group in groups {
                let bit = group
                    .as_u64()
                    .ok_or_else(|| boxed_err("Invalid group number"))?;
                if !(1..=16).contains(&bit) {
                    return Err("Invalid group number".into());
                }
                gear_groups |= 1 << (bit - 1);
            }
            device.conf_gear_groups(gear_groups);
        }
        Some(yaml_serde::Value::Number(groups)) if let Some(g) = groups.as_u64() => {
            let gear_groups =
                u16::try_from(g).map_err(|e| format!("Illegal group bitmask: {}", e))?;
            device.conf_gear_groups(gear_groups);
        }
        Some(_) => return Err("'gearGroups' must be a sequence or a number".into()),
        None => {}
    }
    if let Some(scene_value) = conf.get("scenes") {
        let mut map = Vec::new();
        match scene_value {
            yaml_serde::Value::Sequence(scenes) => {
                if scenes.len() > 16 {
                    return Err("too many scenes".into());
                }
                let mut map = Vec::new();
                for (index, level_val) in scenes.iter().enumerate() {
                    let level = level_val
                        .as_u64()
                        .ok_or_else(|| boxed_err("Invalid level"))?;
                    if !(0..=255).contains(&level) {
                        return Err("Level out of range".into());
                    }
                    map.push((index as u8, level as u8));
                }
            }
            yaml_serde::Value::Mapping(scenes) => {
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
                    map.push((index as u8, level as u8));
                }
            }
            _ => return Err("'scenes' must be a sequence or a mapping".into()),
        }
        device.conf_scenes(&map);
    }
    Ok(())
}

// Create a new label base on old one. If the old label contains a numeric part then it's updated
fn label_offset(label: &str, offset: usize) -> String {
    if offset == 0 {
        return label.to_string();
    }
    if let Some((mut digit_end, _)) = label.rmatch_indices(|c: char| c.is_digit(10)).next() {
        let digit_start = label[0..digit_end]
            .rmatch_indices(|c: char| !c.is_digit(10))
            .next()
            .map(|(p, _)| p + 1) // Move position to first digit
            .unwrap_or(0);
        digit_end += 1;
        // Shouldn't fail since we already checked that it's all digits
        let n = usize::from_str(&label[digit_start..digit_end]).unwrap();
        format!(
            "{0}{1:02$}{3}",
            &label[..digit_start],
            n + offset,
            digit_end - digit_start,
            &label[digit_end..]
        )
    } else {
        format!("{}_{}", label, offset)
    }
}
pub trait CreateGear {
    fn new_gear(&mut self, name: &str, gear_type: &str) -> DynResult<&mut dyn ConfigureGear>;
}

pub fn parse_config<R, C>(conf_file: R, create: &mut C) -> DynResult<()>
where
    R: std::io::Read,
    C: CreateGear,
{
    let conf: yaml_serde::Mapping = yaml_serde::from_reader(conf_file)?;
    let Some(dali_conf) = conf.get("dali") else {
        return Err("No 'dali' tag found in configuration file".into());
    };
    let templates = {
        if let Some(t) = dali_conf.get("template") {
            if let Some(m) = t.as_mapping() {
                Some(m)
            } else {
                return Err("'template' is not a mapping".into());
            }
        } else {
            None
        }
    };

    let Some(device_conf) = dali_conf.get("gears") else {
        return Err("No 'gears' tag found in configuration file".into());
    };
    let yaml_serde::Value::Mapping(device_list) = device_conf else {
        return Err("'gears' is not a mapping".into());
    };
    for (name, device) in device_list {
        let yaml_serde::Value::Mapping(conf) = device else {
            return Err("Item in gear list is not a mapping".into());
        };
        let mut conf = conf.clone();
        if let Some(template_name) = conf.get("template").and_then(|v| v.as_str()) {
            if let Some(templates) = templates {
                let Some(replacements) = templates.get(template_name) else {
                    return Err(format!("No template '{}' found", template_name).into());
                };
                if let Some(replacements) = replacements.as_mapping() {
                    for (name, value) in replacements {
                        if !conf.contains_key(name) {
                            conf.insert(name.clone(), value.clone());
                        }
                    }
                } else {
                    return Err(format!("Template '{}' is not a mapping", template_name).into());
                }
            }
        }
        if let Some(device_type) = conf.get("type").and_then(|v| v.as_str()) {
            log::debug!("Type: {}", device_type);
            let count = conf.get("count").and_then(|v| v.as_i64()).unwrap_or(1) as usize;
            for index in 0..count {
                let Some(base_name) = name.as_str() else {
                    return Err("Gear name is not a string".into());
                };

                let mut name_step = 1u64;
                configure_variable_uint(&conf, "nameStep", |s| name_step = s, 1.., 0)?;

                let dev_name = label_offset(base_name, index * name_step as usize);
                let gear_conf = create.new_gear(&dev_name, device_type)?;
                parse_gear(gear_conf, &conf, index)?;
            }
        } else {
            log::warn!("Gear configuration has no 'type' tag");
        }
    }
    Ok(())
}

#[cfg(test)]

mod test {
    use super as parse;
    use super::DynResult;
    use crate::common::address::Long;
    use crate::gear::address::Short;
    use parse::{ConfigureGear, CreateGear};

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

    struct GearFactory {
        pub gears: Vec<Gear>,
    }

    impl Default for GearFactory {
        fn default() -> Self {
            Self { gears: Vec::new() }
        }
    }
    impl CreateGear for GearFactory {
        fn new_gear(&mut self, name: &str, gear_type: &str) -> DynResult<&mut dyn ConfigureGear> {
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
"
            .as_ref(),
            &mut factory,
        )?;
        assert_eq!(factory.gears.len(), 8);
        assert_eq!(&factory.gears[5].gear_type, "generic_gear");
        assert_eq!(&factory.gears[2].name, "BAR05");
        assert_eq!(&factory.gears[3].short_address, &Some(Short::new(9)));
        Ok(())
    }
}
#[test]
fn test_label_offset() {
    assert_eq!(label_offset("23", 1), "24".to_string());
    assert_eq!(label_offset("BAR9R", 1), "BAR10R".to_string());
    assert_eq!(label_offset("BAR09R", 1), "BAR10R".to_string());
    assert_eq!(label_offset("BAR009R", 1), "BAR010R".to_string());
    assert_eq!(label_offset("BAR009R", 99), "BAR108R".to_string());
    assert_eq!(label_offset("BAR", 99), "BAR_99".to_string());
}
