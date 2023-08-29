use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::thread;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::task::JoinHandle;

use crate::model::{SyncTask, AsyncTask, Operation};

struct AsyncScheduler {
    tasks: HashMap<usize, JoinHandle<()>>,
}

impl AsyncScheduler {
    fn listen(&mut self, receiver: Receiver<AsyncTask>) {
        println!("AsyncScheduler initialized.");

        let num_threads = 1;
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .worker_threads(num_threads)
            .thread_name("scheduler-runtime")
            .build()
            .unwrap();

        runtime.block_on(async {
            loop {
                match receiver.recv() {
                    Ok(async_task) => self.handle(async_task),
                    Err(e) => eprintln!("{}", e),
                }
            }
        });
    }

    fn handle(&mut self, async_task: AsyncTask) {
        match async_task.op {
            Operation::Create => {
                let future = tokio::spawn(async_task.func);
                self.tasks.insert(async_task.id, future);
            }
            Operation::Update => {
                let task = &self.tasks[&async_task.id];
                task.abort_handle().abort();
                self.tasks.remove(&async_task.id);
                println!("Stopped {}", async_task.id);

                let future = tokio::spawn(async_task.func);
                self.tasks.insert(async_task.id, future);
            }
            Operation::Delete => {
                let task = &self.tasks[&async_task.id];
                task.abort_handle().abort();
                self.tasks.remove(&async_task.id);
                println!("Stopped {}", async_task.id);
            }
        }
    }

    fn new() -> Self {
        AsyncScheduler {
            tasks: HashMap::new(),
        }
    }
}

struct TaskRunner {
    id: usize,
    frequency: Duration,
    thread_handle: Option<thread::JoinHandle<()>>,
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
        let frequency = self.frequency.clone();
        let runner_data = self.runner_data.clone();
        let builder = thread::Builder::new().name("task".to_string());

        let handle = builder.spawn(move || {
            loop {
                {
                    if runner_data.lock().unwrap().stopping {
                        break;
                    }
                }

                func();
                thread::sleep(frequency);
            }
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
    fn listen(&mut self, receiver: Receiver<SyncTask>) {
        println!("ThreadScheduler initialized.");

        loop {
            match receiver.recv() {
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

    fn handle(&mut self, task: SyncTask) {
        match task.op {
            Operation::Create => {
                let mut runner = TaskRunner::new(task.id, task.frequency);
                runner.start(task.func);
                self.tasks.lock().unwrap().push(runner);
            }
            Operation::Update => {
                if let Some(idx) = self.find_index(task.id) {
                    let mut runners = self.tasks.lock().unwrap();
                    runners[idx].stop();
                    runners.remove(idx);
                }

                let mut runner = TaskRunner::new(task.id, task.frequency);
                runner.start(task.func);
                self.tasks.lock().unwrap().push(runner);
            }
            Operation::Delete => {
                if let Some(idx) = self.find_index(task.id) {
                    let mut runners = self.tasks.lock().unwrap();
                    runners[idx].stop();
                    runners.remove(idx);
                }
            }
        }
    }

    fn new() -> Self {
        ThreadScheduler {
            tasks: Arc::new(Mutex::new(Vec::<TaskRunner>::new())),
        }
    }
}

pub fn init_async(receiver: Receiver<AsyncTask>) {
    let mut scheduler = AsyncScheduler::new();
    let builder = thread::Builder::new().name("scheduler".to_string());
    builder
        .spawn(move || scheduler.listen(receiver))
        .expect("Failed to spawn scheduler thread.");
}

pub fn init_sync(receiver: Receiver<SyncTask>) {
    let mut scheduler = ThreadScheduler::new();
    let builder = thread::Builder::new().name("scheduler".to_string());
    builder
        .spawn(move || scheduler.listen(receiver))
        .expect("Failed to spawn scheduler thread.");
}
