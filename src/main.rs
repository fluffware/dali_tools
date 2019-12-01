use dali_tools as dali;
use dali::drivers::driver::{self,DALIdriver,DALIcommandError};
use dali::drivers::helvar::helvar510::Helvar510driver;
use dali::defs::gear::cmd;
use futures::executor::block_on;






async fn set_search_addr(driver: &mut dyn DALIdriver, 
                         addr: u32, current: &mut u32)
                         -> Result<u8, DALIcommandError>
{
    let diff = addr ^ *current;
    if (diff & 0xff0000) != 0 {
        let res = driver.send_command(&[cmd::SEARCHADDRH,
                                        (addr>>16 & 0xff) as u8],
                                      driver::PRIORITY_1);
        res.await?;
    }
    if (diff & 0x00ff00) != 0 {
        let res = driver.send_command(&[cmd::SEARCHADDRM,
                                        (addr>>8 & 0xff) as u8], 
                                      driver::PRIORITY_1);
        res.await?;
    }
    if (diff & 0x0000ff) != 0 {
        let res = driver.send_command(&[cmd::SEARCHADDRL, (addr & 0xff) as u8],
                                      driver::PRIORITY_1);
        res.await?;
    }
    *current = addr;
    Ok(0)
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
        set_search_addr(driver, pivot, current_search_addr).await?;
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
                    println!("Found device {:06x}", pivot);
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

async fn find_devices(driver: &mut dyn DALIdriver) 
                      -> Result<u32, DALIcommandError>
{
    let mut current_search_addr = 0x010101;
    // INITILISE all
    let res = driver.send_command(&[cmd::INITIALISE, cmd::INITIALISE_ALL], 
                                  driver::PRIORITY_2|driver::SEND_TWICE);
    if let Err(e) = res.await {
        return Err(e);
    }

    // All address at or below this is already found
    let mut low = 0u32;

    // Lowest address found that has any devices at or below it
    let mut high = 0x1000000u32;
    
    // Highest address tried with only a single address less or equal
    let mut high_single = 0u32; 
    
    
    let mut low_multiple = TOP_SEARCH_ADDR;

    let search_res: Result<(),DALIcommandError> = loop {
        println!("Searching {:06x} - {:06x}", low, high); 
        match find_device(driver, low, high,
                          &mut high_single, &mut low_multiple,
                          &mut current_search_addr).await {
            Ok(addr) => {
                let res = driver.send_command(&[0xbb, 0x00],
                                              driver::PRIORITY_1 
                                              | driver::EXPECT_ANSWER);
                match res.await {
                    Ok(short_addr) => {
                        println!("Found device 0x{:06x} with short address {}",
                                 addr, short_addr);
                        let res = driver.send_command(&[cmd::WITHDRAW, 0x00],
                                                      driver::PRIORITY_1);
                        let _ = res.await?;
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
                        break Err(e)
                    }
                }
                               
                
            },
            Err(DALIcommandError::Timeout) => {
                if high == TOP_SEARCH_ADDR {
                    break Ok(());
                }
                high = TOP_SEARCH_ADDR;
            },
            Err(DALIcommandError::Framing) => {
                low = 0;
                high = TOP_SEARCH_ADDR;
            },
            Err(e) => {
                break Err(e)
            }
        }
    };
    println!("Search terminated: {:?}", search_res);
    println!("Next: {:06x} - {:06x}", high_single, low_multiple);
    // TERMINATE
    let res = driver.send_command(&[cmd::TERMINATE, 0x00], driver::PRIORITY_1);
    let _ = res.await?;
    Ok(0)
}

fn main() {
    let mut driver = Helvar510driver::new();
    match block_on(find_devices(&mut driver)) {
        Ok(_) => {},
        Err(e) => {
            println!("Failed while scanning for devices: {:?}",e);
        }
    }

    
}
