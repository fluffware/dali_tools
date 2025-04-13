use crate::common::address::{AddressImpl, GroupImpl};

pub use crate::common::address::Short;

pub type Group = GroupImpl<16>;
pub type Address = AddressImpl<16>;

#[cfg(test)]
mod test {
    use super::{Address, Group};
    use crate::common::address::DisplayValue;
    use crate::common::address::Short;
    use std::convert::TryFrom;

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

}
