use std::future::Future;
use std::pin::Pin;

#[derive(Clone)]
pub enum Operation {
    Create,
    Update,
    Delete,
}

pub struct AsyncTask {
    pub id: usize,
    pub frequency: u64,
    pub func: Pin<Box<dyn Future<Output = ()> + Send + Sync>>,
    pub op: Operation,
}

impl AsyncTask {
    pub fn new<F>(id: usize, frequency: u64, func: F) -> Self
    where
        F: Future<Output = ()> + Send + Sync + 'static,
    {
        Self {
            id,
            frequency,
            func: Box::pin(func),
            op: Operation::Create,
        }
    }

    pub fn update<F>(id: usize, frequency: u64, func: F) -> Self
    where
        F: Future<Output = ()> + Send + Sync + 'static,
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
            frequency: 0,
            func: Box::pin(async {}),
            op: Operation::Delete,
        }
    }
}
