use super::sim_scheduler::{SimulatorEvent, SimulatorMessageDest, SimulatorTask, SimulatorTaskId};
use crate::drivers::driver::{DaliBusEventType, DaliFrame, DaliSendResult};
use crate::drivers::send_flags::Flags;
use crate::simulator::timing;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct DaliSimBusEvent {
    pub source_id: SimulatorTaskId,
    pub start: Option<Instant>, // Time of first transition of frame.  Only for frames
    pub timestamp: Instant,     // Time of last transition of frame or time of other event
    pub event_type: DaliBusEventType,
}

/*
static NEXT_SOURCE_ID: AtomicU32 = AtomicU32::new(1);
fn get_next_source_id() -> u32 {
    NEXT_SOURCE_ID.fetch_add(1, Ordering::Relaxed)
}
*/
pub struct DaliSimBus {
    events: Arc<RwLock<Vec<DaliSimBusEvent>>>,
    task: Arc<dyn SimulatorTask + Send + Sync>,
}

async fn bus_task(
    task: Arc<dyn SimulatorTask + Send + Sync>,
    events: Arc<RwLock<Vec<DaliSimBusEvent>>>,
) {
    loop {
        let wait_event;
        {
            let events = events.read().unwrap();
            if let Some(event) = events.last() {
                let end_time = event.timestamp;
                wait_event = task.wait_until(end_time);
            } else {
                wait_event = task.wait();
            };
        }

        match wait_event.await {
            SimulatorEvent::Timeout => {
                let now = task.current_time();
                while let Some(event) = events.write().unwrap().pop_if(|ev| ev.timestamp <= now) {
                    task.send_msg(
                        SimulatorMessageDest::Exclude(task.task_id()),
                        Arc::new(event),
                    );
                }
            }
            SimulatorEvent::Message(_) => {}
            SimulatorEvent::Shutdown => break,
        }
    }
}

impl DaliSimBus {
    pub fn new(task: Box<dyn SimulatorTask + Send + Sync>) -> Arc<DaliSimBus> {
        let task: Arc<dyn SimulatorTask + Send + Sync> = Arc::from(task);
        let events = Arc::new(RwLock::new(Vec::new()));
        tokio::spawn(bus_task(task.clone(), events.clone()));
        let bus = Arc::new(DaliSimBus { events, task });
        bus
    }

    pub fn add_event(self: &Arc<DaliSimBus>, new_event: DaliSimBusEvent) {
        use DaliBusEventType::*;
        let mut new_event = new_event;
        match new_event.event_type {
            Frame8(_) | Frame16(_) | Frame24(_) | Frame25(_) | FramingError => {
                let mut events = self.events.write().unwrap();
                if let Some(start) = new_event.start {
                    let first = events.partition_point(|ev| start > ev.timestamp);

                    let mut last = first;
                    while last < events.len() {
                        if let Some(event_start) = events[last].start {
                            if event_start > new_event.timestamp {
                                break;
                            }
                        }
                        last += 1;
                    }
                    if last > first {
                        // Collision
                        if let Some(event_start) = events[first].start {
                            if event_start < start {
                                new_event.start = Some(event_start);
                            }
                        }
                        if events[last - 1].timestamp > new_event.timestamp {
                            new_event.timestamp = events[last - 1].timestamp;
                        }
                        new_event.event_type = DaliBusEventType::FramingError;
                        events.drain(first + 1..last);
                        events[first] = new_event;
                    } else {
                        events.insert(first, new_event);
                    }
                } else {
                    let mut events = self.events.write().unwrap();
                    let pos = events.partition_point(|ev| new_event.timestamp > ev.timestamp);
                    events.insert(pos, new_event);
                }
            }
            _ => {}
        }
        self.task.send_msg(
            SimulatorMessageDest::Task(self.task.task_id()),
            Arc::new(()),
        );
    }
}

impl Drop for DaliSimBus {
    fn drop(&mut self) {
        self.task.shutdown();
    }
}

#[derive(Debug)]
pub enum DaliSimBusDeviceEvent {
    Timeout,
    Shutdown,
    Message(DaliSimBusEvent),
}

pub struct DaliSimBusDevice {
    task: Box<dyn SimulatorTask + Send + Sync>,
    bus: Arc<DaliSimBus>,
}

impl DaliSimBusDevice {
    pub fn new(
        bus: Arc<DaliSimBus>,
        task: Box<dyn SimulatorTask + Send + Sync>,
    ) -> DaliSimBusDevice {
        DaliSimBusDevice { task, bus }
    }
    pub fn add_event(
        &self,
        event_type: DaliBusEventType,
        timestamp: Instant,
        start: Option<Instant>,
    ) {
        self.bus.add_event(DaliSimBusEvent {
            source_id: self.task.task_id(),
            event_type,
            timestamp,
            start,
        });
    }

    pub fn current_time(&self) -> Instant {
        self.task.current_time()
    }

    pub async fn wait_until(&self, when: Instant) -> DaliSimBusDeviceEvent {
        loop {
            match self.task.wait_until(when).await {
                SimulatorEvent::Timeout => return DaliSimBusDeviceEvent::Timeout,
                SimulatorEvent::Message(msg) => {
                    if let Some(bus_event) = msg.downcast_ref::<DaliSimBusEvent>()
                        && bus_event.source_id != self.task.task_id()
                    {
                        return DaliSimBusDeviceEvent::Message(bus_event.clone());
                    }
                }
                SimulatorEvent::Shutdown => return DaliSimBusDeviceEvent::Shutdown,
            }
        }
    }
    pub async fn wait(&self) -> DaliSimBusDeviceEvent {
        loop {
            match self.task.wait().await {
                SimulatorEvent::Timeout => panic!("Timout received in wait"),
                SimulatorEvent::Message(msg) => {
                    if let Some(bus_event) = msg.downcast_ref::<DaliSimBusEvent>()
                        && bus_event.source_id != self.task.task_id()
                    {
                        return DaliSimBusDeviceEvent::Message(bus_event.clone());
                    }
                }
                SimulatorEvent::Shutdown => return DaliSimBusDeviceEvent::Shutdown,
            }
        }
    }

    pub fn send_frame<'a>(
        &'a mut self,
        cmd: DaliFrame,
        flags: Flags,
    ) -> Pin<Box<dyn Future<Output = DaliSendResult> + Send + 'a>> {
        Box::pin(async move {
            let mut frame_start = self.current_time() + timing::send_delay(flags.priority(), false);
            loop {
                match self.wait_until(frame_start).await {
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

            self.add_event(
                DaliBusEventType::from(cmd.clone()),
                frame_end,
                Some(frame_start),
            );
            match self.wait_until(frame_end).await {
                DaliSimBusDeviceEvent::Timeout => {}
                DaliSimBusDeviceEvent::Message(_bus_event) => {
                    return DaliSendResult::Framing;
                }
                DaliSimBusDeviceEvent::Shutdown => {
                    return DaliSendResult::DriverError("Shutdown".into());
                }
            }
            if flags.send_twice() {
                let frame_start = self.current_time() + Duration::from_micros(13500);
                frame_end = frame_start + timing::frame_duration(&cmd);
                match self.wait_until(frame_start).await {
                    DaliSimBusDeviceEvent::Timeout => {}
                    DaliSimBusDeviceEvent::Message(_msg) => {
                        return DaliSendResult::Framing;
                    }
                    DaliSimBusDeviceEvent::Shutdown => {
                        return DaliSendResult::DriverError("Shutdown".into());
                    }
                }

                self.add_event(
                    DaliBusEventType::from(cmd.clone()),
                    frame_end,
                    Some(frame_start),
                );
            }
            match self.wait_until(frame_end).await {
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
                    .wait_until(self.current_time() + Duration::from_millis(50))
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
                    .wait_until(self.current_time() + timing::STOP_CONDITION)
                    .await
                {
                    DaliSimBusDeviceEvent::Timeout => return DaliSendResult::Ok,
                    DaliSimBusDeviceEvent::Message(_bus_event) => {
                        return DaliSendResult::Framing;
                    }
                    DaliSimBusDeviceEvent::Shutdown => {
                        return DaliSendResult::DriverError("Shutdown".into());
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod test {
    use super::DaliBusEventType;
    use super::DaliSimBus;
    use super::DaliSimBusEvent;
    use crate::drivers::simulator::sim_scheduler::SimulatorEvent;
    use crate::drivers::simulator::sim_scheduler::SimulatorScheduler;
    use crate::drivers::simulator::sim_scheduler::SimulatorTask;
    use crate::drivers::simulator::sim_scheduler_impl::SimulatorSchedulerImpl;
    use std::assert_matches;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    async fn bus_thread_1(
        task: Box<dyn SimulatorTask + Send>,
        bus: Arc<DaliSimBus>,
        when: Instant,
    ) {
        println!("Bus thread started");
        task.wait_until(task.current_time() + Duration::from_millis(50))
            .await;
        bus.add_event(DaliSimBusEvent {
            source_id: task.task_id(),
            start: Some(when - Duration::from_millis(15)),
            timestamp: when,
            event_type: DaliBusEventType::Frame16([0x01, 0x02]),
        });
        println!("Bus thread ended");
    }

    #[tokio::test]
    async fn bus_test() {
        let mut sched = SimulatorSchedulerImpl::new();
        let task = sched.new_task();
        let start = task.current_time();
        let bus = DaliSimBus::new(sched.new_task());
        let th1 = tokio::spawn(bus_thread_1(
            sched.new_task(),
            bus.clone(),
            start + Duration::from_millis(100),
        ));
        let SimulatorEvent::Message(msg) = task.wait().await else {
            panic!("Expected message");
        };
        let Some(event) = msg.downcast_ref::<DaliSimBusEvent>() else {
            panic!("Expected DaliBusEvent");
        };
        assert_eq!(event.timestamp, task.current_time());
        th1.await.unwrap();

        // Collision
        let th1 = tokio::spawn(bus_thread_1(
            sched.new_task(),
            bus.clone(),
            start + Duration::from_millis(1010),
        ));
        let th2 = tokio::spawn(bus_thread_1(
            sched.new_task(),
            bus.clone(),
            start + Duration::from_millis(1000),
        ));
        let SimulatorEvent::Message(msg) = task.wait().await else {
            panic!("Expected message");
        };
        let Some(event) = msg.downcast_ref::<DaliSimBusEvent>() else {
            panic!("Expected DaliBusEvent");
        };
        assert_eq!(event.timestamp, task.current_time());
        assert_eq!(event.timestamp, start + Duration::from_millis(1010),);
        assert_eq!(event.start, Some(start + Duration::from_millis(985)));
        assert_matches!(event.event_type, DaliBusEventType::FramingError);
        drop(sched);
        th1.await.unwrap();
        th2.await.unwrap();
    }
}
