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

async fn perform_discovery<C>(commands: &mut C, clear_conflicts: bool, allocate: bool)
where
    C: Commands<Error = DaliSendResult>,
{
    let mut allocated_addrs = AddressSet::new();
    let mut short_conflicts = Vec::new();
    let mut unallocated = Vec::new();
    let mut found = async |device: Discovered| {
        println!(
            "Long: {}, Short: {} {}{}",
            if let Some(long) = device.long {
                format!("0x{:06x}", long)
            } else {
                "None".to_string()
            },
            if let Some(short) = device.short {
                short.to_string()
            } else {
                "None".to_string()
            },
            if device.short_conflict {
                ", Short address conflict"
            } else {
                ""
            },
            if device.long_conflict {
                ", Long address conflict"
            } else {
                ""
            },
        );
        if device.short_conflict {
            short_conflicts.push(device.clone());
        }
        if let Some(addr) = device.short {
            allocated_addrs.insert(addr);
        } else {
            unallocated.push(device);
        }
    };
    if let Err(e) = discover::find_quick(commands, &mut found).await {
        eprintln!("Discovery failed: {}", e);
    }

    if clear_conflicts && !short_conflicts.is_empty() {
        let _ = commands.initialise_all().await;
        for d in short_conflicts {
            if let Some(long) = d.long
                && let Err(e) = clear_short_address(commands, long).await
            {
                eprintln!(
                    "Failed to clear short address for long address {}: {}",
                    long, e,
                );
            }
        }
        let _ = commands.terminate().await;
    }
    if allocate && !unallocated.is_empty() {
        let _ = commands.initialise_no_addr().await;
        let mut next = 0;
        for device in unallocated {
            while next < 64 && allocated_addrs.contains(Short::new(next)) {
                next += 1;
            }
            if next == 64 {
                eprintln!("No free addresses");
                return;
            }
            if let Some(long) = device.long {
                if let Err(e) = program_short_address(commands, long, Short::new(next)).await {
                    eprintln!(
                        "Failed to program short address for long address {}: {}",
                        long, e,
                    );
                } else {
                    next += 1;
                }
            }
        }
        let _ = commands.terminate().await;
    }
}
fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let clear_conflicts = *matches.get_one::<bool>("clear_conflicts").unwrap();
        let allocate = *matches.get_one::<bool>("allocate").unwrap();
        let control = *matches.get_one::<bool>("control").unwrap();

        if control {
            let mut commands = Commands103::from_driver(&mut *ctxt.driver, PRIORITY_1);
            perform_discovery(&mut commands, clear_conflicts, allocate).await;
        } else {
            let mut commands = Commands102::from_driver(&mut *ctxt.driver, PRIORITY_1);
            perform_discovery(&mut commands, clear_conflicts, allocate).await;
        }
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("discover").about("Discover devices with or without short address");
    let cli_cmd = cli_cmd
        .arg(
            Arg::new("clear_conflicts")
                .long("clear-conflicts")
                .action(clap::ArgAction::SetTrue)
                .help("Clear the short address for devices with duplicate addresses"),
        )
        .arg(
            Arg::new("allocate")
                .long("allocate")
                .action(clap::ArgAction::SetTrue)
                .help("Allocate addresses for devices with no address"),
        )
        .arg(
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
