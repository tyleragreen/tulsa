use std::thread;

use crate::QUEUE;

pub fn init() {
    thread::spawn(|| {
        println!("Hello from the scheduler!");

        loop {
            let item = QUEUE.lock().unwrap().pop_back();

            if let Some(feed) = item {
                dbg!(feed);
            }
        }
    });
}
