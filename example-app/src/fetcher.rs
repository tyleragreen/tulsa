use crate::fetcher::transit::FeedMessage;
use prost::bytes::Bytes;
use prost::Message;
use reqwest::Client;
use tokio::time::{Duration, Interval};
use ureq;

mod transit {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

use crate::models::Feed;

async fn fetch(feed: &Feed) -> usize {
    println!("Fetching {}", feed.name);

    let client = Client::new();

    let headers = feed.to_header_map();
    let response = client
        .get(&feed.url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| {
            eprintln!("Error fetching {}: {}", feed.name, e);
            e
        })
        .unwrap();

    let bytes = response
        .bytes()
        .await
        .map_err(|e| eprintln!("Error reading {}: {}", feed.name, e))
        .unwrap();

    let b = FeedMessage::decode(bytes)
        .map_err(|e| eprintln!("Error decoding {}: {}", feed.name, e))
        .unwrap();

    let mut num_trip_updates: usize = 0;
    for e in b.entity {
        if e.trip_update.is_some() {
            num_trip_updates += 1;
        }
    }
    println!("{}: {} trip updates", feed.name, num_trip_updates);

    num_trip_updates
}

pub fn fetch_sync(feed: &Feed) -> usize {
    println!("Fetching {}", feed.name);

    let mut request = ureq::get(&feed.url);
    for (key, value) in feed.headers.iter() {
        request = request.set(key, value);
    }

    let response = request
        .call()
        .map_err(|e| {
            eprintln!("Error fetching {}: {}", feed.name, e);
            e
        })
        .unwrap();

    let mut vec_bytes = Vec::new();
    response.into_reader().read_to_end(&mut vec_bytes).unwrap();
    let bytes: Bytes = vec_bytes.into();

    let b = FeedMessage::decode(bytes)
        .map_err(|e| eprintln!("Error decoding {}: {}", feed.name, e))
        .unwrap();

    let mut num_trip_updates: usize = 0;
    for e in b.entity {
        if e.trip_update.is_some() {
            num_trip_updates += 1;
        }
    }
    println!("{}: {} trip updates", feed.name, num_trip_updates);

    num_trip_updates
}

pub async fn recurring_fetch(feed: Feed) {
    let interval_duration = Duration::from_secs(feed.frequency);
    let mut interval: Interval = tokio::time::interval(interval_duration);

    loop {
        interval.tick().await;
        // It might technically be more accurate timer-wise to spawn this
        // like so: tokio::spawn(fetch(feed.clone()));
        fetch(&feed).await;
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(feature = "use_dependencies"))]
    use crate::deps::mockito;

    use super::*;
    use std::collections::HashMap;
    use std::fs;
    use std::io::Read;
    use std::vec::Vec;

    #[tokio::test]
    async fn test_fetcher() {
        let mut buffer: Vec<u8> = Vec::new();
        let mut file = fs::File::open("fixtures/gtfs-07132023-123501")
            .map_err(|e| eprintln!("Failed to open the file: {}", e))
            .unwrap();
        file.read_to_end(&mut buffer)
            .expect("Failed to read the file");

        let mut server = mockito::Server::new();
        let mock = server
            .mock("GET", "/gtfs")
            .with_status(200)
            .with_body(buffer)
            .create();

        let feed = Feed {
            id: 1,
            name: "Test".to_string(),
            frequency: 5,
            url: format!("{}/gtfs", server.url()),
            headers: HashMap::new(),
        };

        let num_found = fetch(&feed).await;

        mock.assert();
        assert_eq!(num_found, 243);
    }
}
