use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use tokio::runtime::Builder as TokioBuilder;
use tokio::task::JoinHandle as TaskJoinHandle;

use crate::model::{AsyncTask, Operation};

pub(crate) struct AsyncScheduler {
    tasks: HashMap<usize, TaskJoinHandle<()>>,
    num_runtime_threads: usize,
}

impl AsyncScheduler {
    pub(crate) fn new() -> Self {
        AsyncScheduler {
            tasks: HashMap::new(),
            num_runtime_threads: 1,
        }
    }

    pub(crate) fn listen(&mut self, receiver: Arc<Mutex<Receiver<AsyncTask>>>) {
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
