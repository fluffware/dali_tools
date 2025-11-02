use crate::common::address::Short;
use core::ops::RangeInclusive;

#[derive(PartialEq, Debug, Clone, Default)]
pub struct AddressSet(u64);

impl AddressSet {
    pub fn new() -> AddressSet {
        AddressSet::default()
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
        (self.0 & (1 << addr.value())) != 0
    }

    pub fn insert(&mut self, addr: Short) {
        self.0 |= 1u64 << addr.value();
    }

    pub fn insert_set(&mut self, b: &AddressSet) {
        self.0 |= b.0;
    }

    pub fn remove(&mut self, b: Short) {
        self.0 &= !(1u64 << b.value());
    }

    pub fn remove_set(&mut self, b: AddressSet) {
        self.0 &= !b.0;
    }
}

#[cfg(test)]
mod test {
    use super::AddressSet;
    use crate::common::address::Short;

    #[test]
    fn add_test() {
        let mut a = AddressSet::new();
        a.insert(Short::new(5));
        assert_eq!(a, AddressSet::from_slice(&[Short::new(5)]));
        a.insert(Short::new(9));
        assert_eq!(a, AddressSet::from_slice(&[Short::new(9), Short::new(5)]));
    }

    #[test]
    fn sub_test() {
        let mut a = AddressSet::from_slice(&[Short::new(4), Short::new(7), Short::new(6)]);
        a.remove(Short::new(7));
        assert_eq!(a, AddressSet::from_slice(&[Short::new(6), Short::new(4)]));
        a.remove(Short::new(6));
        assert_eq!(a, AddressSet::from_slice(&[Short::new(4)]));
    }

    #[test]
    fn range_test() {
        let mut a = AddressSet::from_range(Short::new(0)..=Short::new(63));
        assert_eq!(
            a,
            (0..64).fold(AddressSet::new(), |mut a, b| {
                a.insert(Short::new(b));
                a
            })
        );
        a.remove(Short::new(9));
        let mut res = AddressSet::from_range(Short::new(0)..=Short::new(8));
        res.insert_set(&AddressSet::from_range(Short::new(10)..=Short::new(63)));
        assert_eq!(a, res);
        let a = AddressSet::from_range(Short::new(7)..=Short::new(62));
        assert_eq!(
            a,
            (7..63).fold(AddressSet::new(), |mut a, b| {
                a.insert(Short::new(b));
                a
            })
        );
    }
}
