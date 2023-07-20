use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Client;
use std::sync::mpsc::Receiver;
use std::thread;
use tokio::runtime::Runtime;
use tokio::time::{Duration, Interval};

use crate::feed::Feed;

async fn fetch(feed: Feed) {
    println!("Fetching {}", feed.name);

    let client = Client::new();

    let mut headers = HeaderMap::new();
    headers.insert("x-api-key", HeaderValue::from_static(""));
    let response = client.get(feed.url).headers(headers).send().await;

    match response {
        Ok(r) => {
            let text = r.text().await.unwrap();
            println!("OK: {}", text)
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

async fn recurring_task(feed: Feed) {
    let interval_duration = Duration::from_secs(feed.frequency);
    let mut interval: Interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;
        tokio::spawn(fetch(feed.clone()));
    }
}

fn scheduler(receiver: Receiver<Feed>) {
    println!("Scheduler initialized.");

    let runtime = Runtime::new().unwrap();
    runtime.block_on(async {
        for item in receiver {
            let _future = tokio::spawn(recurring_task(item.clone()));
        }
    });
}

pub fn init(receiver: Receiver<Feed>) {
    thread::spawn(move || scheduler(receiver));
}
