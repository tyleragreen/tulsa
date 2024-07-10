# app

This is a Rust learning exercise to build a REST server that dynamically launches and manages a set of async tasks. These tasks fetch a GTFS-Realtime feed from an endpoint supplied during a POST call. The tasks are scheduled using the `tulsa` scheduler from this repo.

## Example
Some `tokio` details are not shown here, but these are the initialization steps.
```rust
use app::api;
use app::scheduler::{build, Mode};

# using coroutine scheduling
let interface = build(Mode::Async);
axum::serve(listener, router).await.unwrap();

# using thread scheduling
let interface = build(Mode::Sync);
axum::serve(listener, router).await.unwrap();
```

## Running Locally
```bash
cargo test
cargo run
```
These commands include a few (and hopefully growing number of) re-implemented library crates. I challenged myself to maintain the same interface and learn how these libraries work. To run with the original dependencies, this works.
```bash
cargo test --features "use_dependencies"
```

## Sample Feed
```json
{
    "name": "MTA A Division",
    "frequency": 30,
    "url": "https://api-endint.mta.info/Dataservice/mtagtfsfeeds/nyct%2Fgtfs",
    "headers": {
        "x-api-key": "<key_goes_here>"
    }
}
```
