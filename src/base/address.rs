use std::convert::TryFrom;
use std::convert::TryInto;

#[derive(Debug, Copy, Clone)]
struct BusAddress(u8);

impl BusAddress
{
    pub fn new(a: u8) ->BusAddress
    {
        let mut a = a & 0xfe;
        assert!(a <= 64*2 || (a >= 0x80 && a < 0x90) || a == 0xfe);
        BusAddress{0:a}
    }
}

impl std::convert::From<ShortAddress> for BusAddress
{
    fn from(a: ShortAddress) ->Self
    {
        Self::new((a.0-1)<<1)
    }
}

impl std::convert::From<BroadcastAddress> for BusAddress
{
    fn from(a: BroadcastAddress) ->Self
    {
        Self::new(0xff)
    }
}

impl std::convert::From<BusAddress> for u8
{
    fn from(a: BusAddress) ->Self
    {
        a.0
    }
}



#[derive(Debug, Copy, Clone)]
pub struct ShortAddress(u8);

impl ShortAddress {
    pub fn new(a: u8) ->ShortAddress
    {
        assert!(a >= 1 && a <= 64);
        ShortAddress{0:a}
    }
}

impl std::convert::TryFrom<i32> for ShortAddress
{
    type Error = &'static str;
    fn try_from(a: i32) ->Result<Self, Self::Error>
    {
        if a >= 1 && a <= 64 {
            Ok(Self::new(a as u8))
        } else {
            Err("Short address out of range. 0 <= addr <= 64")
        }
    }
}

impl std::convert::TryFrom<BusAddress> for ShortAddress
{
    type Error = &'static str;
    fn try_from(a: BusAddress) ->Result<Self, Self::Error>
    {
        if a.0 < 64 * 2 {
            Ok(Self::new((a.0>>1)+1))
        } else {
            Err("Not a short address")
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct BroadcastAddress {}

impl BroadcastAddress {
    pub fn new() ->BroadcastAddress
    {
        BroadcastAddress{}
    }
}

impl std::convert::TryFrom<BusAddress> for BroadcastAddress
{
    type Error = &'static str;
    fn try_from(a: BusAddress) ->Result<Self, Self::Error>
    {
        if a.0 == 0xfe {
            Ok(Self::new())
        } else {
            Err("Not a broadcast address")
        }
    }
}


pub const BROADCAST: Address = Address{bus_addr: 0xfe};

pub struct Address
{
    bus_addr: u8 // As it apears on the bus. Upper 7 bits used
}

pub fn device(dev: u8) -> Address
{
    assert!(dev >= 1 && dev <= 64);
    Address{bus_addr: (dev-1) << 1}
}
    
pub fn group(group: u8) -> Address
{
    assert!(group >= 1 && group <= 64);
    Address{bus_addr: ((group - 1) << 1) | 0x80}
}

impl Address
{
    pub fn is_group(&self) -> bool
    {
        (self.bus_addr & 0xc0) == 0x80
    }

    pub fn is_device(&self) -> bool
    {
        (self.bus_addr & 0x80) == 0
    }
    
    pub fn is_broadcast(&self) -> bool
    {
        self.bus_addr == 0xfe
    }
    
    pub fn to_device(&self) -> Option<u8>
    {
        if self.is_device() {
            Some((self.bus_addr >> 1) + 1)
        } else {
            None
        }
    }
    pub fn to_group(&self) -> Option<u8>
    {
        if self.is_group() {
            Some(((self.bus_addr & 0x3e) >> 1) + 1)
        } else {
            None
        }
    }

    pub fn to_bus_addr(&self) -> u8
    {
        self.bus_addr
    }
}

#[test]
fn short_address_test()
{
    let a:ShortAddress = 1.try_into().unwrap();
    let b:BusAddress = a.into();
    
    let a = ShortAddress::try_from(64).unwrap();
    let b:BusAddress = a.into();
    
    let a = ShortAddress::try_from(b).unwrap();
}
