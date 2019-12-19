use dali_tools as dali;
use dali::drivers::helvar::helvar510::Helvar510driver;
use dali::base::address::{Short};
use futures::executor::block_on;
use dali::utils::device_info;
#[macro_use]
extern crate clap;


fn main() {
      let matches = clap_app!(swap_addr =>
                            (about: "Query one or more DALI gears for various information.")
                            (@arg ADDR: +required "Address")
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
    let driver = &mut Helvar510driver::new();

    let info =
        match block_on(device_info::read_device_info(driver, addr)) {
            Ok(i) => i,
            Err(e) => {
                println!("{}", e);
                return;
            }
        };
    println!("{}", info);
    
}
