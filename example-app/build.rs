#[cfg(not(feature = "use_dependencies"))]
include!("src/deps/prost_build/mod.rs");

#[cfg(feature = "use_dependencies")]
extern crate prost_build;

fn main() {
    #[cfg(not(feature = "use_dependencies"))]
    compile_protos(&["src/proto/gtfs-realtime.proto"], &["src/"]).unwrap();

    #[cfg(feature = "use_dependencies")]
    prost_build::compile_protos(&["src/proto/gtfs-realtime.proto"], &["src/"]).unwrap();
}
