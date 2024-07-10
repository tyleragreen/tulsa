use axum::{
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use std::convert::Infallible;
use tracing::info;

// Middleware function to log requests
pub async fn log_request(req: Request<Body>, next: Next) -> Result<Response<Body>, Infallible> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    info!("Incoming request: {} {}", method, uri);

    // Call the next middleware or handler
    let response = next.run(req).await;

    Ok(response)
}
