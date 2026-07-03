use crate::drivers::driver::{
    DaliBusEventResult, DaliBusEventType, DaliDriver, DaliFrame, DaliSendResult, DriverInfo,
    OpenError,
};
use crate::drivers::send_flags::Flags;
use crate::drivers::simulator::device::{DALI_SIMULATOR_DEVICES, DaliSimDeviceEntry};
use crate::drivers::simulator::sim_bus::{DaliSimBus, DaliSimBusDevice, DaliSimBusDeviceEvent};
use crate::drivers::simulator::sim_scheduler::SimulatorScheduler;
use crate::drivers::simulator::sim_scheduler_impl::SimulatorSchedulerImpl;
use crate::drivers::simulator::timing;
use crate::futures::FutureExt;
use crate::utils::dyn_future::DynFuture;
use log::{debug, warn};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::future::{self, Future};
use std::pin::Pin;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum SimDriverError {
    OK,
    QueuingFailed,
    ReplyingFailed,
    ThreadError,
}

impl Error for SimDriverError {}

impl fmt::Display for SimDriverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimDriverError::OK => write!(f, "No error"),
            SimDriverError::QueuingFailed => write!(f, "Queuing failed"),
            SimDriverError::ReplyingFailed => write!(f, "Replying failed"),
            SimDriverError::ThreadError => write!(f, "Thread error"),
        }
    }
}

async fn debug_task(device: DaliSimBusDevice) {
    let start_time = device.current_time();
    loop {
        match device.wait().await {
            DaliSimBusDeviceEvent::Timeout => {}
            DaliSimBusDeviceEvent::Shutdown => break,
            DaliSimBusDeviceEvent::Message(msg) => {
                debug!(
                    "{}: {}: {} -> {} {:?}",
                    (device.current_time() - start_time).as_millis(),
                    msg.source_id,
                    if let Some(start) = msg.start {
                        (start - start_time).as_millis().to_string()
                    } else {
                        "-".to_string()
                    },
                    (msg.timestamp - start_time).as_millis(),
                    msg.event_type
                );
            }
        }
    }
}

pub struct DaliSimDriver {
    sched: Box<dyn SimulatorScheduler + Send>,
    bus_device: DaliSimBusDevice,
}

impl DaliSimDriver {
    pub fn new(
        bus_device: DaliSimBusDevice,
        mut sched: Box<dyn SimulatorScheduler + Send>,
    ) -> DaliSimDriver {
        let driver_task = sched.new_task();
        DaliSimDriver { sched, bus_device }
    }
}

impl DaliDriver for DaliSimDriver {
    fn send_frame<'a>(
        &'a mut self,
        cmd: DaliFrame,
        flags: Flags,
    ) -> Pin<Box<dyn Future<Output = DaliSendResult> + Send + 'a>> {
        Box::pin(async move {
            let mut frame_start =
                self.bus_device.current_time() + timing::send_delay(flags.priority(), false);
            loop {
                match self.bus_device.wait_until(frame_start).await {
                    DaliSimBusDeviceEvent::Timeout => {
                        break;
                    }
                    DaliSimBusDeviceEvent::Message(bus_event) => {
                        frame_start =
                            bus_event.timestamp + timing::send_delay(flags.priority(), false);
                    }
                    DaliSimBusDeviceEvent::Shutdown => {
                        return DaliSendResult::DriverError("Shutdown".into());
                    }
                }
            }
            let mut frame_end = frame_start + timing::frame_duration(&cmd);

            self.bus_device.add_event(
                DaliBusEventType::from(cmd.clone()),
                frame_end,
                Some(frame_start),
            );
            match self.bus_device.wait_until(frame_end).await {
                DaliSimBusDeviceEvent::Timeout => {}
                DaliSimBusDeviceEvent::Message(_bus_event) => {
                    return DaliSendResult::Framing;
                }
                DaliSimBusDeviceEvent::Shutdown => {
                    return DaliSendResult::DriverError("Shutdown".into());
                }
            }
            if flags.send_twice() {
                let frame_start = self.bus_device.current_time() + Duration::from_micros(13500);
                frame_end = frame_start + timing::frame_duration(&cmd);
                match self.bus_device.wait_until(frame_start).await {
                    DaliSimBusDeviceEvent::Timeout => {}
                    DaliSimBusDeviceEvent::Message(_msg) => {
                        return DaliSendResult::Framing;
                    }
                    DaliSimBusDeviceEvent::Shutdown => {
                        return DaliSendResult::DriverError("Shutdown".into());
                    }
                }

                self.bus_device.add_event(
                    DaliBusEventType::from(cmd.clone()),
                    frame_end,
                    Some(frame_start),
                );
            }
            match self.bus_device.wait_until(frame_end).await {
                DaliSimBusDeviceEvent::Timeout => {}
                DaliSimBusDeviceEvent::Message(_msg) => {
                    return DaliSendResult::Framing;
                }
                DaliSimBusDeviceEvent::Shutdown => {
                    return DaliSendResult::DriverError("Shutdown".into());
                }
            }

            if flags.expect_answer() {
                match self
                    .bus_device
                    .wait_until(self.bus_device.current_time() + Duration::from_millis(50))
                    .await
                {
                    DaliSimBusDeviceEvent::Timeout => return DaliSendResult::Timeout,
                    DaliSimBusDeviceEvent::Message(bus_event) => {
                        if let DaliBusEventType::Frame8(data) = bus_event.event_type {
                            return DaliSendResult::Answer(data);
                        } else {
                            return DaliSendResult::Framing;
                        }
                    }

                    DaliSimBusDeviceEvent::Shutdown => {
                        return DaliSendResult::DriverError("Shutdown".into());
                    }
                }
            } else {
                match self
                    .bus_device
                    .wait_until(self.bus_device.current_time() + timing::STOP_CONDITION)
                    .await
                {
                    DaliSimBusDeviceEvent::Timeout => return DaliSendResult::Ok,
                    DaliSimBusDeviceEvent::Message(bus_event) => {
                        debug!("Messsage while waiting for stop condition: {:?}", bus_event);
                        return DaliSendResult::Framing;
                    }
                    DaliSimBusDeviceEvent::Shutdown => {
                        return DaliSendResult::DriverError("Shutdown".into());
                    }
                }
            }
        })
    }

    fn next_bus_event(&mut self) -> DynFuture<'_, DaliBusEventResult> {
        Box::pin(future::ready(Err("Not implemented".into())))
    }

    fn current_timestamp(&self) -> Instant {
        self.bus_device.current_time()
    }

    fn wait_until(&self, end: Instant) -> DynFuture<'_, ()> {
        Box::pin(self.bus_device.wait_until(end).map(|_| ()))
    }
}

fn driver_open(params: HashMap<String, String>) -> Result<Box<dyn DaliDriver>, OpenError> {
    let conf_filename = params
        .get("config")
        .map(|s| s.as_str())
        .unwrap_or("sim.yaml");
    let conf_file = File::open(conf_filename).map_err(|e| {
        OpenError::DriverError(
            format!(
                "Failed to open configuration file '{}': {}",
                conf_filename, e
            )
            .into(),
        )
    })?;
    let conf: yaml_serde::Mapping =
        yaml_serde::from_reader(conf_file).map_err(|e| OpenError::DriverError(e.into()))?;
    let Some(device_conf) = conf.get("devices") else {
        return Err(OpenError::DriverError(
            "No 'devices' tag found in configuration file".into(),
        ));
    };
    let yaml_serde::Value::Sequence(device_list) = device_conf else {
        return Err(OpenError::DriverError("'devices' is not a sequence".into()));
    };
    let mut sched = SimulatorSchedulerImpl::new();
    let bus = DaliSimBus::new(sched.new_task());
    for device in device_list {
        let yaml_serde::Value::Mapping(conf) = device else {
            return Err(OpenError::DriverError(
                "Item in device list is not a mapping".into(),
            ));
        };
        if let Some(device_type) = conf.get("type").and_then(|v| v.as_str()) {
            debug!("Type: {}", device_type);
            let Some(dev_entry) = DALI_SIMULATOR_DEVICES
                .iter()
                .position(|registered| registered.name == device_type)
                .map(|p| &DALI_SIMULATOR_DEVICES[p])
            else {
                return Err(OpenError::DriverError(
                    format!("Devivce type '{}' not available", device_type).into(),
                ));
            };
            let mut device = (dev_entry.init)();
            device.start(DaliSimBusDevice::new(bus.clone(), sched.new_task()));
        } else {
            warn!("Device configuration has no 'type' tag");
        }
    }
    let bus_device = DaliSimBusDevice::new(bus, sched.new_task());
    let driver = DaliSimDriver::new(bus_device, Box::new(sched));
    Ok(Box::new(driver))
}

pub fn driver_info() -> DriverInfo {
    DriverInfo {
        name: "SIM".to_string(),
        description: "Simulated devices".to_string(),
        open: driver_open,
    }
}
