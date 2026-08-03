use crate::simulator;
use log::{debug, warn};
use simulator::device::{DALI_SIMULATOR_DEVICES, DaliSimDevice};
use simulator::sim_bus::{DaliSimBus, DaliSimBusDevice};
use simulator::sim_scheduler::SimulatorScheduler;
use simulator::sim_scheduler_impl::SimulatorSchedulerImpl;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

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

    let Some(device_conf) = dali_conf.get("devices") else {
        return Err("No 'devices' tag found in configuration file".into());
    };
    let yaml_serde::Value::Mapping(device_list) = device_conf else {
        return Err("'devices' is not a mapping".into());
    };
    let mut sched = SimulatorSchedulerImpl::new();
    let bus = DaliSimBus::new(sched.new_task());
    let mut devices = HashMap::new();
    for (name, device) in device_list {
        let yaml_serde::Value::Mapping(conf) = device else {
            return Err("Item in device list is not a mapping".into());
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
                let Some(base_name) = name.as_str() else {
                    return Err("Device name is not a string".into());
                };
                let dev_name = label_offset(base_name, index);

                let mut device = (dev_entry.init)(dev_name.clone());
                device.configure(&conf, index)?;
                device.start(DaliSimBusDevice::new(bus.clone(), sched.new_task()))?;
                devices.insert(dev_name, device);
            }
        } else {
            warn!("Device configuration has no 'type' tag");
        }
    }
    Ok((bus, Box::new(sched), devices))
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
