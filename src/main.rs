use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use model::{Action, ActionType, Feed};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock};
use std::{
    collections::HashMap,
    sync::mpsc::{self, Sender},
};

pub mod transit {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

mod fetcher;
mod model;
mod scheduler;

#[derive(Clone)]
struct AppState {
    feed_id: Arc<RwLock<u32>>,
    db: Arc<RwLock<HashMap<u32, Feed>>>,
    sender: Arc<Mutex<Sender<Action>>>,
}

fn app(sender: Sender<Action>) -> Router {
    let state = AppState {
        feed_id: Arc::new(RwLock::new(1)),
        db: Arc::new(RwLock::new(HashMap::new())),
        sender: Arc::new(Mutex::new(sender)),
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

#[tokio::main]
async fn main() {
    let (sender, receiver) = mpsc::channel();
    scheduler::init(receiver);

    let address: &str = "0.0.0.0:3000";
    println!("Starting server on {}.", address);
    axum::Server::bind(&address.parse().unwrap())
        .serve(app(sender).into_make_service())
        .await
        .unwrap();
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

#[derive(Deserialize, Serialize)]
struct CreateFeed {
    name: String,
    url: String,
    frequency: u64,
    headers: HashMap<String, String>,
}

#[axum_macros::debug_handler]
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

    state.db.write().unwrap().insert(id, feed.clone());
    *(state.feed_id.write().unwrap()) += 1;

    let action = Action {
        id,
        feed: Some(feed.clone()),
        action: ActionType::Create,
    };
    let result = state.sender.lock().unwrap().send(action);

    if let Err(e) = result {
        println!("{}", e);
    }

    (StatusCode::CREATED, Json(feed))
}

async fn get_handler(path: Path<String>, state: State<AppState>) -> impl IntoResponse {
    let id: u32 = match path.parse() {
        Ok(i) => i,
        Err(_) => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let db = state.db.read().unwrap();

    if let Some(feed) = db.get(&id).cloned() {
        Ok(Json(feed))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

async fn put_handler(
    path: Path<String>,
    state: State<AppState>,
    Json(payload): Json<CreateFeed>,
) -> impl IntoResponse {
    let id: u32 = match path.parse() {
        Ok(i) => i,
        Err(_) => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let feed = Feed {
        id,
        name: payload.name,
        url: payload.url,
        frequency: payload.frequency,
        headers: payload.headers,
    };

    state.db.write().unwrap().insert(id, feed.clone());
    let action = Action {
        id,
        feed: Some(feed.clone()),
        action: ActionType::Update,
    };
    let result = state.sender.lock().unwrap().send(action);

    if let Err(e) = result {
        println!("{}", e);
    }

    Ok(Json(feed))
}

async fn delete_handler(path: Path<String>, state: State<AppState>) -> impl IntoResponse {
    let id: u32 = match path.parse() {
        Ok(i) => i,
        Err(_) => {
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    let mut db = state.db.write().unwrap();

    if !db.contains_key(&id) {
        return Err(StatusCode::NOT_FOUND);
    }

    db.remove(&id);
    let action = Action {
        id,
        feed: None,
        action: ActionType::Delete,
    };
    let result = state.sender.lock().unwrap().send(action);

    if let Err(e) = result {
        println!("{}", e);
    }

    Ok(StatusCode::NO_CONTENT)
}

async fn list_handler(state: State<AppState>) -> impl IntoResponse {
    let db = state.db.read().unwrap();
    let feeds: Vec<Feed> = db.values().cloned().collect();
    Json(feeds)
}

#[cfg(test)]
mod api_tests {
    use crate::model::Feed;

    use super::*;
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use tower::ServiceExt; // for `oneshot`

    impl From<CreateFeed> for Body {
        fn from(feed: CreateFeed) -> Self {
            return Body::from(serde_json::to_string(&feed).unwrap());
        }
    }

    #[tokio::test]
    async fn status() {
        // there should be a way to make this a mock channel instead. I struggled with the types
        // here but I may look into this in the future.
        let (sender, _) = mpsc::channel();
        let response = app(sender)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(response.into_body()).await.unwrap();
        assert_eq!(&body[..], b"{\"status\":\"OK\"}");
    }

    #[tokio::test]
    async fn invalid() {
        let (sender, _) = mpsc::channel();
        let response = app(sender.clone())
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

        let response = app(sender.clone())
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
    async fn post() {
        let headers = HashMap::from([("auth".to_string(), "key".to_string())]);
        let input = CreateFeed {
            name: "Name".to_string(),
            url: "http".to_string(),
            frequency: 10,
            headers,
        };
        let (sender, _) = mpsc::channel();
        let response = app(sender)
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

        let (sender, _) = mpsc::channel();
        tokio::spawn(async move {
            axum::Server::bind(&addr.parse().unwrap())
                .serve(app(sender).into_make_service())
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
