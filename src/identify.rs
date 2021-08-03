use dali_tools as dali;
use dali::base::address::{Short,Group,Address::Broadcast};
use dali::drivers::driver::{self, DaliDriver, DaliSendResult};
use dali::defs::gear::cmd;
use std::error::Error;
use dali::drivers::command_utils::{send_device_cmd, send_device_level};


#[macro_use]
extern crate clap;

const HALF_BIT_TIME: std::time::Duration= std::time::Duration::from_millis(250);
const BIT_TIME: std::time::Duration = std::time::Duration::from_millis(500);

fn sleep_delta(last: &mut std::time::Instant, dur: std::time::Duration)
{
    *last += dur;
    let now = std::time::Instant::now();
    if now >= *last {return}
    std::thread::sleep(*last - now);
}
             
             
async fn identify(driver: &mut dyn DaliDriver, space: u8, mark: u8) -> Result<(), Box<dyn Error>>
{
    driver.send_frame_16(&[cmd::DTR0, 0],0).await.check_send()?;
    send_device_cmd(driver, &Broadcast, cmd::SET_FADE_TIME,
                    driver::SEND_TWICE).await.check_send()?;
    send_device_cmd(driver, &Broadcast, cmd::SET_EXTENDED_FADE_TIME,
                    driver::SEND_TWICE).await.check_send()?;
    let mut last = std::time::Instant::now();
    send_device_level(driver, &Broadcast, mark,0).await.check_send()?;
    sleep_delta(&mut last, BIT_TIME);
    send_device_level(driver, &Broadcast, space,0).await.check_send()?;
    let mut current = space;
    let mut next = mark;
    for b in 0..6 {
        println!("Current:: {} Next: {}", current, next);
        sleep_delta(&mut last, HALF_BIT_TIME);
        send_device_level(driver, &Group::new(8+b), next,0).await.check_send()?;
        sleep_delta(&mut last, HALF_BIT_TIME);
        send_device_level(driver, &Broadcast, next,0).await.check_send()?;
        std::mem::swap(&mut current, &mut next);
    }
    
    Ok(())
}
        
async fn identify_setup(driver: &mut dyn DaliDriver) -> Result<(), Box<dyn Error>>
{
    send_device_cmd(driver, &Broadcast,
                    cmd::RECALL_MIN_LEVEL,0).await.check_send()?;
    for i in 0..64 {
        
        
        match send_device_cmd(driver, &Short::new(i+1),
                              cmd::QUERY_DEVICE_TYPE,
                              driver::EXPECT_ANSWER).await {
            DaliSendResult::Answer(t) => {
                println!("Addr {} has type {}", i+1, t);
            },
            DaliSendResult::Timeout => continue,
            e => return Err(Box::new(e))
        }
        send_device_cmd(driver, &Short::new(i+1),
                        cmd::RECALL_MAX_LEVEL,0).await.check_send()?;
        for b in 0..6 {
            let cmd = b + if (i & (1<<b)) == 0 {
                cmd::REMOVE_FROM_GROUP_7
            } else {
                cmd::ADD_TO_GROUP_7
            };
            send_device_cmd(driver, &Short::new(i+1),
                            cmd,driver::SEND_TWICE).await.check_send()?;
        }
        send_device_cmd(driver, &Short::new(i+1),
                        cmd::RECALL_MIN_LEVEL,0).await.check_send()?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = dali::drivers::init() {
	println!("Failed to initialize DALI drivers: {}", e);
    }
    let matches = 
        clap_app!(identify =>
                  (about: "Identify DALI gear")
		  (@arg DEVICE: -d --device "Select DALI-device")
                  (@arg setup: -s --setup "Prepare groups for identification")
                  (@arg repeat: -r --repeat "Repeat identification sequence")
                  (@arg SPACE: --space +takes_value "Idle level")
                  (@arg MARK: --mark +takes_value "Mark level")
      ).get_matches();

    let setup = matches.is_present("setup");
    let repeat = matches.is_present("repeat");

     let space = match u8::from_str_radix(matches.value_of("SPACE")
                                          .unwrap_or("150"),10){
        Ok(x) if x <= 254 => x,
        Ok(_) => {
            println!("Space level out of range");
            return
        }
        Err(e) => {
            println!("Space level invalid: {}",e);
            return
        }
     };
    
    let mark = match u8::from_str_radix(matches.value_of("MARK")
                                        .unwrap_or("200"),10){
        Ok(x) if x <= 254 => x,
        Ok(_) => {
            println!("Mark level out of range");
            return
        },
        Err(e) => {
            println!("Mark level invalid: {}",e);
            return
        }
    };
    println!("Space: {} Mark: {}", space,mark);
    let device_name = 
	matches.value_of("ADDR").unwrap_or_else(|| "default");
    let mut driver = match dali::drivers::open(device_name) {
	Ok(d) => d,
        Err(e) => {
            println!("Failed to open DAIL device: {}", e);
	    return;
        }
    };

    if setup {
        match identify_setup(&mut *driver).await {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
            }
        };
    }

    loop {
        match identify(&mut *driver,space,mark).await {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
            }
        };
        if !repeat {break}
        std::thread::sleep(std::time::Duration::from_millis(1500));
        
    }
}
