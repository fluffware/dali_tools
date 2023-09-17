use dali::base::address::Short;
use dali::drivers::driver::OpenError;
use dali::utils::device_info;
use dali::utils::memory_banks;
use dali_tools as dali;
#[macro_use]
extern crate clap;

#[tokio::main]
async fn main() {
    if let Err(e) = dali::drivers::init() {
        println!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = clap_app!(swap_addr =>
                (about: "Query one or more DALI gears for various information.")
        (@arg DEVICE: -d --device +takes_value "Select DALI-device")
                (@arg ADDR: +required "Address")
                (@arg memory_banks: -m --("memory-banks")
                 "Read information from memory banks")
    )
    .get_matches();

    let addr = match u8::from_str_radix(matches.value_of("ADDR").unwrap(), 10) {
        Ok(x) if x >= 1 && x <= 64 => Short::new(x),
        Ok(_) => {
            println!("Address out of range");
            return;
        }
        Err(e) => {
            println!("Address invalid: {}", e);
            return;
        }
    };
    let device_name = matches.value_of("DEVICE").unwrap_or("default");
    let read_memory = matches.is_present("memory_banks");
    let mut driver = match dali::drivers::open(device_name) {
        Ok(d) => d,
        Err(e) => {
            println!("Failed to open DALI device: {}", e);
            if let OpenError::NotFound = e {
                println!("Available drivers:");
                for name in dali::drivers::driver_names() {
                    println!("  {}", name);
                }
            }
            return;
        }
    };

    let info = match device_info::read_device_info(&mut *driver, addr).await {
        Ok(i) => i,
        Err(e) => {
            println!("Failed to read device info: {}", e);
            return;
        }
    };
    println!("{}", info);
    if read_memory {
        match memory_banks::read_bank_0(&mut *driver, addr, 0, 0, 0x18).await {
            Ok(data) => println!("{}", data),
            Err(e) => {
                println!("Failed to read memory banks: {}", e);
                return;
            }
        }
    }
}
