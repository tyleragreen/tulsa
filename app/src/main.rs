use std::net::SocketAddr;
use tokio::{net::TcpListener, runtime::Builder};

use app::{api, scheduler_interface::build};

fn main() {
    // Initialize tracing subscriber for logging
    tracing_subscriber::fmt::init();

    let address = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Starting server on {}.", address);

    let interface = build();

    // We use a runtime::Builder to specify the number of threads and
    // their name.
    //
    // If we didn't want these customizations, we could just use #[tokio:main]
    // to launch a runtime automatically.
    let runtime = Builder::new_multi_thread()
        .enable_io()
        .worker_threads(1)
        .thread_name("server-runtime")
        .build()
        .unwrap();

    runtime.block_on(async {
        let listener = TcpListener::bind(address).await.unwrap();
        let router = api::app(interface).into_make_service();
        axum::serve(listener, router).await.unwrap();
    });
}
