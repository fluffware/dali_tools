use dali_tools as dali;
use dali::drivers::helvar::helvar510::Helvar510driver;
use dali::base::address::{Short};
use dali::utils::device_info;
use dali::utils::memory_banks ;
#[macro_use]
extern crate clap;

#[tokio::main]
async fn main() {
      let matches = 
        clap_app!(swap_addr =>
                  (about: "Query one or more DALI gears for various information.")
                  (@arg ADDR: +required "Address")
                  (@arg memory_banks: -m --("memory-banks") 
                   "Read information from memory banks")
      ).get_matches();
    
    let addr = match u8::from_str_radix(matches.value_of("ADDR").unwrap(),10){
        Ok(x) if x >= 1 && x <= 64 => Short::new(x),
        Ok(_) => {
            println!("Address out of range");
            return
        }
        Err(e) => {
            println!("Address invalid: {}",e);
            return
        }
    };
    let read_memory = matches.is_present("memory_banks");
    let driver = &mut Helvar510driver::new();

    let info =
        match device_info::read_device_info(driver, addr).await {
            Ok(i) => i,
            Err(e) => {
                println!("Failed to read device info: {}", e);
                return;
            }
        };
    println!("{}", info);
    if read_memory {
        match memory_banks::read_bank_0(driver, addr, 0, 0, 0x18).await {
            Ok(data) => println!("{}", data),
            Err(e) => {
                println!("Failed to read memory banks: {}", e);
                return;
            }
        }
    }
        
}
