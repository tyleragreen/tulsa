#[cfg(test)]
mod tests {
    use gtfs_realtime_rust::api;
    use gtfs_realtime_rust::model::AsyncTask;
    use gtfs_realtime_rust::scheduler;
    use reqwest::blocking::Client;
    use serde_json::json;
    use std::fs::File;
    use std::fs::OpenOptions;
    use std::io::prelude::*;
    use std::process::Command;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use tokio::runtime::Builder;

    static FILE_NAME: &'static str = "/tmp/rust_test_output.txt";

    fn confirm_wc(file_path: &str, expected: i32) {
        let output = Command::new("wc")
            .arg(file_path)
            .output()
            .expect("failed to execute process");

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let output_trimmed = output_str.trim();
            let output_vec: Vec<&str> = output_trimmed.split(" ").collect();
            let output_num: i32 = output_vec[0].parse().unwrap();
            assert_eq!(output_num, expected);
        } else {
            assert!(false);
        }
    }

    fn touch(file_name: &str) {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_name)
            .unwrap();
        file.write_all(b"").unwrap();
    }

    fn create_task(id: usize, millis: u64, count: i32) -> AsyncTask {
        let task = AsyncTask::new(id, async move {
            let duration = Duration::from_millis(millis);
            let mut interval = tokio::time::interval(duration);
            let mut file = File::create(FILE_NAME).unwrap();

            for i in 0..count {
                interval.tick().await;
                file.write_all(i.to_string().as_bytes()).unwrap();
                file.write_all(b"\n").unwrap();
            }
        });

        task
    }

    #[test]
    fn scheduler() {
        let (sender, receiver) = mpsc::channel();
        scheduler::init(receiver);

        // Clear the file and ensure it exists
        touch(FILE_NAME);

        let task = create_task(1, 100, 5);

        // File should still be empty before the send the task to the scheduler
        confirm_wc(FILE_NAME, 0);
        match sender.send(task) {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }

        // Wait for the task to finish and then confirm the file has the correct contents
        thread::sleep(Duration::from_millis(550));
        confirm_wc(FILE_NAME, 5);
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
