use crate::common::address::Short;
use crate::common::commands::Commands;
use crate::utils::memory_banks;
use crate::utils::memory_banks::MemoryError;
use std::error::Error;

pub struct Report {
    energy: f64,
    power: f64,
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Energy:  {}Wh\n", format_e3_prefix(self.energy))?;
        write!(f, "Power: {}W\n", format_e3_prefix(self.power))
    }
}
const SCALE: [f64; 13] = [
    1e-6, 1e-5, 1e-4, 1e-3, 1e-2, 1e-1, 1.0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6,
];

fn scale_int(base: f64, mut exp: i8) -> f64 {
    if exp < -6 {
        exp = -6;
    } else if exp > 6 {
        exp = 6;
    }
    base * SCALE[(exp + 6) as usize]
}

fn format_e3(mut v: f64) -> (String, isize) {
    let mut exp = 0isize;
    while v >= 1000.0 && exp < 3 {
        v /= 1000.0;
        exp += 1;
    }
    while v < 1.0 && exp > -3 {
        v *= 1000.0;
        exp -= 1;
    }
    let d = if v >= 100.0 {
        1
    } else if v >= 10.0 {
        2
    } else {
        3
    };
    (format!("{:.*}", d, v), exp)
}

fn format_e3_prefix(v: f64) -> String {
    const PREFIX: [&str; 7] = ["n", "µ", "m", "", "k", "M", "G"];
    let (s, e) = format_e3(v);
    s + " " + PREFIX[(e+3) as usize]
}

pub async fn read_bank<C>(commands: &mut C, addr: Short, bank: u8) -> Result<Report, Box<dyn Error>>
where
    C: Commands,
    <C as Commands>::Error: Error,
{
    let bytes = memory_banks::read_range(commands, addr, bank, 2, 14).await?;
    if bytes.len() != 14 {
        return Err(Box::new(MemoryError::InvalidMemoryArea));
    }
    let energy = f64::from(u16::from_be_bytes(bytes[3..5].try_into().unwrap())) * 4294967296.0
        + f64::from(u32::from_be_bytes(bytes[5..9].try_into().unwrap()));
    let energy = scale_int(energy, bytes[2] as i8);
    let power = f64::from(u32::from_be_bytes(bytes[10..14].try_into().unwrap()));
    let power = scale_int(power, bytes[9] as i8);

    Ok(Report { energy, power })
}

#[test]
fn test_format_e3() {
    assert_eq!(format_e3(1202.0), ("1.202".to_string(), 1));
    assert_eq!(format_e3(1200.0), ("1.200".to_string(), 1));
    assert_eq!(format_e3(0.2115), ("211.5".to_string(), -1));
    assert_eq!(format_e3(0.0), ("0.000".to_string(), -3));
}
