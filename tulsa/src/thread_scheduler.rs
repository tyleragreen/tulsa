use std::{
    pin::Pin,
    sync::{mpsc::Receiver, Arc, Mutex},
    thread::{sleep, Builder as ThreadBuilder, JoinHandle as ThreadJoinHandle},
    time::Duration,
};

use crate::model::{Operation, SyncTask};

struct TaskRunner {
    id: usize,
    frequency: Duration,
    thread_handle: Option<ThreadJoinHandle<()>>,
    runner_data: Arc<Mutex<RunnerData>>,
}

struct RunnerData {
    stopping: bool,
}

impl TaskRunner {
    fn new(id: usize, frequency: Duration) -> Self {
        let thread_handle = None;
        let runner_data = Arc::new(Mutex::new(RunnerData { stopping: false }));
        Self {
            id,
            frequency,
            thread_handle,
            runner_data,
        }
    }

    fn start(&mut self, func: Pin<Box<dyn Fn() + Send + Sync + 'static>>) {
        println!("Starting {}", self.id);
        let frequency = self.frequency;
        let runner_data = self.runner_data.clone();
        let builder = ThreadBuilder::new().name("task".to_string());

        let handle = builder.spawn(move || loop {
            {
                if runner_data.lock().unwrap().stopping {
                    break;
                }
            }

            func();
            sleep(frequency);
        });

        self.thread_handle = Some(handle.unwrap());
    }

    fn stop(&mut self) {
        println!("Stopping {}", self.id);
        // Use a block so that the lock is released.
        {
            self.runner_data.lock().unwrap().stopping = true;
        }

        if let Some(handle) = self.thread_handle.take() {
            match handle.join() {
                Ok(_) => println!("Stopped {}", self.id),
                Err(e) => panic!("{:?}", e),
            }
        }
    }
}

pub(crate) struct ThreadScheduler {
    tasks: Arc<Mutex<Vec<TaskRunner>>>,
}

impl ThreadScheduler {
    pub(crate) fn new() -> Self {
        ThreadScheduler {
            tasks: Arc::new(Mutex::new(Vec::<TaskRunner>::new())),
        }
    }

    pub(crate) fn listen(&mut self, receiver: Arc<Mutex<Receiver<SyncTask>>>) {
        println!("ThreadScheduler initialized.");

        let r = receiver.clone();
        loop {
            match r.lock().unwrap().recv() {
                Ok(task) => self.handle(task),
                Err(e) => eprintln!("{}", e),
            }
        }
    }

    fn find_index(&mut self, id: usize) -> Option<usize> {
        self.tasks
            .lock()
            .unwrap()
            .iter()
            .position(|runner| runner.id == id)
    }

    fn start(&mut self, task: SyncTask) {
        let mut runner = TaskRunner::new(task.id, task.frequency);
        runner.start(task.func);
        self.tasks.lock().unwrap().push(runner);
    }

    fn stop(&mut self, task_id: usize) {
        if let Some(idx) = self.find_index(task_id) {
            let mut runners = self.tasks.lock().unwrap();
            runners[idx].stop();
            runners.remove(idx);
        }
    }

    fn handle(&mut self, task: SyncTask) {
        match task.op {
            Operation::Create => self.start(task),
            Operation::Delete => self.stop(task.id),
            Operation::Update => {
                self.stop(task.id);
                self.start(task);
            }
        }
    }
}
