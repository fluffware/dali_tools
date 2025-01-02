use crate::drivers;
#[cfg(feature = "dali_rpi_driver")]
use drivers::dali_rpi::dali_rpi;
#[cfg(feature = "dgw521_driver")]
use drivers::dgw521::dgw521;
#[allow(unused_imports)] // In case no drivers are enabled
use drivers::driver::add_driver;
#[cfg(feature = "helvar510_driver")]
use drivers::helvar::helvar510;
#[cfg(feature = "pru_driver")]
use drivers::pru::pru_driver;

pub fn init() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    #[cfg(feature = "helvar510_driver")]
    add_driver(helvar510::driver_info());
    #[cfg(feature = "dgw521_driver")]
    add_driver(dgw521::driver_info());
    #[cfg(feature = "pru_driver")]
    add_driver(pru_driver::driver_info());
    #[cfg(feature = "dali_rpi_driver")]
    add_driver(dali_rpi::driver_info());
    Ok(())
}
