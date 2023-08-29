#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::fs::OpenOptions;
    use std::io::prelude::*;
    use std::process::Command;
    use std::sync::{mpsc, Mutex};
    use std::thread;
    use std::time::Duration;

    use tulsa::scheduler;
    use tulsa::model::{SyncTask, AsyncTask};

    fn confirm_wc(file_path: &str, expected: i32) {
        let output = Command::new("wc")
            .arg(file_path)
            .output()
            .expect("failed to execute process");

        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let output_trimmed = output_str.trim();
            let output_vec: Vec<&str> = output_trimmed.split(" ").collect();
            let output_num: i32 = output_vec[0].parse().unwrap();
            assert_eq!(output_num, expected);
        } else {
            assert!(false);
        }
    }

    fn touch(file_name: &str) {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(file_name)
            .unwrap();
        file.write_all(b"").unwrap();
    }

    fn create_async_task(id: usize, file_name: &'static str, millis: u64) -> AsyncTask {
        AsyncTask::new(id, async move {
            let duration = Duration::from_millis(millis);
            let mut interval = tokio::time::interval(duration);
            let mut file = File::create(file_name).unwrap();

            // Run to a larger number than we expect to reach during this test
            for i in 0..1000 {
                interval.tick().await;
                file.write_all(i.to_string().as_bytes()).unwrap();
                file.write_all(b"\n").unwrap();
            }
        })
    }

    #[test]
    fn async_scheduler_create() {
        let (sender, receiver) = mpsc::channel();
        scheduler::init_async(receiver);

        static FILE_NAME: &'static str = "/tmp/tulsa_async_1.txt";

        // Clear the file and ensure it exists
        touch(FILE_NAME);

        let task = create_async_task(1, FILE_NAME, 100);

        // File should still be empty before the send the task to the scheduler
        confirm_wc(FILE_NAME, 0);
        match sender.send(task) {
            Ok(_) => assert!(true),
            Err(e) => {
                eprintln!("{}", e);
                assert!(false)
            },
        }

        // Wait for the task to finish and then confirm the file has the correct contents
        thread::sleep(Duration::from_millis(550));
        confirm_wc(FILE_NAME, 6);
    }

    #[test]
    fn async_scheduler_delete() {
        let (sender, receiver) = mpsc::channel();
        scheduler::init_async(receiver);

        let task_id: usize = 2;
        static FILE_NAME: &'static str = "/tmp/tulsa_async_2.txt";

        // Clear the file and ensure it exists
        touch(FILE_NAME);

        let task = create_async_task(task_id, FILE_NAME, 100);

        // File should still be empty before the send the task to the scheduler
        confirm_wc(FILE_NAME, 0);
        match sender.send(task) {
            Ok(_) => assert!(true),
            Err(e) => {
                eprintln!("{}", e);
                assert!(false)
            },
        }

        // Wait for the task to finish and then confirm the file has the correct contents
        thread::sleep(Duration::from_millis(550));

        confirm_wc(FILE_NAME, 6);
        let task = AsyncTask::stop(task_id);
        match sender.send(task) {
            Ok(_) => assert!(true),
            Err(e) => {
                eprintln!("{}", e);
                assert!(false)
            },
        }

        thread::sleep(Duration::from_millis(550));
        confirm_wc(FILE_NAME, 6);
    }

    fn create_sync_task(id: usize, file_name: &'static str, millis: u64) -> SyncTask {
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(file_name)
            .unwrap();

        let file_mutex = Mutex::new(file);

        let task = SyncTask::new(id, Duration::from_millis(millis), move || {
            let mut file = file_mutex.lock().unwrap();
            file.write_all("i".to_string().as_bytes()).unwrap();
            file.write_all(b"\n").unwrap();
        });

        task
    }

    #[test]
    fn sync_scheduler_create() {
        let (sender, receiver) = mpsc::channel();
        scheduler::init_sync(receiver);

        static FILE_NAME: &'static str = "/tmp/tulsa_sync_1.txt";

        // Clear the file and ensure it exists
        touch(FILE_NAME);

        let task = create_sync_task(1, FILE_NAME, 100);

        // File should still be empty before the send the task to the scheduler
        confirm_wc(FILE_NAME, 0);
        match sender.send(task) {
            Ok(_) => assert!(true),
            Err(e) => {
                eprintln!("{}", e);
                assert!(false)
            },
        }

        // Wait for the task to finish and then confirm the file has the correct contents
        thread::sleep(Duration::from_millis(550));
        confirm_wc(FILE_NAME, 6);
    }

    #[test]
    fn sync_scheduler_delete() {
        let (sender, receiver) = mpsc::channel();
        scheduler::init_sync(receiver);

        let task_id: usize = 2;
        static FILE_NAME: &'static str = "/tmp/tulsa_sync_2.txt";

        // Clear the file and ensure it exists
        touch(FILE_NAME);

        let task = create_sync_task(task_id, FILE_NAME, 100);

        // File should still be empty before the send the task to the scheduler
        confirm_wc(FILE_NAME, 0);
        match sender.send(task) {
            Ok(_) => assert!(true),
            Err(e) => {
                eprintln!("{}", e);
                assert!(false)
            },
        }

        // Wait for the task to finish and then confirm the file has the correct contents
        thread::sleep(Duration::from_millis(550));

        confirm_wc(FILE_NAME, 6);
        let task = SyncTask::stop(task_id);
        match sender.send(task) {
            Ok(_) => assert!(true),
            Err(e) => {
                eprintln!("{}", e);
                assert!(false)
            },
        }

        thread::sleep(Duration::from_millis(550));
        confirm_wc(FILE_NAME, 6);
    }
}
