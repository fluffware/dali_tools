use super::driver::{DaliDriver, DaliFrame, DaliSendResult};
use super::send_flags::Flags;
use crate::utils::dyn_future::DynFuture;

pub trait DaliDriverExt: DaliDriver {
    fn send_frame16(&mut self, cmd: &[u8; 2], flags: Flags) -> DynFuture<'_, DaliSendResult>;

    fn send_frame24(&mut self, cmd: &[u8; 3], flags: Flags) -> DynFuture<'_, DaliSendResult>;
}

impl<T> DaliDriverExt for T
where
    T: DaliDriver + ?Sized,
{
    fn send_frame16(&mut self, cmd: &[u8; 2], flags: Flags) -> DynFuture<'_, DaliSendResult> {
        let cmd = DaliFrame::Frame16(cmd.clone());
        self.send_frame(cmd, flags)
    }

    fn send_frame24(&mut self, cmd: &[u8; 3], flags: Flags) -> DynFuture<'_, DaliSendResult> {
        let cmd = DaliFrame::Frame24(cmd.clone());
        self.send_frame(cmd, flags)
    }
}
