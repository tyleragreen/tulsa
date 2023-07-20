use reqwest::header::{HeaderMap, HeaderName};
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Feed {
    pub id: u32,
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
pub enum ActionType {
    Create,
    Update,
    Delete,
}

#[derive(Clone)]
pub struct Action {
    pub feed: Option<Feed>,
    pub action: ActionType,
}
