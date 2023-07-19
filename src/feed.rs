use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Feed {
    pub id: u32,
    pub name: String,
    pub url: String,
    pub frequency: u64,
}
