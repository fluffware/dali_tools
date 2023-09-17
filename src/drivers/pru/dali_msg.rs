#![allow(dead_code)]

pub const DALI_OK: u8 = 0;
pub const DALI_SEND_DONE: u8 = 2;
pub const DALI_RECV_FRAME: u8 = 5;
pub const DALI_ERR_FRAMING: u8 = 10; // Received a frame with a framing error
pub const DALI_ERR_BUS_LOW: u8 = 20; // The bus has been low for a long time
pub const DALI_INFO_BUS_HIGH: u8 = 24; // The bus has returned to a high level
                                       /* Failed to send a frame, including retries, due to bus activity. */
pub const DALI_ERR_BUS_BUSY: u8 = 30;
/* Timeout when waiting for a backward frame */
pub const DALI_NO_REPLY: u8 = 40;
/* Hardware or software error */
pub const DALI_ERR_DRIVER: u8 = 50;
/* A timeout wasn't handled quickly enough */
pub const DALI_ERR_TIMING: u8 = 55;

// Another frame was received before the previous one was read
pub const DALI_OVERRUN: u8 = 200;

// Frame length
pub const DALI_FLAGS_LENGTH: u16 = 0x700;
pub const DALI_FLAGS_LENGTH_25: u16 = 0x500;
pub const DALI_FLAGS_LENGTH_24: u16 = 0x300;
pub const DALI_FLAGS_LENGTH_16: u16 = 0x200;
pub const DALI_FLAGS_LENGTH_8: u16 = 0x0100;
pub const DALI_FLAGS_DRIVER: u16 = 0x000; // Driver command, don't send a frame

// Repeat frame
pub const DALI_FLAGS_SEND_TWICE: u16 = 0x20;
pub const DALI_FLAGS_SEND_ONCE: u16 = 0x00;

// Retry
pub const DALI_FLAGS_RETRY: u16 = 0x10;

pub const DALI_FLAGS_EXPECT_ANSWER: u16 = 0x08;

// Ignore collision. Must be used for backward frames.
pub const DALI_FLAGS_NO_COLLISIONS: u16 = 0x40;

pub const DALI_FLAGS_PRIORITY: u16 = 0x07;
pub const DALI_FLAGS_PRIORITY_0: u16 = 0x00; // Backward frame
pub const DALI_FLAGS_PRIORITY_1: u16 = 0x01;
pub const DALI_FLAGS_PRIORITY_2: u16 = 0x02;
pub const DALI_FLAGS_PRIORITY_3: u16 = 0x03;
pub const DALI_FLAGS_PRIORITY_4: u16 = 0x04;
pub const DALI_FLAGS_PRIORITY_5: u16 = 0x05;

#[derive(Debug)]
#[repr(C)]
pub struct DaliMsg {
    seq: u8,
    result: u8,
    flags: u16,
    frame: [u8; 4],
}

fn set_flag(flags: &mut u16, flag: u16, set: bool) {
    *flags = (*flags & !flag) | if set { flag } else { 0 };
}
impl DaliMsg {
    pub fn frame8(seq: u8, frame: &[u8]) -> DaliMsg {
        assert!(frame.len() >= 1);
        DaliMsg {
            seq,
            result: 0,
            flags: DALI_FLAGS_LENGTH_8,
            frame: [frame[0], 0, 0, 0],
        }
    }

    pub fn frame16(seq: u8, frame: &[u8]) -> DaliMsg {
        assert!(frame.len() >= 2);
        DaliMsg {
            seq,
            result: 0,
            flags: DALI_FLAGS_LENGTH_16,
            frame: [frame[0], frame[1], 0, 0],
        }
    }

    pub fn frame24(seq: u8, frame: &[u8]) -> DaliMsg {
        assert!(frame.len() >= 3);
        DaliMsg {
            seq,
            result: 0,
            flags: DALI_FLAGS_LENGTH_24,
            frame: [frame[0], frame[1], frame[2], 0],
        }
    }

    pub fn frame25(seq: u8, frame: &[u8]) -> DaliMsg {
        assert!(frame.len() >= 4);
        DaliMsg {
            seq,
            result: 0,
            flags: DALI_FLAGS_LENGTH_25,
            frame: [frame[0], frame[1], frame[2], frame[3]],
        }
    }

    pub fn seq(&self) -> u8 {
        self.seq
    }

    pub fn result(&self) -> u8 {
        self.result
    }

    pub fn bit_length(&self) -> u16 {
        match self.flags & DALI_FLAGS_LENGTH {
            DALI_FLAGS_LENGTH_25 => 25,
            DALI_FLAGS_LENGTH_24 => 24,
            DALI_FLAGS_LENGTH_16 => 16,
            DALI_FLAGS_LENGTH_8 => 8,
            _ => 8,
        }
    }

    pub fn frame_data(&self) -> &[u8; 4] {
        &self.frame
    }

    pub fn send_twice(&self) -> bool {
        (self.flags & DALI_FLAGS_SEND_TWICE) != 0
    }

    pub fn set_send_twice(&mut self, twice: bool) {
        set_flag(&mut self.flags, DALI_FLAGS_SEND_TWICE, twice);
    }

    pub fn expect_answer(&self) -> bool {
        (self.flags & DALI_FLAGS_EXPECT_ANSWER) != 0
    }

    pub fn set_expect_answer(&mut self, answer: bool) {
        set_flag(&mut self.flags, DALI_FLAGS_EXPECT_ANSWER, answer);
    }

    pub fn retry(&self) -> bool {
        (self.flags & DALI_FLAGS_RETRY) != 0
    }

    pub fn set_retry(&mut self, retry: bool) {
        set_flag(&mut self.flags, DALI_FLAGS_RETRY, retry);
    }

    pub fn ignore_collisions(&self) -> bool {
        (self.flags & DALI_FLAGS_NO_COLLISIONS) != 0
    }

    pub fn set_ignore_collisions(&mut self, ignore: bool) {
        set_flag(&mut self.flags, DALI_FLAGS_NO_COLLISIONS, ignore);
    }

    pub fn priority(&self) -> u16 {
        self.flags & DALI_FLAGS_PRIORITY
    }

    pub fn set_priority(&mut self, priority: u16) {
        self.flags = (self.flags & !DALI_FLAGS_PRIORITY) | (priority & DALI_FLAGS_PRIORITY);
    }
}
