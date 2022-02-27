use crate::defs::gear::cmd;
use crate::drivers::driver::{DaliDriver,DaliSendResult};

use crate::utils::long_address;
use crate::base::address::Short;
use crate::base::address::Long;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Sender,error::SendError};
use std::error::Error;
use crate::drivers::send_flags::{EXPECT_ANSWER, PRIORITY_1, SEND_TWICE};

async fn send_blocking<T>(tx:&mut Sender<T>, item : T)
                    -> Result<(), SendError<T>>
{
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
fn high_bit(mut bits: u32) -> u32
{
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

enum SearchResult
{
    NoneFound,
    Found(u32),
    Conflict(u32),
    ReplyError,
    DriverError(Box<DaliSendResult>)
}

const TOP_SEARCH_ADDR:u32 = 0x1000000; 

/// Search `low`.`.high` for the device with the lowest address.
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

async fn find_device(driver: &mut dyn DaliDriver, mut low: u32, mut high: u32,
                     high_single: &mut Option<u32>, 
                     low_multiple: &mut Option<u32>, 
                     current_search_addr: &mut u32) 
                     -> SearchResult
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
        match long_address::set_search_addr_changed(driver, pivot,
                                                    current_search_addr).await {
            Ok(_) => {},
            Err(e) => return SearchResult::DriverError(Box::new(e))
        }
        // COMPARE
        let res = driver.send_frame16(&[cmd::COMPARE, 0x00],
				      PRIORITY_1 | EXPECT_ANSWER);
        match res.await {
            DaliSendResult::Answer(0xff) => {
                //println!("Found one");
                if pivot >= high_single.unwrap_or(0) {
                    *high_single = Some(pivot + 1);
                }
                    
                if low  >= pivot {
                    //println!("Found device {:06x}", pivot);
                    break SearchResult::Found(pivot);
                }
                high = pivot + 1;
                //pivot = low + high_bit((high - low) / 2);
                assert!(pivot > low);
                pivot -= high_bit((pivot - low) / 2) + 1;
            },
            DaliSendResult::Timeout => {
                //println!("Found none");
                if let Some(lm) =   *low_multiple {
                    if pivot + 2 > lm {
                        break SearchResult::Conflict(pivot + 1);
                    }
                }
                if pivot == high - 1 {
                    break SearchResult::NoneFound;
                        }
                low = pivot + 1;
                pivot += high_bit((high - pivot) / 2);
            },
            DaliSendResult::Framing => {
                //println!("Found multiple");
                *low_multiple = Some(pivot+1);
                if low >= pivot {
                    //println!("Conflict");
                    // There should only be one address <= pivot
                    break SearchResult::Conflict(pivot);
                }
                high = pivot;
                pivot -= high_bit((pivot - low) / 2) + 1;
                
                
            },
	    DaliSendResult::Answer(d) => {
		println!("Got unexpected reply {:02x}",d); 
		// break Err(DaliSendResult::Framing) 
		*low_multiple = Some(pivot+1);
		if low >= pivot {
                    // There should only be one address <= pivot
                break SearchResult::ReplyError;
		}
		high = pivot;
		pivot -= high_bit((pivot - low) / 2);
            
            }
	    e => return SearchResult::DriverError(Box::new(e)),
	}
    }
}


async fn find_devices_no_initialise(driver: &mut dyn DaliDriver, 
                                        tx: &mut Sender<DiscoverItem>) 
                                        -> Result<u32, Box<dyn Error + Send + Sync>>
{
    let mut current_search_addr = 0x010101;
    
    // All address at or below this is already found
    let mut low = 0u32;

    // Lowest address found that has any devices at or below it
    let mut high = 0x1000000u32;
    
    // Highest address tried with only a single address less or equal
    let mut high_single = None; 
    
    
    let mut low_multiple = None;

    let search_res: Result<u32,Box<dyn Error + Send + Sync>> = loop {
        //println!("Searching {:06x} - {:06x}", low, high); 
        match find_device(driver, low, high,
                          &mut high_single, &mut low_multiple,
                          &mut current_search_addr).await {
            SearchResult::Found(addr) => {
                let res = 
                    long_address::set_search_addr_changed(driver, addr,
                                                          &mut current_search_addr);
                if let Err(e) = res.await {
                    return Err(Box::new(e))
                };

                let res = driver.send_frame16(
		    &[cmd::QUERY_SHORT_ADDRESS, 0x00],
                    PRIORITY_1 | EXPECT_ANSWER);
                match res.await {
                    DaliSendResult::Answer(short_addr) => {
                        let short = if short_addr == 0xff {
                            None
                        } else {
                            Some(Short::new((short_addr>>1) + 1))
                        };
                        let msg = Ok(Discovered{
                            long:Some(addr),
                            short: short,
                            long_conflict: false, 
                            short_conflict: false
                        });
                        match send_blocking(tx, msg).await
                        {
                            Ok(()) => {},
                            Err(e) => 
                                return Err(Box::new(e))
                        }; 
                        //println!("Found device 0x{:06x} with short address {}",
                        //addr, (short_addr>>1) + 1);
                        let res = driver.send_frame16(&[cmd::WITHDRAW, 0x00],
                                                       PRIORITY_1);
                        res.await.check_send()?;

                        match (high_single, low_multiple) {
                            (Some(hs), Some(lm)) => {
                                if hs < lm {
                                    low = hs;
                                } else {
                                    low = 0;
                                }
                                high = lm;
                            },
                            (Some(hs), None) => {
                                low = hs;
                                high = TOP_SEARCH_ADDR;
                            },
                            (None, _) => {
                                panic!("Found device but high_single not set");
                            }
                        }
			
                    },
                    DaliSendResult::Framing
			| DaliSendResult::Timeout => {
                            low = 0;
			},
                    res => {
			break Err(Box::new(res))
                    }
		}
	    },
            SearchResult::NoneFound => {
                if high == TOP_SEARCH_ADDR {
                    break Ok(0);
                }
                high = TOP_SEARCH_ADDR;
            },
            SearchResult::Conflict(addr) => {
                let res = 
                    long_address::set_search_addr_changed(driver, addr,
                                                          &mut current_search_addr);
                if let Err(e) = res.await {
                    return Err(Box::new(e))
                };

                let res = driver.send_frame16(&[cmd::WITHDRAW, 0x00],
                                              PRIORITY_1);
                res.await.check_send()?;
                let msg = Ok(Discovered{
                    long:Some(addr),
                    short: None,
                    long_conflict: true, 
                    short_conflict: false
                });
                match send_blocking(tx, msg).await
                {
                    Ok(()) => {},
                    Err(e) => 
                        return Err(Box::new(e))
                };
                low = addr + 1;
                high = TOP_SEARCH_ADDR;
                
            },
            SearchResult::ReplyError => {
                low = 0;
                high = TOP_SEARCH_ADDR;
            },
            SearchResult::DriverError(e) => {
                break Err(e)
            }
	}
    };
    
    //println!("Search terminated: {:?}", search_res);
    //println!("Next: {:06x} - {:06x}", high_single, low_multiple);
    search_res
}

/// Addresses of devices discovered on the bus.
#[derive(Debug)]
pub struct Discovered
{
    /// Random address for device. None if there's conflicting short addresses or the device doesn't report a random address.
    pub long: Option<Long>,
    /// Short address if available
    pub short: Option<Short>,
    /// There's multiple devices with the same random address
    pub long_conflict: bool,
    /// There's multiple devices with the same short address
    pub short_conflict: bool
}


async fn discover_async(d: &mut dyn DaliDriver,
                        tx: &mut Sender<DiscoverItem>)
                        -> Result<(), Box<dyn Error + Send + Sync>>
{
    let mut found_short = [Option::<Long>::None;64];
    for index in 0..64usize {
        let a = Short::new((index+1) as u8);
        //eprintln!("Addr: {}", a);
        let mut retry = 3u32;
        match loop {
            match long_address::get_random_addr(d,&a).await {
                Ok(l) => {
                    found_short[index] = Some(l);
                    break Ok(l);
                }
                Err(DaliSendResult::Timeout) => {
                    retry -= 1;
                    if retry == 0 {
                        //eprintln!("Timeout");
                        break Err(DaliSendResult::Timeout)
                    }
                },
                Err(DaliSendResult::Framing) => {
                    //eprintln!("Multiple");
                    break Err(DaliSendResult::Framing)
                },
                Err(e) => return Err(Box::new(e))
            }
        } {
            Ok(l) => {
                send_blocking(tx, 
                              Ok(Discovered{long:Some(l),
                                            short:Some(a),
                                            long_conflict: false, 
                                            short_conflict: false}))
                    .await?;
            },
            Err(DaliSendResult::Framing) => {
                send_blocking(tx, Ok(Discovered{long:None,
                                                short:Some(a),
                                                long_conflict: false, 
                                                short_conflict: true}))
                    .await?;
            },
            Err(DaliSendResult::Timeout) => {
                
            },
            Err(e) => return Err(Box::new(e))
            
        }
        
    }
    d.send_frame16(&[cmd::INITIALISE, cmd::INITIALISE_ALL], 
                    PRIORITY_1|SEND_TWICE)
	.await.check_send()?;
    
    
    let mut current_search_addr = 0xffffffff;

    // WITHDRAW all devices with a short address
    for index in 0..64usize {
        if let Some(l) = found_short[index] {
            long_address::set_search_addr_changed(d,l, &mut current_search_addr)
                .await?;                }
        d.send_frame16(&[cmd::WITHDRAW, 0x00],
                        PRIORITY_1).await.check_send()?;
    }
    find_devices_no_initialise(d,tx).await?;
    Ok(())
}

pub type DiscoverItem = Result<Discovered, Box<dyn Error + Send + Sync>>;

async fn discover_thread(mut tx: tokio::sync::mpsc::Sender<DiscoverItem>,
                   driver: Arc<Mutex<Box<dyn DaliDriver>>>)
{
    let mut d = driver.lock().await;
    match discover_async(d.as_mut(), &mut tx).await {
        Ok(()) => {},
        Err(e) => {
            send_blocking(&mut tx, Err(e)).await.unwrap();
        }
    };
    match d.send_frame16(&[cmd::TERMINATE, 0x00], PRIORITY_1).await
    {
        // Ignore any errors
        _ => {}
    }    

}

/// Find all devices on the bus.
///
/// Returns a stream of all devices found.
/// Devices are found by first testing all 64 short addresses and then,
/// after WITHDRAWing the addresses found,
/// search for unaddressed devices using COMPARE.

pub fn find_quick<'a>(driver: Arc<Mutex<Box<dyn DaliDriver>>>)
                  -> Pin<Box<dyn Stream<Item = DiscoverItem>>>
{
    let (tx,rx) = tokio::sync::mpsc::channel(64);
    tokio::spawn(async move {
        discover_thread(tx,driver).await
    });
    return Box::pin(ReceiverStream::new(rx))
}
