extern crate futures;
extern crate tokio;
#[macro_use]
extern crate lazy_static;
pub mod error;


pub mod utils {
    pub mod decode;
    pub mod device_info;
    pub mod discover;
    pub mod dyn_future;
    pub mod filtered_vec;
    pub mod long_address;
    pub mod memory_banks;
    pub mod address_assignment;
}

pub mod drivers;

pub mod common;

pub mod gear;
pub mod control;



#[cfg(feature = "httpd")]
pub mod httpd;
