const PRIORITY_MASK: u16 = 0x07;
const PRIORITY_SHIFT: u32 = u16::trailing_zeros(PRIORITY_MASK);

const SEND_TWICE_BIT: u16 = 0x08;
const EXPECT_ANSWER_BIT: u16 = 0x10; // Expect an answer
pub const PRIORITY_1: Flags = Priority(1);
pub const PRIORITY_2: Flags = Priority(2);
pub const PRIORITY_3: Flags = Priority(3);
pub const PRIORITY_4: Flags = Priority(4);
pub const PRIORITY_5: Flags = Priority(5);
pub const EXPECT_ANSWER: Flags = ExpectAnswer(true);
pub const SEND_TWICE: Flags = SendTwice(true);
pub const NO_FLAG: Flags = Combined(0);
pub const PRIORITY_DEFAULT: Flags = PRIORITY_5;

#[derive(Debug, Clone)]
pub enum Flags {
    Empty,
    Priority(u16),
    SendTwice(bool),
    ExpectAnswer(bool),
    Combined(u16),
}

use Flags::*;
impl Flags {
    const fn bits(&self) -> u16 {
        match *self {
            Empty => 0,
            Priority(p) => (p & PRIORITY_MASK) << PRIORITY_SHIFT,
            SendTwice(s) => {
                if s {
                    SEND_TWICE_BIT
                } else {
                    0
                }
            }
            ExpectAnswer(e) => {
                if e {
                    EXPECT_ANSWER_BIT
                } else {
                    0
                }
            }
            Combined(b) => b,
        }
    }

    pub const fn send_twice(&self) -> bool {
        (self.bits() & SEND_TWICE_BIT) != 0
    }

    pub fn expect_answer(&self) -> bool {
        (self.bits() & EXPECT_ANSWER_BIT) != 0
    }
    pub fn priority(&self) -> u16 {
        let p = (self.bits() & PRIORITY_MASK) << PRIORITY_SHIFT;
        if (1..=5).contains(&p) { p } else { 5 }
    }
}

impl std::ops::BitOr<Flags> for Flags {
    type Output = Self;
    fn bitor(self, other: Flags) -> Self::Output {
        let b = self.bits();
        let masked = match other {
            Empty => b,
            Priority(_) => b & !PRIORITY_MASK,
            SendTwice(_) => b & !SEND_TWICE_BIT,
            ExpectAnswer(_) => b & !EXPECT_ANSWER_BIT,
            Combined(_) => b,
        };
        Combined(masked | other.bits())
    }
}
impl std::ops::BitOrAssign<Flags> for Flags {
    fn bitor_assign(&mut self, other: Flags) {
        let b = self.bits();
        let masked = match other {
            Empty => b,
            Priority(_) => b & !PRIORITY_MASK,
            SendTwice(_) => b & !SEND_TWICE_BIT,
            ExpectAnswer(_) => b & !EXPECT_ANSWER_BIT,
            Combined(_) => b,
        };
        *self = Combined(masked | other.bits());
    }
}
