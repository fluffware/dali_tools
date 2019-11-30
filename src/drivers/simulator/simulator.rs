use super::super::driver::{self,DALIdriver, DALIcommandError};
use futures::future::{Future, FutureExt};
use std::pin::Pin;
use futures_locks::Mutex;

struct DALIsimCtxt
{
}

pub struct DALIsim
{
    ctxt: Mutex<DALIsimCtxt>
}

impl DALIsim
{
    pub fn new() -> DALIsim
    {
        DALIsim{ctxt: Mutex::new(DALIsimCtxt{})}
    }
}

async fn sim_driver(driver: Mutex<DALIsimCtxt>, cmd: [u8;2], flags:u16)
    -> Result<u8, DALIcommandError>
{
    Ok(0)
}

impl DALIdriver for DALIsim
{
    fn send_command(&mut self, cmd: & [u8;2], flags:u16) -> 
        Pin<Box<dyn Future<Output = Result<u8, DALIcommandError>> + Send>>
    {
        let cmd = cmd.clone();
        sim_driver(self.ctxt.clone(), cmd, flags).boxed()
    }
}

