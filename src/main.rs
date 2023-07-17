use axum::{
    routing::{get,post},
    extract::{Path,State},
    response::IntoResponse,
    http::StatusCode,
    Json,
    Router
};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use std::sync::{Arc,Mutex,RwLock};
use std::collections::{
    VecDeque,
    HashMap
};

pub mod transit {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

mod scheduler;
mod feed;

lazy_static! {
    static ref QUEUE: Mutex<VecDeque<feed::Feed>> = Mutex::new(VecDeque::new());
}

#[derive(Clone)]
struct AppState {
    feed_id: Arc<RwLock<u32>>,
    db: Arc<RwLock<HashMap<u32, feed::Feed>>>,
}

fn app() -> Router {
    let state = AppState {
        feed_id: Arc::new(RwLock::new(1)),
        db: Arc::new(RwLock::new(HashMap::new())),
    };
    Router::new()
        .route("/", get(status_handler))
        .route("/feed/:key", get(feed_get_handler))
        .route("/feed", post(feed_post_handler))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let feed = transit::FeedMessage::default();
    dbg!(feed);

    scheduler::init();

    let address: &str = "0.0.0.0:3000";
    println!("Starting server on {}.", address);
    axum::Server::bind(&address.parse().unwrap())
        .serve(app().into_make_service())
        .await
        .unwrap();
}

#[derive(Serialize)]
struct Status {
    status: String,
}

async fn status_handler() -> Json<Status> {
    Json(Status{
        status: "OK".to_string()
    })
}

#[derive(Deserialize)]
struct CreateFeed {
    name: String,
    url: String,
    frequency: u32,
}

#[axum_macros::debug_handler]
async fn feed_post_handler(
    state: State<AppState>,
    Json(payload): Json<CreateFeed>,
    ) -> (StatusCode, Json<feed::Feed>) {

    let new_id = *(state.feed_id.read().unwrap());
    let feed = feed::Feed {
        id: new_id,
        name: payload.name,
        url: payload.url,
        frequency: payload.frequency
    };

    state.db.write().unwrap().insert(new_id, feed.clone());
    *(state.feed_id.write().unwrap()) += 1;
    QUEUE.lock().unwrap().push_back(feed.clone());

    (StatusCode::CREATED, Json(feed))
}

#[axum_macros::debug_handler]
async fn feed_get_handler(
    path: Path<String>,
    state: State<AppState>,
    ) -> impl IntoResponse {
    let db = state.db.read().unwrap();
    let feed_id = path.parse().unwrap();

    if let Some(feed) = db.get(&feed_id).cloned() {
        Ok(Json(feed))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}
