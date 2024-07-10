use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1::Builder,
    service::service_fn,
    Request, Response as HyperResponse,
};
use hyper_util::rt::TokioIo;
use std::{
    future::Future,
    net::SocketAddr,
    sync::{Arc, RwLock},
    thread,
};
use tokio::{net::TcpListener, runtime, task::spawn};

use super::{error::MockError, mock::Mock, state::State};

pub struct Server {
    address: SocketAddr,
    state: Arc<RwLock<State>>,
}

impl Server {
    /// Beginning in mockito 1.3.1, calling [`Server::new`] was no longer permitted from inside a
    /// tokio runtime. We use a fake-async server here just to satisfy the interface.
    pub async fn new_async() -> Server {
        Self::new()
    }

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
                let listener = TcpListener::bind(address).await.unwrap();

                while let Ok((stream, _)) = listener.accept().await {
                    let state_c = state_b.clone();
                    spawn(async move {
                        let io = TokioIo::new(stream);
                        let _ = Builder::new()
                            .serve_connection(
                                io,
                                service_fn(move |request: Request<Incoming>| {
                                    handle_request(request, state_c.clone())
                                }),
                            )
                            .await;
                    });
                }
            });
        });

        Server { address, state }
    }

    pub fn mock(&self, method: &str, path: &str) -> Mock {
        Mock::new(self.state.clone(), method, path)
    }

    pub fn url(&self) -> String {
        format!("http://{}", self.address.to_string())
    }
}

async fn handle_request(
    request: Request<Incoming>,
    state: Arc<RwLock<State>>,
) -> Result<HyperResponse<Full<Bytes>>, MockError> {
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
        let response = HyperResponse::new(Full::new(Bytes::from(mock.inner.response.body.clone())));
        Ok(response)
    } else {
        panic!("No matching mock found");
    }
}
