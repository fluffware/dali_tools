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

pub mod drivers;


pub mod defs {
    pub mod common;
    pub mod gear {
        pub mod cmd;
        pub mod status;
        pub mod device_type;
        pub mod light_source;
    }
}
