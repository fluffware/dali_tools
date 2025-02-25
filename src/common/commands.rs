use core::future::Future;
// Commands that are common for gears, app controllers and input devices

pub trait Commands {
    type Address;
    type Short;
    type Error;
    fn initialize(&mut self, device: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn terminate(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn randomize(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn compare(&mut self) -> impl Future<Output = Result<bool, Self::Error>> + Send;
    fn withdraw(&mut self) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn searchaddr_h(&mut self, h: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn searchaddr_m(&mut self, m: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn searchaddr_l(&mut self, l: u8) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn program_short_address(
        &mut self,
        addr: Self::Short,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
    fn verify_short_address(
        &mut self,
        add: Self::Short,
    ) -> impl Future<Output = Result<bool, Self::Error>> + Send;
    fn query_short_address(
        &mut self,
    ) -> impl Future<Output = Result<Self::Short, Self::Error>> + Send;
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
        device: Self::Address,
    ) -> impl Future<Output = Result<u32, Self::Error>> + Send;
    fn read_memory_location(
        &mut self,
        device: Self::Address,
    ) -> impl Future<Output = Result<u8, Self::Error>> + Send;
    fn identify_device(
        &mut self,
        device: Self::Address,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}
