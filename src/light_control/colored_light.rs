#[derive(PartialEq, Clone, Debug)]
pub struct LightValue {
    pub power: f32, // 0 - 100%
    pub color: ColoredLight,
}

#[derive(PartialEq, Clone, Debug)]
pub enum ColoredLight {
    None,
    ColorTemp { kelvin: u32 },
    Coordinate { x: f32, y: f32 }, // CIE 1931 color coordinates
}

#[derive(Clone, Debug)]
pub struct LightSequencePoint<Instant = std::time::Instant>
{
    pub when: Instant,
    pub value: LightValue,
}


// Algorithm from:
// Bongsoon Kang; Ohak Moon; Changhee Hong; Honam Lee; Bonghwan Cho; Youngsun Kim (December 2002).
// "Design of Advanced Color Temperature Control System for HDTV Applications"
// Equations 8 and 9

pub fn cct_to_xy(kelvin: u32) -> (f32, f32) {
    let mired = 1.0e6 / (kelvin as f32);
    let mired2 = mired * mired;
    let mired3 = mired2 * mired;
    let x = if kelvin < 4000 {
        -0.2661239e9 * mired3 - 0.2343589e6 * mired2 + 0.8776956e3 * mired + 0.179910
    } else {
        -3.0258469e9 * mired3 + 2.1070379e6 * mired2 + 0.2226347e3 * mired + 0.24039
    };
    let x2 = x * x;
    let x3 = x2 * x;
    let y = if kelvin < 2222 {
        -1.1063814 * x3 - 1.34811020 * x2 + 2.18555832 * x - 0.20219683
    } else if kelvin < 4000 {
        -0.9549476 * x3 - 1.37418593 * x2 + 2.09137015 * x - 0.16748867
    } else {
        3.0817580 * x3 - 5.8733867 * x2 + 3.75112997 * x - 0.37001483
    };
    (x,y)
}

