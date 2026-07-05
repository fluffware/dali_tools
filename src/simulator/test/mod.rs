use crate as dali;
use dali::defs::gear::cmd;
use dali::drivers::driver::{DaliDriver, DaliSendResult};
//use dali::drivers::simulator::gear;
use dali::drivers::simulator::simulator;
use dali::drivers::simulator::simulator_driver::DaliSimDriver;
use dali::utils::long_address;
//use dali::defs::gear::status;
use dali::drivers::send_flags::Flags;
use dali::utils::discover::{self, DiscoverItem};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::StreamExt;

/*
#[tokio::test]
async fn add_sim_device() {
    let sim = simulator::DaliBusSim::new().await.unwrap();
    let mut dev = gear::DaliSimGear::new();
    dev.random_address = 0x123456;
    sim.add_device(Box::new(dev)).await.unwrap();

    let mut dev = gear::DaliSimGear::new();
    dev.random_address = 0x123457;
    sim.add_device(Box::new(dev)).await.unwrap();
    let (mut driver, driver_dev) = DaliSimDriver::new();
    sim.add_device(driver_dev).await.unwrap();

    match driver
        .send_frame16(&[cmd::INITIALISE, 0x00], Flags::SendTwice(true))
        .await
    {
        DaliSendResult::OK => {}
        res => panic!("INITIALISE failed: {}", res),
    };

    match long_address::set_search_addr(&mut driver, 0x123456).await {
        Err(err) => panic!("set_search_addr failed: {}", err),
        _ => {}
    };

    match driver
        .send_frame16(&[cmd::COMPARE, 0x00], Flags::ExpectAnswer(true))
        .await
    {
        DaliSendResult::Answer(0xff) => {}
        r => panic!("Compare failed: {:?}", r),
    }

    match long_address::set_search_addr(&mut driver, 0x123455).await {
        Err(err) => panic!("set_search_addr failed: {}", err),
        _ => {}
    };

    match driver
        .send_frame16(&[cmd::COMPARE, 0x00], Flags::ExpectAnswer(true))
        .await
    {
        DaliSendResult::Timeout => {}
        r => panic!("Compare failed: {:?}", r),
    }

    match long_address::set_search_addr(&mut driver, 0x123457).await {
        Err(err) => panic!("set_search_addr failed: {}", err),
        _ => {}
    };

    match driver
        .send_frame16(&[cmd::COMPARE, 0x00], Flags::ExpectAnswer(true))
        .await
    {
        DaliSendResult::Framing => {}
        r => panic!("Compare failed: {:?}", r),
    }

    let res = driver
        .send_frame16(&[cmd::TERMINATE, 00], Flags::Empty)
        .await;
    println!("Sent: {:?}", res);
}

#[tokio::test]
async fn discover() {
    let sim = simulator::DaliBusSim::new().await.unwrap();

    let addrs = [0, 1, 0x123456, 0x123457, 0xfffffd, 0xfffffe, 0xffffff];

    for a in &addrs {
        let mut dev = gear::DaliSimGear::new();
        dev.random_address = *a;
        sim.add_device(Box::new(dev)).await.unwrap();
    }

    let mut dev = gear::DaliSimGear::new();
    dev.random_address = 0x123457;
    sim.add_device(Box::new(dev)).await.unwrap();

    let (driver, driver_dev) = DaliSimDriver::new();
    sim.add_device(driver_dev).await.unwrap();

    let sim = Arc::new(Mutex::new(
        Box::new(driver) as Box<dyn dali::drivers::driver::DaliDriver>
    ));
    let v = discover::find_quick(sim)
        .collect::<Vec<DiscoverItem>>()
        .await;
    for i in 0..v.len() {
        match &v[i] {
            Ok(d) => {
                if d.long != Some(addrs[i]) {
                    panic!(
                        "Address {:02x} found, expected {:06x}",
                        d.long.unwrap(),
                        addrs[i]
                    );
                }
            }
            Err(e) => panic!("Discovery failed: {}", e),
        }
    }

    //assert_eq!(v[3].as_ref().unwrap().long_conflict, true);
}

#[tokio::test]
async fn test_queries() {
    let sim = simulator::DaliBusSim::new().await.unwrap();

    let mut dev = gear::DaliSimGear::new();
    dev.random_address = 0x123456;
    dev.short_address = 3;
    dev.status = 0;
    dev.actual_level = 0;
    sim.add_device(Box::new(dev)).await.unwrap();

    let (mut driver, driver_dev) = DaliSimDriver::new();
    sim.add_device(driver_dev).await.unwrap();

    assert_eq!(
        driver
            .send_frame16(&[3 << 1, cmd::QUERY_STATUS], Flags::ExpectAnswer(true))
            .await
            .check_answer()
            .unwrap(),
        0u8
    );

    let mut dev = gear::DaliSimGear::new();
    dev.random_address = 0x123446;
    dev.short_address = 4;
    dev.status = 0x38;
    sim.add_device(Box::new(dev)).await.unwrap();

    assert_eq!(
        driver
            .send_frame16(&[4 << 1, cmd::QUERY_STATUS], Flags::ExpectAnswer(true))
            .await
            .check_answer()
            .unwrap(),
        0x3cu8
    );

    match driver
        .send_frame16(
            &[4 << 1, cmd::QUERY_CONTROL_GEAR_PRESENT],
            Flags::ExpectAnswer(true),
        )
        .await
    {
        DaliSendResult::Answer(0xff) => {}
        r => panic!("Invalid answer: {:?}", r),
    }

    match driver
        .send_frame16(
            &[5 << 1, cmd::QUERY_CONTROL_GEAR_PRESENT],
            Flags::ExpectAnswer(true),
        )
        .await
    {
        DaliSendResult::Answer(0xff) => panic!("Gear not present"),
        DaliSendResult::Timeout => {}
        r => panic!("Invalid answer: {:?}", r),
    }
}
*/
