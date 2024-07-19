use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware::from_fn,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    middleware::log_request,
    models::{CreateFeed, Feed, Status},
    scheduler_interface::ToScheduler,
};

#[derive(Clone)]
struct AppState<T>
where
    T: ToScheduler + Send + Sync + 'static,
{
    next_feed_id: Arc<RwLock<usize>>,
    db: Arc<RwLock<HashMap<usize, Feed>>>,
    scheduler_interface: Arc<T>,
}

pub fn app<T>(scheduler_interface: Arc<T>) -> Router
where
    T: ToScheduler + Send + Sync + 'static,
{
    let state = AppState {
        next_feed_id: Arc::new(RwLock::new(1)),
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
        .layer(from_fn(log_request))
        .with_state(state)
}

async fn status_handler() -> impl IntoResponse {
    Json(Status::new("OK"))
}

async fn post_handler<T>(
    state: State<AppState<T>>,
    Json(CreateFeed {
        name,
        url,
        frequency,
        headers,
    }): Json<CreateFeed>,
) -> Result<impl IntoResponse, StatusCode>
where
    T: ToScheduler + Send + Sync + 'static,
{
    let id = *(state
        .next_feed_id
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?);
    let feed = Feed {
        id,
        name,
        url,
        frequency,
        headers,
    };

    *(state
        .next_feed_id
        .write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?) += 1;
    state
        .db
        .write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .insert(id, feed.clone());

    state
        .scheduler_interface
        .create(feed.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok((StatusCode::CREATED, Json(feed)))
}

async fn get_handler<T>(
    Path(id): Path<usize>,
    state: State<AppState<T>>,
) -> Result<impl IntoResponse, StatusCode>
where
    T: ToScheduler + Send + Sync + 'static,
{
    let feed = state
        .db
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get(&id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(feed))
}

async fn put_handler<T>(
    Path(id): Path<usize>,
    state: State<AppState<T>>,
    Json(CreateFeed {
        name,
        url,
        frequency,
        headers,
    }): Json<CreateFeed>,
) -> impl IntoResponse
where
    T: ToScheduler + Send + Sync + 'static,
{
    if state
        .db
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get(&id)
        .is_none()
    {
        return Err(StatusCode::NOT_FOUND);
    }

    let feed = Feed {
        id,
        name,
        url,
        frequency,
        headers,
    };

    state
        .db
        .write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .insert(id, feed.clone());
    state
        .scheduler_interface
        .update(feed.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(feed))
}

async fn delete_handler<T>(
    Path(id): Path<usize>,
    state: State<AppState<T>>,
) -> Result<impl IntoResponse, StatusCode>
where
    T: ToScheduler + Send + Sync + 'static,
{
    let feed = state
        .db
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .get(&id)
        .cloned()
        .ok_or(StatusCode::NOT_FOUND)?;

    state
        .db
        .write()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .remove(&feed.id);
    state
        .scheduler_interface
        .delete(feed)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_handler<T>(state: State<AppState<T>>) -> Result<impl IntoResponse, StatusCode>
where
    T: ToScheduler + Send + Sync + 'static,
{
    let feeds: Vec<Feed> = state
        .db
        .read()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .values()
        .cloned()
        .collect();
    Ok(Json(feeds))
}

#[cfg(test)]
mod api_tests {
    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use std::{
        net::SocketAddr,
        sync::{mpsc::SendError, Mutex},
    };
    use tokio::net::TcpListener;
    use tower::ServiceExt; // for `oneshot`

    #[cfg(not(feature = "use_dependencies"))]
    use crate::deps::mime;
    use crate::scheduler_interface::{SchedulerInterface, TaskSend};
    use tulsa::{AsyncTask, Task};

    use super::*;

    struct MockSender<T> {
        tasks: Arc<Mutex<Vec<T>>>,
    }

    impl<T> TaskSend<T> for MockSender<T>
    where
        T: Task,
    {
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
        let sender = Arc::new(MockSender::new());
        let interface = Arc::new(SchedulerInterface::new(sender.clone()));
        let response = app(interface)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(sender.count(), 0);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"{\"status\":\"OK\"}");
    }

    #[tokio::test]
    async fn invalid() {
        let sender = Arc::new(MockSender::new());
        let interface = Arc::new(SchedulerInterface::new(sender));
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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Invalid URL: Cannot parse `\"abc\"` to a `u64`");

        let sender = Arc::new(MockSender::new());
        let interface = Arc::new(SchedulerInterface::new(sender));
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

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Invalid URL: Cannot parse `\"-1\"` to a `u64`");
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
        let sender = Arc::new(MockSender::new());
        let interface = Arc::new(SchedulerInterface::new(sender));
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

        let sender = Arc::new(MockSender::new());
        let interface = Arc::new(SchedulerInterface::new(sender));
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
        let sender = Arc::new(MockSender::new());
        assert_eq!(sender.count(), 0);
        let interface = Arc::new(SchedulerInterface::new(sender.clone()));
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
        assert_eq!(sender.count(), 1);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let f: Feed = serde_json::from_slice(&body).unwrap();

        assert_eq!(f.id, 1);
        assert_eq!(f.name, "Name");
        assert_eq!(f.url, "http");
        assert_eq!(f.frequency, 10);
        assert_eq!(f.headers["auth"], "key");
    }

    #[tokio::test]
    async fn full_api_flow() {
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

        let sender = Arc::new(MockSender::new());
        let interface = Arc::new(SchedulerInterface::new(sender));
        let address = SocketAddr::from(([0, 0, 0, 0], 3000));
        tokio::spawn(async move {
            let listener = TcpListener::bind(address).await.unwrap();
            let router = app(interface).into_make_service();
            axum::serve(listener, router).await.unwrap();
        });

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://localhost:3000/feed"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), StatusCode::OK.as_u16());

        let body = response.bytes().await.unwrap();
        let f: Vec<Feed> = serde_json::from_slice(&body).unwrap();
        assert_eq!(f.len(), 0);
        assert_eq!(&body[..], b"[]");

        let response = client
            .post(format!("http://localhost:3000/feed"))
            .header(
                http::header::CONTENT_TYPE.as_str(),
                mime::APPLICATION_JSON.as_ref(),
            )
            .json(&serde_json::json!(input))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), StatusCode::CREATED.as_u16());

        let response = client
            .get(format!("http://localhost:3000/feed"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), StatusCode::OK.as_u16());

        let body = response.bytes().await.unwrap();
        let f: Vec<Feed> = serde_json::from_slice(&body).unwrap();
        assert_eq!(f.len(), 1);

        let response = client
            .put(format!("http://localhost:3000/feed/1"))
            .header(
                http::header::CONTENT_TYPE.as_str(),
                mime::APPLICATION_JSON.as_ref(),
            )
            .json(&serde_json::json!(input_new))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), StatusCode::OK.as_u16());

        let response = client
            .get(format!("http://localhost:3000/feed/1"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), StatusCode::OK.as_u16());

        let body = response.bytes().await.unwrap();
        let f: Feed = serde_json::from_slice(&body).unwrap();

        assert_eq!(f.id, 1);
        assert_eq!(f.name, "Name");
        assert_eq!(f.url, "http");
        assert_eq!(f.frequency, 20);

        let response = client
            .delete(format!("http://localhost:3000/feed/1"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), StatusCode::NO_CONTENT.as_u16());

        let body = response.bytes().await.unwrap();
        assert_eq!(body.len(), 0);

        let response = client
            .get(format!("http://localhost:3000/feed"))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status().as_u16(), StatusCode::OK.as_u16());

        let body = response.bytes().await.unwrap();
        let f: Vec<Feed> = serde_json::from_slice(&body).unwrap();
        assert_eq!(f.len(), 0);
        assert_eq!(&body[..], b"[]");
    }
}
