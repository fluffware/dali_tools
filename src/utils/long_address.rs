use crate::common::address::Long;
use crate::common::commands::Commands;

pub async fn set_search_addr_changed<C>(
    commands: &mut C,
    addr: Long,
    current: &mut Long,
) -> Result<(), C::Error>
where
    C: Commands,
{
    let diff = addr ^ *current;
    if (diff & 0xff0000) != 0 {
        commands.searchaddr_h((addr >> 16 & 0xff) as u8).await?;
    }
    if (diff & 0x00ff00) != 0 {
        commands.searchaddr_m((addr >> 8 & 0xff) as u8).await?;
    }
    if (diff & 0x0000ff) != 0 {
        commands.searchaddr_l((addr & 0xff) as u8).await?;
    }
    *current = addr;
    Ok(())
}

pub async fn set_search_addr<C>(commands: &mut C, addr: Long) -> Result<(), C::Error>
where
    C: Commands,
{
    commands.searchaddr_h((addr >> 16 & 0xff) as u8).await?;
    commands.searchaddr_m((addr >> 8 & 0xff) as u8).await?;
    commands.searchaddr_l((addr >> 16 & 0xff) as u8).await?;
    Ok(())
}
