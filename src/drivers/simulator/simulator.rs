use crate::drivers::driver::{DaliBusEventType,DaliFrame};
use std::sync::Mutex;
use std::sync::Arc;
use tokio::sync::mpsc;
use super::device::{DaliSimEvent, DaliSimDevice, DaliSimHost};
use std::collections::BinaryHeap;
use std::time::Instant;
use tokio::time::timeout_at;
use super::timing;
use std::convert::TryFrom;
use std::sync::atomic::{AtomicU32, Ordering};
use std::pin::Pin;
use std::future::Future;

type DynResult<T> =  Result<T, Box<dyn std::error::Error + Send + Sync>>;
struct TimeOrderedEvent(DaliSimEvent);


impl std::cmp::PartialEq for TimeOrderedEvent
{
    fn eq(&self, other: &Self) -> bool
    {
	self.0.timestamp == other.0.timestamp
    }
}
impl std::cmp::Eq for TimeOrderedEvent {}

impl std::cmp::Ord for TimeOrderedEvent
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering
    {
	self.0.timestamp.cmp(&other.0.timestamp).reverse()
    }
}
impl std::cmp::PartialOrd for TimeOrderedEvent
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering>
    {
	Some(self.cmp(other))
    }
}

#[derive(Clone)]
struct DaliSimDeviceHost
{
    engine: Arc<Mutex<DaliBusSimEngine>>,
    send_event: mpsc::Sender<DaliSimEvent>
}

static NEXT_SOURCE_ID: AtomicU32 = AtomicU32::new(1);
fn get_next_source_id() -> u32
{
    NEXT_SOURCE_ID.fetch_add(1, Ordering::Relaxed)
}

impl DaliSimHost for DaliSimDeviceHost
{
    fn send_event(&mut self, event: DaliSimEvent) 
		  ->  Pin<Box<dyn Future<Output = DynResult<()>> + Send>>
    {
	let send_event = self.send_event.clone();
	Box::pin(async move {
	    match send_event.send(event).await {
		Ok(r) => Ok(r),
		Err(e) => Err(Box::new(e).into())
	    }
	})
    }
    
    fn current_time(&self) -> Instant
    {
	if let Ok(engine) = self.engine.lock() {
	    if engine.real_time {
		Instant::now()
	    } else {
		engine.last_event_time
	    }
	} else {
	    Instant::now()
	}
    }
    
    fn real_time(&self) -> bool
    {
	if let Ok(engine) = self.engine.lock() {
	    engine.real_time
	} else {
	    true
	}
    }
    
    fn next_source_id(&mut self) -> u32
    {
	get_next_source_id()
    }

    fn clone_box(&self) -> Box<dyn DaliSimHost>
    {
	Box::new(DaliSimDeviceHost{engine: self.engine.clone(),
				   send_event: self.send_event.clone()})
    }
}

struct DaliBusSimEngine
{
    devices: Vec<Box<dyn DaliSimDevice + Send>>,
    events: BinaryHeap<TimeOrderedEvent>,
    /* If true then dispatch events at the system time indicated by
    the timestamp, otherwise dispatch events as soon as possible. */
    real_time: bool,
    last_event_time: Instant
}
/// Add an event to the queue of events waiting for dispatch
fn push_event(events: &mut BinaryHeap<TimeOrderedEvent>, event: DaliSimEvent)
{
    events.push(TimeOrderedEvent(event))
}

fn get_next_event(events: &mut BinaryHeap<TimeOrderedEvent>) 
		  -> Option<DaliSimEvent>
{
    match events.pop() {
	Some(TimeOrderedEvent(event)) => {
	    let DaliSimEvent{event_type, timestamp, source_id} = &event;
	    if let Ok(frame) = DaliFrame::try_from(event_type) {
		// Check if frame overlaps next
		match events.peek() {
		    Some(TimeOrderedEvent(DaliSimEvent{
			timestamp: next_timestamp, ..})) => {
			let frame_dur = timing::frame_duration(&frame);
			if *timestamp + frame_dur >= *next_timestamp {
			    Some(DaliSimEvent{
				event_type: DaliBusEventType::FramingError, 
				timestamp: timestamp.clone(), 
				source_id: *source_id})
			} else {
			    Some(event)
			}
		    },
		    None => Some(event)
		}
	    } else {
		Some(event)
	    }
	},
	None => None
    }
}

async fn dispatch_event(bus_arc: Arc<Mutex<DaliBusSimEngine>>,
			mut event_recv: mpsc::Receiver<DaliSimEvent>)
{
    let real_time = match bus_arc.lock() {
	Ok(bus) => bus.real_time,
	Err(_) => return
    };
    loop {

	// Get the time of the next pending event
	let next_timeout = match bus_arc.lock() {
	    Ok(bus) => {
		match bus.events.peek() {
		    Some(TimeOrderedEvent(DaliSimEvent{timestamp, ..})) =>
			Some(timestamp.clone()),
		    None => None
		}
	    },
	    Err(_) => return
	};
	// Dispatch event immediately if the time stamp is in the past
	if let Some(timeout) = next_timeout {
	    if real_time || timeout <= Instant::now() {
		match bus_arc.lock() {
		    Ok(mut bus) => {
			if let Some(event) = 
			    get_next_event(&mut bus.events)
			{
			    let DaliBusSimEngine{events, devices, ..} = &mut *bus;
			    eprintln!("Dispatching event: {:?}", event);
			    for dev in devices.iter_mut() {
				if let Some(new_event) = dev.event(&event) {
				    push_event(events, new_event)
				}
			    }
			}
			bus.last_event_time = timeout;
		    },
		    Err(_) => return
		};
		continue;
	    }
	}

	// Wait for a new event or until it's time to dispatch a queued event
	let event = 
	    if let Some(timeout) = next_timeout {
		if real_time {
		    match timeout_at(tokio::time::Instant::from_std(timeout),
				     event_recv.recv()).await {
			Ok(res) => {
			    match res {
				Some(event) => Some(event),
				None => return
			    }
			},
			Err(_) => None
		    }
		} else {
		   match  event_recv.recv().await {
			Some(event) => Some(event),
			None => return
		    }
		}
	    } else {
		match event_recv.recv().await {
		    Some(event) => Some(event),
		    None => return
		}
	    };
	// If there was a new event then add it to the queue
	if let Some(event) = event {
	    match bus_arc.lock() {
		Ok(mut bus) => {
		    bus.events.push(TimeOrderedEvent(event));
		},
		Err(_) => return
	    }
	}
    }
    
}
			
pub struct DaliBusSim
{
    bus_arc: Arc<Mutex<DaliBusSimEngine>>,
    send_event: mpsc::Sender<DaliSimEvent>
}

impl DaliBusSim
{
    pub async fn new() -> 
	Result<DaliBusSim, Box<dyn std::error::Error + Send + Sync>>
    {
	let (send_event, recv_event) = mpsc::channel(10);
	let bus = DaliBusSimEngine {
	    devices: Vec::new(),
	    events: BinaryHeap::new(),
	    real_time: true,
	    last_event_time: Instant::now(),
	};
	let bus_arc = Arc::new(Mutex::new(bus));
	let dispatch_bus = bus_arc.clone();
	tokio::spawn(dispatch_event(dispatch_bus, recv_event));
	Ok(DaliBusSim {
	    bus_arc,
	    send_event
	})
    }

    pub async fn add_device(&self, mut device: Box<dyn DaliSimDevice + Send>) ->
	Result<(), Box<dyn std::error::Error + Send + Sync>>
    {
	let dev_host = DaliSimDeviceHost {
	    engine: self.bus_arc.clone(),
	    send_event: self.send_event.clone()
	};
	device.start(Box::new(dev_host)).await?;
	if let Ok(mut bus) = self.bus_arc.lock() {
	    bus.devices.push(device);
	}
	Ok(())
    }

    pub async fn add_event(&self, event: DaliSimEvent) -> 
	Result<(), Box<dyn std::error::Error + Send + Sync>>
    {
	self.send_event.send(event).await?;
	Ok(())
    }
}
