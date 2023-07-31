#[cfg(test)]
mod tests {
    use gtfs_realtime_rust::api;
    use gtfs_realtime_rust::scheduler;
    use gtfs_realtime_rust::model::AsyncTask;
    use reqwest::blocking::Client;
    use serde_json::json;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    //use std::fs::File;
    //use std::io::prelude::*;
    use tokio::runtime::Builder;

    #[test]
    fn scheduler() {
        let (sender, receiver) = mpsc::channel();
        scheduler::init(receiver);

        let task = AsyncTask::new(1, async {
            println!("Hello from async task");
            //let mut file = File::create("test.txt").unwrap();

            //file.write_all(b"Hello, world!").unwrap();

            //thread::sleep(Duration::from_millis(1000));

            //file.write_all(b"Goodbye, world!").unwrap();
        });

        //thread::sleep(Duration::from_secs(2));
        match sender.send(task) {
            Ok(_) => {
                assert!(true);
            }
            Err(_) => {
                assert!(false);
            }
        }
    }

    #[test]
    fn integration() {
        let (sender, receiver) = mpsc::channel();
        scheduler::init(receiver);

        thread::spawn(move || {
            let runtime = Builder::new_multi_thread().enable_io().build().unwrap();

            let address: &str = "0.0.0.0:3000";
            runtime.block_on(async {
                axum::Server::bind(&address.parse().unwrap())
                    .serve(api::app(sender).into_make_service())
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
