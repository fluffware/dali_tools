use crate::drivers::driver::{DaliBusEventType, DaliFrame};
use std::collections::BTreeMap;
use std::ops::Bound;
use std::ops::Range;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct DaliSimBusEvent {
    pub source_id: u32,
    pub start: Option<Instant>, // Time of first transition of frame.  Only for frames
    pub end: Instant,           // Time of last transition.
    pub event_type: DaliBusEventType,
}

static NEXT_SOURCE_ID: AtomicU32 = AtomicU32::new(1);
fn get_next_source_id() -> u32 {
    NEXT_SOURCE_ID.fetch_add(1, Ordering::Relaxed)
}

pub struct DaliSimBus {
    events: RwLock<BTreeMap<Instant, DaliSimBusEvent>>,
}

fn extend_range<T>(range: &mut Range<T>, v: T)
where
    T: Ord,
{
    if v < range.start {
        range.start = v;
    } else if v > range.end {
        range.end = v
    }
}

impl DaliSimBus {
    pub fn new() -> DaliSimBus {
        DaliSimBus {
            events: RwLock::new(BTreeMap::new()),
        }
    }

    pub fn add_avent(&self, event: DaliSimBusEvent) {
        use DaliBusEventType::*;
        let mut collision: Option<Range<Instant>> = None;
        let mut remove = Vec::new();
        match event.event_type {
            Frame8(_) | Frame16(_) | Frame24(_) | Frame25(_) | FramingError => {
                if let Some(start) = event.start {
                    let events = self.events.read().unwrap();
                    let mut matched = events.range((Bound::Included(start), Bound::Unbounded));
                    while let Some(m) = &matched.next() {
                        if let Some(m_start) = m.1.start {
                            if m_start > event.end {
                                break;
                            }
                            if m.1.end >= start {
                                let collision = collision.get_or_insert_with(|| Range {
                                    start,
                                    end: event.end,
                                });
                                extend_range(collision, m_start);
                                extend_range(collision, m.1.end);
                                remove.push(m.0.clone());
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        if let Some(collision) = collision {
            let mut events = self.events.write().unwrap();
            for r in remove {
                events.remove(&r);
            }
            events.insert(
                collision.end,
                DaliSimBusEvent {
                    source_id: 0,
                    start: Some(collision.start),
                    end: collision.end,
                    event_type: DaliBusEventType::FramingError,
                },
            );
        } else {
            self.events.write().unwrap().insert(event.end, event);
        }
    }

    pub fn get_events(&self, after: Instant) -> Vec<DaliSimBusEvent> {
        let events = self.events.read().unwrap();
        let matched = events.range((Bound::Included(after), Bound::Unbounded));
        matched.map(|x| x.1.clone()).collect()
    }
}
