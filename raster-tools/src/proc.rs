use crate::cli::*;
use std::sync::Arc;
use std::thread::JoinHandle;

const PROGRESS_UPDATE_MILLIS: u64 = 500;

pub struct Tracker {
    progress: Arc<Progress<DetailCounter>>,
    handle: Option<JoinHandle<()>>,
}

impl Tracker {
    pub fn new(units: &'static str, len: usize) -> Self {
        let progress = Arc::new(Progress::new(DetailCounter::new(units)));
        progress.value.total.store(len);
        let handle = progress
            .clone()
            .spawn_auto_update_thread(std::time::Duration::from_millis(PROGRESS_UPDATE_MILLIS));
        Tracker {
            progress,
            handle: Some(handle),
        }
    }
    pub fn increment(&self) {
        self.progress.value.processed.fetch_add(1);
    }
}
impl Drop for Tracker {
    fn drop(&mut self) {
        self.progress.finish();
        if let Err(_) = self.handle.take().unwrap().join() {
            eprintln!("Warning: progress thread panicked!");
        }
    }
}
