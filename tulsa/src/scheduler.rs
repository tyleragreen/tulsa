use std::{
    sync::{mpsc::Receiver, Arc, Mutex},
    thread::Builder as ThreadBuilder,
};

use crate::{
    async_scheduler::AsyncScheduler,
    model::{AsyncTask, SyncTask},
    thread_scheduler::ThreadScheduler,
};

pub struct Scheduler<T> {
    receiver: Arc<Mutex<Receiver<T>>>,
}

impl<T> Scheduler<T> {
    pub fn new(receiver: Receiver<T>) -> Self {
        let receiver = Arc::new(Mutex::new(receiver));
        Self { receiver }
    }
}

impl Scheduler<AsyncTask> {
    pub fn run(self) {
        ThreadBuilder::new()
            .name("scheduler".to_string())
            .spawn(|| AsyncScheduler::new().listen(self.receiver))
            .expect("Failed to spawn scheduler thread.");
    }
}

impl Scheduler<SyncTask> {
    pub fn run(self) {
        ThreadBuilder::new()
            .name("scheduler".to_string())
            .spawn(|| ThreadScheduler::new().listen(self.receiver))
            .expect("Failed to spawn scheduler thread.");
    }
}
