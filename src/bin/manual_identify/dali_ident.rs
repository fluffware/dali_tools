use super::configuration::{
    ConfigurationDriver, ConfigurationId, ConfigurationInfo, DynResultFuture, GearConfiguration,
    GearId, GearInfo, GearRemap,
};
use dali::common::address::Short;
use dali::common::defs::MASK;
use dali::drivers::command_utils::send16;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
use dali::drivers::send_flags::{NO_FLAG, PRIORITY_1};
use dali::gear::address::Address;
use dali::gear::cmd_defs as cmd;
use dali::gear::commands_102::Commands102;
//use dali::gear::fade::{FadeRate, FadeTime};
use dali::utils::address_assignment::program_short_addresses;
use dali::utils::parse_config::{self, ConfigureGear, CreateGear, DynResult};
use dali_tools as dali;
use dali_tools::common::driver_commands::DriverCommands;
use log::debug;
use std::future;
use std::io::Read;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug)]
pub enum Error {
    ConfigurationError(String),
    InvalidGearId,
    InvalidConfigurartionId,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigurationError(e) => e.fmt(f),
            Self::InvalidGearId => write!(f, "Invalid gear ID"),
            Self::InvalidConfigurartionId => write!(f, "Invalid configuration ID"),
        }
    }
}

#[derive(Debug)]
struct DaliGearConfiguration {
    label: String,
    address: Option<Short>,
    //group: Option<u8>,
    //fade_time: Option<FadeTime>,
    //fade_rate: Option<u8>,
}

impl ConfigureGear for DaliGearConfiguration {
    fn conf_short_address(&mut self, addr_or_mask: Option<Short>) {
        self.address = addr_or_mask;
    }
}

struct ConfigFile {
    dali: Vec<DaliGearConfiguration>, // (ConfigurationId - 1) indexes this vector
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self { dali: Vec::new() }
    }
}
impl CreateGear for ConfigFile {
    fn new_gear(&mut self, name: &str, _gear_type: &str) -> DynResult<&mut dyn ConfigureGear> {
        let gear_conf = DaliGearConfiguration {
            label: name.to_string(),
            address: None,
        };
        let device = self.dali.push_mut(gear_conf);
        Ok(device)
    }
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
        let mut conf_file = ConfigFile::default();
        parse_config::parse_config(reader, &mut conf_file)
            .map_err(|e| Error::ConfigurationError(e.to_string()))?;
        self.conf_file = Some(conf_file);
        Ok(())
    }

    fn get_conf(&self, id: &ConfigurationId) -> Result<&DaliGearConfiguration, Error> {
        let index = (Into::<u16>::into(id.clone()) - 1) as usize;
        if let Some(conf_file) = &self.conf_file {
            if index >= conf_file.dali.len() {
                debug!("Index {}", index);
                return Err(Error::InvalidConfigurartionId);
            }
            Ok(&conf_file.dali[index])
        } else {
            Err(Error::InvalidConfigurartionId)
        }
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
            let addr = Into::<u16>::into(id) - 1;
            if addr >= 64 {
                return Err(Error::InvalidGearId.into());
            }
            let addr = Address::Short(Short::new(addr as u8));
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
            let addr = Into::<u16>::into(id) - 1;
            if addr >= 64 {
                return Err(Error::InvalidGearId.into());
            }
            let addr = Address::Short(Short::new(addr as u8));
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
            for (index, c) in conf_file.dali.iter().enumerate() {
                confs.push(ConfigurationInfo {
                    id: ConfigurationId::try_from(index as u16 + 1).unwrap(),
                    label: format!("{} ({})", c.label, c.address.unwrap().to_string()),
                });
            }
        } else {
            for conf in 1..=64 {
                let id = ConfigurationId::try_from(conf).unwrap();
                let info = ConfigurationInfo {
                    id: id.clone(),
                    label: if conf >= 1 && conf <= 64 {
                        format!("({})", conf)
                    } else {
                        "-".to_string()
                    },
                };
                confs.push(info);
            }
        }
        confs
    }

    // Invalidates all gear ids
    fn commit(&self, gears: Vec<GearConfiguration>) -> DynResultFuture<Vec<GearRemap>> {
        let mut swaps = Vec::new();
        let mut remap = Vec::new();
        for g in gears.iter() {
            let conf = match self.get_conf(&g.conf) {
                Ok(c) => c,
                Err(e) => return Box::pin(future::ready(Err(e.into()))),
            };
            swaps.push((
                Short::new((Into::<u16>::into(g.id.clone()) - 1) as u8),
                conf.address.unwrap(),
            ));
            remap.push(GearRemap {
                old: g.id.clone(),
                new: GearInfo {
                    id: GearId::try_from(conf.address.unwrap().value() as u16 + 1).unwrap(),
                    label: conf.label.clone(),
                    conf: Some(g.conf.clone()),
                },
            });
        }

        let hw_driver = self.hw_driver.clone();
        Box::pin(async move {
            let driver = &mut **hw_driver.lock().await;
            let mut cmd = Commands102::from_driver(driver, PRIORITY_1);
            program_short_addresses(&mut cmd, &swaps).await?;
            Ok(remap)
        })
    }
}
