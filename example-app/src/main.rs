use std::net::SocketAddr;
use std::sync::{mpsc, Arc, Mutex};
use tokio::runtime::Builder;
use tulsa::scheduler;

use gtfs_realtime_rust::api;

fn main() {
    let (sender, receiver) = mpsc::channel();
    scheduler::init_async(receiver);

    let address = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Starting server on {}.", address);

    // We use a runtime::Builder to specify the number of threads and
    // their name.
    // If we didn't want these customizations, we could just use #[tokio:main]
    // to launch a runtime automatically.
    let runtime = Builder::new_multi_thread()
        .enable_io()
        .worker_threads(1)
        .thread_name("server-runtime")
        .build()
        .unwrap();

    runtime.block_on(async {
        axum::Server::bind(&address)
            .serve(api::app(Arc::new(Mutex::new(sender))).into_make_service())
            .await
            .unwrap();
    });
}
