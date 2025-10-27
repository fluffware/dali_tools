use dali::drivers::driver::{DaliFrame, OpenError};
use dali::drivers::send_flags::Flags as SendFlags;
use dali_tools as dali;
use futures::FutureExt;
use tokio::time::Duration;

extern crate clap;
use clap::{Arg, Command, value_parser};

enum Step {
    Frame(DaliFrame),
    Wait(Duration),
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = dali::drivers::init() {
        eprintln!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = Command::new("send_cmd")
        .about("Send an arbitrary DALI command.")
        .arg(
            Arg::new("CMD")
                .num_args(1..)
                .required(true)
                .help("Hex string containg DALI command. Two or three bytes."),
        )
        .arg(
            Arg::new("DEVICE")
                .short('d')
                .long("device")
                .default_value("default")
                .help("Select DALI-device"),
        )
        .arg(
            Arg::new("answer")
                .short('a')
                .long("answer")
                .value_parser(value_parser!(bool))
                .action(clap::ArgAction::SetTrue)
                .help("Expect an answer"),
        )
        .arg(
            Arg::new("twice")
                .short('t')
                .long("twice")
                .value_parser(value_parser!(bool))
                .action(clap::ArgAction::SetTrue)
                .help("Send command twice"),
        )
        .arg(
            Arg::new("priority")
                .short('p')
                .long("priority")
                .value_parser(value_parser!(u16))
                .default_value("3")
                .default_missing_value("true")
                .help("Command priority"),
        )
        .arg(
            Arg::new("repeat")
                .short('r')
                .long("repeat")
                .value_parser(value_parser!(u16))
                .default_value("1")
                .default_missing_value("true")
                .help("Play sequence this many times"),
        )
        .get_matches();

    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let cmd_strings = matches.get_many::<String>("CMD").unwrap();

    let expect_answer = *matches.get_one::<bool>("answer").unwrap();
    let send_twice = *matches.get_one::<bool>("twice").unwrap();
    let priority = *matches.get_one::<u16>("priority").unwrap();
    if !(1..=5).contains(&priority) {
        eprintln!("Priority out of range");
        return;
    }
    let mut repeat = *matches.get_one::<u16>("repeat").unwrap();
    let mut steps = Vec::new();
    for cmd_string in cmd_strings {
        let mut frame = 0u32;
        let mut frame_len = 0;
        if let Some(time_str) = cmd_string.strip_prefix('w') {
            let Ok(ms) = time_str.parse() else {
                eprintln!("Failed to parse milliseconds '{}'", time_str);
                return;
            };
            steps.push(Step::Wait(Duration::from_millis(ms)));
        } else {
            for c in cmd_string.chars() {
                if c.is_whitespace() {
                    // Skip
                } else if let Some(d) = c.to_digit(16) {
                    frame = (frame << 4) | d;
                    frame_len += 4;
                } else {
                    eprintln!("Invalid hex digit in command");
                    return;
                }
            }
            let frame = match frame_len {
                16 => DaliFrame::Frame16([(frame >> 8) as u8, frame as u8]),
                24 => DaliFrame::Frame24([(frame >> 16) as u8, (frame >> 8) as u8, frame as u8]),
                _ => {
                    eprintln!("Invalid frame length");
                    return;
                }
            };
            steps.push(Step::Frame(frame));
        }
    }

    let mut driver = match dali::drivers::open(device_name) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Failed to open DALI device: {}", e);
            if let OpenError::NotFound = e {
                eprintln!("Available drivers:");
                for name in dali::drivers::driver_names() {
                    eprintln!("  {}", name);
                }
            }
            return;
        }
    };
    tokio::time::sleep(Duration::from_millis(200)).await;
    let flags = SendFlags::ExpectAnswer(expect_answer)
        | SendFlags::SendTwice(send_twice)
        | SendFlags::Priority(priority);

    loop {
        for step in steps.iter() {
            match step {
                Step::Frame(frame) => {
                    driver
                        .send_frame(frame.clone(), flags.clone())
                        .then(|res| async move {
                            println!("Result: {}", res);
                        })
                        .await;
                }
                Step::Wait(dur) => tokio::time::sleep(*dur).await,
            }
        }
        if repeat == 1 {
            break;
        } else if repeat > 1 {
            repeat -= 1;
        }
    }
}
