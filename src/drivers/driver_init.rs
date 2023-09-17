use crate::drivers;
#[allow(unused_imports)] // In case no drivers are enabled
use drivers::driver::add_driver;
#[cfg(feature = "helvar510_driver")]
use drivers::helvar::helvar510;
#[cfg(feature = "pru_driver")]
use drivers::pru::pru_driver;

pub fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "helvar510_driver")]
    add_driver(helvar510::driver_info());
    #[cfg(feature = "pru_driver")]
    add_driver(pru_driver::driver_info());
    Ok(())
}
