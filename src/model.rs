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
