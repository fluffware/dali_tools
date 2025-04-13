use super::address_set::AddressSet;
use super::colored_light::LightSequencePoint;
use core::cmp::PartialOrd;
use std::fmt::Debug;

pub trait InstantOps {
    type Duration;
    fn is_near(&self, other: &Self, max: &Self::Duration) -> bool; // True if the difference is less than duration
}

pub struct Sequencer<Instant> {
    current: Vec<(AddressSet, LightSequencePoint<Instant>)>, // Sorted by time stamp
    pending: Vec<(AddressSet, LightSequencePoint<Instant>)>, // Sorted by time stamp
}

impl<Instant, Duration> Sequencer<Instant>
where
    Instant: InstantOps + Ord + Copy + Debug + core::ops::Sub<Instant, Output = Duration>,
    Duration: Ord + Copy,
{
    pub fn new() -> Sequencer<Instant> {
        Sequencer {
            current: Vec::new(),
            pending: Vec::new(),
        }
    }

    pub fn pending(&self) -> &[(AddressSet, LightSequencePoint<Instant>)] {
        &self.pending
    }

    pub fn add_sequence(
        &mut self,
        addrs: AddressSet,
        light: &[LightSequencePoint<Instant>],
        merge_limit: Duration,
    ) {
        if light.is_empty() {
            return;
        }

        let mut pending_index = 0;
        let mut light_index = 0;

        // Find the first pending point after the start of the new sequence
        let pending = &mut self.pending;
        let first_ts = &light[0].when;
        // Remove addresses from following points
        pending.retain_mut(|p| {
            if first_ts <= &p.1.when {
                p.0 -= &addrs;
                !p.0.is_empty()
            } else {
                true
            }
        });
        if pending.is_empty() {
            for l in light {
                pending.push((addrs.clone(), l.clone()));
            }
            return;
        }

        let mut prev = None;
        let mut next = 0;
        'light_loop: for l in light {
            while next < pending.len() && l.when >= pending[next].1.when {
                prev = Some(next);
                next += 1;
            }
            println!("next = {next}");
            assert!(next >= pending.len() || pending[next].1.when >= l.when);
            match prev {
                Some(prev)
                    if l.when - pending[prev].1.when < merge_limit
                        && pending[prev].1.value == l.value =>
                {
                    pending[prev].0 += &addrs;
                    println!("Merge prev {:?} -> {:?}", l.when, pending[prev].1.when);
                }
                _ if next < pending.len()
                    && pending[next].1.when - l.when < merge_limit
                    && pending[next].1.value == l.value =>
                {
                    pending[next].0 += &addrs;
                    println!("Merge next {:?} -> {:?}", l.when, pending[next].1.when);
                }
                _ => {
                    pending.insert(next, (addrs.clone(), l.clone()));
                    println!("Insert at {}", next);
                    next += 1;
                }
            }
        }
    }

    fn update(&mut self, now: Instant) {}

    fn next_update(&self) -> Option<Instant> {
        None
    }
}

#[cfg(test)]
mod test {
    use super::super::{
        address_set::AddressSet,
        colored_light::{ColoredLight, LightSequencePoint, LightValue},
    };
    use super::{InstantOps, Sequencer};
    type Instant = i32; // ms
    type Duration = i32; //ms
    const NEAR_LIMIT: i32 = 200;
    impl InstantOps for Instant {
        type Duration = Duration;
        fn is_near(&self, other: &Instant, limit: &Duration) -> bool {
            (self - other).abs() <= *limit
        }
    }

    fn check_seq(seq: &Sequencer<Instant>, expected: &[(Instant, f32, &[u8])]) {
        let pending = seq.pending();
        assert_eq!(
            pending.len(),
            expected.len(),
            "Pending list has wrong length"
        );
        for (p, e) in pending.iter().zip(expected) {
            assert_eq!(p.1.when, e.0);
            assert_eq!(
                p.1.value,
                LightValue {
                    power: e.1,
                    color: ColoredLight::None
                }
            );
            assert_eq!(p.0.to_vec(), AddressSet::from_slice(e.2).to_vec());
        }
    }

    fn add_seq(seq: &mut Sequencer<Instant>, addrs: &[u8], lights: &[(Instant, f32)]) {
        seq.add_sequence(
            AddressSet::from_slice(addrs),
            &lights
                .iter()
                .map(|l| LightSequencePoint {
                    when: l.0,
                    value: LightValue {
                        power: l.1,
                        color: ColoredLight::None,
                    },
                })
                .collect::<Vec<LightSequencePoint<Instant>>>(),
            NEAR_LIMIT,
        );
    }

    #[test]
    fn test_add_sequence() {
        let mut seq = Sequencer::<Instant>::new();
        let a_set = AddressSet::from_slice(&[3, 6]);
        let b_set = AddressSet::from_slice(&[3, 5]);
        add_seq(&mut seq, &[3, 6], &[(900, 12.4)]);
        check_seq(&seq, &[(900, 12.4, &[3, 6])]);

        add_seq(&mut seq, &[3, 5], &[(300, 45.0)]);
        check_seq(&seq, &[(300, 45.0, &[3, 5]), (900, 12.4, &[6])]);

        add_seq(&mut seq, &[3, 6], &[(600, 45.0)]);
        check_seq(&seq, &[(300, 45.0, &[3, 5]), (600, 45.0, &[3, 6])]);

        add_seq(
            &mut seq,
            &[2, 3],
            &[(310, 49.0), (320, 45.0), (590, 89.0), (595, 45.0)],
        );
        check_seq(
            &seq,
            &[
                (300, 45.0, &[2, 3, 5]),
                (310, 49.0, &[2, 3]),
                (590, 89.0, &[2, 3]),
                (600, 45.0, &[2,3, 6]),
            ],
        );
	add_seq(&mut seq, &[3, 7], &[(590, 92.0)]);
	check_seq(
            &seq,
            &[
                (300, 45.0, &[2, 3, 5]),
                (310, 49.0, &[2, 3]),
                (590, 89.0, &[2]),
		(590, 92.0, &[3,7]),
                (600, 45.0, &[2, 6]),
            ],
        );
    }
}
