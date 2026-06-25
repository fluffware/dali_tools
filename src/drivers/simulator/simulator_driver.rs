use crate::drivers::driver::{
    DaliBusEventResult, DaliBusEventType, DaliDriver, DaliFrame, DaliSendResult, DriverInfo,
    OpenError,
};
use crate::drivers::send_flags::Flags;
use crate::drivers::simulator::sim_bus::{DaliSimBus, DaliSimBusEvent};
use crate::drivers::simulator::sim_scheduler::SimulatorEvent;
use crate::drivers::simulator::sim_scheduler::SimulatorScheduler;
use crate::drivers::simulator::sim_scheduler::SimulatorTask;
use crate::drivers::simulator::sim_scheduler_impl::SimulatorSchedulerImpl;
use crate::drivers::simulator::timing;
use crate::futures::FutureExt;
use crate::utils::dyn_future::DynFuture;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::future::{self, Future};
use std::pin::Pin;
use std::sync::Arc;
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

pub struct DaliSimDriver {
    sched: SimulatorSchedulerImpl,
    driver_task: Box<dyn SimulatorTask + Send + Sync>,
    bus: Arc<DaliSimBus>,
}

impl DaliSimDriver {
    pub fn new() -> DaliSimDriver {
        let mut sched = SimulatorSchedulerImpl::new();
        let driver_task = sched.new_task();
        let bus = DaliSimBus::new(sched.new_task());
        DaliSimDriver {
            sched,
            driver_task,
            bus,
        }
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
                self.driver_task.current_time() + timing::send_delay(flags.priority(), false);
            loop {
                match self.driver_task.wait_until(frame_start).await {
                    SimulatorEvent::Timeout => {
                        break;
                    }
                    SimulatorEvent::Message(msg) => {
                        if let Some(bus_event) = msg.downcast_ref::<DaliSimBusEvent>() {
                            frame_start =
                                bus_event.timestamp + timing::send_delay(flags.priority(), false);
                        }
                    }
                    SimulatorEvent::Shutdown => {
                        return DaliSendResult::DriverError("Shutdown".into());
                    }
                }
            }
            let mut frame_end = frame_start + timing::frame_duration(&cmd);
            let sim_event = DaliSimBusEvent {
                source_id: self.driver_task.task_id(),
                timestamp: frame_end,
                start: Some(frame_start),
                event_type: DaliBusEventType::from(cmd.clone()),
            };
            self.bus.add_event(sim_event);
            match self.driver_task.wait_until(frame_end).await {
                SimulatorEvent::Timeout => {}
                SimulatorEvent::Message(msg) => {
                    if let Some(_bus_event) = msg.downcast_ref::<DaliSimBusEvent>() {
                        return DaliSendResult::Framing;
                    } else {
                        return DaliSendResult::DriverError("Unexpected message".into());
                    }
                }
                SimulatorEvent::Shutdown => return DaliSendResult::DriverError("Shutdown".into()),
            }
            if flags.send_twice() {
                let frame_start = self.driver_task.current_time() + Duration::from_micros(13500);
                frame_end = frame_start + timing::frame_duration(&cmd);
                match self.driver_task.wait_until(frame_start).await {
                    SimulatorEvent::Timeout => {}
                    SimulatorEvent::Message(_msg) => {
                        return DaliSendResult::Framing;
                    }
                    SimulatorEvent::Shutdown => {
                        return DaliSendResult::DriverError("Shutdown".into());
                    }
                }

                let sim_event = DaliSimBusEvent {
                    source_id: self.driver_task.task_id(),
                    timestamp: frame_end,
                    start: Some(frame_start),
                    event_type: DaliBusEventType::from(cmd),
                };
                self.bus.add_event(sim_event);
            }
            match self.driver_task.wait_until(frame_end).await {
                SimulatorEvent::Timeout => {}
                SimulatorEvent::Message(msg) => {
                    if let Some(_bus_event) = msg.downcast_ref::<DaliSimBusEvent>() {
                        return DaliSendResult::Framing;
                    }
                }
                SimulatorEvent::Shutdown => return DaliSendResult::DriverError("Shutdown".into()),
            }

            if flags.expect_answer() {
                match self
                    .driver_task
                    .wait_until(self.driver_task.current_time() + Duration::from_millis(50))
                    .await
                {
                    SimulatorEvent::Timeout => return DaliSendResult::Timeout,
                    SimulatorEvent::Message(msg) => {
                        if let Some(bus_event) = msg.downcast_ref::<DaliSimBusEvent>() {
                            if let DaliBusEventType::Frame8(data) = bus_event.event_type {
                                return DaliSendResult::Answer(data);
                            } else {
                                return DaliSendResult::Framing;
                            }
                        } else {
                            return DaliSendResult::DriverError("Unexpected message".into());
                        }
                    }
                    SimulatorEvent::Shutdown => {
                        return DaliSendResult::DriverError("Shutdown".into());
                    }
                }
            } else {
                match self
                    .driver_task
                    .wait_until(self.driver_task.current_time() + timing::STOP_CONDITION)
                    .await
                {
                    SimulatorEvent::Timeout => return DaliSendResult::Ok,
                    SimulatorEvent::Message(_msg) => return DaliSendResult::Framing,
                    SimulatorEvent::Shutdown => {
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
        self.driver_task.current_time()
    }

    fn wait_until(&self, end: Instant) -> DynFuture<'_, ()> {
        Box::pin(self.driver_task.wait_until(end).map(|_| ()))
    }
}

fn driver_open(params: HashMap<String, String>) -> Result<Box<dyn DaliDriver>, OpenError> {
    let conf_file = params
        .get("config")
        .map(|s| s.as_str())
        .unwrap_or("sim.xml");

    let driver = DaliSimDriver::new();
    Ok(Box::new(driver))
}

pub fn driver_info() -> DriverInfo {
    DriverInfo {
        name: "SIM".to_string(),
        description: "Simulated devices".to_string(),
        open: driver_open,
    }
}
