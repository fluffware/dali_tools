use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command};
use dali_tools::common::commands::Commands;
use dali_tools::common::driver_commands::DriverCommands;
use dali_tools::control::commands_103::Commands103;
use dali_tools::drivers::driver::DaliSendResult;
use dali_tools::drivers::send_flags::PRIORITY_1;
use dali_tools::gear::address::Short;
use dali_tools::gear::commands_102::Commands102;
use dali_tools::utils::address_assignment::{clear_short_address, program_short_address};
use dali_tools::utils::address_set::AddressSet;
use dali_tools::utils::discover::{self, Discovered};
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;

async fn perform_randomize<C>(commands: &mut C, control: bool)
where
    C: Commands<Error = DaliSendResult>,
{
    commands.initialise_no_addr().await?;
    commands.randomize().await?;
    commands.terminate().await?;
}
fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        if control {
            let mut commands = Commands103::from_driver(&mut *ctxt.driver, PRIORITY_1);
            perform_randomize(&mut commands).await;
        } else {
            let mut commands = Commands102::from_driver(&mut *ctxt.driver, PRIORITY_1);
            perform_randomize(&mut commands).await;
        }
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("randomize");
    let cli_cmd = cli_cmd.arg(
        Arg::new("control")
            .long("control")
            .action(clap::ArgAction::SetTrue)
            .help("Discover control devices"),
    );
    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
