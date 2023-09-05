use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

#[derive(Clone)]
pub enum Operation {
    Create,
    Update,
    Delete,
}

pub struct AsyncTask {
    pub id: usize,
    pub func: Pin<Box<dyn Future<Output = ()> + Send>>,
    pub op: Operation,
}

impl AsyncTask {
    pub fn new<F>(id: usize, func: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Self {
            id,
            func: Box::pin(func),
            op: Operation::Create,
        }
    }

    pub fn update<F>(id: usize, func: F) -> Self
    where
        F: Future<Output = ()> + Send + 'static,
    {
        Self {
            id,
            func: Box::pin(func),
            op: Operation::Update,
        }
    }

    pub fn stop(id: usize) -> Self {
        Self {
            id,
            func: Box::pin(async {}),
            op: Operation::Delete,
        }
    }
}

pub struct SyncTask {
    pub id: usize,
    pub frequency: Duration,
    pub func: Pin<Box<dyn Fn() + Send + Sync>>,
    pub op: Operation,
}

impl SyncTask {
    pub fn new<F>(id: usize, frequency: Duration, func: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            id,
            frequency,
            func: Box::pin(func),
            op: Operation::Create,
        }
    }

    pub fn update<F>(id: usize, frequency: Duration, func: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            id,
            frequency,
            func: Box::pin(func),
            op: Operation::Update,
        }
    }

    pub fn stop(id: usize) -> Self {
        Self {
            id,
            frequency: Duration::from_millis(0),
            func: Box::pin(|| {}),
            op: Operation::Delete,
        }
    }
}
