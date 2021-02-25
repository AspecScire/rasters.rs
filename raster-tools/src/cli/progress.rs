use indicatif::ProgressBar;
use std::{fmt::Display, time::Duration};

/// A wrapper for a progress bar with a displayable
/// information. The value type `T` is typically a counter.
pub struct Progress<T> {
    pub bar: ProgressBar,
    pub value: T,
    done: Flag,
}
impl<T> Progress<T> {
    pub fn new(value: T) -> Self {
        let bar = {
            use indicatif::*;
            let progress = ProgressBar::new_spinner();
            progress.set_style(
                ProgressStyle::default_spinner().template("{spinner} [{elapsed_precise}] {msg}"),
            );
            progress
        };
        Progress {
            bar,
            value,
            done: Default::default(),
        }
    }

    pub fn done(&self) -> bool {
        self.done.load()
    }

    pub fn finish(&self) {
        self.done.store(true);
    }

    /// Converts an `Arc` to a wrapper that automatically
    /// calls `finish` when dropped. The wrapper
    /// dereferences to `Self`.
    pub fn finish_on_drop(self: Arc<Self>) -> FinishOnDrop<T, Arc<Self>> {
        FinishOnDrop(self)
    }
}
impl<T: Display> Progress<T> {
    pub fn update_progress(&self) {
        self.bar.set_message(&format!("{}", self.value));
    }

    /// Auto update progress in the current-thread.
    ///
    /// Blocks the current thread, and updates at the
    /// interval provided. This method only exits when
    /// `finish` is called in another thread.
    pub fn update_until_done(&self, timeout: Duration) {
        use std::thread;
        while !self.done() {
            self.update_progress();
            thread::park_timeout(timeout);
        }
    }
}
impl<T: Default> Default for Progress<T> {
    fn default() -> Self {
        Progress::new(Default::default())
    }
}

use std::ops::Deref;
pub struct FinishOnDrop<T, D: Deref<Target = Progress<T>>>(D);
impl<T, D: Deref<Target = Progress<T>>> Deref for FinishOnDrop<T, D> {
    type Target = Progress<T>;
    fn deref(&self) -> &Progress<T> {
        self.0.deref()
    }
}
impl<T, D: Deref<Target = Progress<T>>> Drop for FinishOnDrop<T, D> {
    fn drop(&mut self) {
        self.0.finish();
    }
}

use std::sync::Arc;
use std::thread::JoinHandle;
impl<T: Send + Sync + Display + 'static> Progress<T> {
    pub fn spawn_auto_update_thread(self: Arc<Self>, timeout: Duration) -> JoinHandle<()> {
        std::thread::spawn(move || self.update_until_done(timeout))
    }
}

use std::sync::atomic::{AtomicBool, Ordering};
#[derive(Debug, Default)]
pub struct Flag {
    val: AtomicBool,
}
impl Flag {
    pub fn load(&self) -> bool {
        self.val.load(Ordering::Acquire)
    }

    pub fn store(&self, val: bool) {
        self.val.store(val, Ordering::Release);
    }
}
