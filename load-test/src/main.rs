use std::sync::mpsc;
use std::time::Duration;
use std::thread;
use tokio::time::interval;
use tulsa::{AsyncTask, Scheduler, SyncTask};

fn run_async() {
    let (sender, receiver) = mpsc::channel();
    Scheduler::<AsyncTask>::new(receiver).run();

    for i in 1..10000 {
        let task = AsyncTask::new(i, async {
            let duration = Duration::from_millis(10);
            let mut interval = interval(duration);

            loop {
                interval.tick().await;
            }
        });

        match sender.send(task) {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        };
    }

    thread::sleep(Duration::from_secs(300));
}

fn run_sync() {
    let (sender, receiver) = mpsc::channel();
    Scheduler::<SyncTask>::new(receiver).run();

    for i in 1..10000 {
        let task = SyncTask::new(i, Duration::from_millis(10), || {});

        match sender.send(task) {
            Ok(_) => {},
            Err(e) => eprintln!("{}", e),
        };
    }

    thread::sleep(Duration::from_secs(300));
}

fn main() {
    run_sync();
    run_async();
}
