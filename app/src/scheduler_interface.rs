use std::{
    marker::PhantomData,
    sync::{
        mpsc::{self, SendError, Sender},
        Arc, Mutex,
    },
    time::Duration,
};
use tulsa::{AsyncTask, Scheduler, SyncTask, Task};

use crate::{
    fetcher::{fetch_sync, recurring_fetch},
    models::Feed,
};

pub fn build() -> Arc<impl ToScheduler + Send + 'static> {
    #[cfg(feature = "async_mode")]
    {
        let (sender, receiver) = mpsc::channel();
        Scheduler::<AsyncTask>::new(receiver).run();
        Arc::new(SchedulerInterface::new(Arc::new(Mutex::new(sender))))
    }

    #[cfg(not(feature = "async_mode"))]
    {
        let (sender, receiver) = mpsc::channel();
        Scheduler::<SyncTask>::new(receiver).run();
        Arc::new(SchedulerInterface::new(Arc::new(Mutex::new(sender))))
    }
}

/// An interface to send a `Task`.
pub trait TaskSend<T>
where
    T: Task,
{
    fn send(&self, task: T) -> Result<(), SendError<T>>;
}

impl<T> TaskSend<T> for Sender<T>
where
    T: Task,
{
    fn send(&self, task: T) -> Result<(), SendError<T>> {
        self.send(task)
    }
}

/// The [Feed] will be sent to another thread, so we require ownership.
pub trait ToScheduler: Clone {
    fn create(&self, feed: Feed);
    fn update(&self, feed: Feed);
    fn delete(&self, feed: Feed);
}

pub struct SchedulerInterface<R, T>
where
    R: TaskSend<T> + Send + 'static,
    T: Task,
{
    sender: Arc<Mutex<R>>,
    _marker: PhantomData<T>,
}

impl<R, T> Clone for SchedulerInterface<R, T>
where
    R: TaskSend<T> + Send + 'static,
    T: Task,
{
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            _marker: PhantomData,
        }
    }
}

impl<R, T> SchedulerInterface<R, T>
where
    R: TaskSend<T> + Send + 'static,
    T: Task,
{
    pub fn new(sender: Arc<Mutex<R>>) -> Self {
        Self {
            sender,
            _marker: PhantomData,
        }
    }
}

impl<R> ToScheduler for SchedulerInterface<R, SyncTask>
where
    R: TaskSend<SyncTask> + Send + 'static,
{
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

impl<R> ToScheduler for SchedulerInterface<R, AsyncTask>
where
    R: TaskSend<AsyncTask> + Send + 'static,
{
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
