use super::sub_tool::{SubTool, ToolContext};
use clap::{ArgMatches, Command};
use dali_tools::drivers::driver::{DaliBusEvent, DaliBusEventType};
use dali_tools::utils::decode;
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;
use std::time::Instant;

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    _matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let mut last_ts = Instant::now();
        let mut decoder = decode::DecoderState::new();
        loop {
            match ctxt.driver.next_bus_event().await {
                Ok(DaliBusEvent {
                    timestamp,
                    event_type,
                    ..
                }) => {
                    print!("{:5}:", timestamp.duration_since(last_ts).as_millis());
                    last_ts = timestamp;
                    match event_type {
                        DaliBusEventType::Frame24(ref pkt) => {
                            for b in pkt {
                                print!(" {:02x}", b);
                            }
                            print!(" ");
                            println!("{}", decoder.decode_packet(pkt))
                        }
                        DaliBusEventType::Frame16(ref pkt) => {
                            for b in pkt {
                                print!(" {:02x}", b);
                            }
                            print!("    ");
                            println!("{}", decoder.decode_packet(pkt))
                        }
                        DaliBusEventType::Frame8(value) => {
                            let pkt = [value];
                            print!(" {:02x}", value);
                            print!("       ");
                            println!("{}", decoder.decode_packet(&pkt))
                        }
                        _ => println!("{:?}", event_type),
                    }
                }
                Err(e) => {
                    eprintln!("Failed to wait for event: {}", e);
                    break;
                }
            }
        }
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("monitor").about("Monitor packets sent on the bus");

    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
