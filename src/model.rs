use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use reqwest::header::{HeaderMap, HeaderName};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Feed {
    pub id: usize,
    pub name: String,
    pub url: String,
    pub frequency: u64,
    pub headers: HashMap<String, String>,
}

impl Feed {
    pub fn to_header_map(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (key, value) in self.headers.iter() {
            let new_key: HeaderName = key.parse().unwrap();
            headers.insert(new_key, value.parse().unwrap());
        }
        headers
    }
}

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
