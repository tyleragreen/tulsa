use std::sync::mpsc;
use tokio::runtime::Builder;

use gtfs_realtime_rust::api;
use gtfs_realtime_rust::scheduler;

fn main() {
    let (sender, receiver) = mpsc::channel();
    scheduler::init(receiver);

    let address: &str = "0.0.0.0:3000";
    println!("Starting server on {}.", address);

    // We use a runtime::Builder to specify the number of threads,
    // otherwise we could just use #[tokio:main] to launch a runtime automatically.
    let runtime = Builder::new_multi_thread()
        .enable_io()
        .worker_threads(1)
        .thread_name("server-runtime")
        .build()
        .unwrap();

    runtime.block_on(async {
        axum::Server::bind(&address.parse().unwrap())
            .serve(api::app(sender).into_make_service())
            .await
            .unwrap();
    });
}
