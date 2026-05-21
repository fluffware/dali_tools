use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command};
use dali_tools::control::address::Address as Address24;
use dali_tools::control::cmd_defs::SET_SHORT_ADDRESS as SET_SHORT_ADDRESS_24;
use dali_tools::drivers::command_utils::{send16, send24};
use dali_tools::drivers::send_flags::{NO_FLAG, PRIORITY_1};
use dali_tools::gear::address::Address as Address16;
use dali_tools::gear::cmd_defs::SET_SHORT_ADDRESS as SET_SHORT_ADDRESS_16;
use dali_tools::utils::parse_address::parse_address;
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let control_device = *matches.get_one::<bool>("control").unwrap();
        if control_device {
            let addr_str = matches.get_one::<String>("ADDR").unwrap();
            let addr: Address24 = parse_address(addr_str)?;
            send24::set_dtr0(&mut *ctxt.driver, 0xff, NO_FLAG)
                .await
                .check_send()?;
            send24::cmd(&mut *ctxt.driver, SET_SHORT_ADDRESS_24(addr), PRIORITY_1)
                .await
                .check_send()?;
        } else {
            let addr_str = matches.get_one::<String>("ADDR").unwrap();
            let addr: Address16 = parse_address(addr_str)?;
            send16::set_dtr0(&mut *ctxt.driver, 0xff, NO_FLAG)
                .await
                .check_send()?;
            send16::cmd(&mut *ctxt.driver, SET_SHORT_ADDRESS_16(addr), PRIORITY_1)
                .await
                .check_send()?;
        }
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("clear-addr").about("Clear short address for selected devices");
    let cli_cmd = cli_cmd
        .arg(Arg::new("ADDR").required(true).help("Address"))
        .arg(
            Arg::new("control")
                .short('c')
                .long("control")
                .action(clap::ArgAction::SetTrue)
                .help("Read info from control devices"),
        );

    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
