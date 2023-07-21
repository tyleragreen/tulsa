use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::thread;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Interval};

use crate::model::{Action, ActionType, Feed};
use crate::fetcher::fetch;

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
    fn create(&mut self, action: Action) {
        let optional_feed = action.feed;

        if optional_feed.is_none() {
            return;
        }
        let feed = optional_feed.unwrap();
        let future = tokio::spawn(recurring_task(feed.clone()));
        self.tasks.insert(action.id, future);
    }

    fn delete(&mut self, action: Action) {
        let task = &self.tasks[&action.id];
        task.abort_handle().abort();
        self.tasks.remove(&action.id);
        println!("Stopped {}", action.id);
    }

    fn start(&mut self, receiver: Receiver<Action>) {
        println!("Scheduler initialized.");

        let runtime = Runtime::new().unwrap();
        runtime.block_on(async {
            for action in receiver {
                match action.action {
                    ActionType::Create => {
                        self.create(action);
                    }
                    ActionType::Update => {
                        self.delete(action.clone());
                        self.create(action.clone());
                    }
                    ActionType::Delete => {
                        self.delete(action);
                    }
                }
            }
        });
    }
}

pub fn init(receiver: Receiver<Action>) {
    let mut scheduler = Scheduler {
        tasks: HashMap::new(),
    };
    thread::spawn(move || scheduler.start(receiver));
}
