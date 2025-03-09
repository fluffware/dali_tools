use crate::common::commands::{Commands, ErrorInfo, YesNo};
use crate::common::driver_commands::DriverCommands;
use crate::drivers::driver::{DaliDriver, DaliSendResult};
use crate::drivers::send_flags::PRIORITY_1;

use crate::common::address::Long;
use crate::common::address::Short;
use crate::utils::long_address;
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::mpsc::{error::SendError, Sender};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

async fn send_blocking<T>(tx: &mut Sender<T>, item: T) -> Result<(), SendError<T>> {
    let sent = item;
    loop {
        let res = tx.send(sent).await;
        match res {
            Ok(()) => break,
            Err(r) => {
                return Err(r);
            }
        }
    }
    Ok(())
}

// Clear all bits except for the highest one
fn high_bit(mut bits: u32) -> u32 {
    if bits == 0 {
        0
    } else {
        bits |= bits >> 1;
        bits |= bits >> 2;
        bits |= bits >> 4;
        bits |= bits >> 8;
        bits |= bits >> 16;
        (bits >> 1) + 1
    }
}

enum SearchResult<E> {
    NoneFound,
    Found(u32),
    Conflict(u32),
    ReplyError,
    DriverError(E),
}

const TOP_SEARCH_ADDR: u32 = 0x1000000;

/// Search `low`..`high` for the device with the lowest address.
/// There should be no enabled devices
/// with a random address less than `low`.
/// There should be at least one enabled device
///  with a random address less than `high`.
/// 'high_single` returns the highest address found
/// with just a single device with a random address below it.
/// 'low_multiple' returns the lowest address found
/// with multiple device with a random address below it.
/// `high_single` and `low_multiple` are candidates
/// for `low` and `high` respectively when searching for the next device

async fn find_device<C>(
    commands: &mut C,
    mut low: u32,
    mut high: u32,
    high_single: &mut Option<u32>,
    low_multiple: &mut Option<u32>,
    current_search_addr: &mut u32,
) -> SearchResult<C::Error>
where
    C: Commands,
{
    if low >= high {
        return SearchResult::NoneFound;
    }
    let mut pivot = low + high_bit((high - low) / 2);
    *high_single = None;
    *low_multiple = None;
    // Search until we match a single address
    loop {
        assert!(pivot >= low);
        assert!(pivot < high);
        match long_address::set_search_addr_changed(commands, pivot, current_search_addr).await {
            Ok(_) => {}
            Err(e) => return SearchResult::DriverError(e),
        }
        // COMPARE
        let res = commands.compare().await;
        match res {
            Ok(YesNo::Yes) => {
                //println!("Found one");
                if pivot >= high_single.unwrap_or(0) {
                    *high_single = Some(pivot + 1);
                }

                if low >= pivot {
                    //println!("Found device {:06x}", pivot);
                    break SearchResult::Found(pivot);
                }
                high = pivot + 1;
                //pivot = low + high_bit((high - low) / 2);
                assert!(pivot > low);
                pivot -= high_bit((pivot - low) / 2) + 1;
            }
            Ok(YesNo::No) => {
                //println!("Found none");
                if let Some(lm) = *low_multiple {
                    if pivot + 2 > lm {
                        break SearchResult::Conflict(pivot + 1);
                    }
                }
                if pivot == high - 1 {
                    break SearchResult::NoneFound;
                }
                low = pivot + 1;
                pivot += high_bit((high - pivot) / 2);
            }
            Ok(YesNo::Multiple) => {
                //println!("Found multiple");
                *low_multiple = Some(pivot + 1);
                if low >= pivot {
                    //println!("Conflict");
                    // There should only be one address <= pivot
                    break SearchResult::Conflict(pivot);
                }
                high = pivot;
                pivot -= high_bit((pivot - low) / 2) + 1;
            }
            Err(e) => return SearchResult::DriverError(e),
        }
    }
}

async fn find_devices_no_initialise<C, F>(commands: &mut C, found: &mut F) -> Result<u32, C::Error>
where
    C: Commands,
    F: AsyncFnMut(Discovered),
{
    let mut current_search_addr = 0x010101;

    // All address at or below this is already found
    let mut low = 0u32;

    // Lowest address found that has any devices at or below it
    let mut high = 0x1000000u32;

    // Highest address tried with only a single address less or equal
    let mut high_single = None;

    let mut low_multiple = None;

    // Short addresses found so far, used to detect address collisions
    let mut found_short = 0u64;

    let search_res: Result<u32, C::Error> = loop {
        println!("Searching {:06x} - {:06x}", low, high);
        match find_device(
            commands,
            low,
            high,
            &mut high_single,
            &mut low_multiple,
            &mut current_search_addr,
        )
        .await
        {
            SearchResult::Found(addr) => {
                let res =
                    long_address::set_search_addr_changed(commands, addr, &mut current_search_addr);
                if let Err(e) = res.await {
                    return Err(e);
                };

                let res = commands.query_short_address().await;
                match res {
                    Ok(short) => {
                        let short_conflict = if let Some(short) = short {
                            let addr_mask = 1u64 << short.value();
                            found_short |= addr_mask;
                            (found_short & addr_mask) != 0
                        } else {
                            false
                        };
                        found(Discovered {
                            long: Some(addr),
                            short,
                            long_conflict: false,
                            short_conflict,
                        })
                        .await;

                        //println!("Found device 0x{:06x} with short address {}",
                        //addr, short_addr);

                        commands.withdraw().await?;

                        match (high_single, low_multiple) {
                            (Some(hs), Some(lm)) => {
                                if hs < lm {
                                    low = hs;
                                } else {
                                    low = 0;
                                }
                                high = lm;
                            }
                            (Some(hs), None) => {
                                low = hs;
                                high = TOP_SEARCH_ADDR;
                            }
                            (None, _) => {
                                panic!("Found device but high_single not set");
                            }
                        }
                    }
                    Err(e) if e.is_timeout() | e.is_framing_error() => {
                        low = 0;
                    }
                    Err(e) => return Err(e),
                }
            }
            SearchResult::NoneFound => {
                if high == TOP_SEARCH_ADDR {
                    break Ok(0);
                }
                high = TOP_SEARCH_ADDR;
            }
            SearchResult::Conflict(addr) => {
                let res =
                    long_address::set_search_addr_changed(commands, addr, &mut current_search_addr);
                if let Err(e) = res.await {
                    return Err(e);
                };

                commands.withdraw().await?;

                found(Discovered {
                    long: Some(addr),
                    short: None,
                    long_conflict: true,
                    short_conflict: false,
                })
                .await;
                low = addr + 1;
                high = TOP_SEARCH_ADDR;
            }
            SearchResult::ReplyError => {
                low = 0;
                high = TOP_SEARCH_ADDR;
            }
            SearchResult::DriverError(e) => break Err(e),
        }
    };

    //println!("Search terminated: {:?}", search_res);
    //println!("Next: {:06x} - {:06x}", high_single, low_multiple);
    search_res
}

/// Addresses of devices discovered on the bus.
#[derive(Debug)]
pub struct Discovered {
    /// Random address for device. None if there's conflicting short addresses or the device doesn't report a random address.
    pub long: Option<Long>,
    /// Short address if available
    pub short: Option<Short>,
    /// There's multiple devices with the same random address
    pub long_conflict: bool,
    /// There's multiple devices with the same short address
    pub short_conflict: bool,
}

impl Default for Discovered {
    fn default() -> Self {
        Discovered {
            long: None,
            short: None,
            long_conflict: false,
            short_conflict: false,
        }
    }
}

async fn discover_async<'a, C, F>(commands: &mut C, found: &'a mut F) -> Result<(), C::Error>
where
    C: Commands,
    F: AsyncFnMut(Discovered),
{
    let mut found_short = [Option::<Long>::None; 64];
    for index in 0..64usize {
        let a = Short::new(index as u8);
        //eprintln!("Addr: {}", a);
        let mut retry = 3u32;
        match loop {
            match commands.query_random_address(a).await {
                Ok(l) => {
                    found_short[index] = Some(l);
                    break Ok(l);
                }
                Err(e) => {
                    if e.is_timeout() {
                        retry -= 1;
                        if retry > 0 {
                            continue;
                        }
                    }
                    break Err(e);
                }
            }
        } {
            Ok(l) => {
                found(Discovered {
                    long: Some(l),
                    short: Some(a),
                    long_conflict: false,
                    short_conflict: false,
                })
                .await;
            }
            Err(e) if e.is_framing_error() => {
                found(Discovered {
                    long: None,
                    short: Some(a),
                    long_conflict: false,
                    short_conflict: true,
                })
                .await;
            }
            Err(e) if e.is_timeout() => {}
            Err(e) => return Err(e),
        }
    }
    commands.initialise_all().await?;

    let mut current_search_addr = 0xffffffff;

    // WITHDRAW all devices with a short address
    for index in 0..64usize {
        if let Some(l) = found_short[index] {
            long_address::set_search_addr_changed(commands, l, &mut current_search_addr).await?;
            commands.withdraw().await?;
        }
    }
    find_devices_no_initialise(commands, found).await?;
    Ok(())
}

pub type DiscoverItem<E> = Result<Discovered, E>;

async fn discover_thread<DC>(
    mut tx: tokio::sync::mpsc::Sender<
        DiscoverItem<DaliSendResult>,
    >,
    driver: Arc<Mutex<Box<dyn DaliDriver>>>,
) where
    DC: DriverCommands + Send,
{
    let mut d = driver.lock().await;
    let d_ref = d.as_mut();
    let mut commands = DC::from_driver(d_ref, PRIORITY_1);
    let cb_tx = tx.clone();
    let mut send_cb = async move |item| {
        cb_tx.send(Ok(item)).await.unwrap();
    };
    match discover_async(&mut commands, &mut send_cb).await {
        Ok(()) => {}
        Err(e) => {
            send_blocking(&mut tx, Err(e))
                .await
                .unwrap();
        }
    };
    let _ = commands.terminate().await;
}

/// Find all devices on the bus.
///
/// Returns a stream of all devices found.
/// Devices are found by first testing all 64 short addresses and then,
/// after WITHDRAWing the addresses found,
/// search for unaddressed devices using COMPARE.

pub fn find_quick<DC>(
    driver: Arc<Mutex<Box<dyn DaliDriver>>>,
) -> Pin<
    Box<
        dyn Stream<
            Item = DiscoverItem<<<DC as DriverCommands>::Output<'static> as Commands>::Error>,
        >,
    >,
>
where
    DC: DriverCommands + Send + 'static,
    DC::Error: Error + Send + Sync + 'static,
{
    let (tx, rx) = tokio::sync::mpsc::channel(64);
    tokio::spawn(discover_thread::<DC>(tx, driver));
    return Box::pin(ReceiverStream::new(rx));
}
