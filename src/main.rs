use axum::{
    routing::{get,post},
    http::StatusCode,
    Json,
    Router
};
use serde::{Deserialize, Serialize};

pub mod transit {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
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

#[derive(Serialize, Deserialize)]
struct Feed {
    name: String,
    url: String,
    frequency: u32,
}

#[axum_macros::debug_handler]
async fn post_handler(
    Json(payload): Json<Feed>,
) -> (StatusCode, Json<Feed>) {
    let feed = Feed {
        name: payload.name,
        url: payload.url,
        frequency: payload.frequency
    };

    (StatusCode::CREATED, Json(feed))
}
