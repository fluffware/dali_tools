// Commands that are common for gears, app controllers and input devices

trait Commands {
    type Address;
    type Short;
    type Error;
    async fn initialize(device: u8) -> Result<(), Self::Error>;
    async fn terminate() -> Result<(), Self::Error>;
    async fn randomize() -> Result<(), Self::Error>;
    async fn compare() -> Result<(), Self::Error>;
    async fn withdraw() -> Result<(), Self::Error>;
    async fn searchaddr_h(h: u8) -> Result<(), Self::Error>;
    async fn searchaddr_m(m: u8) -> Result<(), Self::Error>;
    async fn searchaddr_l(l: u8) -> Result<(), Self::Error>;
    async fn program_short_address(addr: Self::Short) -> Result<(), Self::Error>;
    async fn verify_short_address(add: Self::Short) -> Result<bool, Self::Error>;
    async fn query_short_address(addr: Self::Short) -> Result<Self::Short, Self::Error>;
    async fn dtr0(data: u8) -> Result<(), Self::Error>;
    async fn dtr1(data: u8) -> Result<(), Self::Error>;
    async fn dtr2(data: u8) -> Result<(), Self::Error>;
    async fn write_memory_location(data: u8) -> Result<u8, Self::Error>;
    async fn write_memory_location_no_reply(data: u8) -> Result<(), Self::Error>;

    async fn query_random_address() -> Result<u32, Self::Error>;
    async fn read_memory_location() -> Result<u8, Self::Error>;
    async fn identify_device() -> Result<(), Self::Error>;
}
