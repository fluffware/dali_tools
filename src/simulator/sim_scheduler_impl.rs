use super::sim_scheduler::{
    SimulatorEvent, SimulatorMessageDest, SimulatorScheduler, SimulatorTask, SimulatorTaskId,
};
use futures::FutureExt;
use std::any::Any;
use std::future;
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
    task_states: Vec<TaskState>,
    message_queue: Vec<(SimulatorMessageDest, Arc<dyn Any + Send + Sync>)>,
}

pub struct SchedulerData {
    data: RwLock<SchedulerDataMut>,
}

impl SchedulerDataMut {
    fn process_events(&mut self) {
        use TaskState::*;
        // Only process events if all tasks are waiting
        let mut earliest: Option<(Instant, usize)> = Option::None;
        for (task_index, task_state) in self.task_states.iter().enumerate() {
            match task_state {
                TaskState::Running => {
                    return; // All tasks must be waiting
                }
                WaitUntil { when, .. } => {
                    if let Some((earliest_when, _)) = earliest
                        && &earliest_when <= when
                    {
                    } else {
                        earliest = Some((*when, task_index));
                    }
                }
                _ => {}
            }
        }
        if let Some((dest, msg)) = self.message_queue.pop() {
            match dest {
                SimulatorMessageDest::Task(task_id) => {
                    let task_index = usize::try_from(task_id.get()).unwrap() - 1;
                    let task_state = &mut self.task_states[task_index];
                    let old_state = mem::replace(task_state, TaskState::Running);
                    match old_state {
                        WaitUntil { trig, .. } | Wait { trig, .. } => {
                            let _ = trig.send(SimulatorEvent::Message(msg.clone()));
                            return;
                        }
                        None => {
                            *task_state = TaskState::None;
                        }
                        _ => panic!("Sending to non waiting task"),
                    }
                }
                SimulatorMessageDest::Exclude(task_id) => {
                    let exclude_index = usize::try_from(task_id.get()).unwrap() - 1;
                    // Remove here and put it back later
                    for (task_index, task_state) in &mut self.task_states.iter_mut().enumerate() {
                        if exclude_index != task_index {
                            let old_state = mem::replace(task_state, Running);
                            match old_state {
                                WaitUntil { trig, .. } | Wait { trig, .. } => {
                                    let _ = trig.send(SimulatorEvent::Message(msg.clone()));
                                }
                                Running => panic!("Trying to send message to running task"),
                                None => {
                                    *task_state = TaskState::None;
                                }
                            }
                        }
                    }
                    return;
                }
                SimulatorMessageDest::All => {
                    for task_state in &mut self.task_states {
                        let old_state = mem::replace(task_state, Running);
                        match old_state {
                            WaitUntil { trig, .. } | Wait { trig, .. } => {
                                let _ = trig.send(SimulatorEvent::Message(msg.clone()));
                            }
                            Running => panic!("Trying to send message to running task"),
                            None => {
                                *task_state = TaskState::None;
                            }
                        }
                    }
                    return;
                }
            }
        }
        while let Some((earliest_when, earliest_index)) = earliest {
            let task_state = self
                .task_states
                .get_mut(earliest_index)
                .expect("Invalid task index");
            match task_state {
                WaitUntil { when, .. } => {
                    if earliest_when <= self.current_time {
                        let WaitUntil { trig, .. } = mem::replace(task_state, Running) else {
                            panic!("Not WaitUntil");
                        };
                        let _ = trig.send(SimulatorEvent::Timeout);
                        return;
                    } else {
                        self.current_time = *when;
                    }
                }
                Wait { .. } => {}
                _ => panic!("Non waiting tasks"),
            }

            for (task_index, task_state) in self.task_states.iter().enumerate() {
                match task_state {
                    TaskState::Running => return, // All tasks must be waiting
                    WaitUntil { when, .. } => {
                        if let Some((earliest_when, _)) = earliest
                            && &earliest_when <= when
                        {
                        } else {
                            earliest = Some((*when, task_index));
                        }
                    }
                    _ => {}
                }
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
        let data = &mut self.data.data.write().unwrap();
        if let TaskState::None = data.task_states[self.task_index] {
            return Box::pin(future::ready(SimulatorEvent::Shutdown));
        }
        let (tx, rx) = oneshot::channel();

        data.task_states[self.task_index] = TaskState::WaitUntil {
            when: when,
            trig: tx,
        };
        data.process_events();
        Box::pin(rx.map(|r| match r {
            Ok(ev) => ev,
            Err(_) => SimulatorEvent::Timeout,
        }))
    }

    fn wait(&self) -> Pin<Box<dyn Future<Output = SimulatorEvent> + Send>> {
        let mut data = self.data.data.write().unwrap();
        if let TaskState::None = data.task_states[self.task_index] {
            return Box::pin(future::ready(SimulatorEvent::Shutdown));
        }
        let (tx, rx) = oneshot::channel();
        data.task_states[self.task_index] = TaskState::Wait { trig: tx };
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
    fn shutdown(&self) {
        let data = &mut self.data.data.write().unwrap();
        data.task_states[self.task_index] = TaskState::None;
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
        data.task_states[self.task_index] = TaskState::None;
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
            current_time: Instant::now(),
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

impl Drop for SimulatorSchedulerImpl {
    fn drop(&mut self) {
        let data = &mut self.data.data.write().unwrap();
        data.message_queue.clear();
        for task_state in &mut data.task_states {
            let old_state = mem::replace(task_state, TaskState::None);
            match old_state {
                TaskState::Wait { trig, .. } | TaskState::WaitUntil { trig, .. } => {
                    let _ = trig.send(SimulatorEvent::Shutdown);
                }
                _ => {}
            };
            *task_state = TaskState::None;
        }
    }
}
impl SimulatorScheduler for SimulatorSchedulerImpl {
    fn new_task(&mut self) -> Box<dyn SimulatorTask + Send + Sync> {
        let data = &mut self.data.data.write().unwrap();
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
    use rand::RngExt;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;
    use std::any::Any;
    use std::sync::Arc;
    use std::time::Duration;
    use std::time::Instant;
    use tokio::task::JoinSet;

    async fn wait_task(task: Box<dyn SimulatorTask + Send + Sync>, delay: Duration) {
        let start_time = task.current_time();
        println!("Task waiting");
        task.wait_until(start_time + delay).await;
        assert_eq!(task.current_time(), start_time + delay);
        println!("Task done");
    }

    #[tokio::test]
    async fn scheduler_test() {
        let mut sched = SimulatorSchedulerImpl::new();
        let task1 = sched.new_task();
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

    async fn event_task(task: Box<dyn SimulatorTask + Send + Sync>) {
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
        let task1 = sched.new_task();
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

    async fn random_task(task: Box<dyn SimulatorTask + Send + Sync>, dest: SimulatorMessageDest) {
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
                    let ts = msg.as_ref().downcast_ref::<Instant>().unwrap();
                    assert_eq!(*ts, task.current_time());
                    continue;
                }
            }
            task.send_msg(dest.clone(), Arc::new(task.current_time() as Instant));
        }
    }
    #[tokio::test]
    async fn random_test() {
        let mut rng = rand::rng();
        let mut sched = SimulatorSchedulerImpl::new();
        let mut threads = JoinSet::new();
        for _ in 0..TASK_COUNT {
            let task = sched.new_task();
            threads.spawn(random_task(
                task,
                SimulatorMessageDest::Task(
                    SimulatorTaskId::new(rng.random_range(1u32..=TASK_COUNT as u32)).unwrap(),
                ),
            ));
        }
        let task = sched.new_task();
        let end_time = task.current_time() + Duration::from_secs(60);
        task.wait_until(end_time).await;
        assert_eq!(task.current_time(), end_time);
        task.send_msg(SimulatorMessageDest::All, Arc::new(true));
        drop(sched);
        while let Some(_res) = threads.join_next().await {}
    }
}
