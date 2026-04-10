use std::num::NonZeroU16;
use std::pin::Pin;

pub type DynResultFuture<T> =
    Pin<Box<dyn Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>> + Send>>;

#[derive(Clone, Debug)]
pub struct GearConfiguration {
    pub id: GearId,
    pub conf: ConfigurationId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GearId(NonZeroU16);
impl TryFrom<u16> for GearId {
    type Error = <NonZeroU16 as TryFrom<u16>>::Error;
    fn try_from(id: u16) -> Result<GearId, Self::Error> {
        Ok(GearId(NonZeroU16::try_from(id)?))
    }
}

impl Into<u16> for GearId {
    fn into(self) -> u16 {
        u16::from(self.0)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ConfigurationId(NonZeroU16);
impl TryFrom<u16> for ConfigurationId {
    type Error = <NonZeroU16 as TryFrom<u16>>::Error;
    fn try_from(id: u16) -> Result<ConfigurationId, Self::Error> {
        Ok(ConfigurationId(NonZeroU16::try_from(id)?))
    }
}

impl Into<u16> for ConfigurationId {
    fn into(self) -> u16 {
        u16::from(self.0)
    }
}

#[derive(Clone, Debug)]
pub struct ConfigurationInfo {
    pub id: ConfigurationId,
    pub label: String,
}

#[derive(Clone, Debug)]
pub struct GearInfo {
    pub id: GearId,
    pub label: String,
    pub conf: Option<ConfigurationId>,
}

pub trait ConfigurationDriver: Send + Sync {
    fn start_configuration(&self) -> DynResultFuture<()>;
    fn end_configuration(&self) -> DynResultFuture<()>;
    fn set_low(&self, id: GearId) -> DynResultFuture<()>;
    fn set_all_low(&self) -> DynResultFuture<()>;
    fn set_high(&self, id: GearId) -> DynResultFuture<()>;
    fn find_all(&self, found: Box<dyn FnMut(GearInfo) + Send>) -> DynResultFuture<()>;
    fn configurations(&self) -> Vec<ConfigurationInfo>;

    // Invalidates all gear ids
    fn commit(&self, gears: Vec<GearConfiguration>) -> DynResultFuture<()>;
}
