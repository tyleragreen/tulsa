use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::fetcher::Feed;
use crate::scheduler::SchedulerInterface;

#[derive(Clone)]
struct AppState {
    feed_id: Arc<RwLock<usize>>,
    db: Arc<RwLock<HashMap<usize, Feed>>>,
    scheduler_interface: Arc<dyn SchedulerInterface + Send + Sync>,
}

pub fn app(scheduler_interface: Arc<dyn SchedulerInterface + Send + Sync>) -> Router {
    let state = AppState {
        feed_id: Arc::new(RwLock::new(1)),
        db: Arc::new(RwLock::new(HashMap::new())),
        scheduler_interface,
    };
    Router::new()
        .route("/", get(status_handler))
        .route(
            "/feed/:key",
            get(get_handler).put(put_handler).delete(delete_handler),
        )
        .route("/feed", post(post_handler).get(list_handler))
        .with_state(state)
}

#[derive(Serialize)]
struct Status {
    status: String,
}

async fn status_handler() -> Json<Status> {
    Json(Status {
        status: "OK".to_string(),
    })
}

#[derive(Clone, Deserialize, Serialize)]
struct CreateFeed {
    name: String,
    url: String,
    frequency: u64,
    headers: HashMap<String, String>,
}

async fn post_handler(
    state: State<AppState>,
    Json(payload): Json<CreateFeed>,
) -> (StatusCode, Json<Feed>) {
    let id = *(state.feed_id.read().unwrap());
    let feed = Feed {
        id,
        name: payload.name,
        url: payload.url,
        frequency: payload.frequency,
        headers: payload.headers,
    };

    *(state.feed_id.write().unwrap()) += 1;
    state.db.write().unwrap().insert(id, feed.clone());
    state.scheduler_interface.create(feed.clone());

    (StatusCode::CREATED, Json(feed))
}

async fn get_handler(path: Path<String>, state: State<AppState>) -> impl IntoResponse {
    let id: usize = match path.parse() {
        Ok(i) => i,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    match state.db.read().unwrap().get(&id).cloned() {
        Some(feed) => Ok(Json(feed)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

async fn put_handler(
    path: Path<String>,
    state: State<AppState>,
    Json(payload): Json<CreateFeed>,
) -> impl IntoResponse {
    let id: usize = match path.parse() {
        Ok(i) => i,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    if state.db.read().unwrap().get(&id).is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let feed = Feed {
        id,
        name: payload.name,
        url: payload.url,
        frequency: payload.frequency,
        headers: payload.headers,
    };

    state.db.write().unwrap().insert(id, feed.clone());
    state.scheduler_interface.update(feed.clone());

    Ok(Json(feed))
}

async fn delete_handler(path: Path<String>, state: State<AppState>) -> impl IntoResponse {
    let id: usize = match path.parse() {
        Ok(i) => i,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };
    let feed = match state.db.read().unwrap().get(&id).cloned() {
        Some(f) => f,
        None => return Err(StatusCode::NOT_FOUND),
    };

    state.db.write().unwrap().remove(&feed.id);
    state.scheduler_interface.delete(feed);

    Ok(StatusCode::NO_CONTENT)
}

async fn list_handler(state: State<AppState>) -> impl IntoResponse {
    let feeds: Vec<Feed> = state.db.read().unwrap().values().cloned().collect();
    Json(feeds)
}

#[cfg(test)]
mod api_tests {
    use crate::deps::mime;
    use crate::fetcher::Feed;
    use crate::scheduler::{Scheduler, TaskSender};
    use tulsa::model::AsyncTask;

    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use std::sync::mpsc::SendError;
    use std::sync::Mutex;
    use tower::ServiceExt; // for `oneshot`

    struct MockSender<T> {
        tasks: Arc<Mutex<Vec<T>>>,
    }

    impl<T> TaskSender<T> for MockSender<T> {
        fn send(&self, task: T) -> Result<(), SendError<T>> {
            self.tasks.lock().unwrap().push(task);
            Ok(())
        }
    }

    impl MockSender<AsyncTask> {
        fn new() -> Self {
            MockSender {
                tasks: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn count(&self) -> usize {
            self.tasks.lock().unwrap().iter().count()
        }
    }

    impl From<CreateFeed> for Body {
        fn from(feed: CreateFeed) -> Self {
            return Body::from(serde_json::to_string(&feed).unwrap());
        }
    }

    #[tokio::test]
    async fn status() {
        let sender = Arc::new(Mutex::new(MockSender::new()));
        let interface = Arc::new(Scheduler::new(sender.clone()));
        let response = app(interface)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(sender.lock().unwrap().count(), 0);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], b"{\"status\":\"OK\"}");
    }

    #[tokio::test]
    async fn invalid() {
        let sender = Arc::new(Mutex::new(MockSender::new()));
        let interface = Arc::new(Scheduler::new(sender));
        let response = app(interface)
            .oneshot(
                Request::builder()
                    .uri("/feed/abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(body.len(), 0);

        let sender = Arc::new(Mutex::new(MockSender::new()));
        let interface = Arc::new(Scheduler::new(sender));
        let response = app(interface)
            .oneshot(
                Request::builder()
                    .uri("/feed/-1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(body.len(), 0);
    }

    #[tokio::test]
    async fn invalid_put() {
        let headers = HashMap::from([("auth".to_string(), "key".to_string())]);
        let input = CreateFeed {
            name: "Name".to_string(),
            url: "http".to_string(),
            frequency: 10,
            headers,
        };
        let sender = Arc::new(Mutex::new(MockSender::new()));
        let interface = Arc::new(Scheduler::new(sender));
        let response = app(interface)
            .oneshot(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri("/feed/10")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(input.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let sender = Arc::new(Mutex::new(MockSender::new()));
        let interface = Arc::new(Scheduler::new(sender));
        let response = app(interface)
            .oneshot(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri("/feed/-1")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(input.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn post() {
        let headers = HashMap::from([("auth".to_string(), "key".to_string())]);
        let input = CreateFeed {
            name: "Name".to_string(),
            url: "http".to_string(),
            frequency: 10,
            headers,
        };
        let sender = Arc::new(Mutex::new(MockSender::new()));
        assert_eq!(sender.lock().unwrap().count(), 0);
        let interface = Arc::new(Scheduler::new(sender.clone()));
        let response = app(interface)
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/feed")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(input))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(sender.lock().unwrap().count(), 1);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let f: Feed = serde_json::from_slice(&body).unwrap();

        assert_eq!(f.id, 1);
        assert_eq!(f.name, "Name");
        assert_eq!(f.url, "http");
        assert_eq!(f.frequency, 10);
        assert_eq!(f.headers["auth"], "key");
    }

    #[tokio::test]
    async fn full_api_flow() {
        let addr: &str = "0.0.0.0:3000";
        let input = CreateFeed {
            name: "Name".to_string(),
            url: "http".to_string(),
            frequency: 10,
            headers: HashMap::new(),
        };
        let input_new = CreateFeed {
            name: "Name".to_string(),
            url: "http".to_string(),
            frequency: 20,
            headers: HashMap::new(),
        };

        let sender = Arc::new(Mutex::new(MockSender::new()));
        let interface = Arc::new(Scheduler::new(sender));
        tokio::spawn(async move {
            axum::Server::bind(&addr.parse().unwrap())
                .serve(app(interface).into_make_service())
                .await
                .unwrap();
        });

        let client = hyper::Client::new();

        let response = client
            .request(
                Request::builder()
                    .uri(format!("http://localhost:3000/feed"))
                    .body(hyper::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let f: Vec<Feed> = serde_json::from_slice(&body).unwrap();
        assert_eq!(f.len(), 0);
        assert_eq!(&body[..], b"[]");

        let response = client
            .request(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("http://localhost:3000/feed")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(input))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let response = client
            .request(
                Request::builder()
                    .uri(format!("http://localhost:3000/feed"))
                    .body(hyper::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let f: Vec<Feed> = serde_json::from_slice(&body).unwrap();
        assert_eq!(f.len(), 1);

        let response = client
            .request(
                Request::builder()
                    .method(http::Method::PUT)
                    .uri("http://localhost:3000/feed/1")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::from(input_new))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let response = client
            .request(
                Request::builder()
                    .method(http::Method::GET)
                    .uri("http://localhost:3000/feed/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let f: Feed = serde_json::from_slice(&body).unwrap();

        assert_eq!(f.id, 1);
        assert_eq!(f.name, "Name");
        assert_eq!(f.url, "http");
        assert_eq!(f.frequency, 20);

        let response = client
            .request(
                Request::builder()
                    .method(http::Method::DELETE)
                    .uri("http://localhost:3000/feed/1")
                    .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(body.len(), 0);

        let response = client
            .request(
                Request::builder()
                    .uri(format!("http://localhost:3000/feed"))
                    .body(hyper::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        let f: Vec<Feed> = serde_json::from_slice(&body).unwrap();
        assert_eq!(f.len(), 0);
        assert_eq!(&body[..], b"[]");
    }
}
