use clap::{Arg, Command};
use dali::drivers::driver::OpenError;
use dali::simulator;
use dali_tools as dali;
use log::debug;
use std::fs::File;
use tokio::signal;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = dali::drivers::init() {
        eprintln!("Failed to initialize DALI drivers: {}", e);
    }
    let mut cli_cmd = Command::new("dali_simulator")
        .about("Simulate DALI-devices on a bus ")
        .arg(Arg::new("CONFIG").required(true).help("Configuration file"));
    let matches = cli_cmd.get_matches();
    let conf_filename = matches.get_one::<String>("CONFIG").unwrap();
    let conf_file = match File::open(conf_filename) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "Failed to open configuration file '{}': {}",
                conf_filename, e
            );
            return;
        }
    };
    let (bus, mut sched) = match simulator::setup::setup_simulator(conf_file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to start simulator: {}", e);
            return;
        }
    };
    signal::ctrl_c().await.expect("Failed to wait for Ctrl-C");
    debug!("Exiting");
}
