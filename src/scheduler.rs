use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::thread;
use tokio::runtime::Builder;
use tokio::task::JoinHandle;

use crate::model::{AsyncTask, Operation};

struct Scheduler {
    tasks: HashMap<usize, JoinHandle<()>>,
}

impl Scheduler {
    fn listen(&mut self, receiver: Receiver<AsyncTask>) {
        println!("Scheduler initialized.");

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
        Scheduler {
            tasks: HashMap::new(),
        }
    }
}

pub fn init(receiver: Receiver<AsyncTask>) {
    let mut scheduler = Scheduler::new();
    let builder = thread::Builder::new().name("scheduler".to_string());
    builder
        .spawn(move || scheduler.listen(receiver))
        .expect("Failed to spawn scheduler thread.");
}
