use crate::drivers::driver::DALIcommandError;

pub trait DALIsimDevice
{
    fn power(&mut self, on: bool);
    fn forward16(&mut self,cmd: &[u8], flags:u16) 
                 ->Result<u8, DALIcommandError>;
}
