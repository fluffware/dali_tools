use crate::drivers::driver;
use std::time::Duration;

use driver::DaliFrame;

pub const HALF_BIT_MICROS: u64 = 417;
pub const BIT_MICROS:u64 = 833;
// Includes start bit
pub const FRAME_8_DURATION: Duration = Duration::from_micros(BIT_MICROS * 9);
pub const FRAME_16_DURATION: Duration = Duration::from_micros(BIT_MICROS * 17);
pub const FRAME_24_DURATION: Duration = Duration::from_micros(BIT_MICROS * 25);
pub const FRAME_25_DURATION: Duration = Duration::from_micros(BIT_MICROS * 26);
pub const SEND_TWICE_DURATION: Duration = Duration::from_millis(94);
pub const REPLY_DELAY: Duration = Duration::from_millis(5);
pub const INIT_TIMEOUT: Duration = Duration::from_secs(15*60);

pub fn frame_duration(frame: &DaliFrame) -> Duration
{
    use DaliFrame::*;
    Duration::from_micros(
	match frame {
	    Frame8(f) => {
		// If the frame ends with a 1 then the last transition is
		// in the middle of the last bit.
	    
		9 * BIT_MICROS - if f & 1 == 1 {HALF_BIT_MICROS} else {0}
	    },
	    Frame16(f) => {
	    17 * BIT_MICROS - if f[1] & 1 == 1 {HALF_BIT_MICROS} else {0}
	    },
	    Frame24(f) => {
		25 * BIT_MICROS - if f[2] & 1 == 1 {HALF_BIT_MICROS} else {0}
	    }, 
	    Frame25(f) => {
		26 * BIT_MICROS 
		    - if f[3] & 0x80 == 0x80 {HALF_BIT_MICROS} else {0}
	    }
	})
    }
