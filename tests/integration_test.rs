#[cfg(test)]
mod tests {
    use gtfs_realtime_rust::add;
    use gtfs_realtime_rust::scheduler;
    use std::sync::mpsc;

    #[test]
    fn it_works() {
        assert_eq!(add(2, 2), 4);
    }

    #[test]
    fn integration() {
        let (_, receiver) = mpsc::channel();
        scheduler::init(receiver);
    }
}
