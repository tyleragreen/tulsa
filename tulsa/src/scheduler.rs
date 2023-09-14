use std::collections::HashMap;
use std::pin::Pin;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use std::thread::{self, Builder as ThreadBuilder, JoinHandle as ThreadJoinHandle};
use std::time::Duration;
use tokio::runtime::Builder as TokioBuilder;
use tokio::task::JoinHandle as TaskJoinHandle;

use crate::model::{AsyncTask, Operation, SyncTask};

struct AsyncScheduler {
    tasks: HashMap<usize, TaskJoinHandle<()>>,
    num_runtime_threads: usize,
}

impl AsyncScheduler {
    fn new() -> Self {
        AsyncScheduler {
            tasks: HashMap::new(),
            num_runtime_threads: 1,
        }
    }

    fn listen(&mut self, receiver: Arc<Mutex<Receiver<AsyncTask>>>) {
        println!("AsyncScheduler initialized.");

        let runtime = TokioBuilder::new_multi_thread()
            .enable_all()
            .worker_threads(self.num_runtime_threads)
            .thread_name("scheduler-runtime")
            .build()
            .unwrap();

        let r = receiver.clone();

        runtime.block_on(async {
            loop {
                match r.lock().unwrap().recv() {
                    Ok(async_task) => self.handle(async_task),
                    Err(e) => eprintln!("{}", e),
                }
            }
        });
    }

    fn start(&mut self, task: AsyncTask) {
        let future = tokio::spawn(task.func);
        self.tasks.insert(task.id, future);
    }

    fn stop(&mut self, task_id: usize) {
        let task = &self.tasks[&task_id];
        task.abort_handle().abort();
        self.tasks.remove(&task_id);
        println!("Stopped {}", task_id);
    }

    fn handle(&mut self, task: AsyncTask) {
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
            thread::sleep(frequency);
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

struct ThreadScheduler {
    tasks: Arc<Mutex<Vec<TaskRunner>>>,
}

impl ThreadScheduler {
    fn new() -> Self {
        ThreadScheduler {
            tasks: Arc::new(Mutex::new(Vec::<TaskRunner>::new())),
        }
    }

    fn listen(&mut self, receiver: Arc<Mutex<Receiver<SyncTask>>>) {
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

pub struct Scheduler<T> {
    receiver: Arc<Mutex<Receiver<T>>>,
}

impl<T> Scheduler<T> {
    pub fn new(receiver: Receiver<T>) -> Self {
        let receiver = Arc::new(Mutex::new(receiver));
        Self { receiver }
    }
}

impl Scheduler<AsyncTask> {
    pub fn run(self) {
        ThreadBuilder::new()
            .name("scheduler".to_string())
            .spawn(|| AsyncScheduler::new().listen(self.receiver))
            .expect("Failed to spawn scheduler thread.");
    }
}

impl Scheduler<SyncTask> {
    pub fn run(self) {
        ThreadBuilder::new()
            .name("scheduler".to_string())
            .spawn(|| ThreadScheduler::new().listen(self.receiver))
            .expect("Failed to spawn scheduler thread.");
    }
}
