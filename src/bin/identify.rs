use dali::base::address::{Address::Broadcast, Short};
use dali::defs::gear::cmd;
use dali::drivers::command_utils::{send_device_cmd, send_set_dtr0};
use dali::drivers::driver::OpenError;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
use dali::drivers::send_flags::{EXPECT_ANSWER, NO_FLAG, PRIORITY_1, PRIORITY_5, SEND_TWICE};
use dali_tools as dali;
use std::error::Error;
extern crate clap;
use clap::{value_parser, Arg, Command};

const BIT_SEQ: [u16; 64] = [
    0x210, 0x211, 0x212, 0x213, 0x214, 0x215, 0x216, 0x217, 0x218, 0x219, 0x21a, 0x21b, 0x21c,
    0x21d, 0x21e, 0x222, 0x223, 0x224, 0x225, 0x226, 0x227, 0x229, 0x22a, 0x22b, 0x22c, 0x22d,
    0x22e, 0x231, 0x232, 0x233, 0x234, 0x235, 0x236, 0x237, 0x239, 0x23a, 0x23b, 0x249, 0x24a,
    0x24b, 0x24d, 0x24e, 0x252, 0x253, 0x255, 0x256, 0x259, 0x25a, 0x25b, 0x25d, 0x266, 0x26a,
    0x26b, 0x26d, 0x26e, 0x273, 0x275, 0x276, 0x2aa, 0x2ab, 0x2ad, 0x2b5, 0x2bb, 0x2db,
];

#[cfg(test)]
const SEQ_MAP: [u8; 1024] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 255, 1, 255, 2, 255,
    3, 255, 4, 255, 5, 255, 6, 255, 7, 255, 8, 255, 9, 255, 10, 255, 11, 255, 12, 255, 13, 255, 14,
    255, 255, 255, 255, 0, 8, 255, 15, 1, 16, 255, 17, 2, 18, 255, 19, 3, 20, 255, 15, 4, 21, 255,
    22, 5, 23, 255, 24, 6, 25, 255, 26, 7, 255, 255, 1, 8, 27, 255, 28, 9, 29, 255, 30, 10, 31,
    255, 32, 11, 33, 255, 16, 12, 34, 255, 35, 13, 36, 255, 255, 14, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 0, 4, 8, 12, 255, 17, 15, 24, 1, 30, 16, 255, 255, 17, 17, 37, 2, 38, 18, 39,
    255, 37, 19, 40, 3, 41, 20, 255, 255, 2, 15, 28, 4, 42, 21, 43, 255, 38, 22, 44, 5, 45, 23,
    255, 255, 18, 24, 46, 6, 47, 25, 48, 255, 39, 26, 49, 7, 255, 255, 255, 255, 255, 1, 9, 8, 21,
    27, 34, 255, 37, 28, 46, 9, 50, 29, 255, 255, 19, 30, 50, 10, 51, 31, 52, 255, 40, 32, 53, 11,
    54, 33, 255, 255, 3, 16, 29, 12, 43, 34, 55, 255, 41, 35, 56, 13, 57, 36, 255, 255, 20, 255,
    255, 14, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 0, 2, 4, 6, 8, 10, 12, 14, 255, 15, 17, 19, 15, 22, 24, 26, 1, 28, 30, 32, 16, 35,
    255, 255, 255, 4, 17, 30, 17, 38, 37, 41, 2, 42, 38, 45, 18, 47, 39, 255, 255, 21, 37, 50, 19,
    51, 40, 54, 3, 43, 41, 57, 20, 255, 255, 255, 255, 255, 2, 10, 15, 22, 28, 35, 4, 38, 42, 47,
    21, 51, 43, 255, 255, 22, 38, 51, 22, 58, 44, 59, 5, 44, 45, 60, 23, 59, 255, 255, 255, 5, 18,
    31, 24, 44, 46, 56, 6, 45, 47, 61, 25, 60, 48, 255, 255, 23, 39, 52, 26, 59, 49, 62, 7, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 1, 5, 9, 13, 8, 18, 21, 25, 27, 31, 34, 255,
    255, 24, 37, 40, 28, 44, 46, 49, 9, 46, 50, 53, 29, 56, 255, 255, 255, 6, 19, 32, 30, 45, 50,
    57, 10, 47, 51, 60, 31, 61, 52, 255, 255, 25, 40, 53, 32, 60, 53, 63, 11, 48, 54, 63, 33, 255,
    255, 255, 255, 255, 3, 11, 16, 23, 29, 36, 12, 39, 43, 48, 34, 52, 55, 255, 255, 26, 41, 54,
    35, 59, 56, 62, 13, 49, 57, 63, 36, 62, 255, 255, 255, 7, 20, 33, 255, 255, 255, 255, 14, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 255, 255, 8, 15, 16, 17, 18, 19, 20, 15, 21,
    22, 23, 24, 25, 26, 255, 1, 27, 28, 29, 30, 31, 32, 33, 16, 34, 35, 36, 255, 255, 255, 255,
    255, 255, 4, 12, 17, 24, 30, 255, 17, 37, 38, 39, 37, 40, 41, 255, 2, 28, 42, 43, 38, 44, 45,
    255, 18, 46, 47, 48, 39, 49, 255, 255, 255, 9, 21, 34, 37, 46, 50, 255, 19, 50, 51, 52, 40, 53,
    54, 255, 3, 29, 43, 55, 41, 56, 57, 255, 20, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 2, 6, 10, 14, 15, 19, 22, 26, 28, 32, 35, 255, 4, 30, 38, 41, 42, 45, 47, 255, 21, 50, 51,
    54, 43, 57, 255, 255, 255, 10, 22, 35, 38, 47, 51, 255, 22, 51, 58, 59, 44, 60, 59, 255, 5, 31,
    44, 56, 45, 61, 60, 255, 23, 52, 59, 62, 255, 255, 255, 255, 255, 255, 5, 13, 18, 25, 31, 255,
    24, 40, 44, 49, 46, 53, 56, 255, 6, 32, 45, 57, 47, 60, 61, 255, 25, 53, 60, 63, 48, 63, 255,
    255, 255, 11, 23, 36, 39, 48, 52, 255, 26, 54, 59, 62, 49, 63, 62, 255, 7, 33, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    1, 3, 5, 7, 9, 11, 13, 255, 8, 16, 18, 20, 21, 23, 25, 255, 27, 29, 31, 33, 34, 36, 255, 255,
    255, 12, 24, 255, 37, 39, 40, 255, 28, 43, 44, 255, 46, 48, 49, 255, 9, 34, 46, 255, 50, 52,
    53, 255, 29, 55, 56, 255, 255, 255, 255, 255, 255, 255, 6, 14, 19, 26, 32, 255, 30, 41, 45,
    255, 50, 54, 57, 255, 10, 35, 47, 255, 51, 59, 60, 255, 31, 56, 61, 255, 52, 62, 255, 255, 255,
    13, 25, 255, 40, 49, 53, 255, 32, 57, 60, 255, 53, 63, 63, 255, 11, 36, 48, 255, 54, 62, 63,
    255, 33, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 3, 7, 11, 255, 16, 20, 23, 255,
    29, 33, 36, 255, 12, 255, 39, 255, 43, 255, 48, 255, 34, 255, 52, 255, 55, 255, 255, 255, 255,
    14, 26, 255, 41, 255, 54, 255, 35, 255, 59, 255, 56, 255, 62, 255, 13, 255, 49, 255, 57, 255,
    63, 255, 36, 255, 62, 255, 255, 255, 255, 255, 255, 255, 7, 255, 20, 255, 33, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 14, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
];

//const HALF_BIT_TIME: std::time::Duration = std::time::Duration::from_millis(250);
const BIT_TIME: std::time::Duration = std::time::Duration::from_millis(500);

/*
fn sleep_delta(last: &mut std::time::Instant, dur: std::time::Duration) {
    *last += dur;
    let now = std::time::Instant::now();
    if now >= *last {
        return;
    }
    std::thread::sleep(*last - now);
}
*/
async fn identify(driver: &mut dyn DaliDriver) -> Result<(), Box<dyn Error>> {
    let mut next = tokio::time::Instant::now();
    for s in 0..10 {
        send_device_cmd(driver, &Broadcast, cmd::GO_TO_SCENE_6 + s, PRIORITY_1)
            .await
            .check_send()?;
        next += BIT_TIME;
        tokio::time::sleep_until(next).await;
    }

    Ok(())
}

async fn identify_setup(
    driver: &mut dyn DaliDriver,
    space: u8,
    mark: u8,
) -> Result<(), Box<dyn Error>> {
    send_device_cmd(driver, &Broadcast, cmd::RECALL_MIN_LEVEL, NO_FLAG)
        .await
        .check_send()?;
    for i in 0..64 {
        match send_device_cmd(
            driver,
            &Short::new(i + 1),
            cmd::QUERY_DEVICE_TYPE,
            EXPECT_ANSWER,
        )
        .await
        {
            DaliSendResult::Answer(t) => {
                println!("Addr {} has type {}", i + 1, t);
            }
            DaliSendResult::Timeout => continue,
            e => return Err(Box::new(e)),
        }
        send_set_dtr0(driver, space, PRIORITY_5);
        for b in 0..10 {
            let cmd = cmd::SET_SCENE_6 + b;
            if (BIT_SEQ[i as usize] & (1 << b)) == 0 {
                send_device_cmd(driver, &Short::new(i + 1), cmd, SEND_TWICE)
                    .await
                    .check_send()?;
            }
        }
        send_set_dtr0(driver, mark, PRIORITY_5);
        for b in 0..10 {
            let cmd = cmd::SET_SCENE_6 + b;
            if (BIT_SEQ[i as usize] & (1 << b)) != 0 {
                send_device_cmd(driver, &Short::new(i + 1), cmd, SEND_TWICE)
                    .await
                    .check_send()?;
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = dali::drivers::init() {
        println!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = Command::new("identify")
        .about("Identify DALI gear")
        .arg(
            Arg::new("DEVICE")
                .short('d')
                .long("device")
                .default_value("default")
                .help("Select DALI-device"),
        )
        .arg(
            Arg::new("setup")
                .short('s')
                .long("setup")
                .default_value("false")
                .default_missing_value("true")
                .value_parser(value_parser!(bool))
                .help("Prepare groups for identification"),
        )
        .arg(
            Arg::new("repeat")
                .short('r')
                .long("repeat")
                .default_value("false")
                .default_missing_value("true")
                .value_parser(value_parser!(bool))
                .help("Repeat identification sequence"),
        )
        .arg(
            Arg::new("SPACE")
                .long("space")
                .default_value("150")
                .value_parser(value_parser!(u8))
                .help("Idle level"),
        )
        .arg(
            Arg::new("MARK")
                .long("mark")
                .default_value("200")
                .value_parser(value_parser!(u8))
                .help("Mark level"),
        )
        .get_matches();

    let setup = *matches.get_one::<bool>("setup").unwrap_or(&false);
    let repeat = *matches.get_one::<bool>("repeat").unwrap_or(&false);

    let space = match matches.try_get_one::<u8>("SPACE") {
        Ok(Some(&x)) if x <= 254 => x,
        Ok(Some(_)) => {
            println!("Space level out of range");
            return;
        }
        Ok(None) => {
            println!("Space level missing");
            return;
        }

        Err(e) => {
            println!("Space level invalid: {}", e);
            return;
        }
    };

    let mark = match matches.try_get_one::<u8>("MARK") {
        Ok(Some(&x)) if x <= 254 => x,
        Ok(Some(_)) => {
            println!("Mark level out of range");
            return;
        }
        Ok(None) => {
            println!("Mark level missing");
            return;
        }

        Err(e) => {
            println!("Mark level invalid: {}", e);
            return;
        }
    };
    println!("Space: {} Mark: {}", space, mark);
    let device_name = matches.get_one::<String>("DEVICE").unwrap();
    let mut driver = match dali::drivers::open(device_name) {
        Ok(d) => d,
        Err(e) => {
            println!("Failed to open DAIL device: {}", e);
            if let OpenError::NotFound = e {
                println!("Available drivers:");
                for name in dali::drivers::driver_names() {
                    println!("  {}", name);
                }
            }
            return;
        }
    };

    if setup {
        match identify_setup(&mut *driver, space, mark).await {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
            }
        };
    }

    loop {
        match identify(&mut *driver).await {
            Ok(_) => {}
            Err(e) => {
                println!("{}", e);
            }
        };
        if !repeat {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1500));
    }
}

#[test]
fn build_lookup() {
    let mut map = [0xffu8; 1024];
    for (i, b) in BIT_SEQ.iter().enumerate() {
        let mut b = *b as usize;
        for _ in 0..10 {
            assert!(map[b] == 0xff || map[b] == i as u8);
            map[b] = i as u8;
            b = (b >> 1) | ((b & 1) << 9);
        }
    }
    assert_eq!(map, SEQ_MAP);
    for a in map {
        println!("{},", a);
    }
}
