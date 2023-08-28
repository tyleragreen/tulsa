#[cfg(test)]
mod tests {
    use gtfs_realtime_rust::api;
    use gtfs_realtime_rust::scheduler;
    use reqwest::blocking::Client;
    use serde_json::json;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use tokio::runtime::Builder;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[test]
    fn integration() {
        let (sender, receiver) = mpsc::channel();
        scheduler::init_sync(receiver);

        thread::spawn(move || {
            let runtime = Builder::new_multi_thread().enable_io().build().unwrap();

            let address: &str = "0.0.0.0:3000";
            runtime.block_on(async {
                axum::Server::bind(&address.parse().unwrap())
                    .serve(api::app(Arc::new(Mutex::new(sender))).into_make_service())
                    .await
                    .unwrap();
            });
        });

        thread::sleep(Duration::from_millis(250));

        let client = Client::new();
        let data = json!({
            "name": "MTA",
            "frequency": 5,
            "url": "no_url",
            "headers": {
                "x-api-key": "no_key"
            },
        });

        let response = client.post("http://localhost:3000/feed").json(&data).send();

        match response {
            Ok(response) => {
                assert_eq!(response.status(), 201);
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                assert!(false);
            }
        }
    }
}
