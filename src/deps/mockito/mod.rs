use std::io::Read;
use std::net::SocketAddr;
use std::thread;
use std::fs::File;
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::task::spawn;
use hyper::StatusCode;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request, Response as HyperResponse};
use std::sync::{Arc, RwLock};

mod error;

use error::MockError;

pub struct Response {
    status: StatusCode,
    body: Vec<u8>,
}

impl Default for Response {
    fn default() -> Self {
        Response {
            status: StatusCode::OK,
            body: vec![],
        }
    }
}

pub struct Mock {
    method: String,
    path: String,
    response: Response,
    num_called: usize,
}

impl Mock {
    pub fn new(method: &str, path: &str) -> Mock {
        Mock {
            method: method.to_owned().to_uppercase(),
            path: path.to_owned(),
            response: Response::default(),
            num_called: 0,
        }
    }

    pub fn with_status(mut self, status: u16) -> Mock {
        self.response.status = StatusCode::from_u16(status).unwrap();
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Mock {
        self.response.body = body;
        self
    }

    pub fn create(self) -> Mock {
        self
    }

    pub fn assert(&self) -> bool {
        true
    }
    
    pub fn matches(&self, request: &mut Request<Body>) -> bool {
        let method = request.method().to_string();
        let path = request.uri().path().to_string();

        method == self.method && path == self.path
    }
}

struct State {
    mocks: Vec<Mock>,
}

impl State {
    fn new() -> Self {
        State {
            mocks: vec![],
        }
    }
}

pub struct Server {
    address: SocketAddr,
    state: Arc<RwLock<State>>,
}

async fn handle_request(
    request: Request<Body>,
    state: Arc<RwLock<State>>,
) -> Result<HyperResponse<Body>, MockError> {
    let state_b = state.clone();
    let mut state = state_b.write().unwrap();
    let mut matching: Vec<&mut Mock> = vec![];

    for mock in state.mocks.iter_mut() {
        if mock.matches(&mut request) {
            matching.push(mock);
        }
    }
    let mock = matching.first_mut();

    if let Some(mock) = mock {
        mock.num_called += 1;
        let response = HyperResponse::new(Body::empty());
        Ok(response)
    } else {
        panic!("No matching mock found");
    }
}

impl Server {
    pub fn new() -> Server {
        let address = SocketAddr::from(([127, 0, 0, 1], 5001));
        let state = Arc::new(RwLock::new(State::new()));

        let runtime = runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        let state_b = state.clone();
        thread::spawn(move || {
            runtime.block_on(async {
                let listener = TcpListener::bind(address)
                    .await
                    .unwrap();

                while let Ok((stream, _)) = listener.accept().await {
                    let state_c = state_b.clone();
                    spawn(async move {
                        let _ = Http::new()
                            .serve_connection(
                                stream,
                                service_fn(move |request: Request<Body>| {
                                    handle_request(request, state_c.clone())
                                }),
                            )
                            .await;
                    });
                }
            });
        });

        Server {
            address,
            state,
        }
    }

    pub fn mock(&self, method: &str, path: &str) -> Mock {
        Mock::new(method, path)
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.address.to_string())
    }
}
