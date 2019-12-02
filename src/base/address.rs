use std::convert::TryFrom;
use std::convert::TryInto;

#[derive(Debug, Copy, Clone)]
struct BusAddress(u8);

#[derive(Debug, Copy, Clone, PartialEq)]
enum AddressType
{
    Short(ShortAddress),
    Group(GroupAddress),
    Broadcast
}
    
impl BusAddress
{
    
    pub fn new(a: u8) ->BusAddress
    {
        let a = a & 0xfe;
        assert!(a <= 64*2 || (a >= 0x80 && a < 0xa0) || a == 0xfe);
        BusAddress{0:a}
    }

    pub fn address(&self) ->AddressType
    {
        match self.0 >> 1 {
            a @ 0..=63 =>
                AddressType::Short(ShortAddress::new(a+1)),
            a @ 0x40..=0x4f => 
                AddressType::Group(GroupAddress::new((a & 0x0f) + 1)),
            _ => AddressType::Broadcast
        }
    }
}

impl std::convert::From<ShortAddress> for BusAddress
{
    fn from(a: ShortAddress) ->Self
    {
        Self::new((a.0-1)<<1)
    }
}

impl std::convert::From<&ShortAddress> for BusAddress
{
    fn from(a: &ShortAddress) ->Self
    {
        Self::new((a.0-1)<<1)
    }
}

impl std::convert::From<GroupAddress> for BusAddress
{
    fn from(a: GroupAddress) ->Self
    {
        Self::new((a.0-1)<<1 | 0x80)
    }
}

impl std::convert::From<&GroupAddress> for BusAddress
{
    fn from(a: &GroupAddress) ->Self
    {
        Self::new((a.0-1)<<1 | 0x80)
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

impl std::cmp::PartialEq<BusAddress> for BusAddress
{
    fn eq(&self, other: &BusAddress) -> bool
    {
        self.0 == other.0
    }
}

impl std::cmp::PartialEq<ShortAddress> for BusAddress
{
    fn eq(&self, other: &ShortAddress) -> bool
    {
        self.0 == BusAddress::from(other).0
    }
}

impl std::cmp::PartialEq<GroupAddress> for BusAddress
{
    fn eq(&self, other: &GroupAddress) -> bool
    {
        self.0 == BusAddress::from(other).0
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
            Err("Short address out of range. 1 <= addr <= 64")
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

impl std::cmp::PartialEq<ShortAddress> for ShortAddress
{
    fn eq(&self, other: &ShortAddress) -> bool
    {
        self.0 == other.0
    }
}

    
#[derive(Debug, Copy, Clone)]
pub struct GroupAddress(u8);

impl GroupAddress {
    pub fn new(a: u8) ->GroupAddress
    {
        assert!(a >= 1 && a <= 16);
        GroupAddress{0:a}
    }
}

impl std::convert::TryFrom<i32> for GroupAddress
{
    type Error = &'static str;
    fn try_from(a: i32) ->Result<Self, Self::Error>
    {
        if a >= 1 && a <= 16 {
            Ok(Self::new(a as u8))
        } else {
            Err("Group address out of range. 1 <= addr <= 16")
        }
    }
}

impl std::convert::TryFrom<BusAddress> for GroupAddress
{
    type Error = &'static str;
    fn try_from(a: BusAddress) ->Result<Self, Self::Error>
    {
        if a.0 >= 0x80 && a.0 < 0xa0{
            Ok(Self::new(((a.0>>1) & 0x0f) + 1))
        } else {
            Err("Not a group address")
        }
    }
}

impl std::cmp::PartialEq<GroupAddress> for GroupAddress
{
    fn eq(&self, other: &GroupAddress) -> bool
    {
        self.0 == other.0
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
    assert_eq!(b, ShortAddress::new(1));
    assert_eq!(b, BusAddress::new(0x00));
    
    let a = ShortAddress::try_from(64).unwrap();
    let b:BusAddress = a.into();
    assert_eq!(b, ShortAddress::new(64));
    assert_eq!(b, BusAddress::new(0x3f<<1));
    
    let a = ShortAddress::try_from(b).unwrap();
    assert_eq!(a, ShortAddress(64));
}

#[test]
fn group_address_test()
{
    let a:GroupAddress = 1.try_into().unwrap();
    let b:BusAddress = a.into();
    assert_eq!(b, GroupAddress::new(1));
    assert_eq!(b, BusAddress::new(0x80));
    
    let a = GroupAddress::try_from(16).unwrap();
    let b:BusAddress = a.into();
    assert_eq!(b, GroupAddress::new(16));
    assert_eq!(b, BusAddress::new(0x9e));
    
    let a = GroupAddress::try_from(b).unwrap();
    assert_eq!(a, GroupAddress(16));
}

#[test]
fn bus_address_test()
{
    let b = BusAddress::from(ShortAddress::new(7));
    assert_eq!(b.address(), AddressType::Short(ShortAddress::new(7)));
    
    let b = BusAddress::from(GroupAddress::new(13));
    assert_eq!(b.address(), AddressType::Group(GroupAddress::new(13)));
    
    let b = BusAddress::from(BroadcastAddress::new());
    assert_eq!(b.address(), AddressType::Broadcast);
}
