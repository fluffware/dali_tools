use crate::simulator;
use crate::utils::parse_config::{self, ConfigureGear, CreateGear, DynResult};
use simulator::device::{DALI_SIMULATOR_DEVICES, DaliSimDevice};
use simulator::sim_bus::{DaliSimBus, DaliSimBusDevice};
use simulator::sim_scheduler::SimulatorScheduler;
use simulator::sim_scheduler_impl::SimulatorSchedulerImpl;
use std::collections::HashMap;
use std::sync::Arc;

struct GearFactory {
    pub gears: Vec<(String, Box<dyn DaliSimDevice>)>,
}

impl GearFactory {
    fn new() -> Self {
        GearFactory { gears: Vec::new() }
    }
}

impl CreateGear for GearFactory {
    fn new_gear(&mut self, name: &str, gear_type: &str) -> DynResult<&mut dyn ConfigureGear> {
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

pub fn setup_simulator<R>(
    conf_file: R,
) -> Result<
    (
        Arc<DaliSimBus>,
        Box<dyn SimulatorScheduler + Send + Sync>,
        HashMap<String, Box<dyn DaliSimDevice>>,
    ),
    Box<dyn std::error::Error + Sync + Send>,
>
where
    R: std::io::Read,
{
    let mut sched = SimulatorSchedulerImpl::new();
    let bus = DaliSimBus::new(sched.new_task());
    let mut factory = GearFactory::new();
    parse_config::parse_config(conf_file, &mut factory)?;
    let mut map = HashMap::new();
    for (name, mut gear) in factory.gears.drain(..) {
        gear.start(DaliSimBusDevice::new(bus.clone(), sched.new_task()))?;
        map.insert(name, gear);
    }
    Ok((bus, Box::new(sched), map))
}
