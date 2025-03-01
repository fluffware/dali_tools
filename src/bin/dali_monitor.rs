use dali::drivers::driver::{DaliBusEvent, DaliBusEventType};
use dali::utils::decode;
use dali_tools as dali;
use std::time::Instant;

extern crate clap;
use clap::{Arg, Command};

#[tokio::main]
async fn main() {
    if let Err(e) = dali::drivers::init() {
        println!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = Command::new("swap_addr")
        .about("Print DALI bus traffic.")
        .arg(
            Arg::new("DEVICE")
                .short('d')
                .long("device")
                .default_value("default")
                .help("Select DALI-device"),
        )
        .get_matches();

    let mut last_ts = Instant::now();
    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let mut driver = dali::drivers::open(device_name).unwrap();
    loop {
        if let Ok(DaliBusEvent {
            timestamp,
            event_type,
            ..
        }) = driver.next_bus_event().await
        {
            print!("{:5}:", timestamp.duration_since(last_ts).as_millis());
            last_ts = timestamp;
            match event_type {
                DaliBusEventType::Frame24(ref pkt) => {
                    for b in pkt {
                        print!(" {:02x}", b);
                    }
                    print!(" ");
                    println!("{}", decode::decode_packet(pkt))
                }
                DaliBusEventType::Frame16(ref pkt) => {
                    for b in pkt {
                        print!(" {:02x}", b);
                    }
                    print!("    ");
                    println!("{}", decode::decode_packet(pkt))
                }
                _ => println!("{:?}", event_type),
            }
        }
    }
}
