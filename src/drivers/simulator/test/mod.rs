use crate as dali;
use dali::drivers::simulator::simulator;
use dali::drivers::simulator::gear;
use futures::executor::block_on;
use dali::drivers::driver::DALIdriver;
use dali::utils::long_address;
use dali::drivers::driver::{self, DALIcommandError};
use dali::defs::gear::cmd;
use dali::utils::discover::{self, DiscoverItem};
use std::sync::{Arc,Mutex};
use futures::stream::StreamExt;

#[test]
fn create_sim()
{
    let mut sim = simulator::DALIsim::new();
    let res = block_on(sim.send_command(&[0xa1,00], 0));
    println!("Sent: {:?}", res);
}



#[test]
fn add_sim_device()
{
    let mut sim = simulator::DALIsim::new();
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123456;
    sim.add_device(Box::new(dev));
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123457;
    sim.add_device(Box::new(dev));
    
    match block_on(sim.send_command(&[cmd::INITIALISE,0x00],
                                    driver::SEND_TWICE)) {
        Err(err) => panic!("INITIALISE failed: {}", err),
        _ => {}
    };
    
    match block_on(long_address::set_search_addr(&mut sim,0x123456)) {
        Err(err) => panic!("set_search_addr failed: {}", err),
        _ => {}
    };
    
    match block_on(sim.send_command(&[cmd::COMPARE,0x00], driver::EXPECT_ANSWER))
    {
        Ok(0xff) => {},
        r => panic!("Compare failed: {:?}", r)
    }
    
    match block_on(long_address::set_search_addr(&mut sim,0x123455)) {
        Err(err) => panic!("set_search_addr failed: {}", err),
        _ => {}
    };
    
    match block_on(sim.send_command(&[cmd::COMPARE,0x00], driver::EXPECT_ANSWER))
    {
        Err(DALIcommandError::Timeout) => {},
        r => panic!("Compare failed: {:?}", r)
    }
    
    match block_on(long_address::set_search_addr(&mut sim,0x123457)) {
        Err(err) => panic!("set_search_addr failed: {}", err),
        _ => {}
    };
    
    match block_on(sim.send_command(&[cmd::COMPARE,0x00], driver::EXPECT_ANSWER))
    {
        Err(DALIcommandError::Framing) => {},
        r => panic!("Compare failed: {:?}", r)
    }
    
    let res = block_on(sim.send_command(&[cmd::TERMINATE,00], 0));
    println!("Sent: {:?}", res);
}

#[test]
fn discover()
{
    let mut sim = simulator::DALIsim::new();

    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x000000;
    sim.add_device(Box::new(dev));
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x000001;
    sim.add_device(Box::new(dev));

    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123456;
    sim.add_device(Box::new(dev));
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123457;
    sim.add_device(Box::new(dev));
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0x123457;
    sim.add_device(Box::new(dev));
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0xfffffd;
    sim.add_device(Box::new(dev));
    
    let mut dev = gear::DALIsimGear::new();
    dev.random_address = 0xfffffe;
    sim.add_device(Box::new(dev));

    
    let sim = Arc::new(Mutex::new(sim));
    let v = block_on(discover::find_quick(sim).collect::<Vec<DiscoverItem>>());
    println!("{:?}",v);
}
