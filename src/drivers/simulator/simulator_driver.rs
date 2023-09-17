use crate::drivers::driver::{
    DaliBusEvent, DaliBusEventResult, DaliBusEventType, DaliDriver, DaliFrame, DaliSendResult,
};
use crate::drivers::send_flags::Flags;
use crate::drivers::simulator::device::{DaliSimDevice, DaliSimEvent, DaliSimHost};
use crate::drivers::simulator::timing;
use crate::utils::dyn_future::DynFuture;
use std::error::Error;
use std::fmt;
use std::future::{self, Future};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
type DynResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
use std::convert::TryFrom;
use tokio::sync::oneshot;

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

struct PendingResult {
    expect_answer: bool,
    // Send reply back to user
    reply: oneshot::Sender<DaliSendResult>,
    // For request with answers this is when the request times
    // out. For other requests this is when it's done and returns OK
    request_end: Instant,
}

// Data shared by the device and the driver
struct DaliSimDriverCtxt {
    // Queue for events to the simulated  bus
    host: Option<Box<dyn DaliSimHost>>,
    last_transition: Instant,
    // Request waiting for an answer
    pending_result: Option<PendingResult>,
    monitor_reply: Option<oneshot::Sender<DaliBusEvent>>,
    source_id: u32,
    queued_event: Option<DaliBusEvent>,
}

pub struct DaliSimDriverDevice {
    ctxt: Arc<Mutex<DaliSimDriverCtxt>>,
}

impl DaliSimDevice for DaliSimDriverDevice {
    fn start(
        &mut self,
        mut host: Box<dyn DaliSimHost>,
    ) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>> {
        if let Ok(mut ctxt) = self.ctxt.lock() {
            ctxt.source_id = host.next_source_id();
            ctxt.host = Some(host);
        }
        Box::pin(future::ready(Ok(())))
    }

    fn stop(&mut self) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>> {
        Box::pin(future::ready(Ok(())))
    }

    fn event(&mut self, event: &DaliSimEvent) -> Option<DaliSimEvent> {
        let mut ctxt = match self.ctxt.lock() {
            Ok(ctxt) => ctxt,
            Err(_) => return None,
        };

        // Calculate the last transition of the frame
        if let Ok(frame) = DaliFrame::try_from(&event.event_type) {
            ctxt.last_transition = event.timestamp + timing::frame_duration(&frame);
        }
        // End the request if the end time is past
        if let Some(pending_result) = ctxt.pending_result.take() {
            if pending_result.request_end <= event.timestamp {
                pending_result
                    .reply
                    .send(if pending_result.expect_answer {
                        DaliSendResult::Timeout
                    } else {
                        DaliSendResult::Ok
                    })
                    .unwrap_or(());
            }
        }
        // Ignore events sent by this driver
        if event.source_id == ctxt.source_id {
            return None;
        }
        let mut sent_answer = false;
        match event {
            DaliSimEvent {
                event_type: DaliBusEventType::Frame8(answer),
                ..
            } => {
                if let Some(pending_result) = ctxt.pending_result.take() {
                    pending_result
                        .reply
                        .send(DaliSendResult::Answer(*answer))
                        .unwrap_or(());
                    sent_answer = true;
                }
            }
            DaliSimEvent {
                event_type: DaliBusEventType::FramingError,
                ..
            } => {
                if let Some(pending_result) = ctxt.pending_result.take() {
                    pending_result
                        .reply
                        .send(DaliSendResult::Framing)
                        .unwrap_or(());
                    sent_answer = true;
                }
            }
            _ => {}
        };

        /* We expected an answer but there was some other event instead.
        Treat this as a timeout since no acceptable answer was received. */
        if let Some(pending_result) = ctxt.pending_result.take() {
            pending_result
                .reply
                .send(DaliSendResult::Timeout)
                .unwrap_or(());
        }

        // If no answer was sent then this is an unrelated frame
        if !sent_answer {
            let DaliSimEvent {
                timestamp,
                event_type,
                ..
            } = event;
            let bus_event = DaliBusEvent {
                timestamp: *timestamp,
                event_type: event_type.clone(),
            };
            if let Some(monitor_reply) = ctxt.monitor_reply.take() {
                monitor_reply.send(bus_event).unwrap_or(());
            } else {
                if let Some(old_event) = &ctxt.queued_event {
                    ctxt.queued_event = Some(DaliBusEvent {
                        timestamp: old_event.timestamp,
                        event_type: DaliBusEventType::Overrun,
                    });
                } else {
                    ctxt.queued_event = Some(bus_event);
                }
            }
        }

        None
    }
}

pub struct DaliSimDriver {
    ctxt: Arc<Mutex<DaliSimDriverCtxt>>,
}

impl DaliSimDriver {
    pub fn new() -> (DaliSimDriver, Box<DaliSimDriverDevice>) {
        let now = Instant::now();
        let ctxt = DaliSimDriverCtxt {
            // Queue for events to the simulated  bus
            host: None,
            last_transition: now,
            // Request waiting for an answer
            pending_result: None,
            monitor_reply: None,
            queued_event: None,
            source_id: 0,
        };
        let ctxt1 = Arc::new(Mutex::new(ctxt));
        let ctxt2 = ctxt1.clone();

        (
            DaliSimDriver { ctxt: ctxt1 },
            Box::new(DaliSimDriverDevice { ctxt: ctxt2 }),
        )
    }
}

impl DaliDriver for DaliSimDriver {
    fn send_frame(
        &mut self,
        cmd: DaliFrame,
        flags: Flags,
    ) -> Pin<Box<dyn Future<Output = DaliSendResult> + Send>> {
        let sim_event;
        let sim_event2;
        let answer_recv;
        let mut host: Box<dyn DaliSimHost>;
        if let Ok(ref mut ctxt) = &mut self.ctxt.lock() {
            if let Some(h) = &ctxt.host {
                host = h.clone_box();
            } else {
                return Box::pin(future::ready(DaliSendResult::DriverError(
                    "No host for device".into(),
                )));
            }
            let frame_dur = timing::frame_duration(&cmd);
            sim_event = DaliSimEvent {
                source_id: ctxt.source_id,
                timestamp: host.current_time(),
                event_type: DaliBusEventType::from(cmd),
            };
            sim_event2 = if flags.send_twice() {
                Some({
                    let mut ev = sim_event.clone();
                    ev.timestamp = sim_event.timestamp + frame_dur + Duration::from_micros(13500);
                    ev
                })
            } else {
                None
            };

            let (send, recv) = oneshot::channel();
            ctxt.pending_result = Some(PendingResult {
                reply: send,
                expect_answer: true,
                request_end: host.current_time() + frame_dur,
            });
            answer_recv = Some(recv);
        } else {
            return Box::pin(future::ready(DaliSendResult::DriverError(
                "Context lock failed".into(),
            )));
        }

        Box::pin(async move {
            if host.send_event(sim_event).await.is_err() {
                return DaliSendResult::DriverError("Sending to queue failed".into());
            }
            if let Some(sim_event) = sim_event2 {
                tokio::time::sleep_until(tokio::time::Instant::from_std(sim_event.timestamp)).await;
                if host.send_event(sim_event).await.is_err() {
                    return DaliSendResult::DriverError("Sending to queue failed".into());
                }
            }
            if let Some(answer_recv) = answer_recv {
                match tokio::time::timeout(Duration::from_millis(50), answer_recv).await {
                    Ok(Ok(res)) => res,
                    Ok(Err(_)) => DaliSendResult::DriverError("No answer was queued".into()),
                    Err(_) => DaliSendResult::Timeout,
                }
            } else {
                DaliSendResult::Ok
            }
        })
    }

    fn next_bus_event(&mut self) -> DynFuture<DaliBusEventResult> {
        Box::pin(future::ready(Err("Not implemented".into())))
    }

    fn current_timestamp(&self) -> Instant {
        Instant::now()
    }

    fn wait_until(&self, _end: Instant) -> DynFuture<()> {
        Box::pin(future::ready(()))
    }
}
