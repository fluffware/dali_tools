use crate::simulator;
use crate::utils::parse_config::{self, ConfigureGear, CreateGear, DynResult};
use simulator::device::{DALI_SIMULATOR_DEVICES, DaliSimDevice};
use simulator::sim_bus::{DaliSimBus, DaliSimBusDevice};
use simulator::sim_scheduler::SimulatorScheduler;
use simulator::sim_scheduler_impl::SimulatorSchedulerImpl;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

struct GearFactory {
    pub gears: Vec<(String, Box<dyn DaliSimDevice + Send + Sync>)>,
}

impl GearFactory {
    fn new() -> Self {
        GearFactory { gears: Vec::new() }
    }
}

impl CreateGear for GearFactory {
    fn new_gear(
        &mut self,
        name: &str,
        gear_type: &str,
    ) -> DynResult<&mut (dyn ConfigureGear + Send + Sync)> {
        let Some(dev_entry) = DALI_SIMULATOR_DEVICES
            .iter()
            .position(|registered| registered.name == gear_type)
            .map(|p| &DALI_SIMULATOR_DEVICES[p])
        else {
            return Err(format!("Device type '{}' not available", gear_type).into());
        };
        let device = (dev_entry.init)(name.to_string());
        let device = self.gears.push_mut((name.to_string(), device));
        Ok(device.1.as_mut())
    }
}

pub struct Simulator {
    scheduler: Arc<RwLock<dyn SimulatorScheduler + Send + Sync>>,
    bus: Arc<DaliSimBus>,
    devices: Arc<RwLock<HashMap<String, Box<dyn DaliSimDevice + Sync + Send>>>>,
}

impl Simulator {
    pub fn new() -> Simulator {
        let mut scheduler = SimulatorSchedulerImpl::new();
        let bus = DaliSimBus::new(scheduler.new_task());
        let devices = Arc::new(RwLock::new(HashMap::new()));
        Simulator {
            scheduler: Arc::new(RwLock::new(scheduler)),
            bus,
            devices,
        }
    }

    pub fn configure<R>(&self, conf_file: R) -> Result<(), Box<dyn std::error::Error + Sync + Send>>
    where
        R: std::io::Read,
    {
        let mut factory = GearFactory::new();
        let mut devices = self.devices.write().unwrap();
        parse_config::parse_config(conf_file, &mut factory)?;
        for gear in devices.values_mut() {
            gear.stop()?;
        }
        devices.clear();
        for (name, mut gear) in factory.gears.drain(..) {
            gear.start(DaliSimBusDevice::new(
                self.bus.clone(),
                self.scheduler.write().unwrap().new_task(),
            ))?;
            devices.insert(name, gear);
        }
        Ok(())
    }

    pub fn bus(&self) -> Arc<DaliSimBus> {
        self.bus.clone()
    }

    pub fn scheduler(&self) -> Arc<RwLock<dyn SimulatorScheduler + Send + Sync>> {
        self.scheduler.clone()
    }
    pub fn devices(&self) -> Arc<RwLock<HashMap<String, Box<dyn DaliSimDevice + Send + Sync>>>> {
        self.devices.clone()
    }
}
