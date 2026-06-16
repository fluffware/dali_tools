use super::sim_scheduler::{
    SimulatorEvent, SimulatorMessageDest, SimulatorScheduler, SimulatorTask, SimulatorTaskId,
};
use futures::FutureExt;
use std::any::Any;
use std::mem;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::Instant;
use tokio::sync::oneshot;

pub enum TaskState {
    // No task in this slot
    None,
    Running,
    // Waiting forever
    Wait {
        trig: oneshot::Sender<SimulatorEvent>,
    },
    // Waiting for an instant
    WaitUntil {
        trig: oneshot::Sender<SimulatorEvent>,
        when: Instant,
    },
}

type TaskIndex = usize;
pub struct SchedulerDataMut {
    current_time: Instant,
    active_task_count: usize,
    // Contains indices into task_states for those states that are
    // waiting.  Sorted by Wait first, followed by WaitUntil with the
    // earliest last
    wait_queue: Vec<TaskIndex>,
    task_states: Vec<TaskState>,
    message_queue: Vec<(SimulatorMessageDest, Arc<dyn Any + Send + Sync>)>,
}

pub struct SchedulerData {
    data: RwLock<SchedulerDataMut>,
}

impl SchedulerDataMut {
    fn update_task(&mut self, task_index: TaskIndex, new_state: TaskState) -> TaskState {
        use TaskState::*;
        let current_state = self
            .task_states
            .get(task_index)
            .expect("Invalid task index");
        let queue = &mut self.wait_queue;
        match (&current_state, &new_state) {
            (Wait { .. }, None)
            | (Wait { .. }, Running)
            | (WaitUntil { .. }, None)
            | (WaitUntil { .. }, Running)
            | (WaitUntil { .. }, Wait { .. })
            | (Wait { .. }, WaitUntil { .. })
            | (WaitUntil { .. }, WaitUntil { .. }) => {
                queue.retain(|x| task_index != *x);
            }
            _ => (),
        }
        match &new_state {
            WaitUntil {
                when: task_when, ..
            } => {
                let index =
                    queue.partition_point(|queue_index| match self.task_states[*queue_index] {
                        WaitUntil {
                            when: queued_when, ..
                        } => queued_when >= *task_when,
                        Wait { .. } => true,
                        _ => panic!("Non waiting tasks in wait queue"),
                    });
                queue.insert(index, task_index);
            }
            Wait { .. } => queue.insert(0, task_index),
            _ => {}
        };
        mem::replace(&mut self.task_states[task_index], new_state)
    }

    fn process_events(&mut self) {
        use TaskState::*;
        // Only process events if all tasks are waiting
        if self.active_task_count > self.wait_queue.len() {
            return;
        }
        if let Some((dest, msg)) = self.message_queue.pop() {
            match dest {
                SimulatorMessageDest::Task(task_id) => {
                    let task_index = usize::try_from(task_id.get()).unwrap() - 1;

                    let old_state = self.update_task(task_index, Running);
                    match old_state {
                        WaitUntil { trig, .. } | Wait { trig, .. } => {
                            let _ = trig.send(SimulatorEvent::Message(msg.clone()));
                            self.update_task(task_index, Running);
                            return;
                        }
                        _ => panic!("Sending to non waiting task"),
                    }
                }
                SimulatorMessageDest::Exclude(task_id) => {
                    let exclude_index = usize::try_from(task_id.get()).unwrap() - 1;
                    // Remove here and put it back later
                    self.wait_queue.swap_remove(exclude_index);
                    for task_index in 0..self.task_states.len() {
                        let old_state = self.update_task(task_index, Running);
                        match old_state {
                            WaitUntil { trig, .. } | Wait { trig, .. } => {
                                let _ = trig.send(SimulatorEvent::Message(msg.clone()));
                            }
                            Running => panic!("Trying to send message to running task"),
                            None => {}
                        }
                    }
                    self.wait_queue.push(exclude_index);
                    return;
                }
                SimulatorMessageDest::All => {
                    for task_index in 0..self.task_states.len() {
                        let old_state = self.update_task(task_index, Running);
                        match old_state {
                            WaitUntil { trig, .. } | Wait { trig, .. } => {
                                let _ = trig.send(SimulatorEvent::Message(msg.clone()));
                            }
                            Running => panic!("Trying to send message to running task"),
                            None => {}
                        }
                    }
                    return;
                }
            }
        }

        while let Some(state_index) = self.wait_queue.last().cloned() {
            let task_state = self
                .task_states
                .get(state_index)
                .expect("Invalid index in queue");
            match task_state {
                WaitUntil { when, .. } => {
                    if *when <= self.current_time {
                        self.wait_queue.pop().unwrap();
                        let WaitUntil { trig, .. } = self.update_task(state_index, Running) else {
                            panic!("Not WaitUntil");
                        };
                        let _ = trig.send(SimulatorEvent::Timeout);
                    } else {
                        if self.active_task_count > self.wait_queue.len() {
                            break;
                        }
                        self.current_time = *when;
                    }
                }
                Wait { .. } => break,
                _ => panic!("Non waiting tasks in wait queue"),
            }
        }
    }
}

impl SimulatorTask for SimulatorTaskImpl {
    fn current_time(&self) -> Instant {
        self.data.data.read().unwrap().current_time
    }

    /// All tasks must eventually wait, otherwise time may not
    /// progress
    fn wait_until(&self, when: Instant) -> Pin<Box<dyn Future<Output = SimulatorEvent> + Send>> {
        let (tx, rx) = oneshot::channel();
        let data = &mut self.data.data.write().unwrap();
        data.update_task(
            self.task_index,
            TaskState::WaitUntil {
                when: when,
                trig: tx,
            },
        );
        data.process_events();
        Box::pin(rx.map(|r| match r {
            Ok(ev) => ev,
            Err(_) => SimulatorEvent::Timeout,
        }))
    }

    fn wait(&self) -> Pin<Box<dyn Future<Output = SimulatorEvent> + Send>> {
        let (tx, rx) = oneshot::channel();
        let mut data = self.data.data.write().unwrap();
        data.update_task(self.task_index, TaskState::Wait { trig: tx });
        data.process_events();
        Box::pin(rx.map(|r| match r {
            Ok(ev) => ev,
            Err(_) => SimulatorEvent::Timeout,
        }))
    }

    fn send_msg(&self, dest: SimulatorMessageDest, msg: Arc<dyn Any + Send + Sync>) {
        let data = &mut self.data.data.write().unwrap();
        data.message_queue.push((dest, msg));
        data.process_events();
    }

    fn real_time(&self) -> bool {
        false
    }
    fn task_id(&self) -> SimulatorTaskId {
        SimulatorTaskId::new(u32::try_from(self.task_index + 1).unwrap()).expect("Invalid task id")
    }
}

impl Drop for SimulatorTaskImpl {
    fn drop(&mut self) {
        let data = &mut self.data.data.write().unwrap();
        data.active_task_count -= 1;
        data.update_task(self.task_index, TaskState::None);
        data.process_events();
    }
}

pub struct SimulatorTaskImpl {
    data: Arc<SchedulerData>,
    task_index: TaskIndex,
}

pub struct SimulatorSchedulerImpl {
    data: Arc<SchedulerData>,
}

impl SimulatorSchedulerImpl {
    pub fn new() -> SimulatorSchedulerImpl {
        let data = SchedulerDataMut {
            active_task_count: 0,
            current_time: Instant::now(),
            wait_queue: Vec::new(),
            message_queue: Vec::new(),
            task_states: Vec::new(),
        };
        SimulatorSchedulerImpl {
            data: Arc::new(SchedulerData {
                data: RwLock::new(data),
            }),
        }
    }
}

impl SimulatorScheduler for SimulatorSchedulerImpl {
    fn new_task(&mut self) -> Box<dyn SimulatorTask + Send + Sync> {
        let data = &mut self.data.data.write().unwrap();
        data.active_task_count += 1;
        data.task_states.push(TaskState::Running);
        Box::new(SimulatorTaskImpl {
            task_index: data.task_states.len() - 1,
            data: self.data.clone(),
        })
    }
}

#[cfg(test)]
mod test {
    use super::SimulatorMessageDest;
    use super::SimulatorScheduler;
    use super::SimulatorSchedulerImpl;
    use super::{SimulatorEvent, SimulatorTask, SimulatorTaskId};
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;
    use std::any::Any;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::time::Duration;
    use std::time::Instant;

    async fn wait_task(mut task: Box<dyn SimulatorTask + Send + Sync>, delay: Duration) {
        let start_time = task.current_time();
        println!("Task waiting");
        task.wait_until(start_time + delay).await;
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
            .await;
        assert_eq!(task1.current_time(), start_time + Duration::from_secs(1));
        let task2 = sched.new_task();
        let w2 = tokio::spawn(wait_task(task2, Duration::from_secs(3)));
        task1
            .wait_until(task1.current_time() + Duration::from_secs(1))
            .await;
        assert_eq!(task1.current_time(), start_time + Duration::from_secs(2));

        let when = task1.current_time() + Duration::from_secs(4);
        drop(task1.wait_until(when));

        println!("Task1 waiting");
        task1.wait_until(when).await;
        assert_eq!(task1.current_time(), start_time + Duration::from_secs(6));
        println!("Task1 done");
        let _ = w2.await;
    }

    fn assert_message<T>(ev: &SimulatorEvent, v: &T)
    where
        T: Any + PartialEq + std::fmt::Debug,
    {
        match ev {
            SimulatorEvent::Message(msg) => {
                assert_eq!(msg.downcast_ref::<T>().expect("Wrong type for message"), v)
            }
            _ => panic!("Not a message"),
        }
    }

    async fn event_task(mut task: Box<dyn SimulatorTask + Send + Sync>) {
        let start_time = task.current_time();
        assert_message(&task.wait().await, &3);
        println!("Task 2: Message received");
        assert_eq!(task.current_time(), start_time + Duration::from_secs(1));
        task.wait_until(start_time + Duration::from_secs(4)).await;
        assert_eq!(task.current_time(), start_time + Duration::from_secs(4));
        task.send_msg(SimulatorMessageDest::All, Arc::new(7));
        println!("Task 2: Message sent");
        assert_message(&task.wait().await, &7);

        println!("Task done");
    }

    #[tokio::test]
    async fn event_test() {
        let mut sched = SimulatorSchedulerImpl::new();
        let mut task1 = sched.new_task();
        let task2 = sched.new_task();
        let task2_id = task2.task_id();
        let w2 = tokio::spawn(event_task(task2));
        let start_time = task1.current_time();
        task1.wait_until(start_time + Duration::from_secs(1)).await;
        task1.send_msg(SimulatorMessageDest::Task(task2_id), Arc::new(3));
        assert!(matches!(
            task1.wait_until(start_time + Duration::from_secs(2)).await,
            SimulatorEvent::Timeout
        ));
        assert_eq!(task1.current_time(), start_time + Duration::from_secs(2));
        assert_message(
            &task1.wait_until(start_time + Duration::from_secs(5)).await,
            &7,
        );
        println!("Task 1: Message received");
        let _ = w2.await;
        assert_eq!(task1.current_time(), start_time + Duration::from_secs(4));
    }

    const TASK_COUNT: usize = 10;

    async fn random_task(
        mut task: Box<dyn SimulatorTask + Send + Sync>,
        dest: SimulatorMessageDest,
    ) {
        let mut rng = SmallRng::seed_from_u64(u64::from(task.task_id().get()));
        loop {
            let end_time = task.current_time() + Duration::from_millis(rng.random_range(300..7000));
            let ev = task.wait_until(end_time).await;
            match ev {
                SimulatorEvent::Shutdown => break,
                SimulatorEvent::Timeout => {
                    assert_eq!(task.current_time(), end_time);
                }
                SimulatorEvent::Message(msg) => {
                    let ts = msg.downcast_ref::<Instant>().unwrap();
                    assert_eq!(*ts, task.current_time());
                    continue;
                }
            }
            task.send_msg(dest.clone(), Arc::new(task.current_time()));
        }
    }
    #[tokio::test]
    async fn random_test() {
        let mut rng = rand::rng();
        let mut sched = SimulatorSchedulerImpl::new();
        for _ in 0..TASK_COUNT {
            let task = sched.new_task();
            tokio::spawn(random_task(
                task,
                SimulatorMessageDest::Task(
                    SimulatorTaskId::new(rng.random_range(1u32..=TASK_COUNT as u32)).unwrap(),
                ),
            ));
        }
        let mut task = sched.new_task();
        let end_time = task.current_time() + Duration::from_secs(60);
        task.wait_until(end_time).await;
        assert_eq!(task.current_time(), end_time);
        task.send_msg(SimulatorMessageDest::All, Arc::new(true));
        while !matches!(task.wait().await, SimulatorEvent::Message(_)) {}
    }
}
