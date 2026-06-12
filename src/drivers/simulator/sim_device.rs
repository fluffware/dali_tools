pub trait DaliSimDevice: Send {
    /// Called when the device is connected to a bus
    fn start(
        &mut self,
        host: Box<dyn SimulatorTask>,
    ) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>;
    /// Called when disconnected from the host
    fn stop(&mut self) -> Pin<Box<dyn Future<Output = DynResult<()>> + Send>>;
    /// A new event has been dispatched on the bus
    fn event(&mut self, event: &DaliSimBusEvent) -> Option<DaliSimBusEvent>;
}
