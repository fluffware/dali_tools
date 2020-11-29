use crate::defs::gear::cmd;
use crate::drivers::driver::{self,DALIdriver,DALIcommandError};
use crate::utils::long_address;
use crate::base::address::Short;
use crate::base::address::Long;
use tokio::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc::{Sender,error::SendError};
use std::error::Error;

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
    DriverError(Arc<DALIcommandError>)
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

async fn find_device(driver: &mut dyn DALIdriver, mut low: u32, mut high: u32,
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
            Err(e) => return SearchResult::DriverError(Arc::new(e))
        }
        // COMPARE
        let res = driver.send_command(&[cmd::COMPARE, 0x00], 
                                      driver::PRIORITY_1|driver::EXPECT_ANSWER);
        match res.await {
            Ok(0xff) => {
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
            Err(e) => {
                match e {
                    DALIcommandError::Timeout => {
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
                    DALIcommandError::Framing => {
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
                    _ => return SearchResult::DriverError(Arc::new(e))
                }
            },
            Ok(d) => {
                println!("Got unexpected reply {:02x}",d); 
                // break Err(DALIcommandError::Framing) 
                *low_multiple = Some(pivot+1);
                if low >= pivot {
                    // There should only be one address <= pivot
                    break SearchResult::ReplyError;
                }
                high = pivot;
                pivot -= high_bit((pivot - low) / 2);
                
            }
        };
    }
}


async fn find_devices_no_initialise(driver: &mut dyn DALIdriver, 
                                        tx: &mut Sender<DiscoverItem>) 
                                        -> Result<u32, Arc<dyn Error + Send + Sync>>
{
    let mut current_search_addr = 0x010101;
    
    // All address at or below this is already found
    let mut low = 0u32;

    // Lowest address found that has any devices at or below it
    let mut high = 0x1000000u32;
    
    // Highest address tried with only a single address less or equal
    let mut high_single = None; 
    
    
    let mut low_multiple = None;

    let search_res: Result<u32,Arc<dyn Error + Send + Sync>> = loop {
        //println!("Searching {:06x} - {:06x}", low, high); 
        match find_device(driver, low, high,
                          &mut high_single, &mut low_multiple,
                          &mut current_search_addr).await {
            SearchResult::Found(addr) => {
                let res = 
                    long_address::set_search_addr_changed(driver, addr,
                                                          &mut current_search_addr);
                if let Err(e) = res.await {
                    return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
                };

                let res = driver.send_command(&[cmd::QUERY_SHORT_ADDRESS, 0x00],
                                              driver::PRIORITY_1 
                                              | driver::EXPECT_ANSWER);
                match res.await {
                    Ok(short_addr) => {
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
                                return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
                        }; 
                        //println!("Found device 0x{:06x} with short address {}",
                        //addr, (short_addr>>1) + 1);
                        let res = driver.send_command(&[cmd::WITHDRAW, 0x00],
                                                      driver::PRIORITY_1);
                        if let Err(e) = res.await {
                            return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
                        };
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
                    Err(DALIcommandError::Framing) 
                        | Err(DALIcommandError::Timeout) => {
                            low = 0;
                        },
                    Err(e) => {
                        break Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
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
                    return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
                };

                let res = driver.send_command(&[cmd::WITHDRAW, 0x00],
                                              driver::PRIORITY_1);
                if let Err(e) = res.await {
                    return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
                };
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
                        return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
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


async fn discover_async(d: &mut dyn DALIdriver,
                        tx: &mut Sender<DiscoverItem>)
                        -> Result<(), Arc<dyn Error + Send + Sync>>
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
                Err(DALIcommandError::Timeout) => {
                    retry -= 1;
                    if retry == 0 {
                        //eprintln!("Timeout");
                        break Err(DALIcommandError::Timeout)
                    }
                },
                Err(DALIcommandError::Framing) => {
                    //eprintln!("Multiple");
                    break Err(DALIcommandError::Framing)
                },
                Err(e) => return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
            }
        } {
            Ok(l) => {
                match send_blocking(tx, 
                                    Ok(Discovered{long:Some(l),
                                                  short:Some(a),
                                                  long_conflict: false, 
                                                  short_conflict: false}))
                    .await {
                    Ok(()) => {},
                    Err(e) => return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
                };
            },
            Err(DALIcommandError::Framing) => {
                match send_blocking(tx, Ok(Discovered{long:None,
                                                      short:Some(a),
                                                      long_conflict: false, 
                                                      short_conflict: true}))
                .await {
                    Ok(()) => {},
                    Err(e) => return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
                };
            },
            Err(DALIcommandError::Timeout) => {
                
            },
            Err(e) => return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
            
        }
        
    }
    match d.send_command(&[cmd::INITIALISE, cmd::INITIALISE_ALL], 
                         driver::PRIORITY_1|driver::SEND_TWICE).await {
        Ok(_) => (),
        Err(e) => return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
    };

    let mut current_search_addr = 0xffffffff;

    // WITHDRAW all devices with a short address
    for index in 0..64usize {
        if let Some(l) = found_short[index] {
            match long_address::set_search_addr_changed(d,l, 
                                                        &mut current_search_addr)
                .await {
                    Ok(_) => {},
                    Err(e) => return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
                }
            match d.send_command(&[cmd::WITHDRAW, 0x00],
                                      driver::PRIORITY_1).await {
                Ok(_) => {},
                Err(e) => return Err(Arc::new(e) as Arc<dyn Error + Send + Sync>)
            }
        }
    }
    match find_devices_no_initialise(d,tx).await {
        Ok(_) => {},
        Err(e) => return Err(e)
    }
    Ok(())
}

pub type DiscoverItem = Result<Discovered, Arc<dyn Error + Send + Sync>>;

async fn discover_thread(mut tx: tokio::sync::mpsc::Sender<DiscoverItem>,
                   driver: Arc<Mutex<Box<dyn DALIdriver>>>)
{
    let mut d = driver.lock().await;
    match discover_async(d.as_mut(), &mut tx).await {
        Ok(()) => {},
        Err(e) => {
            send_blocking(&mut tx, Err(e)).await.unwrap();
        }
    };
    match d.send_command(&[cmd::TERMINATE, 0x00], driver::PRIORITY_1).await
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

pub fn find_quick<'a>(driver: Arc<Mutex<Box<dyn DALIdriver>>>)
                  -> Pin<Box<dyn Stream<Item = DiscoverItem>>>
{
    let (tx,rx) = tokio::sync::mpsc::channel(64);
    tokio::spawn(async move {
        discover_thread(tx,driver).await
    });
    return Box::pin(rx)
}
