use serde_derive::Deserialize;
use std::str::FromStr;

#[derive(Debug)]
pub enum Error {
    InvalidFadeTimeStr,
    InvalidFadeRateStr,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFadeTimeStr => write!(
                f,
                "Fade time must be an integer or an number followed by \"min\" or \"s\""
            ),
            Self::InvalidFadeRateStr => write!(
                f,
                "Fade rate must be an integer or an number followed by \"min\" or \"s\""
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FadeTime(u8);

const FADE_TIMES: [f32; 15] = [
    0.707, 1.0, 1.414, 2.0, 2.828, 4.0, 5.657, 8.0, 11.314, 16.0, 22.627, 32.0, 45.255, 64.0,
    90.510,
];
impl FadeTime {
    pub fn from_value(value: u8) -> FadeTime {
        assert!(value <= 15);
        FadeTime(value)
    }
    pub fn from_secs(ms: f32) -> FadeTime {
        FadeTime(
            match FADE_TIMES.binary_search_by(|a| a.partial_cmp(&ms).unwrap()) {
                Ok(index) => index + 1,
                Err(index) => {
                    if index == 0 {
                        1
                    } else if index == 15 {
                        15
                    } else {
                        let low = ms - FADE_TIMES[index - 1];
                        let high = FADE_TIMES[index] - ms;
                        if low < high { index } else { index + 1 }
                    }
                }
            } as u8,
        )
    }

    pub fn to_secs(&self) -> f32 {
        if self.0 == 0 {
            0.0
        } else {
            FADE_TIMES[self.0 as usize - 1]
        }
    }

    pub fn value(&self) -> u8 {
        self.0
    }
}

impl FromStr for FadeTime {
    type Err = Error;
    fn from_str(s: &str) -> Result<FadeTime, Error> {
        Ok(if let Some(s_str) = s.strip_suffix("s") {
            FadeTime::from_secs(f32::from_str(s_str.trim()).map_err(|_| Error::InvalidFadeTimeStr)?)
        } else if let Some(min_str) = s.strip_suffix("min") {
            FadeTime::from_secs(
                f32::from_str(min_str.trim()).map_err(|_| Error::InvalidFadeTimeStr)? * 60.0,
            )
        } else {
            FadeTime::from_value(u32::from_str(s).map_err(|_| Error::InvalidFadeTimeStr)? as u8)
        })
    }
}
impl std::fmt::Display for FadeTime {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:.3}s", self.to_secs())
    }
}

//pub struct FadeRate(u8);

#[test]
fn test_fade_time() {
    let ft = FadeTime::from_value(15);
    assert_eq!(ft.value(), 15);
    let ft = FadeTime::from_value(0);
    assert_eq!(ft.value(), 0);
    assert_eq!(FadeTime::from_value(1), FadeTime::from_secs(0.707));
    assert_eq!(FadeTime::from_value(2), FadeTime::from_secs(1.206));
    assert_eq!(FadeTime::from_value(3), FadeTime::from_secs(1.207));
    assert_eq!(FadeTime::from_value(1), FadeTime::from_secs(0.700));
    assert_eq!(FadeTime::from_value(15), FadeTime::from_secs(90.700));
    assert_eq!(
        FadeTime::from_value(4),
        FadeTime::from_str("2.002s").unwrap()
    );
    assert_eq!(FadeTime::from_value(8), FadeTime::from_str("8").unwrap());
    assert_eq!(
        FadeTime::from_value(14),
        FadeTime::from_str("1.2min").unwrap()
    );
    assert_eq!("0.707s", &format!("{}", FadeTime::from_value(1)));
    assert_eq!("2.000s", &format!("{}", FadeTime::from_value(4)));
    assert_eq!("0.000s", &format!("{}", FadeTime::from_value(0)));
    assert_eq!(
        "90.510s",
        &format!("{}", FadeTime::from_str("1.5min").unwrap())
    );
}
