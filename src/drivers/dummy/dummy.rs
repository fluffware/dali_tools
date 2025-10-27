use crate::drivers;
use crate::utils::dyn_future::DynFuture;
use drivers::driver::{
    DaliBusEventResult, DaliDriver, DaliFrame, DaliSendResult, DriverInfo, OpenError,
};
use drivers::send_flags::Flags;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct DummyDriver {
    log: Option<Arc<Mutex<dyn Write + Send>>>,
}

impl DaliDriver for DummyDriver {
    fn send_frame(
        &mut self,
        cmd: DaliFrame,
        flags: Flags,
    ) -> Pin<Box<dyn Future<Output = DaliSendResult> + Send>> {
        let log = self.log.clone();
        Box::pin(async move {
            if let Some(f) = log {
                let mut buf = String::new();
                if flags.send_twice() {
                    buf += "T ";
                } else if flags.expect_answer() {
                    buf += "A ";
                } else {
                    buf += "  ";
                }
                match cmd {
                    DaliFrame::Frame24(frame) => {
                        buf += &format!("{:02x} {:02x} {:02x}", frame[0], frame[1], frame[2])
                    }
                    DaliFrame::Frame16(frame) => {
                        buf += &format!("{:02x} {:02x}", frame[0], frame[1])
                    }
                    DaliFrame::Frame8(frame) => buf += &format!("{:02x}", frame),
                    DaliFrame::Frame25(frame) => {
                        buf += &format!(
                            "{:02x} {:02x} {:02x} {:1x}",
                            frame[0], frame[1], frame[2], frame[3]
                        )
                    }
                }
                buf += "\n";
                let mut f = f.lock().unwrap();
                if let Err(e) = f.write(buf.as_bytes()) {
                    return DaliSendResult::DriverError(e.into());
                }
            }
            if flags.send_twice() {
                tokio::time::sleep(Duration::from_millis(19)).await;
                DaliSendResult::Ok
            } else if flags.expect_answer() {
                tokio::time::sleep(Duration::from_millis(11)).await;
                DaliSendResult::Timeout
            } else {
                tokio::time::sleep(Duration::from_millis(9)).await;
                DaliSendResult::Ok
            }
        })
    }

    fn next_bus_event(&mut self) -> DynFuture<'_, DaliBusEventResult> {
        Box::pin(std::future::pending())
    }

    fn current_timestamp(&self) -> std::time::Instant {
        Instant::now()
    }

    fn wait_until(&self, end: std::time::Instant) -> DynFuture<'_, ()> {
        Box::pin(tokio::time::sleep_until(end.into()))
    }
}

fn driver_open(params: HashMap<String, String>) -> Result<Box<dyn DaliDriver>, OpenError> {
    let mut log: Option<Arc<Mutex<dyn Write + Send>>> = None;
    if let Some(filename) = params.get("log") {
        if filename == "-" {
            log = Some(Arc::new(Mutex::new(std::io::stdout())));
        } else {
            match File::create(filename) {
                Ok(f) => log = Some(Arc::new(Mutex::new(f))),
                Err(e) => {
                    return Err(OpenError::ParameterError(format!(
                        "Failed to log open file {}: {}",
                        filename, e
                    )));
                }
            }
        }
    }
    Ok(Box::new(DummyDriver { log }))
}
pub fn driver_info() -> DriverInfo {
    DriverInfo {
        name: "DUMMY".to_string(),
        description: "Dummy driver. Emulates an empty bus.".to_string(),
        open: driver_open,
    }
}
