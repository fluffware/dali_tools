use crate::base::address::Long;
use crate::base::address::Short;
use crate::drivers::driver::{DaliDriver,DaliSendResult};
use crate::drivers::command_utils::send_device_cmd;
use crate::defs::gear::cmd;
use crate::drivers::send_flags::{PRIORITY_1, EXPECT_ANSWER};

pub async fn set_search_addr_changed(driver: &mut dyn DaliDriver, 
                         addr: Long, current: &mut Long)
                         -> Result<u8, DaliSendResult>
{
    let diff = addr ^ *current;
    if (diff & 0xff0000) != 0 {
        let res = driver.send_frame16(&[cmd::SEARCHADDRH,
					(addr>>16 & 0xff) as u8],
                                      PRIORITY_1);
        res.await.check_send()?;
    }
    if (diff & 0x00ff00) != 0 {
        let res = driver.send_frame16(&[cmd::SEARCHADDRM,
					(addr>>8 & 0xff) as u8], 
                                      PRIORITY_1);
        res.await.check_send()?;
    }
    if (diff & 0x0000ff) != 0 {
        let res = driver.send_frame16(&[cmd::SEARCHADDRL, 
					(addr & 0xff) as u8],
				      
				      PRIORITY_1);
        res.await.check_send()?;
    }
    *current = addr;
    Ok(0)
}

pub async fn set_search_addr(driver: &mut dyn DaliDriver, addr: Long)
                         -> Result<u8, DaliSendResult>
{
    
    let res = driver.send_frame16(&[cmd::SEARCHADDRH,
                                     (addr>>16 & 0xff) as u8],
                                   PRIORITY_1);
    res.await.check_send()?;
    
    let res = driver.send_frame16(&[cmd::SEARCHADDRM,
                                     (addr>>8 & 0xff) as u8], 
                                   PRIORITY_1);
    res.await.check_send()?;
    
    let res = driver.send_frame16(&[cmd::SEARCHADDRL, (addr & 0xff) as u8],
                                   PRIORITY_1);
    res.await.check_send()?;

    Ok(0)
}

pub async fn get_random_addr(driver: &mut dyn DaliDriver, addr: &Short)
                         -> Result<Long, DaliSendResult>
{
    
    let res = send_device_cmd(driver, addr,
                              cmd::QUERY_RANDOM_ADDRESS_H,
                              PRIORITY_1|EXPECT_ANSWER);
    let h = res.await.check_answer()?;
    
    let res = send_device_cmd(driver, addr,
                              cmd::QUERY_RANDOM_ADDRESS_M,
                              PRIORITY_1|EXPECT_ANSWER);
    let m = res.await.check_answer()?;
    
    let res = send_device_cmd(driver, addr, 
                              cmd::QUERY_RANDOM_ADDRESS_L,
                              PRIORITY_1|EXPECT_ANSWER);
    let l = res.await.check_answer()?;
    
    Ok(((h as Long) << 16) | ((m as Long) << 8) | (l as Long))
}

