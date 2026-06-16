use super::sim_scheduler::{SimulatorEvent, SimulatorMessageDest, SimulatorTask, SimulatorTaskId};
use crate::drivers::driver::DaliBusEventType;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;

#[derive(Clone)]
pub struct DaliSimBusEvent {
    pub source_id: SimulatorTaskId,
    pub start: Option<Instant>, // Time of first transition of frame.  Only for frames
    pub end: Instant,           // Time of last transition.
    pub event_type: DaliBusEventType,
}

/*
static NEXT_SOURCE_ID: AtomicU32 = AtomicU32::new(1);
fn get_next_source_id() -> u32 {
    NEXT_SOURCE_ID.fetch_add(1, Ordering::Relaxed)
}
*/
pub struct DaliSimBus {
    events: RwLock<Vec<DaliSimBusEvent>>,
}

async fn bus_task(task: Box<dyn SimulatorTask>, bus: Arc<DaliSimBus>) {
    loop {
        if let Some(event) = bus.events.read().unwrap().last() {
            let end_time = event.end;
            match task.wait_until(end_time).await {
                SimulatorEvent::Timeout => {
                    while let Some(event) =
                        bus.events.write().unwrap().pop_if(|ev| ev.end <= end_time)
                    {
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
}

impl DaliSimBus {
    pub fn new(task: Box<dyn SimulatorTask>) -> Arc<DaliSimBus> {
        Arc::new(DaliSimBus {
            events: RwLock::new(Vec::new()),
        })
    }

    pub fn add_avent(self: &Arc<DaliSimBus>, new_event: DaliSimBusEvent) {
        use DaliBusEventType::*;
        let mut new_event = new_event;
        match new_event.event_type {
            Frame8(_) | Frame16(_) | Frame24(_) | Frame25(_) | FramingError => {
                let mut events = self.events.write().unwrap();
                if let Some(start) = new_event.start {
                    let first = events.partition_point(|ev| start > ev.end);

                    let mut last = first;
                    while last < events.len() {
                        if let Some(event_start) = events[last].start {
                            if event_start > new_event.end {
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
                        if events[last - 1].end > new_event.end {
                            new_event.end = events[last - 1].end;
                        }
                        new_event.event_type = DaliBusEventType::FramingError;
                        events.drain(first + 1..last);
                        events[first] = new_event;
                    } else {
                        events.insert(first, new_event);
                    }
                } else {
                    let mut events = self.events.write().unwrap();
                    let pos = events.partition_point(|ev| new_event.end > ev.end);
                    events.insert(pos, new_event);
                }
            }
            _ => {}
        }
    }
}
