# gtfs-realtime-rust

This is a Rust learning exercise to build a REST server that dynamically launches and manages a set of async tasks. These tasks fetch a GTFS-Realtime feed from an endpoint supplied during a POST call.

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
