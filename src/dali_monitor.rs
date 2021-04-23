use dali_tools as dali;
use dali::drivers::helvar::helvar510::Helvar510driver;
use dali::drivers::monitor::DALImonitor;
use dali::drivers::monitor::DaliBusEventType;
use dali::utils::decode;
use tokio::stream::StreamExt;
use tokio::time::Instant;

#[macro_use]
extern crate clap;

#[tokio::main]
async fn main() {
    let _matches = 
        clap_app!(swap_addr =>
                  (about: "Print DALI bus traffic.")
        ).get_matches();
    
    let mut last_ts = Instant::now();
    let driver = &mut Helvar510driver::new();
    let mut monitor = driver.monitor_stream().unwrap();
    loop {
        let event = monitor.next().await;
        let event = match event {
            None => break,
            Some(e) => e
        };
        print!("{:5}:", event.timestamp.duration_since(last_ts).as_millis());
        last_ts = event.timestamp;
        match event.event {
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
