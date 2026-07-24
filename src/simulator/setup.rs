use crate::simulator;
use log::{debug, warn};
use simulator::device::{DALI_SIMULATOR_DEVICES, DaliSimDevice};
use simulator::sim_bus::{DaliSimBus, DaliSimBusDevice};
use simulator::sim_scheduler::SimulatorScheduler;
use simulator::sim_scheduler_impl::SimulatorSchedulerImpl;
use std::sync::Arc;

pub fn setup_simulator<R>(
    conf_file: R,
) -> Result<
    (
        Arc<DaliSimBus>,
        Box<dyn SimulatorScheduler + Send + Sync>,
        Vec<Box<dyn DaliSimDevice>>,
    ),
    Box<dyn std::error::Error + Sync + Send>,
>
where
    R: std::io::Read,
{
    let conf: yaml_serde::Mapping = yaml_serde::from_reader(conf_file)?;
    let Some(device_conf) = conf.get("devices") else {
        return Err("No 'devices' tag found in configuration file".into());
    };
    let yaml_serde::Value::Sequence(device_list) = device_conf else {
        return Err("'devices' is not a sequence".into());
    };
    let mut sched = SimulatorSchedulerImpl::new();
    let bus = DaliSimBus::new(sched.new_task());
    let mut devices = Vec::new();
    for device in device_list {
        let yaml_serde::Value::Mapping(conf) = device else {
            return Err("Item in device list is not a mapping".into());
        };
        if let Some(device_type) = conf.get("type").and_then(|v| v.as_str()) {
            debug!("Type: {}", device_type);
            let Some(dev_entry) = DALI_SIMULATOR_DEVICES
                .iter()
                .position(|registered| registered.name == device_type)
                .map(|p| &DALI_SIMULATOR_DEVICES[p])
            else {
                return Err(format!("Devivce type '{}' not available", device_type).into());
            };
            let count = conf.get("count").and_then(|v| v.as_i64()).unwrap_or(1) as usize;
            for index in 0..count {
                let mut device = (dev_entry.init)();
                device.configure(conf, index)?;
                device.start(DaliSimBusDevice::new(bus.clone(), sched.new_task()))?;
                devices.push(device);
            }
        } else {
            warn!("Device configuration has no 'type' tag");
        }
    }
    Ok((bus, Box::new(sched), devices))
}
