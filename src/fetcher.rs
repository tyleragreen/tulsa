use crate::feed::Feed;
use bytes::IntoBuf;
use prost::Message;
use reqwest::Client;

use crate::transit::FeedMessage;

pub async fn fetch(feed: Feed) {
    println!("Fetching {}", feed.name);

    let client = Client::new();

    let headers = feed.to_header_map();
    let response = client
        .get(feed.url)
        .headers(headers)
        .send()
        .await
        .expect("fetch failed!");
    let bytes = response.bytes().await.unwrap();

    let b = FeedMessage::decode(bytes.into_buf()).unwrap();

    let mut num_trip_updates: u32 = 0;
    for e in b.entity {
        if e.trip_update.is_some() {
            num_trip_updates += 1;
        }
    }
    println!("{}: {}", feed.name, num_trip_updates);
}
