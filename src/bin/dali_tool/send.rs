use super::sub_tool::{SubTool, ToolContext};
use clap::{Arg, ArgMatches, Command, value_parser};
use dali_tools::drivers::driver::DaliFrame;
use dali_tools::drivers::send_flags::Flags as SendFlags;
use futures::FutureExt;
#[allow(unused_imports)]
use log::debug;
use std::pin::Pin;
use std::time::Duration;

enum Step {
    Frame(DaliFrame),
    Wait(Duration),
}

fn execute<'a>(
    ctxt: &'a mut ToolContext,
    matches: &'a ArgMatches,
) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error>>> + 'a>> {
    Box::pin(async {
        let cmd_strings = matches.get_many::<String>("CMD").unwrap();

        let expect_answer = *matches.get_one::<bool>("answer").unwrap();
        let send_twice = *matches.get_one::<bool>("twice").unwrap();
        let priority = *matches.get_one::<u16>("priority").unwrap();
        if !(1..=5).contains(&priority) {
            return Err("Priority out of range".into());
        }
        let mut repeat = *matches.get_one::<u16>("repeat").unwrap();
        let mut steps = Vec::new();
        for cmd_string in cmd_strings {
            let mut frame = 0u32;
            let mut frame_len = 0;
            if let Some(time_str) = cmd_string.strip_prefix('w') {
                let Ok(ms) = time_str.parse() else {
                    return Err(format!("Failed to parse milliseconds '{}'", time_str).into());
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
                        return Err("Invalid hex digit in command".into());
                    }
                }
                let frame = match frame_len {
                    16 => DaliFrame::Frame16([(frame >> 8) as u8, frame as u8]),
                    24 => {
                        DaliFrame::Frame24([(frame >> 16) as u8, (frame >> 8) as u8, frame as u8])
                    }
                    _ => {
                        return Err("Invalid frame length".into());
                    }
                };
                steps.push(Step::Frame(frame));
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
        let flags = SendFlags::ExpectAnswer(expect_answer)
            | SendFlags::SendTwice(send_twice)
            | SendFlags::Priority(priority);

        loop {
            for step in steps.iter() {
                match step {
                    Step::Frame(frame) => {
                        ctxt.driver
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
        Ok(())
    })
}

pub fn init_subtool() -> SubTool {
    let cli_cmd = Command::new("send").about("Send arbitrary DALI command");
    let cli_cmd = cli_cmd
        .arg(
            Arg::new("CMD")
                .num_args(1..)
                .required(true)
                .help("Hex string containg DALI command. Two or three bytes."),
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
        );

    SubTool {
        sub_command: cli_cmd,
        execute: execute,
    }
}
