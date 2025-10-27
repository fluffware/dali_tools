use crate::common::address::Short;
use core::future::Future;
// Commands that are common for gears, app controllers and input devices

/// Check for specific kinds of errors
pub trait ErrorInfo {
    fn is_timeout(&self) -> bool;
    fn is_framing_error(&self) -> bool;
}

pub enum YesNo {
    Yes,
    No,
    Multiple,
}

pub trait Commands {
    type Address;
    type Error: ErrorInfo + Send + 'static;
    fn initialise_all(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn initialise_no_addr(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn initialise_addr(
        &mut self,
        addr: Short,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn terminate(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn randomize(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn compare(&mut self) -> impl Future<Output = Result<YesNo, Self::Error>> + Send;
    fn withdraw(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn searchaddr_h(&mut self, h: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn searchaddr_m(&mut self, m: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn searchaddr_l(&mut self, l: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn program_short_address(
        &mut self,
        addr: Option<Short>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn verify_short_address(
        &mut self,
        add: Short,
    ) -> impl Future<Output = Result<YesNo, Self::Error>> + Send;

    /// Request short address for devces whose long address matches the search address.
    ///
    /// Returns None if no address set
    fn query_short_address(
        &mut self,
    ) -> impl Future<Output = Result<Option<Short>, Self::Error>> + Send;
    fn dtr0(&mut self, data: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn dtr1(&mut self, data: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn dtr2(&mut self, data: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn write_memory_location(
        &mut self,
        data: u8,
    ) -> impl Future<Output = Result<u8, Self::Error>> + Send;
    fn write_memory_location_no_reply(
        &mut self,
        data: u8,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;

    fn query_random_address(
        &mut self,
        device: Short,
    ) -> impl Future<Output = Result<u32, Self::Error>> + Send;
    fn read_memory_location(
        &mut self,
        device: Short,
    ) -> impl Future<Output = Result<u8, Self::Error>> + Send;
    fn identify_device(
        &mut self,
        device: Self::Address,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
