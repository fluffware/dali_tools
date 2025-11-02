use super::cmd_defs::AddressByte;
use core::ops::RangeInclusive;
use core::str::FromStr;

/// Value used for display, normally 1 based
pub trait DisplayValue {
    fn display_value(&self) -> u8;
    fn from_display_value<A>(value: A) -> Result<Self, AddressError>
    where
        A: TryInto<u8>,
        Self: Sized;
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AddressError {
    OK,
    NotShort,
    NotGroup,
    InvalidAddress,
}

impl std::fmt::Display for AddressError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        match self {
            AddressError::OK => write!(fmt, "OK"),
            AddressError::NotShort => write!(fmt, "Not a short address"),
            AddressError::NotGroup => write!(fmt, "Not a group address"),
            AddressError::InvalidAddress => write!(fmt, "InvalidAddress"),
        }
    }
}

impl std::error::Error for AddressError {}

#[derive(Debug, Copy, Clone)]
pub struct Short(u8);

impl Short {
    const DISPLAY_RANGE: RangeInclusive<u8> = 1..=64;
    pub fn new(a: u8) -> Short {
        assert!(a < 64);
        Short(a)
    }

    fn convert_display_value<A>(a: A) -> Result<u8, AddressError>
    where
        A: TryInto<u8>,
    {
        let Ok(a) = a.try_into() else {
            return Err(AddressError::InvalidAddress);
        };
        if !Self::DISPLAY_RANGE.contains(&a) {
            return Err(AddressError::InvalidAddress);
        }
        Ok(a - Self::DISPLAY_RANGE.start())
    }

    pub fn try_add(&self, add: i8) -> Result<Short, AddressError> {
        let a = (self.0 as i8 + add) as u8;
        if a <= 64 {
            Ok(Short(a))
        } else {
            Err(AddressError::InvalidAddress)
        }
    }

    /// Address 0..64
    pub fn value(&self) -> u8 {
        self.0
    }
}
/*
impl std::convert::TryFrom<i32> for Short {
type Error = AddressError;
    fn try_from(a: i32) -> Result<Self, Self::Error> {
        if a >= 1 && a <= 64 {
            Ok(Self::new(a as u8))
        } else {
            Err(AddressError::NotShort)
        }
    }
}
 */
impl std::cmp::PartialEq<Short> for Short {
    fn eq(&self, other: &Short) -> bool {
        self.0 == other.0
    }
}

impl std::cmp::Eq for Short {}

impl std::cmp::PartialOrd for Short {
    fn partial_cmp(&self, other: &Short) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Short {
    fn cmp(&self, other: &Short) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl From<Short> for AddressByte {
    fn from(short: Short) -> Self {
        AddressByte((short.0 << 1) | 1)
    }
}

impl From<Option<Short>> for AddressByte {
    fn from(short_or_mask: Option<Short>) -> AddressByte {
        if let Some(addr) = short_or_mask {
            AddressByte::from(addr)
        } else {
            AddressByte(0xff)
        }
    }
}

impl DisplayValue for Short {
    fn display_value(&self) -> u8 {
        self.0 + Self::DISPLAY_RANGE.start()
    }
    fn from_display_value<A>(a: A) -> Result<Short, AddressError>
    where
        A: TryInto<u8>,
    {
        Self::convert_display_value(a).map(Short)
    }
}

impl<const MAX_GROUP: u8> std::convert::TryFrom<AddressImpl<MAX_GROUP>> for Short {
    type Error = AddressError;
    fn try_from(addr: AddressImpl<MAX_GROUP>) -> Result<Short, Self::Error> {
        if let AddressImpl::Short(s) = addr {
            Ok(s)
        } else {
            Err(AddressError::NotShort)
        }
    }
}
impl std::fmt::Display for Short {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        self.display_value().fmt(fmt)
    }
}

impl FromStr for Short {
    type Err = AddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u8::from_str(s).map_or(Err(AddressError::InvalidAddress), |a| {
            Self::from_display_value(a)
        })
    }
}

pub type Long = u32;

#[derive(Debug, Copy, Clone)]
pub struct GroupImpl<const MAX: u8>(u8);

impl<const MAX: u8> GroupImpl<MAX> {
    const DISPLAY_RANGE: RangeInclusive<u8> = 1..=MAX;
    pub fn new(a: u8) -> GroupImpl<MAX> {
        assert!(a < MAX);
        GroupImpl(a)
    }

    fn convert_display_value<A>(a: A) -> Result<u8, AddressError>
    where
        A: TryInto<u8>,
    {
        let Ok(a) = a.try_into() else {
            return Err(AddressError::InvalidAddress);
        };
        if !Self::DISPLAY_RANGE.contains(&a) {
            return Err(AddressError::InvalidAddress);
        }
        Ok(a - Self::DISPLAY_RANGE.start())
    }
}

/*
impl<const MAX: u8> std::convert::TryFrom<i32> for GroupImpl<MAX> {
    type Error = AddressError;
    fn try_from(a: i32) -> Result<Self, Self::Error> {
        if a >= 1 && a <= i32::from(MAX) {
            Ok(Self::new(a as u8))
        } else {
            Err(AddressError::NotGroup)
        }
    }
}
*/

impl<const MAX_GROUP: u8> std::convert::TryFrom<AddressImpl<MAX_GROUP>> for GroupImpl<MAX_GROUP> {
    type Error = AddressError;
    fn try_from(addr: AddressImpl<MAX_GROUP>) -> Result<GroupImpl<MAX_GROUP>, Self::Error> {
        if let AddressImpl::Group(g) = addr {
            Ok(g)
        } else {
            Err(AddressError::NotGroup)
        }
    }
}
impl<const MAX: u8> std::cmp::PartialEq<GroupImpl<MAX>> for GroupImpl<MAX> {
    fn eq(&self, other: &GroupImpl<MAX>) -> bool {
        self.0 == other.0
    }
}

impl<const MAX: u8> From<GroupImpl<MAX>> for AddressByte {
    fn from(group: GroupImpl<MAX>) -> AddressByte {
        AddressByte((group.0 << 1) | 0x81)
    }
}

impl<const MAX: u8> DisplayValue for GroupImpl<MAX> {
    fn display_value(&self) -> u8 {
        self.0 + Self::DISPLAY_RANGE.start()
    }

    fn from_display_value<A>(a: A) -> Result<GroupImpl<MAX>, AddressError>
    where
        A: TryInto<u8>,
    {
        Self::convert_display_value(a).map(GroupImpl)
    }
}

impl<const MAX: u8> std::fmt::Display for GroupImpl<MAX> {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::result::Result<(), std::fmt::Error> {
        self.0.fmt(fmt)
    }
}

impl<const MAX: u8> FromStr for GroupImpl<MAX> {
    type Err = AddressError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u8::from_str(s).map_or(Err(AddressError::InvalidAddress), |a| {
            Self::from_display_value(a)
        })
    }
}
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AddressImpl<const MAX_GROUP: u8> {
    Short(Short),
    Group(GroupImpl<MAX_GROUP>),
    Broadcast,
    BroadcastUnaddressed,
}

impl<const MAX_GROUP: u8> AddressImpl<MAX_GROUP> {
    pub fn from_bus_address(bus: u8) -> Result<AddressImpl<MAX_GROUP>, AddressError> {
        match bus >> 1 {
            a @ 0..64 => Ok(AddressImpl::Short(Short::new(a))),
            a @ 0x40..=0x4f => Ok(AddressImpl::Group(GroupImpl::new(a & 0x0f))),
            0x7f => Ok(AddressImpl::Broadcast),
            _ => Err(AddressError::InvalidAddress),
        }
    }
}

impl<const MAX_GROUP: u8> std::convert::From<Short> for AddressImpl<MAX_GROUP> {
    fn from(a: Short) -> Self {
        AddressImpl::Short(a)
    }
}

impl<const MAX_GROUP: u8> std::convert::From<GroupImpl<MAX_GROUP>> for AddressImpl<MAX_GROUP> {
    fn from(a: GroupImpl<MAX_GROUP>) -> Self {
        AddressImpl::Group(a)
    }
}

impl<const MAX_GROUP: u8> std::cmp::PartialEq<Short> for AddressImpl<MAX_GROUP> {
    fn eq(&self, other: &Short) -> bool {
        match self {
            AddressImpl::Short(a) => a == other,
            _ => false,
        }
    }
}

impl<const MAX_GROUP: u8> std::cmp::PartialEq<GroupImpl<MAX_GROUP>> for AddressImpl<MAX_GROUP> {
    fn eq(&self, other: &GroupImpl<MAX_GROUP>) -> bool {
        match self {
            AddressImpl::Group(a) => a == other,
            _ => false,
        }
    }
}
impl<const MAX_GROUP: u8> From<AddressImpl<MAX_GROUP>> for AddressByte {
    fn from(addr: AddressImpl<MAX_GROUP>) -> AddressByte {
        match addr {
            AddressImpl::Short(a) => a.into(),
            AddressImpl::Group(a) => a.into(),
            AddressImpl::Broadcast => AddressByte(0xff),
            AddressImpl::BroadcastUnaddressed => AddressByte(0xfd),
        }
    }
}
