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
                "Fade time must be an integer or an integer followed by \"ms\""
            ),
            Self::InvalidFadeRateStr => write!(
                f,
                "Fade rate must be an integer or an integer followed by \"ms/s\""
            ),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct FadeTime(u8);

const FADE_TIMES: [u32; 15] = [
    707, 1000, 1414, 2000, 2828, 4000, 5657, 8000, 11314, 16000, 22627, 32000, 45255, 64000, 90510,
];
impl FadeTime {
    pub fn from_value(value: u8) -> FadeTime {
        assert!(value <= 15);
        FadeTime(value)
    }
    pub fn from_millis(ms: u32) -> FadeTime {
        FadeTime(match FADE_TIMES.binary_search(&ms) {
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
        } as u8)
    }
    pub fn value(&self) -> u8 {
        self.0
    }
}

impl FromStr for FadeTime {
    type Err = Error;
    fn from_str(s: &str) -> Result<FadeTime, Error> {
        Ok(if let Some(ms_str) = s.strip_suffix("ms") {
            FadeTime::from_millis(
                u32::from_str(ms_str.trim()).map_err(|_| Error::InvalidFadeTimeStr)?,
            )
        } else {
            FadeTime::from_value(u32::from_str(s).map_err(|_| Error::InvalidFadeTimeStr)? as u8)
        })
    }
}

//pub struct FadeRate(u8);

#[test]
fn test_fade_time() {
    let ft = FadeTime::from_value(15);
    assert_eq!(ft.value(), 15);
    let ft = FadeTime::from_value(0);
    assert_eq!(ft.value(), 0);
    assert_eq!(FadeTime::from_value(1), FadeTime::from_millis(707));
    assert_eq!(FadeTime::from_value(2), FadeTime::from_millis(1206));
    assert_eq!(FadeTime::from_value(3), FadeTime::from_millis(1207));
    assert_eq!(FadeTime::from_value(1), FadeTime::from_millis(700));
    assert_eq!(FadeTime::from_value(15), FadeTime::from_millis(90700));
    assert_eq!(
        FadeTime::from_value(4),
        FadeTime::from_str("2002ms").unwrap()
    );
    assert_eq!(FadeTime::from_value(8), FadeTime::from_str("8").unwrap());
}
