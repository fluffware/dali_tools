use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command};
use dali_tools::common::address::Short;
use dali_tools::common::commands::Commands;
use dali_tools::common::driver_commands::DriverCommands;
use dali_tools::control::commands_103::Commands103;
use dali_tools::drivers::driver::DaliSendResult;
use dali_tools::drivers::send_flags::PRIORITY_1;
use dali_tools::gear::commands_102::Commands102;
use dali_tools::utils::address_assignment;
use dali_tools::utils::address_assignment::program_short_address;
use dali_tools::utils::parse_address;
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;

async fn swap_addr<C>(
    commands: &mut C,
    addr1: Short,
    addr2: Short,
) -> Result<(), address_assignment::Error<DaliSendResult>>
where
    C: Commands<Error = DaliSendResult>,
{
    let long1 = match commands.query_random_address(addr1).await {
        Ok(a) => Some(a),
        Err(DaliSendResult::Timeout) => None,
        Err(e) => return Err(e.into()),
    };
    println!(
        "{}: {}",
        addr1,
        long1
            .map(|x| { format!("0x{x:06x}") })
            .unwrap_or_else(|| "-".to_string())
    );
    let long2 = match commands.query_random_address(addr2).await {
        Ok(a) => Some(a),
        Err(DaliSendResult::Timeout) => None,
        Err(e) => return Err(e.into()),
    };
    println!(
        "{}: {}",
        addr2,
        long2
            .map(|x| { format!("0x{x:06x}") })
            .unwrap_or_else(|| "-".to_string())
    );
    commands.initialise_all().await?;
    debug!("initialise_all done");
    if let Some(l) = long1 {
        program_short_address(commands, l, addr2).await?;
    }
    if let Some(l) = long2 {
        program_short_address(commands, l, addr1).await?;
    }
    commands.terminate().await?;
    Ok(())
}

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let addr1 = matches.get_one::<Short>("ADDR1").unwrap();
        let addr2 = matches.get_one::<Short>("ADDR2").unwrap();
        let control = *matches.get_one::<bool>("control").unwrap();
        if control {
            let mut commands = Commands103::from_driver(&mut *ctxt.driver, PRIORITY_1);
            swap_addr(&mut commands, *addr1, *addr2).await?;
        } else {
            let mut commands = Commands102::from_driver(&mut *ctxt.driver, PRIORITY_1);
            swap_addr(&mut commands, *addr1, *addr2).await?;
        }
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("swap").about("Swaps short addresses of two devices. If only one is present then the address of that one is changed.");
    let cli_cmd = cli_cmd
        .arg(
            Arg::new("control")
                .long("control")
                .action(clap::ArgAction::SetTrue)
                .help("Discover control devices"),
        )
        .arg(
            Arg::new("ADDR1")
                .required(true)
                .value_parser(parse_address::parse_short)
                .help("First address"),
        )
        .arg(
            Arg::new("ADDR2")
                .required(true)
                .value_parser(parse_address::parse_short)
                .help("Second address"),
        );
    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
