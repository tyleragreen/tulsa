use std::sync::mpsc::{self, SendError, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tulsa::model::{AsyncTask, SyncTask};
use tulsa::scheduler;

use crate::fetcher::{fetch_sync, recurring_fetch, Feed};

pub enum Mode {
    Sync,
    Async,
}

pub fn build(mode: Mode) -> Arc<dyn SchedulerInterface + Send + Sync + 'static> {
    match mode {
        Mode::Async => {
            let (sender, receiver) = mpsc::channel();
            scheduler::init_async(receiver);
            Arc::new(Scheduler::new(Arc::new(Mutex::new(sender))))
        }
        Mode::Sync => {
            let (sender, receiver) = mpsc::channel();
            scheduler::init_sync(receiver);
            Arc::new(Scheduler::new(Arc::new(Mutex::new(sender))))
        }
    }
}

pub trait TaskSender<T> {
    fn send(&self, task: T) -> Result<(), SendError<T>>;
}

/// The [Feed] will be sent to another thread, so we require ownership.
pub trait SchedulerInterface {
    fn create(&self, feed: Feed);
    fn update(&self, feed: Feed);
    fn delete(&self, feed: Feed);
}

pub struct Scheduler<T> {
    sender: Arc<Mutex<dyn TaskSender<T> + Send + 'static>>,
}

impl<T> TaskSender<T> for Sender<T> {
    fn send(&self, task: T) -> Result<(), SendError<T>> {
        self.send(task)
    }
}

impl<T> Scheduler<T> {
    pub fn new(sender: Arc<Mutex<dyn TaskSender<T> + Send + 'static>>) -> Self {
        Self { sender }
    }
}

impl SchedulerInterface for Scheduler<SyncTask> {
    fn create(&self, feed: Feed) {
        let action = SyncTask::new(feed.id, Duration::from_secs(feed.frequency), move || {
            fetch_sync(&feed);
        });
        let result = self.sender.lock().unwrap().send(action);

        if let Err(e) = result {
            println!("{}", e);
        }
    }

    fn update(&self, feed: Feed) {
        let action = SyncTask::update(feed.id, Duration::from_secs(feed.frequency), move || {
            fetch_sync(&feed);
        });
        let result = self.sender.lock().unwrap().send(action);

        if let Err(e) = result {
            println!("{}", e);
        }
    }

    fn delete(&self, feed: Feed) {
        let action = SyncTask::stop(feed.id);
        let result = self.sender.lock().unwrap().send(action);

        if let Err(e) = result {
            println!("{}", e);
        }
    }
}

impl SchedulerInterface for Scheduler<AsyncTask> {
    fn create(&self, feed: Feed) {
        let action = AsyncTask::new(feed.id, recurring_fetch(feed));
        let result = self.sender.lock().unwrap().send(action);

        if let Err(e) = result {
            println!("{}", e);
        }
    }

    fn update(&self, feed: Feed) {
        let action = AsyncTask::update(feed.id, recurring_fetch(feed));
        let result = self.sender.lock().unwrap().send(action);

        if let Err(e) = result {
            println!("{}", e);
        }
    }

    fn delete(&self, feed: Feed) {
        let action = AsyncTask::stop(feed.id);
        let result = self.sender.lock().unwrap().send(action);

        if let Err(e) = result {
            println!("{}", e);
        }
    }
}
