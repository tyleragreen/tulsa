include!("src/deps/prost_build/mod.rs");

fn main() {
    compile_protos(&["src/gtfs-realtime.proto"], &["src/"]).unwrap();
}
