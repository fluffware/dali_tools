use super::super::driver::{DALIdriver, DALIcommandError};
use futures::future::{Future, FutureExt};
use std::pin::Pin;
use futures::lock::Mutex;
use std::sync::Arc;

#[derive(Debug,Clone)]
enum SimDriverError
{
    OK
}

struct DALIsimCtxt
{
}

pub struct DALIsim
{
    ctxt: Arc<Mutex<DALIsimCtxt>>
}


impl DALIsim
{
    pub fn new() -> DALIsim
    {
        DALIsim{ctxt: Arc::new(Mutex::new(DALIsimCtxt{}))}
    }
}

async fn sim_driver(_driver: Arc<Mutex<DALIsimCtxt>>, _cmd: [u8;2], _flags:u16)
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

