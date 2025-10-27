use crate as dali;
use crate::common::commands::Commands;
use crate::common::commands::ErrorInfo;
use crate::utils::long_address::set_search_addr;
use dali::common::address::{Long, Short};
use log::debug;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug)]
pub enum Error<E> {
    Send(E),
    AddressValidation,
    AddressCollision,
}

impl<E> From<E> for Error<E> {
    fn from(result: E) -> Error<E> {
        Self::Send(result)
    }
}

impl<E> std::error::Error for Error<E> where E: std::fmt::Display + std::fmt::Debug {}

impl<E> std::fmt::Display for Error<E>
where
    E: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Send(res) => res.fmt(f),
            Error::AddressValidation => {
                write!(f, "Failed to set short address")
            }
            Error::AddressCollision => {
                write!(f, "Duplicate short addresses")
            }
        }
    }
}

pub async fn program_short_address<C>(
    commands: &mut C,
    long: Long,
    short: Short,
) -> Result<(), Error<C::Error>>
where
    C: Commands,
{
    debug!("{} set address {:?}", long, short);
    set_search_addr(commands, long).await?;
    commands.program_short_address(Some(short)).await?;
    let a = commands.query_short_address().await?;
    match a {
        Some(a) if a == short => {}
        Some(_) | None => return Err(Error::AddressValidation),
    }
    //println!("Set {}, got {}", short, (a>>1)+1);
    Ok(())
}

pub async fn clear_short_address<C>(commands: &mut C, long: Long) -> Result<(), Error<C::Error>>
where
    C: Commands,
{
    debug!("Clearing {}", long);
    set_search_addr(commands, long).await?;
    commands.program_short_address(None).await?;
    let a = commands.query_short_address().await?;
    if !a.is_none() {
        return Err(Error::AddressValidation);
    }
    Ok(())
}

pub async fn program_short_addresses<C>(
    commands: &mut C,
    map: &[(Short, Short)],
) -> Result<(), Error<C::Error>>
where
    C: Commands,
{
    // Keep track of unused addresses
    let mut old_set = BTreeSet::new();
    // All long addresses before remapping
    let mut old_map = BTreeMap::new();

    // Gather all long addresses
    for (old, new) in map {
        debug!("Map {} -> {}", old, new);
        if !old_set.insert(old) {
            return Err(Error::AddressCollision);
        }
        if !old_map.contains_key(old) {
            let long_old = commands.query_random_address(*old).await?;
            old_map.insert(old, long_old);
        }
        if !old_map.contains_key(new) {
            match commands.query_random_address(*new).await {
                Ok(long_new) => {
                    old_map.insert(new, long_new);
                }
                Err(e) => {
                    if !e.is_timeout() {
                        return Err(e.into());
                    }
                }
            }
        }
    }
    commands.initialise_all().await?;

    // Remap according to list
    for (old, new) in map {
        if let Some(long) = old_map.get(new) {
            clear_short_address(commands, *long).await?;
        }
        old_set.remove(new);
        if let Some(long) = old_map.remove(old) {
            program_short_address(commands, long, *new).await?;
        }
    }

    // Assign unused addresses
    for (long, new) in std::iter::zip(old_map.values(), old_set) {
        program_short_address(commands, *long, *new).await?;
    }
    commands.terminate().await?;
    Ok(())
}
