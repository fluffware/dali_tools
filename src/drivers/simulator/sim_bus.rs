use super::sim_scheduler::{SimulatorEvent, SimulatorMessageDest, SimulatorTask, SimulatorTaskId};
use crate::drivers::driver::DaliBusEventType;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;

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
        println!("Bus loop");

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
