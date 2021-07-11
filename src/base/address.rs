
pub trait BusAddress
{
    fn bus_address(&self) -> u8;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Address
{
    Short(Short),
    Group(Group),
    Broadcast
}


#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AddressError
{
    OK,
    NotShort,
    NotGroup,
    InvalidAddress
}

impl std::fmt::Display for AddressError
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter) 
       -> std::result::Result<(), std::fmt::Error>
    {
        match self {
            AddressError::OK => write!(fmt, "OK"),
            AddressError::NotShort => write!(fmt, "Not a short address"),
            AddressError::NotGroup => write!(fmt, "Not a group address"),
            AddressError::InvalidAddress => write!(fmt, "InvalidAddress")
        }
    }
}

impl std::error::Error for AddressError
{
}

impl Address
{
    pub fn from_bus_address(bus: u8) -> Result<Address, AddressError>
    {
        match bus >> 1 {
            a @ 0..=63 =>
                Ok(Address::Short(Short::new(a+1))),
            a @ 0x40..=0x4f =>
                Ok(Address::Group(Group::new((a & 0x0f) + 1))),
            0x7f => Ok(Address::Broadcast),
            _ => Err(AddressError::InvalidAddress)
        }
    }
}

impl std::convert::From<Short> for Address
{
    fn from(a: Short) ->Self
    {
        Address::Short(a)
    }
}

impl std::convert::From<Group> for Address
{
    fn from(a: Group) ->Self
    {
        Address::Group(a)
    }
}

impl std::cmp::PartialEq<Short> for Address
{
    fn eq(&self, other: &Short) -> bool
    {
        match self {
            Address::Short(a) => a == other,
            _ => false
        }
    }
}

impl std::cmp::PartialEq<Group> for Address
{
    fn eq(&self, other: &Group) -> bool
    {
        match self {
            Address::Group(a) => a == other,
            _ => false
        }
    }
}

impl BusAddress for Address
{
    fn bus_address(&self) ->u8
    {
        match self {
            Address::Short(a) => a.bus_address(),
            Address::Group(a) => a.bus_address(),
            Address::Broadcast => 0xfe
        }
    }
}
#[derive(Debug, Copy, Clone)]
pub struct Short(u8);

impl Short {
    pub fn new(a: u8) ->Short
    {
        assert!(a >= 1 && a <= 64);
        Short{0:a}
    }
}

impl std::convert::TryFrom<i32> for Short
{
    type Error = AddressError;
    fn try_from(a: i32) ->Result<Self, Self::Error>
    {
        if a >= 1 && a <= 64 {
            Ok(Self::new(a as u8))
        } else {
            Err(AddressError::NotShort)
        }
    }
}

impl std::convert::TryFrom<Address> for Short
{
    type Error = AddressError;
    fn try_from(any: Address) ->Result<Self, Self::Error>
    {
        if let Address::Short(addr) = any {
            Ok(addr)
        } else {
            Err(AddressError::NotShort)
        }
    }
}

impl std::cmp::PartialEq<Short> for Short
{
    fn eq(&self, other: &Short) -> bool
    {
        self.0 == other.0
    }
}

impl BusAddress for Short
{
    fn bus_address(&self) ->u8
    {
        (self.0 - 1) << 1
    }
}

impl std::fmt::Display for Short
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter) 
           -> std::result::Result<(), std::fmt::Error>
    {
        self.0.fmt(fmt)
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Group(u8);

impl Group {
    pub fn new(a: u8) ->Group
    {
        assert!(a >= 1 && a <= 16);
        Group{0:a}
    }
}

impl std::convert::TryFrom<i32> for Group
{
    type Error = AddressError;
    fn try_from(a: i32) ->Result<Self, Self::Error>
    {
        if a >= 1 && a <= 16 {
            Ok(Self::new(a as u8))
        } else {
            Err(AddressError::NotGroup)
        }
    }
}

impl std::convert::TryFrom<Address> for Group
{
    type Error = AddressError;
    fn try_from(any: Address) ->Result<Self, Self::Error>
    {
        if let Address::Group(addr) = any {
            Ok(addr)
        } else {
            Err(AddressError::NotGroup)
        }
    }
}

impl std::cmp::PartialEq<Group> for Group
{
    fn eq(&self, other: &Group) -> bool
    {
        self.0 == other.0
    }
}

impl BusAddress for Group
{
    fn bus_address(&self) ->u8
    {
        ((self.0 - 1) << 1) | 0x80
    }
}
    
impl std::fmt::Display for Group
{
    fn fmt(&self, fmt: &mut std::fmt::Formatter) 
           -> std::result::Result<(), std::fmt::Error>
    {
        self.0.fmt(fmt)
    }
}

pub type Long = u32;

#[cfg(test)]
use std::convert::TryFrom;

#[test]
fn short_address_test()
{
    let a:Short = Short::new(1);
    let b:Address = a.into();
    assert_eq!(b, Short::new(1));
    assert_eq!(b, Address::from_bus_address(0x00).unwrap());
    
    let a = Short::new(64);
    let b:Address = a.into();
    assert_eq!(b, Short::new(64));
    assert_eq!(b, Address::from_bus_address(0x3f<<1).unwrap());
    
    let a = Short::try_from(b).unwrap();
    assert_eq!(a, Short::new(64));
}

#[test]
fn group_address_test()
{
    let a:Group = Group::new(1);
    let b:Address = a.into();
    assert_eq!(b, Group::new(1));
    assert_eq!(b, Address::from_bus_address(0x80).unwrap());
    
    let a = Group::try_from(16).unwrap();
    let b:Address = a.into();
    assert_eq!(b, Group::new(16));
    assert_eq!(b, Address::from_bus_address(0x9e).unwrap());
    
    let a = Group::try_from(b).unwrap();
    assert_eq!(a, Group(16));
}

#[cfg(test)]

fn use_any_bus_address(bus_addr: &dyn BusAddress) -> u8
{
    bus_addr.bus_address()
}

#[test]
fn bus_address_test()
{
    let b = Address::from(Short::new(7));
    assert_eq!(b, Address::Short(Short::new(7)));
    
    let b = Address::from(Group::new(13));
    assert_eq!(b, Address::Group(Group::new(13)));
    assert_eq!( use_any_bus_address(&Short::new(7)),(7-1)<<1);
    assert_eq!( use_any_bus_address(&Group::new(12)),(12-1)<<1 | 0x80);
    assert_eq!( use_any_bus_address(&Address::Broadcast),0xfe);
}


