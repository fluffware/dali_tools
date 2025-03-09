use super::commands::Commands;
use crate::common::commands::ErrorInfo;
use crate::drivers::driver::{DaliDriver, DaliSendResult};
use crate::drivers::send_flags::Flags;

impl ErrorInfo for DaliSendResult {
    fn is_timeout(&self) -> bool {
        matches!(self, DaliSendResult::Timeout)
    }
    fn is_framing_error(&self) -> bool {
        matches!(self, DaliSendResult::Framing)
    }
}

/// Commands implemented on DaliDriver
pub trait DriverCommands: Commands {
    type Output<'a>: Commands<Error = DaliSendResult> + Send;
    fn from_driver<'a>(driver: &'a mut dyn DaliDriver, flags: Flags) -> Self::Output<'a>;
}
