use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command};
use dali_tools::common::cmd_defs::AddressByte;
use dali_tools::drivers::command_utils::send16;
use dali_tools::drivers::driver::DaliDriver;
use dali_tools::drivers::send_flags::NO_FLAG;
use dali_tools::gear::address::Address;
use dali_tools::gear::cmd_defs;
use dali_tools::utils::parse_address;
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;
use std::str::FromStr;

fn parse_group(s: &str) -> Result<u16, String> {
    if s == "all" {
        Ok(0xffff)
    } else {
        let mut group_mask = 0;
        for g in s.split(",") {
            let g = u32::from_str(g).map_err(|_| "Invalid group number".to_string())?;
            if g > 16 || g < 1 {
                return Err("Group number out of range".to_string());
            }
            group_mask |= 1 << (g - 1);
        }
        Ok(group_mask)
    }
}

async fn change_groups(
    driver: &mut dyn DaliDriver,
    addr: &Address,
    add_mask: u16,
    remove_mask: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    for g in 0..16 {
        if (add_mask & (1 << g)) != 0 {
            send16::cmd(
                driver,
                cmd_defs::ADD_TO_GROUP(AddressByte::from(*addr), g as u8),
                NO_FLAG,
            )
            .await
            .check_send()?;
        } else if (remove_mask & (1 << g)) != 0 {
            send16::cmd(
                driver,
                cmd_defs::REMOVE_FROM_GROUP(AddressByte::from(*addr), g as u8),
                NO_FLAG,
            )
            .await
            .check_send()?;
        }
    }
    Ok(())
}

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let addr = matches.get_one::<Address>("ADDR").unwrap();
        match matches.subcommand() {
            Some(("set", matches)) => {
                let set_mask = matches.get_one::<u16>("GROUP").unwrap();
                change_groups(&mut *ctxt.driver, addr, *set_mask, !*set_mask).await?;
            }
            Some(("add", matches)) => {
                let add_mask = matches.get_one::<u16>("GROUP").unwrap();
                change_groups(&mut *ctxt.driver, addr, *add_mask, 0).await?;
            }
            Some(("remove", matches)) => {
                let remove_mask = matches.get_one::<u16>("GROUP").unwrap();
                change_groups(&mut *ctxt.driver, addr, 0, *remove_mask).await?;
            }
            _ => {}
        }
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("group")
        .about("Add or remove group membership")
        .arg(
            Arg::new("ADDR")
                .required(true)
                .value_parser(parse_address::parse_address::<16>)
                .help("Address or group"),
        );

    let group_arg = Arg::new("GROUP")
        .required(true)
        .value_parser(parse_group)
        .help("Comma separated list of groups or 'all'");

    let set_cmd = Command::new("set").about("Set groups").arg(&group_arg);
    let add_cmd = Command::new("add").about("Add groups").arg(&group_arg);
    let remove_cmd = Command::new("remove")
        .about("Remove groups")
        .arg(&group_arg);

    let cli_cmd = cli_cmd
        .subcommand(add_cmd)
        .subcommand(set_cmd)
        .subcommand(remove_cmd);

    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}

#[test]
fn test_parse_group() {
    assert_eq!(parse_group("1,2,3").unwrap(), 0b0000_0000_0000_0111);
    assert_eq!(parse_group("16,2,3").unwrap(), 0b1000_0000_0000_0110);
    assert_eq!(parse_group("all").unwrap(), 0xffff);
    assert!(parse_group("17").is_err());
    assert!(parse_group("0").is_err());
    assert!(parse_group("a").is_err());
}
