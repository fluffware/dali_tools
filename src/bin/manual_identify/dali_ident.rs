use super::configuration::{
    ConfigurationDriver, ConfigurationId, ConfigurationInfo, DynResultFuture, GearConfiguration,
    GearId, GearInfo,
};
use dali::common::address::Short;
use dali::common::defs::MASK;
use dali::drivers::command_utils::send16;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
use dali::drivers::send_flags::{NO_FLAG, PRIORITY_1};
use dali::gear::address::Address;
use dali::gear::cmd_defs as cmd;
use dali::gear::commands_102::Commands102;
use dali::utils::address_assignment::program_short_addresses;
use dali_tools as dali;
use dali_tools::common::driver_commands::DriverCommands;
use log::debug;
use serde::de::MapAccess;
use serde::de::Visitor;
use serde::de::{Deserialize, Deserializer};
use serde_derive::Deserialize;
use std::future;
use std::io::Read;
use std::sync::Arc;
use tokio::sync::Mutex;
use yaml_serde;

#[derive(Debug)]
pub enum Error {
    Yaml(yaml_serde::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Yaml(e) => e.fmt(f),
        }
    }
}

impl From<yaml_serde::Error> for Error {
    fn from(err: yaml_serde::Error) -> Error {
        Self::Yaml(err)
    }
}

#[derive(Deserialize, Debug)]
struct DaliGearConfiguration {
    #[serde(skip)]
    label: String,
    address: u8,
    group: Option<u8>,
    fade_time: Option<u8>,
    fade_rate: Option<u8>,
}

struct GearConfVisitor {}

impl GearConfVisitor {
    fn new() -> Self {
        Self {}
    }
}
impl<'de> Visitor<'de> for GearConfVisitor {
    type Value = VecDaliGearConfiguration;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a gear configuration")
    }

    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: MapAccess<'de>,
    {
        let mut conf = Vec::new();
        while let Some((key, mut value)) = map.next_entry::<String, DaliGearConfiguration>()? {
            value.label = key;
            conf.push(value);
        }
        debug!("{:?}", conf);
        Ok(VecDaliGearConfiguration(conf))
    }
}

struct VecDaliGearConfiguration(Vec<DaliGearConfiguration>);

impl<'de> Deserialize<'de> for VecDaliGearConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let visitor = GearConfVisitor::new();
        deserializer.deserialize_map(visitor)
    }
}

#[derive(Deserialize)]
struct ConfigFile {
    dali: VecDaliGearConfiguration,
}

pub struct DaliConfigurationDriver {
    hw_driver: Arc<Mutex<Box<dyn DaliDriver>>>,
    low_level: u8,
    high_level: u8,
    conf_file: Option<ConfigFile>,
}
impl DaliConfigurationDriver {
    pub fn new(hw_driver: Arc<Mutex<Box<dyn DaliDriver>>>) -> DaliConfigurationDriver {
        DaliConfigurationDriver {
            hw_driver,
            low_level: MASK,
            high_level: MASK,
            conf_file: None,
        }
    }
    /*
    fn get_conf_addr(&self, conf: ConfigurationId) -> Short {
        let a: u16 = conf.into();
        assert!(a >= 1 && a <= 64);
        Short::new(a as u8)
    }*/

  
    pub fn read_config<R: Read>(&mut self, reader: R) -> Result<(), Error> {
        self.conf_file = Some(yaml_serde::from_reader(reader)?);
        Ok(())
    }
}

impl ConfigurationDriver for DaliConfigurationDriver {
    fn start_configuration(&self) -> DynResultFuture<()> {
        Box::pin(future::ready(Ok(())))
    }
    fn end_configuration(&self) -> DynResultFuture<()> {
        Box::pin(future::ready(Ok(())))
    }
    fn set_low(&self, id: GearId) -> DynResultFuture<()> {
        let hw_driver = self.hw_driver.clone();
        let low_level = self.low_level;
        Box::pin(async move {
            let driver = &mut **hw_driver.lock().await;
            let addr = Address::Short(Short::new((Into::<u16>::into(id) - 1) as u8));
            match if low_level == MASK {
                send16::cmd(driver, cmd::RECALL_MIN_LEVEL(addr), NO_FLAG).await
            } else {
                send16::device_level(driver, addr, low_level, NO_FLAG).await
            } {
                DaliSendResult::Ok => {}
                e => return Err(e.into()),
            }
            Ok(())
        })
    }
    fn set_all_low(&self) -> DynResultFuture<()> {
        let hw_driver = self.hw_driver.clone();
        let low_level = self.low_level;
        Box::pin(async move {
            let driver = &mut **hw_driver.lock().await;
            let addr = Address::Broadcast;
            match if low_level == MASK {
                send16::cmd(driver, cmd::RECALL_MIN_LEVEL(addr), NO_FLAG).await
            } else {
                send16::device_level(driver, addr, low_level, NO_FLAG).await
            } {
                DaliSendResult::Ok => {}
                e => return Err(e.into()),
            }
            Ok(())
        })
    }

    fn set_high(&self, id: GearId) -> DynResultFuture<()> {
        let hw_driver = self.hw_driver.clone();
        let high_level = self.high_level;
        Box::pin(async move {
            let driver = &mut **hw_driver.lock().await;
            let addr = Address::Short(Short::new((Into::<u16>::into(id) - 1) as u8));
            match if high_level == MASK {
                send16::cmd(driver, cmd::RECALL_MAX_LEVEL(addr), NO_FLAG).await
            } else {
                send16::device_level(driver, addr, high_level, NO_FLAG).await
            } {
                DaliSendResult::Ok => {}
                e => return Err(e.into()),
            }
            Ok(())
        })
    }

    fn find_all(&self, mut found: Box<dyn FnMut(GearInfo) + Send>) -> DynResultFuture<()> {
        let hw_driver = self.hw_driver.clone();
        Box::pin(async move {
            for addr in 0..64 {
                debug!("Checking {}", addr);
                let driver = &mut **hw_driver.lock().await;
                let mut cmd = Commands102::from_driver(driver, PRIORITY_1);
                match cmd.query(cmd::QUERY_STATUS(Short::new(addr))).await {
                    Ok(_s) => {
                        found(GearInfo {
                            id: GearId::try_from(addr as u16 + 1).unwrap(),
                            label: format!("{}", addr + 1),
                            conf: None,
                        });
                    }
                    Err(DaliSendResult::Timeout) => {}
                    Err(e) => return Err(e.into()),
                };
            }
            Ok(())
        })
    }
    fn configurations(&self) -> Vec<ConfigurationInfo> {
        let mut confs = Vec::new();
        if let Some(conf_file) = &self.conf_file {
            for c in &conf_file.dali.0 {
                confs.push(ConfigurationInfo {
                    id: ConfigurationId::try_from(c.address as u16).unwrap(),
                    label: format!("{} ({})",c.label, c.address),
                });
            }
        } else {
            for conf in 1..=64 {
	        let id = ConfigurationId::try_from(conf).unwrap();
		let info = ConfigurationInfo {
                    id: id.clone(),
                    label: 	if conf >= 1 && conf <= 64 {
			format!("({})", conf)
		    } else {
			"-".to_string()
		    }
		};
		confs.push(info);
		
            }
        }
        confs
    }

    // Invalidates all gear ids
    fn commit(&self, gears: Vec<GearConfiguration>) -> DynResultFuture<()> {
        let hw_driver = self.hw_driver.clone();
        Box::pin(async move {
            let driver = &mut **hw_driver.lock().await;
            let mut swaps = Vec::new();
            for g in gears.iter() {
                swaps.push((
                    Short::new((Into::<u16>::into(g.id.clone()) - 1) as u8),
                    Short::new((Into::<u16>::into(g.conf.clone()) - 1) as u8),
                ));
            }
            let mut cmd = Commands102::from_driver(driver, PRIORITY_1);
            program_short_addresses(&mut cmd, &swaps).await?;
            Ok(())
        })
    }
}
