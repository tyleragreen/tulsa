use reqwest::header::{HeaderMap, HeaderName};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
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

/// This represents a [`Feed`] but without an ID, which are used in POST bodies.
#[derive(Clone, Deserialize, Serialize)]
pub struct CreateFeed {
    pub name: String,
    pub url: String,
    pub frequency: u64,
    pub headers: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct Status {
    status: String,
}

impl Status {
    pub fn new(status: &str) -> Self {
        Self {
            status: status.to_string(),
        }
    }
}
