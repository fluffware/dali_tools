use dali_tools as dali;
use dali::drivers::helvar::helvar510::Helvar510driver;
use dali::base::address::{Short,Group,Address::Broadcast};
use dali::drivers::driver::{self, DALIdriver, DALIcommandError};
use dali::defs::gear::cmd;
use std::error::Error;


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
             
             
async fn identify(driver: &mut dyn DALIdriver, space: u8, mark: u8) -> Result<(), Box<dyn Error>>
{
    driver.send_command(&[cmd::DTR0, 0],0).await?;
    driver.send_device_cmd(&Broadcast, cmd::SET_FADE_TIME,
                           driver::SEND_TWICE).await?;
    driver.send_device_cmd(&Broadcast, cmd::SET_EXTENDED_FADE_TIME,
                           driver::SEND_TWICE).await?;
    let mut last = std::time::Instant::now();
    driver.send_device_level(&Broadcast, mark,0).await?;
    sleep_delta(&mut last, BIT_TIME);
    driver.send_device_level(&Broadcast, space,0).await?;
    let mut current = space;
    let mut next = mark;
    for b in 0..6 {
        println!("Current:: {} Next: {}", current, next);
        sleep_delta(&mut last, HALF_BIT_TIME);
        driver.send_device_level(&Group::new(8+b), next,0).await?;
        sleep_delta(&mut last, HALF_BIT_TIME);
        driver.send_device_level(&Broadcast, next,0).await?;
        std::mem::swap(&mut current, &mut next);
    }
    
    Ok(())
}
        
async fn identify_setup(driver: &mut dyn DALIdriver) -> Result<(), Box<dyn Error>>
{
    driver.send_device_cmd(&Broadcast,
                           cmd::RECALL_MIN_LEVEL,0).await?;
    for i in 0..64 {
        
        
        match driver.send_device_cmd(&Short::new(i+1),
                                     cmd::QUERY_DEVICE_TYPE,
                                     driver::EXPECT_ANSWER).await {
            Ok(t) => {
                println!("Addr {} has type {}", i+1, t);
            },
            Err(DALIcommandError::Timeout) => continue,
            Err(e) => return Err(Box::new(e))
        }
        driver.send_device_cmd(&Short::new(i+1),
                               cmd::RECALL_MAX_LEVEL,0).await?;
        for b in 0..6 {
            let cmd = b + if (i & (1<<b)) == 0 {
                cmd::REMOVE_FROM_GROUP_7
            } else {
                cmd::ADD_TO_GROUP_7
            };
            driver.send_device_cmd(&Short::new(i+1),
                                   cmd,driver::SEND_TWICE).await?;
        }
        driver.send_device_cmd(&Short::new(i+1),
                               cmd::RECALL_MIN_LEVEL,0).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
      let matches = 
        clap_app!(identify =>
                  (about: "Identify DALI gear")
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
    let driver = &mut Helvar510driver::new();

    if setup {
        match identify_setup(driver).await {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
            }
        };
    }

    loop {
        match identify(driver,space,mark).await {
            Ok(_) => {},
            Err(e) => {
                println!("{}", e);
            }
        };
        if !repeat {break}
        std::thread::sleep(std::time::Duration::from_millis(1500));
        
    }
}
