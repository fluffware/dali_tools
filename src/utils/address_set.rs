use core::ops::RangeInclusive;
use core::ops::{Add, AddAssign, Sub, SubAssign};
use crate::common::address::Short;

#[derive(PartialEq, Debug, Clone)]
pub struct AddressSet(u64);

impl AddressSet {
    pub fn new() -> AddressSet {
        AddressSet(0)
    }

    pub fn from_slice(addrs: &[Short]) -> AddressSet {
        let mut s = 0u64;
        for addr in addrs {
            s |= 1 << addr.value();
        }
        AddressSet(s)
    }

    pub fn to_vec(&self) -> Vec<Short> {
        (0..64)
            .filter_map(|b| {
                if (self.0 & 1 << b) != 0 {
                    Some(Short::new(b))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn from_range(addrs: RangeInclusive<Short>) -> AddressSet {
        if addrs.is_empty() {
            return AddressSet(0);
        }
        let start_bit = 1u64 << addrs.start().value();
        let end_bit = 1u64 << addrs.end().value();
        let s = !(start_bit - 1) & ((end_bit - 1) + end_bit);
        AddressSet(s)
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    pub fn contains(&self, addr: Short) -> bool {
	(self.0 & (1<< addr.value())) != 0
    }
}

impl Add<Short> for AddressSet {
    type Output = AddressSet;
    fn add(self, b: Short) -> Self::Output {
        Self(self.0 | (1u64 << b.value()))
    }
}

impl AddAssign<Short> for AddressSet {
    fn add_assign(&mut self, b: Short) {
        self.0 |= 1u64 << b.value();
    }
}

impl Add<AddressSet> for AddressSet {
    type Output = AddressSet;
    fn add(self, b: AddressSet) -> Self::Output {
        Self(self.0 | b.0)
    }
}

impl Add<&AddressSet> for AddressSet {
    type Output = AddressSet;
    fn add(self, b: &AddressSet) -> Self::Output {
        Self(self.0 | b.0)
    }
}

impl AddAssign<&AddressSet> for AddressSet {
    fn add_assign(&mut self, b: &AddressSet) {
        self.0 |= b.0;
    }
}

impl Sub<Short> for AddressSet {
    type Output = AddressSet;
    fn sub(self, b: Short) -> Self::Output {
        Self(self.0 & !(1u64 << b.value()))
    }
}

impl SubAssign<Short> for AddressSet {
    fn sub_assign(&mut self, b: Short) {
        self.0 &= !(1u64 << b.value());
    }
}

impl Sub<AddressSet> for AddressSet {
    type Output = AddressSet;
    fn sub(self, b: AddressSet) -> Self::Output {
        Self(self.0 & !b.0)
    }
}

impl Sub<&AddressSet> for AddressSet {
    type Output = AddressSet;
    fn sub(self, b: &AddressSet) -> Self::Output {
        Self(self.0 & !b.0)
    }
}

impl SubAssign<AddressSet> for AddressSet {
    fn sub_assign(&mut self, b: AddressSet) {
        self.0 &= !b.0;
    }
}

impl SubAssign<&AddressSet> for AddressSet {
    fn sub_assign(&mut self, b: &AddressSet) {
        self.0 &= !b.0;
    }
}

#[cfg(test)]
mod test {
    use super::AddressSet;
    use crate::common::address::Short;

    #[test]
    fn add_test() {
        let a = AddressSet::new();
        let mut b = a + Short::new(5);
        assert_eq!(b, AddressSet::from_slice(&[Short::new(5)]));
        b += Short::new(9);
        assert_eq!(b, AddressSet::from_slice(&[Short::new(9), Short::new(5)]));
    }

    #[test]
    fn sub_test() {
        let a = AddressSet::from_slice(&[Short::new(4), Short::new(7), Short::new(6)]);
        let mut b = a - Short::new(7);
        assert_eq!(b, AddressSet::from_slice(&[Short::new(6), Short::new(4)]));
        b -= Short::new(6);
        assert_eq!(b, AddressSet::from_slice(&[Short::new(4)]));
    }

    #[test]
    fn range_test() {
        let mut a = AddressSet::from_range(Short::new(0)..=Short::new(63));
        assert_eq!(a, (0..64).fold(AddressSet::new(), |a, b| a + Short::new(b)));
        a -= Short::new(9);
        assert_eq!(
            a,
            AddressSet::from_range(Short::new(0)..=Short::new(8)) + AddressSet::from_range(Short::new(10)..=Short::new(63))
        );
        let a = AddressSet::from_range(Short::new(7)..=Short::new(62));
        assert_eq!(a, (7..63).fold(AddressSet::new(), |a, b| a + Short::new(b)));
    }
}
