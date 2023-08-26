use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::thread;
use std::pin::Pin;
use std::future::Future;
use tokio::runtime::Builder;
use tokio::task::JoinHandle;

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

struct ThreadScheduler {
    tasks: HashMap<usize, thread::JoinHandle<Pin<Box<dyn Future<Output = ()> + Send>>>>,
}

impl ThreadScheduler {
    fn listen(&mut self, receiver: Receiver<AsyncTask>) {
        println!("ThreadScheduler initialized.");

        loop {
            let async_task = receiver.recv().unwrap();
            self.handle(async_task);
        }
    }

    fn handle(&mut self, async_task: AsyncTask) {
        match async_task.op {
            Operation::Create => {
                let future = thread::spawn(move || {
                    loop {
                        async_task.func;
                        thread::sleep(std::time::Duration::from_secs(2));
                    }
                });
                self.tasks.insert(async_task.id, future);
            }
            Operation::Update => {
                let task = &self.tasks[&async_task.id];
                //task.abort_handle().abort();
                self.tasks.remove(&async_task.id);
                println!("Stopped {}", async_task.id);

                let future = thread::spawn(move || async_task.func);
                self.tasks.insert(async_task.id, future);
            }
            Operation::Delete => {
                //let task = &self.tasks[&async_task.id];
                //task.abort_handle().abort();
                self.tasks.remove(&async_task.id);
                println!("Stopped {}", async_task.id);
            }
        }
    }

    fn new() -> Self {
        ThreadScheduler {
            tasks: HashMap::new(),
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
