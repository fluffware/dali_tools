use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command};
use dali_tools::common::address::{Long, Short};
use dali_tools::common::commands::Commands;
use dali_tools::common::driver_commands::DriverCommands;
use dali_tools::control::commands_103::Commands103;
use dali_tools::drivers::send_flags::PRIORITY_1;
use dali_tools::gear::commands_102::Commands102;
use dali_tools::utils::address_assignment::program_short_address;
use dali_tools::utils::parse_address::parse_short;
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let control_device = *matches.get_one::<bool>("control").unwrap();
        let long_str = matches.get_one::<String>("LONG_ADDR").unwrap();
        let long: Long = if let Some(long_str) = str::strip_prefix(&long_str, "0x") {
            u32::from_str_radix(long_str, 16)?
        } else {
            u32::from_str_radix(long_str, 10)?
        };
        let addr_str = matches.get_one::<String>("SHORT_ADDR").unwrap();
        if control_device {
            let short: Short = parse_short(addr_str)?;
            let mut commands = Commands103::from_driver(&mut *ctxt.driver, PRIORITY_1);
            commands.initialise_all().await?;
            let res = program_short_address(&mut commands, long, short).await;
            commands.terminate().await?;
            res?;
        } else {
            let short: Short = parse_short(addr_str)?;
            let mut commands = Commands102::from_driver(&mut *ctxt.driver, PRIORITY_1);
            commands.initialise_all().await?;
            let res = program_short_address(&mut commands, long, short).await;
            commands.terminate().await?;
            res?;
        }
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("set-addr").about("Set short address for selected long address");
    let cli_cmd = cli_cmd
        .arg(Arg::new("LONG_ADDR").required(true).help("Long address"))
        .arg(Arg::new("SHORT_ADDR").required(true).help("Short address"))
        .arg(
            Arg::new("control")
                .short('c')
                .long("control")
                .action(clap::ArgAction::SetTrue)
                .help("Set address for control device"),
        );

    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
