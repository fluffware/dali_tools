use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command};
use dali_tools::common::cmd_defs::AddressByte;
use dali_tools::drivers::command_utils::send16;
use dali_tools::drivers::send_flags::{NO_FLAG, PRIORITY_1, SEND_TWICE};
use dali_tools::gear::address::Address;
use dali_tools::gear::cmd_defs;
use dali_tools::gear::fade::FadeTime;
use dali_tools::utils::parse_address;
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;
use std::str::FromStr;

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let addr = matches.get_one::<Address>("ADDR").unwrap();
        let fade_time = matches.get_one::<FadeTime>("FADE_TIME").unwrap();
        send16::set_dtr0(&mut *ctxt.driver, fade_time.value(), NO_FLAG)
            .await
            .check_send()?;
        send16::cmd(
            &mut *ctxt.driver,
            cmd_defs::SET_FADE_TIME(AddressByte::from(*addr)),
            SEND_TWICE | PRIORITY_1,
        )
        .await
        .check_send()?;
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("fade-time")
        .about("Set fade time")
        .arg(
            Arg::new("ADDR")
                .required(true)
                .value_parser(parse_address::parse_address::<16>)
                .help("Address or group"),
        )
        .arg(
            Arg::new("FADE_TIME")
                .required(true)
                .value_parser(FadeTime::from_str)
                .help("Fade time step or fade time as value followed by 's' or 'min'"),
        );
    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
