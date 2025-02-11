use crate::common::address::{AddressImpl, GroupImpl};

pub use crate::common::address::Short;

pub type Group = GroupImpl<16>;
pub type Address = AddressImpl<16>;

#[cfg(test)]
mod test {
    use super::{Address,Group};
    use crate::common::address::{BusAddress,  Short};
    use std::convert::TryFrom;
    use crate::common::address::DisplayValue;
    
    #[test]
    fn short_address_test() {
        let a: Short = Short::new(1);
        let b: Address = a.into();
        assert_eq!(b, Short::new(1));
        assert_eq!(b, Address::from_bus_address(0x02).unwrap());

        let a = Short::new(63);
        let b: Address = a.into();
        assert_eq!(b, Short::new(63));
        assert_eq!(b, Address::from_bus_address(0x3f << 1).unwrap());

        let a = Short::try_from(b).unwrap();
        assert_eq!(a, Short::new(63));
    }
    #[test]
    fn group_address_test() {
        let a: Group = Group::new(0);
        let b: Address = a.into();
        assert_eq!(b, Group::new(0));
        assert_eq!(b, Address::from_bus_address(0x80).unwrap());

        let a = Group::from_display_value(16).unwrap();
        let b: Address = a.into();
        assert_eq!(b, Group::new(15));
        assert_eq!(b, Address::from_bus_address(0x9e).unwrap());

        let a = Group::try_from(b).unwrap();
        assert_eq!(a, Group::new(15));
    }

    #[cfg(test)]

    fn use_any_bus_address(bus_addr: &dyn BusAddress) -> u8 {
        bus_addr.bus_address()
    }

    #[test]
    fn bus_address_test() {
        let b = Address::from(Short::new(7));
        assert_eq!(b, Address::Short(Short::new(7)));

        let b = Address::from(Group::new(13));
        assert_eq!(b, Address::Group(Group::new(13)));
        assert_eq!(use_any_bus_address(&Short::new(7)), (7) << 1);
        assert_eq!(use_any_bus_address(&Group::new(12)), (12 - 1) << 1 | 0x80);
        assert_eq!(use_any_bus_address(&Address::Broadcast), 0xfe);
    }
}
