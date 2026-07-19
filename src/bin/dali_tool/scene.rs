use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command};
use dali_tools::common::cmd_defs::AddressByte;
use dali_tools::common::defs::MASK;
use dali_tools::drivers::command_utils::send16;
use dali_tools::drivers::send_flags::{NO_FLAG, PRIORITY_1, SEND_TWICE};
use dali_tools::gear::address::Address;
use dali_tools::gear::cmd_defs;
use dali_tools::utils::parse_address;
#[allow(unused_imports)]
use log::debug;
use std::num::ParseIntError;
use std::pin::Pin;
use std::str::FromStr;

fn parse_power(s: &str) -> Result<u8, ParseIntError> {
    u8::from_str(s)
}

fn parse_scene(s: &str) -> Result<u8, String> {
    if s == "all" {
        return Ok(255);
    }
    let scene = u8::from_str(s).map_err(|e| e.to_string())?;
    if !(1..=16).contains(&scene) {
        return Err("Invalid scene number".to_string());
    }
    return Ok(scene);
}

// Parse a scene and a power level separated by ':'
fn parse_scene_power(s: &str) -> Result<(u8, u8), String> {
    let Some((scene_str, power_str)) = s.split_once(':') else {
        return Err("No colon".to_string());
    };
    let scene = parse_scene(scene_str)?;
    let power = parse_power(power_str).map_err(|e| e.to_string())?;
    Ok((scene, power))
}
fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let addr = matches.get_one::<Address>("ADDR").unwrap();
        match matches.subcommand() {
            Some(("set", matches)) => {
                let (scene, power) = matches.get_one::<(u8, u8)>("SCENE:POWER").unwrap();
                send16::set_dtr0(&mut *ctxt.driver, *power, NO_FLAG)
                    .await
                    .check_send()?;
                send16::cmd(
                    &mut *ctxt.driver,
                    cmd_defs::SET_SCENE(AddressByte::from(*addr), *scene - 1),
                    SEND_TWICE | PRIORITY_1,
                )
                .await
                .check_send()?;
            }
            Some(("remove", matches)) => {
                let scene = matches.get_one::<u8>("SCENE").unwrap();
                send16::set_dtr0(&mut *ctxt.driver, MASK, NO_FLAG)
                    .await
                    .check_send()?;
                send16::cmd(
                    &mut *ctxt.driver,
                    cmd_defs::REMOVE_FROM_SCENE(AddressByte::from(*addr), *scene - 1),
                    NO_FLAG,
                )
                .await
                .check_send()?;
            }
            Some(("goto", matches)) => {
                let scene = matches.get_one::<u8>("SCENE").unwrap();
                if *scene != 255 {
                    send16::cmd(
                        &mut *ctxt.driver,
                        cmd_defs::GOTO_SCENE(AddressByte::from(*addr), *scene - 1),
                        NO_FLAG,
                    )
                    .await
                    .check_send()?;
                }
            }
            _ => {}
        }
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("scene").about("Manipulate scenes").arg(
        Arg::new("ADDR")
            .required(true)
            .value_parser(parse_address::parse_address::<16>)
            .help("Address or group"),
    );

    let scene_arg = Arg::new("SCENE")
        .required(true)
        .value_parser(parse_scene)
        .help("Scene number (1-16) or 'all'");

    let set_cmd = Command::new("set").about("Set scene").arg(
        Arg::new("SCENE:POWER")
            .required(true)
            .value_parser(parse_scene_power)
            .num_args(1..)
            .help("Scene and corresponding power levels"),
    );
    let remove_cmd = Command::new("remove").about("Remove scene").arg(&scene_arg);
    let goto_cmd = Command::new("goto").about("Goto scene").arg(&scene_arg);

    let cli_cmd = cli_cmd
        .subcommand(goto_cmd)
        .subcommand(set_cmd)
        .subcommand(remove_cmd);

    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
