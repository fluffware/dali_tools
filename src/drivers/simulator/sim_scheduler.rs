use std::any::Any;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

pub enum SimulatorEvent {
    Timeout,
    Message(Arc<dyn Any + Send + Sync>),
    Shutdown,
}

pub type SimulatorTaskId = NonZeroU32;

pub trait SimulatorTask {
    /// Current simulated instant
    fn current_time(&self) -> Instant;

    /// All tasks must evetually wait, otherwise
    /// time may not progress
    fn wait_until(&mut self, when: Instant)
    -> Pin<Box<dyn Future<Output = SimulatorEvent> + Send>>;

    // Let time progress without waiting for this task
    fn wait(&mut self) -> Pin<Box<dyn Future<Output = SimulatorEvent> + Send>>;

    fn send_msg(&self, task_id: Option<SimulatorTaskId>, msg: Arc<dyn Any + Send + Sync>);

    fn real_time(&self) -> bool;
}

pub trait SimulatorScheduler {
    fn new_task(&mut self) -> Box<dyn SimulatorTask + Send + Sync>;
}
