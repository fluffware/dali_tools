use core::ops::{Add, AddAssign, Sub, SubAssign};
use core::ops::Range;

#[derive(PartialEq, Debug, Clone)]
pub struct AddressSet(u64);

impl AddressSet {
    pub fn new() -> AddressSet {
        AddressSet(0)
    }

    pub fn from_slice(addrs: &[u8]) -> AddressSet {
        let mut s = 0u64;
        for addr in addrs {
            s |= 1 << addr;
        }
        AddressSet(s)
    }
    
    pub fn to_vec(&self) -> Vec<u8> {
	(0..64).filter_map(|b| if (self.0 & 1<<b) != 0 {Some(b)} else {None}).collect()
	    
    }

    

    pub fn from_range(addrs: Range<u8>) -> AddressSet {
	if addrs.is_empty() {
	    return AddressSet(0)
	}
        let start_bit = 1u64 << addrs.start;
        let end_bit = 1u64 << addrs.end-1;
        let s = !(start_bit - 1) & ((end_bit -1) + end_bit);
        AddressSet(s)
    }
    
    pub fn is_empty(&self) -> bool {
	self.0 == 0
    }
    
}

impl Add<u8> for AddressSet {
    type Output = AddressSet;
    fn add(self, b: u8) -> Self::Output {
        Self(self.0 | (1u64 << b))
    }
}

impl AddAssign<u8> for AddressSet {
    fn add_assign(&mut self, b: u8) {
        self.0 |= 1u64 << b;
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

impl Sub<u8> for AddressSet {
    type Output = AddressSet;
    fn sub(self, b: u8) -> Self::Output {
        Self(self.0 & !(1u64 << b))
    }
}

impl SubAssign<u8> for AddressSet {
    fn sub_assign(&mut self, b: u8) {
        self.0 &= !(1u64 << b);
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

    #[test]
    fn add_test() {
        let a = AddressSet::new();
        let mut b = a + 5;
        assert_eq!(b, AddressSet::from_slice(&[5]));
        b += 9;
        assert_eq!(b, AddressSet::from_slice(&[9, 5]));
    }

    #[test]
    fn sub_test() {
        let a = AddressSet::from_slice(&[4, 7, 6]);
        let mut b = a - 7;
        assert_eq!(b, AddressSet::from_slice(&[6, 4]));
        b -= 6;
        assert_eq!(b, AddressSet::from_slice(&[4]));
    }

    #[test]
    fn range_test() {
        let mut a = AddressSet::from_range(0u8..64u8);
	assert_eq!(a, (0..64).fold(AddressSet::new(), |a,b| a+b));
	a -= 9;
	assert_eq!(a , AddressSet::from_range(0u8..9) + AddressSet::from_range(10u8..64));
        let a = AddressSet::from_range(7u8..63u8);
	assert_eq!(a, (7..63).fold(AddressSet::new(), |a,b| a+b));
        let a = AddressSet::from_range(0u8..0u8);
	assert_eq!(a, AddressSet::new());
	
    }
}
