use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command};
use dali_tools::common::address::Short;
use dali_tools::common::commands::Commands;
use dali_tools::control::commands_103::Commands103;
use dali_tools::drivers::driver::DaliSendResult;
use dali_tools::gear::commands_102::Commands102;
use dali_tools::utils::device_info;
use dali_tools::utils::memory_banks;
use dali_tools::utils::parse_address;
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let addrs = matches.get_one::<Vec<Short>>("ADDR_RANGE").unwrap();
        let read_memory = *matches.get_one::<bool>("memory_banks").unwrap();
        let control_device = *matches.get_one::<bool>("control").unwrap();
        let try_all = *matches.get_one::<bool>("try-all").unwrap();
        for addr in addrs {
            if control_device {
                let mut commands = Commands103::new(&mut *ctxt.driver);
                let long = commands.query_random_address(*addr).await;
                if let Ok(long) = long {
                    println!("Long address: 0x{:06x}", long);
                }
                if try_all || long.is_ok() {
                    let info = match device_info::read_control_info(&mut *ctxt.driver, *addr).await
                    {
                        Ok(i) => i,
                        Err(e) => {
                            return Err(format!("Failed to read device info: {}", e).into());
                        }
                    };
                    println!("{}", info);
                }
            } else {
                let mut commands = Commands102::new(&mut *ctxt.driver);
                let long = commands.query_random_address(*addr).await;
                match long {
                    Ok(long) => {
                        println!("Long address: 0x{:06x}", long);
                    }
                    Err(DaliSendResult::Timeout) => {}
                    Err(e) => {
                        return Err(format!(
                            "Failed to read long address for short address {}: {}",
                            *addr, e
                        )
                        .into());
                    }
                }
                if try_all || long.is_ok() {
                    let info = match device_info::read_gear_info(&mut *ctxt.driver, *addr).await {
                        Ok(i) => i,
                        Err(e) => {
                            return Err(format!("Failed to read device info: {}", e).into());
                        }
                    };
                    println!("{}", info);
                    if read_memory {
                        match memory_banks::read_bank_0(&mut *ctxt.driver, *addr, 0, 0, 0x18).await
                        {
                            Ok(data) => println!("{}", data),
                            Err(e) => {
                                return Err(format!("Failed to read memory banks: {}", e).into());
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("query").about("Query a device for common parameters");
    let cli_cmd = cli_cmd.arg(
            Arg::new("ADDR_RANGE")
                .required(true)
                .value_parser(parse_address::parse_short_range)
                .help("Address range (<addr> or <start>-<end>)"),
        )
        .arg(
            Arg::new("memory_banks")
                .short('m')
                .long("memory-banks")
		 .action(clap::ArgAction::SetTrue)
                .help("Read information from memory banks"),
        )
        .arg(
            Arg::new("control")
                .short('c')
                .long("control")
                .action(clap::ArgAction::SetTrue)
                .help("Read info from control devices"),
        )
	.arg(
            Arg::new("try-all")
                .long("try-all")
                .action(clap::ArgAction::SetTrue)
                .help("Try reading parameters even from devices that doesn't respond with a long address"));

    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
