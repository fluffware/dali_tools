//pub mod device;
//pub mod simulator;
//pub mod simulator_bus;
pub mod sim_bus;
//pub mod simulator_driver;
pub mod timing;
//pub mod gear;
pub mod sim_scheduler;
pub mod sim_scheduler_impl;
pub mod simulator_driver {
    use crate::drivers::driver::{DaliDriver, DriverInfo, OpenError};
    use std::collections::HashMap;
    fn driver_open(_params: HashMap<String, String>) -> Result<Box<dyn DaliDriver>, OpenError> {
        Err(OpenError::NotFound)
    }

    pub fn driver_info() -> DriverInfo {
        DriverInfo {
            name: "SIM".to_string(),
            description: "Simulated devices".to_string(),
            open: driver_open,
        }
    }
}
//#[cfg(test)]
//mod test;
