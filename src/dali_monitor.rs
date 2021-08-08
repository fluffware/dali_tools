use dali_tools as dali;
use dali::drivers::driver::{DaliBusEvent, DaliBusEventType};
use dali::utils::decode;
use std::time::Instant;

#[macro_use]
extern crate clap;

#[tokio::main]
async fn main() {
    if let Err(e) = dali::drivers::init() {
	println!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = 
        clap_app!(swap_addr =>
                  (about: "Print DALI bus traffic.")
		  (@arg DEVICE: -d --device +takes_value"Select DALI-device")
        ).get_matches();
    
    let mut last_ts = Instant::now();
    let device_name = 
	matches.value_of("DEVICE").unwrap_or("default");
    let mut driver = dali::drivers::open(device_name).unwrap();
    loop {
        let DaliBusEvent{timestamp, event} = driver.next_bus_event().await;
        print!("{:5}:", timestamp.duration_since(last_ts).as_millis());
        last_ts = timestamp;
        match event {
            DaliBusEventType::Recv24bitFrame(ref pkt) => {
                for b in pkt {
                    print!(" {:02x}", b);
                }
                print!(" ");
                println!("{}",decode::decode_packet(pkt))
            },
            DaliBusEventType::Recv16bitFrame(ref pkt) => {
                for b in pkt {
                    print!(" {:02x}", b);
                }
                print!("    ");
                println!("{}",decode::decode_packet(pkt))
            },
            _ => println!("{:?}", event)
        }
    }
}
