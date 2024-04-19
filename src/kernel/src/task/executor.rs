use alloc::{
    boxed::Box,
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    task::Wake,
};
use core::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

pub enum AwaitType {
    AlwaysPoll,
    WakePoll,
}

use crossbeam_queue::SegQueue;

use crate::println;

use super::interrupt_wakeups::interrupt_wakeups;

struct Task {
    task: Pin<Box<dyn Future<Output = ()>>>,
    typ: AwaitType,
}

impl Task {
    fn new(task: Pin<Box<dyn Future<Output = ()>>>, typ: AwaitType) -> Self {
        Self { task, typ }
    }
}

type TaskId = usize;

pub struct Executor {
    task_queue: VecDeque<Task>,
    wake_queue: Arc<SegQueue<TaskId>>,
    pending_tasks: BTreeMap<TaskId, Task>,
}

impl Executor {
    pub fn new() -> Self {
        Executor {
            task_queue: VecDeque::new(),
            wake_queue: Arc::new(SegQueue::new()),
            pending_tasks: BTreeMap::new(),
        }
    }

    pub fn spawn(&mut self, task: impl Future<Output = ()> + 'static, typ: AwaitType) {
        self.task_queue.push_back(Task::new(Box::pin(task), typ));
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.run_ready_tasks();
            self.apply_interrupt_wakeups();
            self.wake_waiting_tasks();
            self.hlt_if_idle();
        }
    }

    fn run_ready_tasks(&mut self) {
        while let Some(mut task) = self.task_queue.pop_front() {
            let waker = self.create_waker(&task).into();
            let mut context = Context::from_waker(&waker);
            match task.task.as_mut().poll(&mut context) {
                Poll::Ready(()) => {}
                Poll::Pending => match task.typ {
                    AwaitType::AlwaysPoll => {
                        self.task_queue.push_back(task);
                        break;
                    }
                    AwaitType::WakePoll => {
                        let task_id = Self::task_id(&task);
                        if self.pending_tasks.insert(task_id, task).is_some() {
                            panic!("Task with same ID already in pending_tasks");
                        }
                    }
                },
            }
        }
    }

    fn apply_interrupt_wakeups(&mut self) {
        while let Ok(waker) = interrupt_wakeups().pop() {
            waker.wake();
        }
    }

    fn wake_waiting_tasks(&mut self) {
        while let Ok(task_id) = self.wake_queue.pop() {
            if let Some(task) = self.pending_tasks.remove(&task_id) {
                self.task_queue.push_back(task);
            } else {
                println!("WARNING: woken task not found in pending_tasks");
            }
        }
    }

    fn hlt_if_idle(&self) {
        if self.task_queue.is_empty() {
            x86_64::instructions::interrupts::disable();
            if interrupt_wakeups().is_empty() {
                x86_64::instructions::interrupts::enable_and_hlt();
            } else {
                x86_64::instructions::interrupts::enable();
            }
        }
    }

    fn task_id(task: &Task) -> TaskId {
        let future_ref: &dyn Future<Output = ()> = &*task.task;
        future_ref as *const _ as *const () as usize
    }

    fn create_waker(&self, task: &Task) -> Arc<Waker> {
        Arc::new(Waker {
            wake_queue: self.wake_queue.clone(),
            task_id: Self::task_id(task),
        })
    }
}

pub struct Waker {
    wake_queue: Arc<SegQueue<TaskId>>,
    task_id: TaskId,
}

impl Wake for Waker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_queue.push(self.task_id);
    }
}
