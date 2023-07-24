use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::thread;
use tokio::runtime::Builder;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Interval};

use crate::fetcher::fetch;
use crate::model::{Action, ActionType, Feed};

async fn recurring_task(feed: Feed) {
    let interval_duration = Duration::from_secs(feed.frequency);
    let mut interval: Interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;
        tokio::spawn(fetch(feed.clone()));
    }
}

struct Scheduler {
    tasks: HashMap<u32, JoinHandle<()>>,
}

impl Scheduler {
    fn create(&mut self, action: &Action) {
        if let Some(feed) = &action.feed {
            let future = tokio::spawn(recurring_task(feed.clone()));
            self.tasks.insert(action.id, future);
        }
    }

    fn delete(&mut self, action: &Action) {
        let task = &self.tasks[&action.id];
        task.abort_handle().abort();
        self.tasks.remove(&action.id);
        println!("Stopped {}", action.id);
    }

    fn start(&mut self, receiver: Receiver<Action>) {
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
                let action = receiver.recv().unwrap();
                match action.action {
                    ActionType::Create => {
                        self.create(&action);
                    }
                    ActionType::Update => {
                        self.delete(&action);
                        self.create(&action);
                    }
                    ActionType::Delete => {
                        self.delete(&action);
                    }
                }

            }
        });
    }

    fn new() -> Self {
        Scheduler {
            tasks: HashMap::new(),
        }
    }
}

pub fn init(receiver: Receiver<Action>) {
    let mut scheduler = Scheduler::new();
    let builder = thread::Builder::new().name("scheduler".to_string());
    let _ = builder.spawn(move || scheduler.start(receiver));
}
