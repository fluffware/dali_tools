use super::sim_scheduler::{SimulatorEvent, SimulatorScheduler, SimulatorTask, SimulatorTaskId};
use futures::FutureExt;
use std::any::Any;
use std::collections::BinaryHeap;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::time::Instant;
use tokio::sync::oneshot;

pub struct WaitingTask {
    task_id: SimulatorTaskId,
    when: Option<Instant>, // None means it never times out
    trig: oneshot::Sender<SimulatorEvent>,
}

impl std::cmp::PartialEq for WaitingTask {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when
    }
}
impl std::cmp::Eq for WaitingTask {}

impl std::cmp::Ord for WaitingTask {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.when.cmp(&other.when).reverse()
    }
}
impl std::cmp::PartialOrd for WaitingTask {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub struct SchedulerData {
    current_time: RwLock<Instant>,
    active_task_count: AtomicUsize,
    wait_queue: RwLock<BinaryHeap<WaitingTask>>,
}

impl SchedulerData {
    fn check_timeout(self: &Arc<Self>) {
        let mut queue = self.wait_queue.write().unwrap();
        // Remove waiting futures that has been dropped
        queue.retain(|w| !w.trig.is_closed());
        let mut current_time = self.current_time.write().unwrap();
        while let Some(next) = queue.peek() {
            if let Some(when) = next.when {
                if when <= *current_time {
                    let task = queue.pop().unwrap();
                    println!("Trigged");
                    let _ = task.trig.send(SimulatorEvent::Timeout);
                } else {
                    println!(
                        "Linked: {} Queued: {}",
                        self.active_task_count.load(Ordering::Relaxed),
                        queue.len(),
                    );
                    if self.active_task_count.load(Ordering::Relaxed) > queue.len() {
                        break;
                    }
                    *current_time = when;
                }
            } else {
                break;
            }
        }
    }
}

impl SimulatorTask for SimulatorTaskImpl {
    fn current_time(&self) -> Instant {
        *self.data.current_time.read().unwrap()
    }

    /// All tasks must eventually wait, otherwise time may not
    /// progress
    fn wait_until(
        &mut self,
        when: Instant,
    ) -> Pin<Box<dyn Future<Output = SimulatorEvent> + Send>> {
        let (tx, rx) = oneshot::channel();
        let mut queue = self.data.wait_queue.write().unwrap();
        queue.push(WaitingTask {
            task_id: self.task_id,
            when: Some(when),
            trig: tx,
        });
        drop(queue);
        self.data.check_timeout();
        Box::pin(rx.map(|r| match r {
            Ok(ev) => ev,
            Err(_) => SimulatorEvent::Timeout,
        }))
    }
    fn wait(&mut self) -> Pin<Box<dyn Future<Output = SimulatorEvent> + Send>> {
        let (tx, rx) = oneshot::channel();
        let mut queue = self.data.wait_queue.write().unwrap();
        queue.push(WaitingTask {
            task_id: self.task_id,
            when: None,
            trig: tx,
        });
        drop(queue);
        self.data.check_timeout();
        Box::pin(rx.map(|r| match r {
            Ok(ev) => ev,
            Err(_) => SimulatorEvent::Timeout,
        }))
    }

    fn send_msg(&self, task_id: Option<SimulatorTaskId>, msg: Arc<dyn Any + Send + Sync>) {
        let mut queue = self.data.wait_queue.write().unwrap();
        if let Some(task_id) = task_id {
            for ev in queue.iter() {
                if task_id == task_id {
                    ev.trig.send(SimulatorEvent::Message(msg.clone()));
                    false
                } else {
                    true
                }
            }
        } else {
            for ev in queue.drain() {
                ev.trig.send(SimulatorEvent::Message(msg.clone()));
            }
            queue.clear();
        }
        drop(queue);
    }

    fn real_time(&self) -> bool {
        false
    }
}

impl Drop for SimulatorTaskImpl {
    fn drop(&mut self) {
        println!("Dropping");
        if self.active.swap(false, Ordering::Relaxed) {
            self.data.active_task_count.fetch_sub(1, Ordering::Relaxed);
        }
        self.data.check_timeout();
    }
}

static NEXT_TASK_ID: AtomicU32 = AtomicU32::new(1);
fn get_next_task_id() -> SimulatorTaskId {
    SimulatorTaskId::new(NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed)).unwrap()
}

pub struct SimulatorTaskImpl {
    data: Arc<SchedulerData>,
    active: AtomicBool,
    task_id: SimulatorTaskId,
}

pub struct SimulatorSchedulerImpl {
    data: Arc<SchedulerData>,
}

impl SimulatorSchedulerImpl {
    pub fn new() -> SimulatorSchedulerImpl {
        let data = SchedulerData {
            active_task_count: AtomicUsize::new(0),
            current_time: RwLock::new(Instant::now()),
            wait_queue: RwLock::new(BinaryHeap::new()),
        };
        SimulatorSchedulerImpl {
            data: Arc::new(data),
        }
    }
}

impl SimulatorScheduler for SimulatorSchedulerImpl {
    fn new_task(&mut self) -> Box<dyn SimulatorTask + Send + Sync> {
        self.data.active_task_count.fetch_add(1, Ordering::Relaxed);
        Box::new(SimulatorTaskImpl {
            task_id: get_next_task_id(),
            data: self.data.clone(),
            active: AtomicBool::new(true),
        })
    }
}

#[cfg(test)]
mod test {
    use super::super::sim_scheduler::SimulatorTask;
    use super::SimulatorScheduler;
    use super::SimulatorSchedulerImpl;
    use std::pin::Pin;
    use std::time::Duration;

    async fn wait_task(mut task: Box<dyn SimulatorTask + Send + Sync>, delay: Duration) {
        let start_time = task.current_time();
        println!("Task waiting");
        task.wait_until(start_time + delay).await.unwrap();
        assert_eq!(task.current_time(), start_time + delay);
        println!("Task done");
    }
    #[tokio::test]
    async fn scheduler_test() {
        let mut sched = SimulatorSchedulerImpl::new();
        let mut task1 = sched.new_task();
        let start_time = task1.current_time();
        task1
            .wait_until(task1.current_time() + Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(task1.current_time(), start_time + Duration::from_secs(1));
        let task2 = sched.new_task();
        let w2 = tokio::spawn(wait_task(task2, Duration::from_secs(3)));
        task1
            .wait_until(task1.current_time() + Duration::from_secs(1))
            .await
            .unwrap();
        assert_eq!(task1.current_time(), start_time + Duration::from_secs(2));

        let when = task1.current_time() + Duration::from_secs(4);
        drop(task1.wait_until(when));

        println!("Task1 waiting");
        task1.wait_until(when).await.unwrap();
        assert_eq!(task1.current_time(), start_time + Duration::from_secs(6));
        println!("Task1 done");
        w2.await;
    }
}
