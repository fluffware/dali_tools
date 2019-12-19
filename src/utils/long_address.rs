use crate::base::address::Long;
use crate::base::address::Short;
use crate::drivers::driver::{self,DALIdriver,DALIcommandError};
use crate::defs::gear::cmd;

pub async fn set_search_addr_changed(driver: &mut dyn DALIdriver, 
                         addr: Long, current: &mut Long)
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

pub async fn set_search_addr(driver: &mut dyn DALIdriver, addr: Long)
                         -> Result<u8, DALIcommandError>
{
    
    let res = driver.send_command(&[cmd::SEARCHADDRH,
                                    (addr>>16 & 0xff) as u8],
                                  driver::PRIORITY_1);
    res.await?;
    
    let res = driver.send_command(&[cmd::SEARCHADDRM,
                                    (addr>>8 & 0xff) as u8], 
                                  driver::PRIORITY_1);
    res.await?;
    
    let res = driver.send_command(&[cmd::SEARCHADDRL, (addr & 0xff) as u8],
                                  driver::PRIORITY_1);
    res.await?;

    Ok(0)
}

pub async fn get_random_addr(driver: &mut dyn DALIdriver, addr: &Short)
                         -> Result<Long, DALIcommandError>
{
    
    let res = driver.send_device_cmd(addr,
                                     cmd::QUERY_RANDOM_ADDRESS_H,
                                     driver::PRIORITY_1|driver::EXPECT_ANSWER);
    let h = res.await?;
    
    let res = driver.send_device_cmd(addr,
                                     cmd::QUERY_RANDOM_ADDRESS_M,
                                     driver::PRIORITY_1|driver::EXPECT_ANSWER);
    let m = res.await?;
    
    let res = driver.send_device_cmd(addr, 
                                     cmd::QUERY_RANDOM_ADDRESS_L,
                                     driver::PRIORITY_1|driver::EXPECT_ANSWER);
    let l = res.await?;
    
    Ok(((h as Long) << 16) | ((m as Long) << 8) | (l as Long))
}

