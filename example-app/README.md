# gtfs-realtime-rust

This is a Rust learning exercise to build a REST server that dynamically launches and manages a set of async tasks. These tasks fetch a GTFS-Realtime feed from an endpoint supplied during a POST call. The tasks are scheduled using the `tulsa` scheduler from this repo.

## Example
Some tokio details are not shown here, but these are the initialization steps.
```
use gtfs_realtime_rust::api;
use gtfs_realtime_rust::scheduler::{build, Mode};

# using coroutine scheduling
let interface = build(Mode::Async);
axum::Server::bind(&address)
    .serve(api::app(interface).into_make_service())
    .await
    .unwrap();

# using thread scheduling
let interface = build(Mode::Sync);
axum::Server::bind(&address)
    .serve(api::app(interface).into_make_service())
    .await
    .unwrap();
```

## Running Locally
```
cargo test
cargo run
```

## Sample Feed
```
{
    "name": "MTA A Division",
    "frequency": 30,
    "url": "https://api-endint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs",
    "headers": {
        "x-api-key": "<key_goes_here>"
    }
}
```
