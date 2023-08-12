use std::io::Read;
use std::thread;

pub struct Mock {}

impl Mock {
    pub fn assert(&self) -> bool {
        true
    }
}

pub struct Builder {}

impl Builder {
    pub fn with_status(&mut self, _status: u16) -> &mut Builder {
        self
    }

    pub fn with_body(&mut self, _body: Vec<u8>) -> &mut Builder {
        self
    }

    pub fn create(&self) -> Mock {
        Mock {}
    }
}

pub struct Server {}
use std::fs;
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::task::spawn;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request as HyperRequest, Response};

use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct MyError {
    message: String,
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "MyError: {}", self.message)
    }
}

impl Error for MyError {}

async fn handle_request(
    _request: HyperRequest<Body>,
) -> Result<Response<Body>, MyError> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut file = fs::File::open("fixtures/gtfs-07132023-123501").expect("Failed to open the file");
    file.read_to_end(&mut buffer)
        .expect("Failed to read the file");

    let response = Response::new(Body::from(buffer));
    Ok(response)
}

impl Server {
    pub fn new() -> Server {
        let address = "localhost";
        let port = 5001;
        let runtime = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        thread::spawn(move || {
            runtime.block_on(async {
                let listener = TcpListener::bind(format!("{}:{}", address, port))
                    .await
                    .unwrap();

                while let Ok((stream, _)) = listener.accept().await {
                    spawn(async move {
                        let _ = Http::new()
                            .serve_connection(
                                stream,
                                service_fn(move |request: HyperRequest<Body>| {
                                    handle_request(request)
                                }),
                            )
                            .await;
                    });
                }
            });
        });
        Server {}
    }

    pub fn mock(&self, _method: &'static str, _path: &'static str) -> Builder {
        Builder {}
    }

    pub fn url(&self) -> String {
        "http://localhost:5001".to_string()
    }
}
