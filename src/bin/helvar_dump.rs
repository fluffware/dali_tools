extern crate futures;
extern crate libusb_async;
extern crate tokio;

use libusb_async::{Context, DeviceHandle};

fn send_hid_report(dev: &DeviceHandle, data: &[u8]) -> libusb_async::TransferFuture {
    let mut trans = dev.alloc_transfer(0).unwrap();
    trans.fill_control_write(0x21, 0x09, 0x0203, 0, data);
    trans.submit()
}

fn read_hid_report(dev: &DeviceHandle) -> libusb_async::TransferFuture {
    let mut trans = dev.alloc_transfer(0).unwrap();
    trans.fill_interrupt_read(0x81, 128);
    trans.submit()
}

#[tokio::main]
async fn main() {
    let usb_ctxt = Context::new().unwrap();
    // Print out information about all connected devices
    let mut device: Option<DeviceHandle> = None;
    for dev in usb_ctxt.devices().unwrap().iter() {
        //println!("{:#?}", info);
        let dev_descr = dev.device_descriptor().unwrap();
        let product_id = dev_descr.product_id();
        let vendor_id = dev_descr.vendor_id();
        //let serial_idx = dev_descr.serial_number_string_index();

        match (product_id, vendor_id) {
            (0x0510, 0x16eb) => {
                println!("Device: {:04x}:{:04x}", vendor_id, product_id);
                match dev.open() {
                    Ok(d) => {
                        device = Some(d);
                    }

                    Err(e) => {
                        println!("Failed to open device: {}", e);
                        return;
                    }
                }
                break;
            }
            _ => {}
        }
    }
    let mut device = match device {
        Some(d) => d,
        None => {
            println!("No device found");
            return;
        }
    };
    if device.kernel_driver_active(0).unwrap_or(false) {
        device.detach_kernel_driver(0).unwrap();
    }
    device.claim_interface(0).unwrap();

    let send = [2, 0x82, 0x04];
    match send_hid_report(&device, &send).await {
        Ok(_) => {
            println!("Sent {} bytes", send.len());
        }
        Err(e) => {
            println!("Failed to send {} bytes: {}", send.len(), e);
        }
    };

    loop {
        let read_reply = read_hid_report(&device);
        let r = read_reply.await.unwrap();
        let buf = r.get_buffer();
        let buf_len = buf.len();
        if buf_len > 0 {
            println!("Len: {}", buf_len);
            for b in buf[1..usize::from(buf[0]) + 1].iter() {
                print!(" {:02x}", b);
            }
            println!("");
        }
    }
}
