extern crate tokio;
extern crate futures;
#[macro_use]
extern crate lazy_static;
pub mod base {
    pub mod address;
    pub mod status;
    pub mod device_type;
}

pub mod utils {
    pub mod discover;
    pub mod long_address;
    pub mod device_info;
    pub mod memory_banks;
    pub mod decode;
}

pub mod drivers {
    pub mod driver;
    pub mod driver_init;
    pub use driver_init::init as init;
    pub use driver::open as open;
    pub use driver::driver_names as driver_names;
    pub mod utils;
    pub mod command_utils;
    pub mod monitor;
    #[cfg(feature = "helvar510_driver")]
    pub mod helvar {
        pub mod helvar510;
        mod idle_future;
    }
    #[cfg(feature = "simulator")]
    pub mod simulator {
        pub mod simulator;
        pub mod device;
        pub mod gear;
        #[cfg(test)]
        mod test;
    }
    #[cfg(feature = "pru_driver")]
    pub mod pru;
}

pub mod defs {
    pub mod common;
    pub mod gear {
        pub mod cmd;
        pub mod status;
        pub mod device_type;
        pub mod light_source;
    }
}
