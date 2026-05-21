use crate::common::address::{AddressError, AddressImpl, GroupImpl, Short};
use log::debug;
use std::str::FromStr;

pub fn parse_short(s: &str) -> Result<Short, AddressError> {
    Short::from_str(s)
}

pub fn parse_group<const MAX: u8>(s: &str) -> Result<GroupImpl<MAX>, AddressError> {
    let s = s.strip_prefix('G').unwrap_or(s);
    GroupImpl::from_str(s)
}

pub fn parse_group_or_short<const MAX: u8>(s: &str) -> Result<AddressImpl<MAX>, AddressError> {
    if s.starts_with("G") {
        Ok(AddressImpl::Group(parse_group(s)?))
    } else {
        Ok(AddressImpl::Short(parse_short(s)?))
    }
}

pub fn parse_address<const MAX: u8>(s: &str) -> Result<AddressImpl<MAX>, AddressError> {
    if let Ok(a) = parse_group_or_short(s) {
        Ok(a)
    } else if s == "all" {
        Ok(AddressImpl::Broadcast)
    } else if s == "unaddressed" {
        Ok(AddressImpl::BroadcastUnaddressed)
    } else {
        Err(AddressError::InvalidAddress)
    }
}

pub fn parse_short_range(s: &str) -> Result<Vec<Short>, AddressError> {
    debug!("Parsing range {s}");
    if let Some((a, b)) = s.split_once('-') {
        let start = parse_short(a)?.value();
        let end = parse_short(b)?.value();
        Ok((start..=end).map(|a| Short::new(a)).collect())
    } else {
        Ok(vec![parse_short(s)?])
    }
}

#[cfg(test)]
mod test {
    use super::{parse_address, parse_group, parse_short, parse_short_range};
    use crate::common::address::Short;
    use crate::gear::address::{Address, Group};
    #[test]
    fn test_parse_short() {
        assert_eq!(parse_short("1").unwrap(), Short::new(0));
        assert_eq!(parse_short("64").unwrap(), Short::new(63));
    }

    #[test]
    fn test_parse_group() {
        assert_eq!(parse_group("1").unwrap(), Group::new(0));
        assert_eq!(parse_group("16").unwrap(), Group::new(15));
        assert_eq!(parse_group("G16").unwrap(), Group::new(15));
    }

    #[test]
    fn test_parse_address() {
        assert_eq!(parse_address("1").unwrap(), Address::Short(Short::new(0)));
        assert_eq!(parse_address("G7").unwrap(), Address::Group(Group::new(6)));
        assert_eq!(parse_address("all").unwrap(), Address::Broadcast);
        assert_eq!(
            parse_address("unaddressed").unwrap(),
            Address::BroadcastUnaddressed
        );
    }
    #[test]
    fn test_parse_short_range() {
        assert_eq!(
            &parse_short_range("5-7").unwrap(),
            &[Short::new(4), Short::new(5), Short::new(6)]
        );
    }
}
