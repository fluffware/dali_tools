extern crate futures;
extern crate tokio;
#[macro_use]
extern crate lazy_static;
pub mod base {
    pub mod address;
    pub mod device_type;
    pub mod status;
}

pub mod utils {
    pub mod decode;
    pub mod device_info;
    pub mod discover;
    pub mod dyn_future;
    pub mod long_address;
    pub mod memory_banks;
}

pub mod drivers;

pub mod defs {
    pub mod common;
    pub mod gear {
        pub mod cmd;
        pub mod device_type;
        pub mod light_source;
        pub mod status;
    }
}
