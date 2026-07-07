use dali_tools::drivers::driver::{DaliBusEventType, DaliFrame, DaliSendResult};
use dali_tools::drivers::send_flags::Flags;
use dali_tools::simulator::sim_bus::{DaliSimBusDevice, DaliSimBusDeviceEvent};
use log::debug;
use std::cmp::min;
use std::time::{Duration, Instant};
use tokio::io::AsyncReadExt;
use tokio_serial::SerialStream;
use tokio_util::sync::CancellationToken;

struct Driver {
    bus_device: DaliSimBusDevice,
    serial: SerialStream,
    last_ts: Instant,
    buffer: [u8; 8],
    buffer_len: usize,
}

const BUFFER_SIZE: usize = 8;
impl Driver {
    pub fn new(bus_device: DaliSimBusDevice, serial: SerialStream) -> Driver {
        Driver {
            bus_device,
            serial,
            last_ts: Instant::now(),
            buffer: [0u8; BUFFER_SIZE],
            buffer_len: 0,
        }
    }

    async fn handle_cmd(&mut self) -> [u8; 8] {
        let seq = self.buffer[0];
        let flag_byte = self.buffer[1];
        let mut flags = Flags::Empty;
        flags |= Flags::ExpectAnswer((flag_byte & 0b1) != 0);
        flags |= Flags::SendTwice((flag_byte & 0b10) != 0);
        flags |= Flags::Priority(u16::from(self.buffer[2] & 0x07));
        let bit_length = self.buffer[3];
        let buffer = &self.buffer;
        let frame = match bit_length {
            8 => DaliFrame::Frame8(buffer[4]),
            16 => DaliFrame::Frame16([buffer[4], buffer[5]]),
            24 => DaliFrame::Frame24([buffer[4], buffer[5], buffer[6]]),
            _ => return [seq, 0, 0, 0, 0, 0, 0, 0],
        };
        let now = Instant::now();
        loop {
            match self.bus_device.wait_until(now).await {
                DaliSimBusDeviceEvent::Timeout => {
                    break;
                }
                _ => {}
            }
        }
        let res = self.bus_device.send_frame(frame, flags).await;
        loop {
            match self.bus_device.wait_until(now).await {
                DaliSimBusDeviceEvent::Timeout => {
                    break;
                }
                _ => {}
            }
        }
        [0u8; 8]
    }

    async fn handle_message(&mut self, data: &[u8]) {
        let now = Instant::now();
        if now - self.last_ts > Duration::from_millis(100) {
            self.buffer_len = 0;
        }
        let mut left = data;
        while !left.is_empty() {
            let copy_len = min(left.len(), BUFFER_SIZE - self.buffer_len);
            self.buffer[self.buffer_len..self.buffer_len + copy_len]
                .copy_from_slice(&left[..copy_len]);
            self.buffer_len += copy_len;
            left = &left[copy_len..];
            if self.buffer_len == BUFFER_SIZE {
                self.handle_cmd().await;
                self.buffer_len = 0;
            }
        }
        self.last_ts = now;
    }
}

pub async fn start_serial(
    bus_device: DaliSimBusDevice,
    port_path: &str,
    cancel: CancellationToken,
) -> Result<(), Box<dyn std::error::Error>> {
    let serial = SerialStream::open(&tokio_serial::new(port_path, 9600))?;
    let mut driver = Driver::new(bus_device, serial);
    let mut buffer = [0u8; 16];
    loop {
        #[rustfmt::skip]
        tokio::select! {
            res = driver.serial.read(&mut buffer) => {
		match res {
		    Ok(count) => {
			debug!("Received {:x?}", &buffer[..count]);
			driver.handle_message(&buffer[..count]).await;
		    }
		    Err(e) => {
			return Err(e.into());
		    }
		}
            }
            _res = cancel.cancelled() => {
		break;
            }
        }
    }
    Ok(())
}
