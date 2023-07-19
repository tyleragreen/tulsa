use std::thread;
use tokio::runtime::Runtime;
use tokio::time::{Duration, Interval};

use crate::feed::Feed;
use crate::QUEUE;

async fn process() {
    println!("Hello, World!");
}

async fn recurring_task(feed: Feed) {
    let interval_duration = Duration::from_secs(feed.frequency);
    let mut interval: Interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;
        tokio::spawn(process());
    }
}

fn scheduler() {
    println!("Scheduler initialized.");

    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        loop {
            let item = QUEUE.lock().unwrap().pop_back();

            match item {
                Some(i) => {
                    let _future = tokio::spawn(recurring_task(i.clone()));
                }
                None => {
                    continue;
                }
            }
        }
    });
}

pub fn init() {
    thread::spawn(scheduler);
}
