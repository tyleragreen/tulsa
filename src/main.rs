pub mod transit {
    include!(concat!(env!("OUT_DIR"), "/transit_realtime.rs"));
}

fn main() {
    let _feed = transit::FeedMessage::default();
    println!("Done!");
}
