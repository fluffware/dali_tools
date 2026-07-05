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

/// Identifies a specific task
pub type SimulatorTaskId = NonZeroU32;

/// Specifies the receiver of a message
#[derive(Clone)]
pub enum SimulatorMessageDest {
    /// Only this task
    Task(SimulatorTaskId),
    /// All tasks except this one
    Exclude(SimulatorTaskId),
    /// All tasks
    All,
}

pub trait SimulatorTask {
    /// Current simulated instant
    fn current_time(&self) -> Instant;

    /// All tasks must evetually wait, otherwise
    /// time may not progress
    fn wait_until(&self, when: Instant) -> Pin<Box<dyn Future<Output = SimulatorEvent> + Send>>;

    // Let time progress without waiting for this task
    fn wait(&self) -> Pin<Box<dyn Future<Output = SimulatorEvent> + Send>>;

    fn send_msg(&self, dest: SimulatorMessageDest, msg: Arc<dyn Any + Send + Sync>);

    fn shutdown(&self);

    fn real_time(&self) -> bool;

    fn task_id(&self) -> SimulatorTaskId;
}

pub trait SimulatorScheduler {
    fn new_task(&mut self) -> Box<dyn SimulatorTask + Send + Sync>;
}
