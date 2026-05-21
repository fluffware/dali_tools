use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command};
use dali_tools::common::cmd_defs::AddressByte;
use dali_tools::drivers::driver_utils::DaliDriverExt;
use dali_tools::drivers::send_flags::NO_FLAG;
use dali_tools::gear::address::Address;
use dali_tools::utils::parse_address;
use std::num::ParseIntError;
use std::pin::Pin;
use std::str::FromStr;

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let addr = matches.get_one::<Address>("ADDR").unwrap();
        let power = matches.get_one::<u8>("POWER").unwrap();
        ctxt.driver
            .send_frame16(&[AddressByte::from(*addr).0 & 0xfe, *power], NO_FLAG)
            .await
            .check_send()?;
        Ok(())
    })
}

fn parse_power(s: &str) -> Result<u8, ParseIntError> {
    u8::from_str(s)
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("power").about("Sets arc power of selected devices");
    let cli_cmd = cli_cmd
        .arg(
            Arg::new("ADDR")
                .required(true)
                .value_parser(parse_address::parse_address::<16>)
                .help("Address (<short>, G<group>, 'all' or 'unaddressed')"),
        )
        .arg(
            Arg::new("POWER")
                .required(true)
                .value_parser(parse_power)
                .help("Arc power as steps (0 - 254)"),
        );

    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
