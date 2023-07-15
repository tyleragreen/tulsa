use axum::{
    routing::{get,post},
    http::StatusCode,
    Json,
    Router
};
use serde::{Deserialize, Serialize};
use lazy_static::lazy_static;
use std::sync::Mutex;

pub mod transit {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

lazy_static! {
    static ref ID: Mutex<u32> = Mutex::new(1 as u32);
}

#[tokio::main]
async fn main() {
    let feed = transit::FeedMessage::default();
    dbg!(feed);

    let app = Router::new()
        .route("/", get(get_handler))
        .route("/", post(post_handler));

    let address: &str = "0.0.0.0:3000";
    println!("Starting server on {}.", address);
    axum::Server::bind(&address.parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_handler() -> String {
    format!("Hello, World!")
}

#[derive(Serialize)]
struct Feed {
    id: u32,
    name: String,
    url: String,
    frequency: u32,
}

#[derive(Deserialize)]
struct CreateFeed {
    name: String,
    url: String,
    frequency: u32,
}

#[axum_macros::debug_handler]
async fn post_handler(
    Json(payload): Json<CreateFeed>,
) -> (StatusCode, Json<Feed>) {
    let feed = Feed {
        id: *(ID.lock().unwrap()),
        name: payload.name,
        url: payload.url,
        frequency: payload.frequency
    };

    *(ID.lock().unwrap()) += 1;

    (StatusCode::CREATED, Json(feed))
}
