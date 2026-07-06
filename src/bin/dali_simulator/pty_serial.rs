use dali_tools::simulator::sim_bus::DaliSimBusDevice;
use log::{debug, error};
use polling::{Event, Events, Poller};
use portable_pty::{PtySize, native_pty_system};
use std::os::fd::BorrowedFd;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use tokio_util::sync::CancellationToken;

struct Cancel {
    pub poller: Poller,
    pub cancelled: AtomicBool,
}

fn run(
    bus_device: DaliSimBusDevice,
    cancel: Arc<Cancel>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let pty_system = native_pty_system();
    let mut pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        // Not all systems support pixel_width, pixel_height,
        // but it is good practice to set it to something
        // that matches the size of the selected font.  That
        // is more complex than can be shown here in this
        // brief example though!
        pixel_width: 0,
        pixel_height: 0,
    })?;
    let master = pair.master;
    let Some(name) = master.tty_name() else {
        return Err("No name for pty".into());
    };
    eprintln!("Connect to {}", name.to_string_lossy());
    let mut reader = master.try_clone_reader()?;
    let mut writer = master.take_writer()?;
    let mut buffer = [0u8, 16];
    let fd = unsafe { BorrowedFd::borrow_raw(master.as_raw_fd().unwrap()) };
    unsafe {
        cancel.poller.add(&fd, Event::readable(1)).unwrap();
    }
    let mut events = Events::new();
    loop {
        cancel.poller.modify(&fd, Event::readable(1)).unwrap();
        cancel.poller.wait(&mut events, None).unwrap();
        if cancel.cancelled.load(Ordering::Acquire) {
            break;
        }
        let r = reader.read(&mut buffer)?;
        debug!("Read: {:?}", r);
        events.clear();
    }
    let _ = cancel.poller.delete(fd);
    Ok(())
}

pub async fn start_pty(
    bus_device: DaliSimBusDevice,
    cancel: CancellationToken,
) -> Result<(), Box<dyn std::error::Error + Sync + Send>> {
    let cancel_poller = Arc::new(Cancel {
        poller: Poller::new().unwrap(),
        cancelled: AtomicBool::new(false),
    });
    let thread_cancel = cancel_poller.clone();
    let thread = tokio::task::spawn_blocking(|| run(bus_device, thread_cancel));
    cancel.cancelled().await;
    debug!("start_pty cancelled");
    cancel_poller.cancelled.store(true, Ordering::Release);
    cancel_poller.poller.notify().unwrap();
    thread.await?
}
