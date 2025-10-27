use core::ops::Sub;

pub trait Timestamp: Sub<Self, Output = Self::Duration> + PartialOrd + Sized {
    type Duration;
}
