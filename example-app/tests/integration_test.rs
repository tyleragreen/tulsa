#[cfg(test)]
mod tests {
    use reqwest::blocking::Client;
    use serde_json::json;
    use tokio::net::TcpListener;
    use std::net::SocketAddr;
    use std::thread;
    use std::time::Duration;
    use tokio::runtime::Builder;

    use gtfs_realtime_rust::api;
    use gtfs_realtime_rust::scheduler_interface::{build, Mode};

    fn run(mode: Mode) {
        let interface = build(mode);

        thread::spawn(move || {
            let runtime = Builder::new_multi_thread().enable_io().build().unwrap();

            let address = SocketAddr::from(([0, 0, 0, 0], 3000));
            runtime.block_on(async {
                let listener = TcpListener::bind(address).await.unwrap();
                let router = api::app(interface).into_make_service();
                axum::serve(listener, router).await.unwrap();
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

    #[test]
    fn async_run() {
        run(Mode::Async);
    }

    #[test]
    fn sync_run() {
        run(Mode::Sync);
    }
}
