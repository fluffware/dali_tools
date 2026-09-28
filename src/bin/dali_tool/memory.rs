use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command, value_parser};
use dali_tools::common::commands::Commands;
use dali_tools::common::driver_commands::DriverCommands;
use dali_tools::drivers::send_flags::PRIORITY_1;
use dali_tools::gear::address::Short;
use dali_tools::gear::commands_102::Commands102;
use dali_tools::utils::memory_banks;
use dali_tools::utils::parse_address;
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;

fn print_data(start: u8, data: &[u8]) {
    let mut a = start;
    for line in data.chunks(16) {
        print!("{a:02x}:");
        a += 16;
        for b in line {
            print!(" {b:02x}");
        }
        println!("");
    }
}

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let addr = matches.get_one::<Short>("ADDR").unwrap();
        let bank = matches.get_one::<u8>("BANK").unwrap();
        let commands = &mut Commands102::from_driver(&mut *ctxt.driver, PRIORITY_1);
        match matches.subcommand() {
            Some(("read", matches)) => {
                let mem_addr = matches.get_one::<u8>("MEM_ADDR").unwrap();
                let mem_count = matches.get_one::<u8>("MEM_COUNT").unwrap();
                let data =
                    memory_banks::read_range(commands, *addr, *bank, *mem_addr, *mem_count).await?;
                print_data(*mem_addr, &data);
            }
            Some(("write", matches)) => {
                let mem_addr = matches.get_one::<u8>("MEM_ADDR").unwrap();
                let mem_data = matches.get_many::<u8>("BYTE").unwrap();
                let mem_data: Vec<u8> = mem_data.cloned().collect();
                memory_banks::write(
                    commands,
                    <Commands102 as Commands>::Address::from(*addr),
                    *bank,
                    *mem_addr,
                    &mem_data,
                )
                .await?;
            }
            _ => {}
        }
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("memory-bank")
        .about("Read or write memory banks")
        .arg(
            Arg::new("ADDR")
                .required(true)
                .value_parser(parse_address::parse_short)
                .help("Short address"),
        )
        .arg(
            Arg::new("BANK")
                .required(true)
                .value_parser(value_parser!(u8))
                .help("Memory bank"),
        );

    let read_cmd = Command::new("read")
        .about("Read memory bank")
        .arg(
            Arg::new("MEM_ADDR")
                .required(true)
                .value_parser(value_parser!(u8))
                .help("First memory location to read"),
        )
        .arg(
            Arg::new("MEM_COUNT")
                .required(true)
                .value_parser(value_parser!(u8))
                .help("Number of memory locations to read"),
        );

    let write_cmd = Command::new("write")
        .about("Write memory block")
        .arg(
            Arg::new("MEM_ADDR")
                .required(true)
                .value_parser(value_parser!(u8))
                .help("First memory location to write"),
        )
        .arg(
            Arg::new("BYTE")
                .required(true)
                .num_args(1..)
                .value_parser(value_parser!(u8))
                .help("Data to write to memory bank"),
        );

    let reset_cmd = Command::new("reset").about("Reset memory bank");

    let cli_cmd = cli_cmd
        .subcommand(read_cmd)
        .subcommand(write_cmd)
        .subcommand(reset_cmd);

    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
