use std::collections::HashMap;
use std::sync::mpsc::Receiver;
use std::thread;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use tokio::time::{Duration, Interval};

use crate::feed::{Action, ActionType, Feed};
use crate::fetcher::fetch;

async fn recurring_task(feed: Feed) {
    let interval_duration = Duration::from_secs(feed.frequency);
    let mut interval: Interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;
        tokio::spawn(fetch(feed.clone()));
    }
}

struct Work {
    _feed: Feed,
    _work: JoinHandle<()>,
}

fn scheduler(receiver: Receiver<Action>) {
    println!("Scheduler initialized.");

    let mut feeds: Box<HashMap<u32, Feed>> = Box::default();
    let mut tasks: Box<HashMap<u32, Work>> = Box::default();

    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        for action in receiver {
            match action.action {
                ActionType::Create => {
                    let item = action.feed;

                    if item.is_none() {
                        continue;
                    }
                    let feed = item.unwrap();
                    feeds.insert(feed.id, feed.clone());
                    let future = tokio::spawn(recurring_task(feeds[&feed.id].clone()));
                    let w = Work {
                        _feed: feeds[&feed.id].clone(),
                        _work: future,
                    };
                    tasks.insert(feed.id, w);
                }
                ActionType::Update => {}
                ActionType::Delete => {}
            }
        }
    });
}

pub fn init(receiver: Receiver<Action>) {
    thread::spawn(move || scheduler(receiver));
}
