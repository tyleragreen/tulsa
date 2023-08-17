pub mod api;
pub mod fetcher;
pub mod model;
pub mod scheduler;

// The deps module is an effort to re-implement my third-party dependencies as
// a learning exercise. I do not plan to make this code public and my
// implementations will definitely be similar to the original since I am not
// yet fluent in Rust.
pub mod deps {
    // mime = "0.3.17" -> https://docs.rs/mime/0.3.17/mime/
    pub mod mime;
    // mockito = "1.1.0" -> https://docs.rs/mockito/1.1.0/mockito/
    pub mod mockito;
}
