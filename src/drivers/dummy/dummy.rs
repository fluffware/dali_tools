use crate::drivers;
use crate::utils::dyn_future::DynFuture;
use drivers::driver::{
    DaliBusEventResult, DaliDriver, DaliFrame, DaliSendResult,
    DriverInfo, OpenError,
};
use drivers::send_flags::Flags;
use std::collections::HashMap;
use std::pin::Pin;
use std::time::{Duration, Instant};

pub struct DummyDriver;
impl DaliDriver for DummyDriver {
    fn send_frame(
        &mut self,
        _cmd: DaliFrame,
        flags: Flags,
    ) -> Pin<Box<dyn Future<Output = DaliSendResult> + Send>> {
        Box::pin(async move {
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

    fn next_bus_event(&mut self) -> DynFuture<DaliBusEventResult> {
        Box::pin( std::future::pending() )
    }

    fn current_timestamp(&self) -> std::time::Instant {
        Instant::now()
    }

    fn wait_until(&self, end: std::time::Instant) -> DynFuture<()> {
        Box::pin(tokio::time::sleep_until(end.into()))
    }
}

fn driver_open(_params: HashMap<String, String>) -> Result<Box<dyn DaliDriver>, OpenError> {
    Ok(Box::new(DummyDriver))
}
pub fn driver_info() -> DriverInfo {
    DriverInfo {
        name: "DUMMY".to_string(),
        description: "Dummy driver. Emulates an empty bus.".to_string(),
        open: driver_open,
    }
}
