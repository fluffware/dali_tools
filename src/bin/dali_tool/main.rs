use dali::drivers::driver::OpenError;
use dali_tools as dali;
use std::collections::HashMap;
use std::mem;

use clap::{Arg, Command};
mod sub_tool;
use sub_tool::{ExecuteTool, SubTool, ToolContext};
mod clear_addr;
mod discover;
mod fade;
mod monitor;
mod power;
mod query;
mod randomize;
mod scene;
mod send;
mod set_addr;
mod swap;
use log::debug;

type ToolMap = HashMap<String, ExecuteTool>;

fn add_tool(sub_tool: SubTool, tool_map: &mut ToolMap, cli_cmd: &mut Command) {
    let tool_name = sub_tool.sub_command.get_name();
    debug!("Registering command {}", tool_name);
    tool_map.insert(tool_name.to_string(), sub_tool.execute);
    *cli_cmd = mem::take(cli_cmd).subcommand(sub_tool.sub_command);
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = dali::drivers::init() {
        eprintln!("Failed to initialize DALI drivers: {}", e);
    }

    let mut cli_cmd = Command::new("dali_tool")
        .about("Query, configure and control DALI-devices")
        .arg(
            Arg::new("DEVICE")
                .short('d')
                .long("device")
                .env("DALI_DEVICE")
                .default_value("default")
                .help("Select DALI-device"),
        );
    let mut tool_map = HashMap::new();
    add_tool(query::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(power::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(discover::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(clear_addr::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(set_addr::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(randomize::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(swap::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(send::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(monitor::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(scene::init_subtool(), &mut tool_map, &mut cli_cmd);
    add_tool(fade::init_subtool(), &mut tool_map, &mut cli_cmd);
    let matches = cli_cmd.get_matches();

    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let driver = match dali::drivers::open(device_name) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to open DALI device '{}': {}", device_name, e);
            if let OpenError::NotFound = e {
                eprintln!("Available drivers:");
                for name in dali::drivers::driver_names() {
                    eprintln!("  {}", name);
                }
            }
            return;
        }
    };
    let mut tool_ctxt = ToolContext { driver };
    if let Some((subname, subargs)) = matches.subcommand() {
        if let Some(execute) = tool_map.get(subname) {
            if let Err(e) = execute(&mut tool_ctxt, subargs).await {
                eprintln!("Command failed: {e}");
            }
        } else {
            eprintln!("No sub command named {}", subname);
        }
    } else {
        eprintln!("Select a sub command");
    }
}
