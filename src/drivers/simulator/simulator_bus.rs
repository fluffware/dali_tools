use crate::drivers::driver::{
    DaliBusEvent, DaliBusEventResult, DaliBusEventType, DaliDriver, DaliFrame, DaliSendResult,
};
use crate::drivers::send_flags::Flags;
use crate::drivers::simulator::timing;
use crate::utils::dyn_future::DynFuture;
use std::borrow::BorrowMut;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex};
use std::time::{Instant, Duration};
use tokio::sync::mpsc::{self, error::TryRecvError};
use tokio::sync::oneshot;
use std::cmp::Reverse;
use tokio::time;





pub struct DaliSimulatorDriver {
    bus: Arc<DaliSimulatorBus>,
    bus_events: mpsc::Receiver<DaliBusEvent>,
    overrun: bool,
}

enum SendState {
    Delay,        // Make sure the interval since last transition is correct depending on priority
    SendingFrame, // Send frame, check for collision
    DelayTwice,   // Wait to send next frame if send twice
    SendingSecondFrame,
    WaitReply,
}

impl DaliDriver for DaliSimulatorDriver {
    fn send_frame(&mut self, cmd: DaliFrame, flags: Flags) -> DynFuture<DaliSendResult> {
        let mut state = self.bus.state.lock().unwrap();
        let mut send_state = SendState::Delay;
        let mut end_frame = state.data.current_timestamp;
        let (sender, recv) = oneshot::channel();
        let mut sender = Some(sender);
        let mut handler: Box<dyn FnMut(&mut DaliSimulatorBusData) -> bool + Send> =
            Box::new(move |state: &mut DaliSimulatorBusData| {
                match send_state {
                    SendState::Delay => {
                        if state.active_frame_count > 0 {
                            return true;
                        }
                        let send_delay =
                            timing::send_delay(flags.priority(), state.random_send_delay);
                        let frame_start = state.last_transition + send_delay;
                        if state.current_timestamp >= frame_start {
                            state.active_frame_count += 1;
                            send_state = SendState::SendingFrame;
                            end_frame = frame_start + timing::frame_duration(&cmd);
                            state.internal_timeout_at(end_frame);
                        } else {
                            state.internal_timeout_at(state.last_transition + send_delay);
                        }
                    }
                    SendState::SendingFrame => {
                        if state.current_timestamp >= end_frame {
                            state.active_frame_count -= 1;
                            if flags.send_twice() {
                                send_state = SendState::DelayTwice;
                            } else {
                                sender.take().unwrap().send(DaliSendResult::Ok).unwrap();
                                return false;
                            }
                        } else {
                        }
                    }
                    _ => {}
                }
                return true;
            });
        if handler(&mut state.data) {
            drop(state);
            self.bus.add_handler(handler);
        }
        Box::pin(async {
            match recv.await {
                Ok(res) => res,
                Err(e) => DaliSendResult::DriverError(e.into()),
            }
        })
    }

    fn next_bus_event(&mut self) -> DynFuture<DaliBusEventResult> {
        Box::pin(async {
            self.bus_events
                .recv()
                .await
                .ok_or_else(|| "Bus not available".into())
        })
    }

    fn current_timestamp(&self) -> Instant {
        self.bus.current_timestamp()
    }

    fn wait_until(&self, end: Instant) -> DynFuture<()> {
        let (sender, recv) = oneshot::channel();
        let mut sender = Some(sender);
        {
            let mut state = self.bus.state.lock().unwrap();
            let handler = Box::new(move |data: &mut DaliSimulatorBusData| {
                if data.current_timestamp >= end {
                    let _ = sender.take().unwrap().send(());
                    return false;
                }
                true
            });
            self.bus.add_handler(handler);
            state.data.internal_timeout_at(end);
        }
        Box::pin(async move {
            let _ = recv.await;
        })
    }
}

struct DaliSimulatorBusData {
    timer_queue: BinaryHeap<Reverse<Instant>>,
    current_bus_event: Option<DaliBusEvent>,
    active_frame_count: u32, // Number of frames being sent, a value greater than 1 means collision
    current_timestamp: Instant,
    last_transition: Instant,
    random_send_delay: bool,
    state_changed: mpsc::Sender<()>,
}

impl DaliSimulatorBusData {
    fn internal_timeout_at(&mut self, t: Instant) {
        self.timer_queue.push(Reverse(t));
        //let _ = self.state_changed.try_send(());
    }

    fn send_bus_event(&mut self, event: DaliBusEvent) -> bool {
        if self.current_bus_event.is_some() {
            return false;
        }
        self.current_bus_event = Some(event);
        let _ = self.state_changed.try_send(());
        true
    }
}

struct DaliSimulatorBusState {
    data: DaliSimulatorBusData,
    state_changed: Vec<Box<dyn FnMut(&mut DaliSimulatorBusData) -> bool + Send>>,
}

async fn bus_engine(bus: Arc<DaliSimulatorBus>, mut recv: mpsc::Receiver<()>) {
    loop {
        
        match recv.try_recv() {
            Ok(event) => {
		println!("Event: {:?}", event);
	    }
            Err(TryRecvError::Empty) => {
		let mut bump_time = false;
                {
                    let mut state = bus.state.lock().unwrap();
		    loop {
			let next_time = match state.data.timer_queue.pop()
			{
			    None => break,
			    Some(t) => t.0
			};
			if state.data.current_timestamp < next_time {
			    state.data.current_timestamp = next_time;
			    bump_time = true ;
			}
		    }
                }
		if !bump_time {
                    if recv.recv().await.is_none() {
			break;
                    }
		}
            }
            Err(TryRecvError::Disconnected) => {
                break;
            }
        }
	{
            let mut state = bus.state.lock().unwrap();
            let mut i = 0;
            while i < state.state_changed.len() {
                let state: &mut DaliSimulatorBusState = state.borrow_mut();
                if state.state_changed[i](&mut state.data) {
                    i += 1;
                } else {
                    let _ = state.state_changed.swap_remove(i);
                }
            }
            state.data.current_bus_event = None;
        }
    }
}

pub struct DaliSimulatorBus {
    state: Mutex<DaliSimulatorBusState>,
}

impl DaliSimulatorBus {
    pub fn new() -> Arc<DaliSimulatorBus> {
        let (state_changed, state_changed_recv) = mpsc::channel(1);
        let now = Instant::now();
        let data = DaliSimulatorBusData {
            timer_queue: BinaryHeap::new(),
            current_bus_event: None,
            active_frame_count: 0,
            current_timestamp: now,
            last_transition: now,
            random_send_delay: false,
            state_changed,
        };
        let bus = Arc::new(DaliSimulatorBus {
            state: Mutex::new(DaliSimulatorBusState {
                data,
                state_changed: Vec::new(),
            }),
        });
	tokio::spawn(bus_engine(bus.clone(), state_changed_recv));
	bus
    }

    fn current_timestamp(self: &Arc<DaliSimulatorBus>) -> Instant {
        self.state.lock().unwrap().data.current_timestamp
    }
    /*
        fn state_changed(&self) {
            let mut state = self.state.lock().unwrap();
            let mut i = 0;
            while state.data.state_changed {
                state.data.state_changed = false;
                while i < state.state_changed.len() {
                    let state: &mut DaliSimulatorBusState = state.borrow_mut();
                    if state.state_changed[i](&mut state.data) {
                        i += 1;
                    } else {
                        let _ = state.state_changed.swap_remove(i);
                    }
                    if state.data.state_changed {
                        break;
                    }
                }
            }
        }
    */

    fn add_handler(
        self: &Arc<DaliSimulatorBus>,
        mut handler: Box<dyn FnMut(&mut DaliSimulatorBusData) -> bool + Send>,
    ) {
        let mut state = self.state.lock().unwrap();
	handler(&mut state.data);
        state.state_changed.push(handler);
	
    }

    pub fn get_driver_instance(self: &Arc<DaliSimulatorBus>) -> Box<dyn DaliDriver> {
        let (tx, rx) = mpsc::channel(5);
        //self.state.lock().unwrap().data.bus_events.push(tx);
        Box::new(DaliSimulatorDriver {
            bus: self.clone(),
            bus_events: rx,
            overrun: false,
        })
    }
}

#[cfg(test)]

#[tokio::test]
async fn test_timer()
{
    let bus = DaliSimulatorBus::new();
    let mut count = 4;
    let start_ts = bus.current_timestamp();
    let end = start_ts + Duration::from_millis(600);
    bus.add_handler(Box::new(move |data| {
	data.internal_timeout_at(end);
	println!("Handler called at {:.3}", (data.current_timestamp-start_ts).as_secs_f32());
	return data.current_timestamp < end;
    }));
    
    time::sleep(Duration::from_secs(2)).await;
}
