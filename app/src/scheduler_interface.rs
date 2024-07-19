use std::{
    marker::PhantomData,
    sync::{
        mpsc::{self, SendError, Sender},
        Arc,
    },
    time::Duration,
};
use tulsa::{AsyncTask, Scheduler, SyncTask, Task};

use crate::{
    fetcher::{fetch_sync, recurring_fetch},
    models::Feed,
};

/// Used to indicate an action by a `ToScheduler` was unsuccessful.
pub struct AppSendError;

pub fn build() -> Arc<impl ToScheduler + Send + Sync + 'static> {
    #[cfg(feature = "async_mode")]
    {
        let (sender, receiver) = mpsc::channel();
        Scheduler::<AsyncTask>::new(receiver).run();
        Arc::new(SchedulerInterface::new(Arc::new(sender)))
    }

    #[cfg(not(feature = "async_mode"))]
    {
        let (sender, receiver) = mpsc::channel();
        Scheduler::<SyncTask>::new(receiver).run();
        Arc::new(SchedulerInterface::new(Arc::new(sender)))
    }
}

/// An interface to send a `Task`. This allows clients to mock a `Sender` for unit tests.
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
pub trait ToScheduler {
    fn create(&self, feed: Feed) -> Result<(), AppSendError>;
    fn update(&self, feed: Feed) -> Result<(), AppSendError>;
    fn delete(&self, feed: Feed) -> Result<(), AppSendError>;
}

pub struct SchedulerInterface<R, T>
where
    R: TaskSend<T> + Send + 'static,
    T: Task,
{
    sender: Arc<R>,
    _marker: PhantomData<T>,
}

impl<R, T> SchedulerInterface<R, T>
where
    R: TaskSend<T> + Send + 'static,
    T: Task,
{
    pub fn new(sender: Arc<R>) -> Self {
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
    fn create(&self, feed: Feed) -> Result<(), AppSendError> {
        let action = SyncTask::new(feed.id, Duration::from_secs(feed.frequency), move || {
            fetch_sync(&feed);
        });
        self.sender.send(action).map_err(|_| AppSendError)
    }

    fn update(&self, feed: Feed) -> Result<(), AppSendError> {
        let action = SyncTask::update(feed.id, Duration::from_secs(feed.frequency), move || {
            fetch_sync(&feed);
        });
        self.sender.send(action).map_err(|_| AppSendError)
    }

    fn delete(&self, feed: Feed) -> Result<(), AppSendError> {
        let action = SyncTask::stop(feed.id);
        self.sender.send(action).map_err(|_| AppSendError)
    }
}

impl<R> ToScheduler for SchedulerInterface<R, AsyncTask>
where
    R: TaskSend<AsyncTask> + Send + 'static,
{
    fn create(&self, feed: Feed) -> Result<(), AppSendError> {
        let action = AsyncTask::new(feed.id, recurring_fetch(feed));
        self.sender.send(action).map_err(|_| AppSendError)
    }

    fn update(&self, feed: Feed) -> Result<(), AppSendError> {
        let action = AsyncTask::update(feed.id, recurring_fetch(feed));
        self.sender.send(action).map_err(|_| AppSendError)
    }

    fn delete(&self, feed: Feed) -> Result<(), AppSendError> {
        let action = AsyncTask::stop(feed.id);
        self.sender.send(action).map_err(|_| AppSendError)
    }
}
