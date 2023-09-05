use std::net::SocketAddr;
use std::thread;
use tokio::net::TcpListener;
use tokio::runtime;
use tokio::task::spawn;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request, Response as HyperResponse};
use std::sync::{Arc, RwLock};

use super::error::MockError;
use super::mock::Mock;
use super::state::State;

pub struct Server {
    address: SocketAddr,
    state: Arc<RwLock<State>>,
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
        Mock::new(self.state.clone(), method, path)
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.address.to_string())
    }
}

async fn handle_request(
    request: Request<Body>,
    state: Arc<RwLock<State>>,
) -> Result<HyperResponse<Body>, MockError> {
    let state_b = state.clone();
    let mut state = state_b.write().unwrap();
    let mut matching: Vec<&mut Mock> = vec![];

    for mock in state.mocks.iter_mut() {
        if mock.matches(&request) {
            matching.push(mock);
        }
    }
    let mock = matching.first_mut();

    if let Some(mock) = mock {
        mock.inner.num_called += 1;
        let response = HyperResponse::new(Body::from(mock.inner.response.body.clone()));
        Ok(response)
    } else {
        panic!("No matching mock found");
    }
}
