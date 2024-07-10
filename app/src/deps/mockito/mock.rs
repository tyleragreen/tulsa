use hyper::body::Incoming;
use hyper::Request;
use hyper::StatusCode;
use rand;
use std::sync::{Arc, RwLock};

use super::state::State;

#[derive(Clone)]
pub struct Response {
    pub status: StatusCode,
    pub body: Vec<u8>,
}

impl Default for Response {
    fn default() -> Self {
        Response {
            status: StatusCode::OK,
            body: vec![],
        }
    }
}

#[derive(Clone)]
pub struct InnerMock {
    pub id: usize,
    pub method: String,
    pub path: String,
    pub response: Response,
    pub num_called: usize,
}

#[derive(Clone)]
pub struct Mock {
    pub state: Arc<RwLock<State>>,
    pub inner: InnerMock,
}

impl Mock {
    pub fn new(state: Arc<RwLock<State>>, method: &str, path: &str) -> Mock {
        Mock {
            state,
            inner: InnerMock {
                id: rand::random(),
                method: method.to_owned().to_uppercase(),
                path: path.to_owned(),
                response: Response::default(),
                num_called: 0,
            },
        }
    }

    pub fn with_status(mut self, status: u16) -> Mock {
        self.inner.response.status = StatusCode::from_u16(status).unwrap();
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Mock {
        self.inner.response.body = body;
        self
    }

    pub fn create(self) -> Mock {
        let state = self.state.clone();
        let mut state = state.write().unwrap();
        state.mocks.push(self.clone());
        self
    }

    pub fn assert(&self) {
        let state = self.state.clone();
        let state = state.read().unwrap();
        let num_called = state
            .mocks
            .iter()
            .find(|mock| mock.inner.id == self.inner.id)
            .unwrap()
            .inner
            .num_called;

        if num_called == 0 {
            panic!("Mock not called");
        }
    }

    pub fn matches(&self, request: &Request<Incoming>) -> bool {
        let method = request.method().to_string();
        let path = request.uri().path().to_string();

        method == self.inner.method && path == self.inner.path
    }
}
