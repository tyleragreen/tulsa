use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::thread;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::future::Future;
use tokio::runtime::Builder;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Interval};

use crate::model::{AsyncTask, Operation};

struct AsyncScheduler {
    tasks: HashMap<usize, JoinHandle<()>>,
}

impl AsyncScheduler {
    fn listen(&mut self, receiver: Receiver<AsyncTask>) {
        println!("AsyncScheduler initialized.");

        let num_threads = 1;
        let runtime = Builder::new_multi_thread()
            .enable_time()
            .enable_io()
            .worker_threads(num_threads)
            .thread_name("scheduler-runtime")
            .build()
            .unwrap();

        runtime.block_on(async {
            loop {
                let async_task = receiver.recv().unwrap();
                self.handle(async_task);
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

struct AsyncTaskRunner {
    id: usize,
    frequency: u64,
    thread_handle: Option<thread::JoinHandle<()>>,
    runner_data: Arc<Mutex<RunnerData>>,
}

struct RunnerData {
    stopping: bool,
}

async fn wait(freq: u64) {
    let interval_duration = Duration::from_secs(freq);
    let mut interval: Interval = tokio::time::interval(interval_duration);

    interval.tick().await;
}

impl AsyncTaskRunner {
    fn new(id: usize, frequency: u64) -> Self {
        let thread_handle = None;
        let runner_data = Arc::new(Mutex::new(RunnerData { stopping: false }));
        Self {
            id,
            frequency,
            thread_handle,
            runner_data,
        }
    }

    fn start(&mut self, mut func: Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>) {
        println!("Starting {}", self.id);
        let freq = self.frequency.clone();
        let runner_data = self.runner_data.clone();
        let builder = thread::Builder::new().name("task".to_string());
        let handle = builder.spawn(move || {
            let local_runner_data = runner_data.clone();
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    loop {
                        {
                            let runner_data = local_runner_data.lock().unwrap();
                            if runner_data.stopping {
                                break;
                            }
                        }
                        wait(freq).await;
                        func.as_mut().await;
                    }
                });
        });

        self.thread_handle = Some(handle.unwrap());
    }

    fn stop(&mut self) {
        println!("Stopping {}", self.id);
        let mut runner_data = self.runner_data.lock().unwrap();
        runner_data.stopping = true;
        if let Some(handle) = self.thread_handle.take() {
            handle.join().unwrap();
        }
    }
}

struct ThreadScheduler {
    tasks: Arc<Mutex<Vec<AsyncTaskRunner>>>,
}

impl ThreadScheduler {
    fn listen(&mut self, receiver: Receiver<AsyncTask>) {
        println!("ThreadScheduler initialized.");

        loop {
            let task = receiver.recv().unwrap();
            self.handle(task);
        }
    }

    fn handle(&mut self, task: AsyncTask) {
        match task.op {
            Operation::Create => {
                let mut runner = AsyncTaskRunner::new(task.id, task.frequency);
                runner.start(task.func);

                self.tasks.lock().unwrap().push(runner);
            }
            Operation::Update => {
            }
            Operation::Delete => {
                let runner_idx = self.tasks
                    .lock()
                    .unwrap()
                    .iter()
                    .position(|runner| runner.id == task.id);

                if let Some(idx) = runner_idx {
                    let mut runners = self.tasks.lock().unwrap();
                    runners[idx].stop();
                    runners.remove(idx);
                }
            }
        }
    }

    fn new() -> Self {
        ThreadScheduler {
            tasks: Arc::new(Mutex::new(Vec::<AsyncTaskRunner>::new())),
        }
    }
}

pub fn init(receiver: Receiver<AsyncTask>) {
    let mut scheduler = ThreadScheduler::new();
    let builder = thread::Builder::new().name("scheduler".to_string());
    builder
        .spawn(move || scheduler.listen(receiver))
        .expect("Failed to spawn scheduler thread.");
}
