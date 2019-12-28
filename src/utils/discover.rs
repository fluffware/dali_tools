use crate::defs::gear::cmd;
use crate::drivers::driver::{self,DALIdriver,DALIcommandError};
use crate::utils::long_address;
use crate::base::address::Short;
use crate::base::address::Long;
use futures::stream::Stream;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;
use futures::executor::block_on;
use std::thread;
use std::time::Duration;
use futures::channel::mpsc::{Sender,TrySendError};
use std::error::Error;

fn send_blocking<T>(tx:&mut Sender<T>, item : T)
                    -> Result<(), TrySendError<T>>
{
    let mut sent = item;
    loop {
        let res = tx.try_send(sent);
        match res {
            Ok(()) => break,
            Err(r) => {
                if r.is_full() {
                    thread::sleep(Duration::from_millis(1000));
                    sent = r.into_inner();
                } else {
                return Err(r);
                }
            }
        }
    }
    Ok(())
}

// Clear all bits except for the highest one
fn high_bit(mut bits: u32) -> u32
{
    bits |= bits >> 1;
    bits |= bits >> 2;
    bits |= bits >> 4;
    bits |= bits >> 8;
    bits |= bits >> 16;
    (bits >> 1) + 1
}

/* Search low..high for the device with the lowest address 

 */
const TOP_SEARCH_ADDR:u32 = 0x1000000; 

async fn find_device(driver: &mut dyn DALIdriver, mut low: u32, mut high: u32,
                     high_single: &mut u32, low_multiple: &mut u32, 
                     current_search_addr: &mut u32) 
                     -> Result<u32, DALIcommandError>
{
    let mut pivot = low + high_bit((high - low) / 2);
    *high_single = low;
    *low_multiple = TOP_SEARCH_ADDR;
    // Search until we match a single address
    loop {
        long_address::set_search_addr_changed(driver, pivot,
                                              current_search_addr).await?;
        // COMPARE
        let res = driver.send_command(&[cmd::COMPARE, 0x00], 
                                      driver::PRIORITY_1|driver::EXPECT_ANSWER);
        match res.await {
            Ok(0xff) => {
                //println!("Found one");
                if pivot > *high_single {
                    *high_single = pivot + 1;
                }
                if low  >= pivot {
                    //println!("Found device {:06x}", pivot);
                    break Ok(pivot);
                }
                high = pivot + 1;
                pivot -= high_bit((pivot - low) / 2);
            },
            Err(e) => {
                match e {
                    DALIcommandError::Timeout => {
                        //println!("Found none");
                        if pivot == high - 1 {
                            // high must have at least one below it
                            break Err(DALIcommandError::Timeout);
                        }
                        low = pivot + 1;
                        pivot += high_bit((high - pivot) / 2);
                    },
                    DALIcommandError::Framing => {
                        //println!("Found multiple");
                        *low_multiple = pivot+1;
                        if low >= pivot {
                            // There should only be one address <= pivot
                            break Err(DALIcommandError::Framing);
                        }
                        high = pivot;
                        pivot -= high_bit((pivot - low) / 2);
                        
                        
                    },
                    _ => return Err(e)
                }
            },
            Ok(d) => {
                println!("Got unexpected reply {:02x}",d); 
                // break Err(DALIcommandError::Framing) 
                *low_multiple = pivot+1;
                if low >= pivot {
                    // There should only be one address <= pivot
                    break Err(DALIcommandError::Framing);
                }
                high = pivot;
                pivot -= high_bit((pivot - low) / 2);
                
            }
        };
    }
}

pub async fn find_devices(driver: &mut dyn DALIdriver) 
                                        -> Result<u32, DALIcommandError>
{
    // INITILISE all
    let res = driver.send_command(&[cmd::INITIALISE, cmd::INITIALISE_ALL], 
                                  driver::PRIORITY_2|driver::SEND_TWICE);
    res.await?;
    /*
    let res =find_devices_no_initialise(driver).await;
*/
    driver.send_command(&[cmd::TERMINATE, 0x00], driver::PRIORITY_1).await?;
    //res
    Ok(0)
}
pub async fn find_devices_no_initialise(driver: &mut dyn DALIdriver, 
                                        tx: &mut Sender<DiscoverItem>) 
                                        -> Result<u32, Box<dyn Error + Send>>
{
    let mut current_search_addr = 0x010101;
    
    // All address at or below this is already found
    let mut low = 0u32;

    // Lowest address found that has any devices at or below it
    let mut high = 0x1000000u32;
    
    // Highest address tried with only a single address less or equal
    let mut high_single = 0u32; 
    
    
    let mut low_multiple = TOP_SEARCH_ADDR;

    let search_res: Result<u32,Box<dyn Error + Send>> = loop {
        println!("Searching {:06x} - {:06x}", low, high); 
        match find_device(driver, low, high,
                          &mut high_single, &mut low_multiple,
                          &mut current_search_addr).await {
            Ok(addr) => {
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
                        match send_blocking(tx, msg)
                        {
                            Ok(()) => {},
                            Err(e) => 
                                return Err(Box::new(e) as Box<dyn Error + Send>)
                        }; 
                        //println!("Found device 0x{:06x} with short address {}",
                        //addr, (short_addr>>1) + 1);
                        let res = driver.send_command(&[cmd::WITHDRAW, 0x00],
                                                      driver::PRIORITY_1);
                        if let Err(e) = res.await {
                            return Err(Box::new(e) as Box<dyn Error + Send>)
                        }; 
                        if high_single < low_multiple {
                            low = high_single;
                        } else {
                            low = 0;
                        }
                        high = low_multiple;

                    },
                    Err(DALIcommandError::Framing) 
                        | Err(DALIcommandError::Timeout) => {
                            low = 0;
                        },
                    Err(e) => {
                        break Err(Box::new(e) as Box<dyn Error + Send>)
                    }
                }
                               
                
            },
            Err(DALIcommandError::Timeout) => {
                if high == TOP_SEARCH_ADDR {
                    break Ok(0);
                }
                high = TOP_SEARCH_ADDR;
            },
            Err(DALIcommandError::Framing) => {
                low = 0;
                high = TOP_SEARCH_ADDR;
            },
            Err(e) => {
                break Err(Box::new(e) as Box<dyn Error + Send>)
            }
        }
    };
    
    //println!("Search terminated: {:?}", search_res);
    //println!("Next: {:06x} - {:06x}", high_single, low_multiple);
    search_res
}

#[derive(Debug)]
pub struct Discovered
{
    long: Option<Long>,
    short: Option<Short>,
    long_conflict: bool,
    short_conflict: bool
}


async fn discover_async(d: &mut dyn DALIdriver,
                        tx: &mut Sender<DiscoverItem>)
                        -> Result<(), Box<dyn Error + Send>>
{
    let mut found_short = [Option::<Long>::None;64];
    for index in 0..64usize {
        let a = Short::new((index+1) as u8);
        eprintln!("Addr: {}", a);
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
                        eprintln!("Timeout");
                        break Err(DALIcommandError::Timeout)
                    }
                },
                Err(DALIcommandError::Framing) => {
                    eprintln!("Multiple");
                    break Err(DALIcommandError::Framing)
                },
                Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>)
            }
        } {
            Ok(l) => {
                match send_blocking(tx, Ok(Discovered{long:Some(l),
                                                      short:Some(a),
                                                      long_conflict: false, 
                                                      short_conflict: false})) {
                    Ok(()) => {},
                    Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>)
                };
            },
            Err(DALIcommandError::Framing) => {
                match send_blocking(tx, Ok(Discovered{long:None,
                                                      short:Some(a),
                                                      long_conflict: false, 
                                                      short_conflict: true})) {
                    Ok(()) => {},
                    Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>)
                };
            },
            Err(DALIcommandError::Timeout) => {
                
            },
            Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>)
            
        }
        
    }
    match d.send_command(&[cmd::INITIALISE, cmd::INITIALISE_ALL], 
                         driver::PRIORITY_1|driver::SEND_TWICE).await {
        Ok(_) => (),
        Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>)
    };

    let mut current_search_addr = 0xffffffff;

    // WITHDRAW all devices with a short address
    for index in 0..64usize {
        if let Some(l) = found_short[index] {
            match long_address::set_search_addr_changed(d,l, 
                                                        &mut current_search_addr)
                .await {
                    Ok(_) => {},
                    Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>)
                }
            match d.send_command(&[cmd::WITHDRAW, 0x00],
                                      driver::PRIORITY_1).await {
                Ok(_) => {},
                Err(e) => return Err(Box::new(e) as Box<dyn Error + Send>)
            }
        }
    }
    match find_devices_no_initialise(d,tx).await {
        Ok(_) => {},
        Err(e) => return Err(e)
    }
    Ok(())
}

pub type DiscoverItem = Result<Discovered, Box<dyn Error + Send>>;
fn discover_thread(mut tx: futures::channel::mpsc::Sender<DiscoverItem>,
                   driver: Arc<Mutex<dyn DALIdriver>>)
{
    let mut d = driver.lock().unwrap();
    match block_on(discover_async(&mut *d, &mut tx)) {
        Ok(()) => {},
        Err(e) => {
            send_blocking(&mut tx, Err(e)).unwrap();
        }
    };
    block_on(d.send_command(&[cmd::TERMINATE, 0x00], driver::PRIORITY_1))
        .unwrap();

}

pub fn find_quick<'a>(driver: Arc<Mutex<dyn DALIdriver>>)
                  -> Pin<Box<dyn Stream<Item = DiscoverItem>>>
{
    let (tx,rx) = futures::channel::mpsc::channel(64);
    std::thread::spawn(|| {
        discover_thread(tx,driver)
    });
    return Box::pin(rx)
}
