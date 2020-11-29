use crate as dali;
use dali::drivers::simulator::simulator;
use dali::drivers::simulator::gear;
use dali::drivers::driver::DALIdriver;
use dali::utils::long_address;
use dali::drivers::driver::{self, DALIcommandError};
use dali::defs::gear::cmd;
use dali::defs::gear::status;
use dali::utils::discover::{self, DiscoverItem};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::stream::StreamExt;

#[tokio::test]
async fn create_sim()
{
    let mut sim = simulator::DALIsim::new();
    let res = sim.send_command(&[0xa1,00], 0).await;
    println!("Sent: {:?}", res);
}



#[tokio::test]
async fn add_sim_device()
{
    let mut sim = simulator::DALIsim::new();
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123456;
    sim.add_device(Box::new(dev));
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123457;
    sim.add_device(Box::new(dev));
    
    match sim.send_command(&[cmd::INITIALISE,0x00],
                           driver::SEND_TWICE).await {
        Err(err) => panic!("INITIALISE failed: {}", err),
        _ => {}
    };
    
    match long_address::set_search_addr(&mut sim,0x123456).await {
        Err(err) => panic!("set_search_addr failed: {}", err),
        _ => {}
    };
    
    match sim.send_command(&[cmd::COMPARE,0x00], driver::EXPECT_ANSWER).await
    {
        Ok(0xff) => {},
        r => panic!("Compare failed: {:?}", r)
    }
    
    match long_address::set_search_addr(&mut sim,0x123455).await {
        Err(err) => panic!("set_search_addr failed: {}", err),
        _ => {}
    };
    
    match sim.send_command(&[cmd::COMPARE,0x00], driver::EXPECT_ANSWER).await
    {
        Err(DALIcommandError::Timeout) => {},
        r => panic!("Compare failed: {:?}", r)
    }
    
    match long_address::set_search_addr(&mut sim,0x123457).await {
        Err(err) => panic!("set_search_addr failed: {}", err),
        _ => {}
    };
    
    match sim.send_command(&[cmd::COMPARE,0x00], driver::EXPECT_ANSWER).await
    {
        Err(DALIcommandError::Framing) => {},
        r => panic!("Compare failed: {:?}", r)
    }
    
    let res = sim.send_command(&[cmd::TERMINATE,00], 0).await;
    println!("Sent: {:?}", res);
}

#[tokio::test]
async fn discover()
{
    let sim = simulator::DALIsim::new();

    let addrs = [0,1,0x123456, 0x123457,  0xfffffd, 0xfffffe, 0xffffff];

    for a in &addrs {
        let mut dev = gear::DALIsimGear::new();
        dev.random_address = *a;
        sim.add_device(Box::new(dev));
    }
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123457;
    sim.add_device(Box::new(dev));

    let sim = Arc::new(Mutex::new(Box::new(sim) as Box<dyn dali::drivers::driver::DALIdriver>));
    let v = discover::find_quick(sim).collect::<Vec<DiscoverItem>>().await;
    for i in 0..v.len() {
        match &v[i] {
            Ok(d) => {
                if d.long != Some(addrs[i]) {
                    panic!("Address {:02x} found, expected {:06x}",
                           d.long.unwrap(), addrs[i]);
                }
            },
            Err(e) => panic!("Discovery failed: {}", e)
        }
    }

    assert_eq!(v[3].as_ref().unwrap().long_conflict, true);
}

#[tokio::test]
async fn test_queries()
{
    let mut sim = simulator::DALIsim::new();
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123456;
    dev.short_address = 3;
    dev.status = 0;
    dev.actual_level = 0;
    sim.add_device(Box::new(dev));

    assert_eq!(sim.send_command(&[3<<1, cmd::QUERY_STATUS], 
                                         driver::EXPECT_ANSWER).await.unwrap(),
               0u8);
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123446;
    dev.short_address = 4;
    dev.status = 0x38;
    sim.add_device(Box::new(dev));
    
    assert_eq!(sim.send_command(&[4<<1, cmd::QUERY_STATUS], 
                                driver::EXPECT_ANSWER).await.unwrap(),
               0x3cu8);

    match sim.send_command(&[4<<1, cmd::QUERY_CONTROL_GEAR_PRESENT],
                           driver::EXPECT_ANSWER).await {
        Ok(0xff) => {},
        r => panic!("Invalid answer: {:?}", r)
    }
    
    match sim.send_command(&[5<<1, cmd::QUERY_CONTROL_GEAR_PRESENT],
                           driver::EXPECT_ANSWER).await {
        Ok(0xff) => panic!("Gear not present"),
        Err(DALIcommandError::Timeout) => {},
        r => panic!("Invalid answer: {:?}", r)
    }
}
