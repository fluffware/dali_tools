use clap::{Arg, Command};
use dali_tools::common::address::DisplayValue;
use dali_tools::common::address::{Long, Short};
use dali_tools::utils::parse_config::{self, ConfigureGear, CreateGear, DynResult};
use serde::Serializer;
use serde_derive::Serialize;
use std::collections::BTreeMap;
use std::fs::File;

fn serialize_short_addr<S>(v: &Option<Option<Short>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match v {
        None => serializer.serialize_none(),
        Some(addr_or_mask) => {
            if let Some(addr) = addr_or_mask {
                serializer.serialize_u8(addr.display_value())
            } else {
                serializer.serialize_str("MASK")
            }
        }
    }
}

#[derive(Serialize)]
struct Gear {
    #[serde(rename = "type")]
    gear_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    random_address: Option<Long>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(serialize_with = "serialize_short_addr")]
    short_address: Option<Option<Short>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_light_level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    power_on_level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_failure_level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_level: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fade_time: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fade_rate: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extended_fade_time_base: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extended_fade_time_multiplier: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    gear_groups: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scenes: Option<Vec<(u8, u8)>>,
}

impl Default for Gear {
    fn default() -> Gear {
        Gear {
            gear_type: String::new(),
            random_address: None,
            short_address: None,
            last_light_level: None,
            target_level: None,
            power_on_level: None,
            system_failure_level: None,
            min_level: None,
            max_level: None,
            fade_time: None,
            fade_rate: None,
            extended_fade_time_base: None,
            extended_fade_time_multiplier: None,
            gear_groups: None,
            scenes: None,
        }
    }
}
impl Gear {
    fn new(gear_type: &str) -> Self {
        let mut gear = Self::default();
        gear.gear_type = gear_type.to_string();
        gear
    }
}
impl ConfigureGear for Gear {
    fn conf_random_address(&mut self, addr: Long) {
        self.random_address = Some(addr);
    }
    fn conf_short_address(&mut self, addr_or_mask: Option<Short>) {
        self.short_address = Some(addr_or_mask)
    }
    fn conf_last_light_level(&mut self, level: u8) {
        self.last_light_level = Some(level)
    }
    fn conf_target_level(&mut self, level: u8) {
        self.target_level = Some(level)
    }
    fn conf_power_on_level(&mut self, level: u8) {
        self.power_on_level = Some(level)
    }
    fn conf_system_failure_level(&mut self, level: u8) {
        self.system_failure_level = Some(level)
    }
    fn conf_min_level(&mut self, level: u8) {
        self.min_level = Some(level)
    }
    fn conf_max_level(&mut self, level: u8) {
        self.max_level = Some(level)
    }
    fn conf_fade_time(&mut self, time: u8) {
        self.fade_time = Some(time)
    }
    fn conf_fade_rate(&mut self, rate: u8) {
        self.fade_rate = Some(rate)
    }
    fn conf_extended_fade_time_base(&mut self, base: u8) {
        self.extended_fade_time_base = Some(base)
    }
    fn conf_extended_fade_time_multiplier(&mut self, multiplier: u8) {
        self.extended_fade_time_multiplier = Some(multiplier)
    }
    fn conf_gear_groups(&mut self, groups: u16) {
        self.gear_groups = Some(groups)
    }
    fn conf_scenes(&mut self, map: &[(u8, u8)]) {
        self.scenes = Some(Vec::from(map))
    } // (scene number (0 based), scene level)
}

#[derive(Serialize)]
struct GearFactory {
    pub gears: BTreeMap<String, Gear>,
}

impl Default for GearFactory {
    fn default() -> Self {
        Self {
            gears: BTreeMap::new(),
        }
    }
}

impl CreateGear for GearFactory {
    fn new_gear(&mut self, name: &str, gear_type: &str) -> DynResult<&mut dyn ConfigureGear> {
        eprintln!("new_gear: {}", name);
        let gear = Gear::new(gear_type);
        self.gears.insert(name.to_string(), gear);
        Ok(self.gears.get_mut(name).unwrap())
    }
}

#[derive(Serialize)]
struct ConfFile {
    dali: GearFactory,
}
fn main() {
    let cli_cmd = Command::new("dali_expand_config")
        .about("Simplify DALI configuration file")
        .arg(Arg::new("CONFIG").required(true).help("Configuration file"));
    let matches = cli_cmd.get_matches();
    let conf_filename = matches.get_one::<String>("CONFIG").unwrap();
    let conf_file = match File::open(conf_filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "Failed to open configuration file '{}': {}",
                conf_filename, e
            );
            return;
        }
    };
    let mut factory = GearFactory::default();
    if let Err(e) = parse_config::parse_config(conf_file, &mut factory) {
        eprintln!("Failed to parse configuration file: {}", e);
        return;
    }
    let conf_file = ConfFile { dali: factory };
    println!("{}", yaml_serde::to_string(&conf_file).unwrap());
}
