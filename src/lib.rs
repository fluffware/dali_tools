extern crate tokio;
extern crate futures;

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
    pub mod utils;
    pub mod monitor;
    pub mod helvar {
        pub mod helvar510;
        mod idle_future;
    }
    pub mod simulator {
        pub mod simulator;
        pub mod device;
        pub mod gear;
        #[cfg(test)]
        mod test;
    }
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
