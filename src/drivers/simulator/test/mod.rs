use crate as dali;
use dali::drivers::simulator::simulator;
use futures::executor::block_on;
use dali::drivers::driver::DALIdriver;
#[test]
fn create_sim()
{
    let mut sim = simulator::DALIsim::new();
    let res = block_on(sim.send_command(&[0xa1,00], 0));
    println!("Sent: {:?}", res);
}
