use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Default)]
pub struct Counter {
    val: AtomicUsize,
}
impl Counter {
    pub fn load(&self) -> usize {
        self.val.load(Ordering::Acquire)
    }

    pub fn store(&self, val: usize) {
        self.val.store(val, Ordering::Release);
    }

    pub fn fetch_add(&self, inc: usize) -> usize {
        self.val.fetch_add(inc, Ordering::AcqRel)
    }

    pub fn fetch_sub(&self, inc: usize) -> usize {
        self.val.fetch_sub(inc, Ordering::AcqRel)
    }
}
impl fmt::Display for Counter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.load())
    }
}

#[derive(Debug)]
pub struct DetailCounter {
    pub total: Counter,
    pub processed: Counter,
    pub skipped: Counter,
    name: &'static str,
}
impl DetailCounter {
    pub fn new(name: &'static str) -> Self {
        DetailCounter {
            total: Default::default(),
            processed: Default::default(),
            skipped: Default::default(),
            name,
        }
    }
}
impl fmt::Display for DetailCounter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: completed {}", self.name, self.processed.load())?;
        let skipped = self.skipped.load();
        if skipped > 0 {
            write!(f, " (skipped {})", skipped)?;
        }
        write!(f, " of {}.", self.total.load())
    }
}

// #[derive(Debug)]
// pub struct ChunkCounter {
//     pub chunk: DetailCounter,
//     pub detail: DetailCounter,
// }
// impl ChunkCounter {
//     pub fn new(name: &'static str, detail: &'static str) -> Self {
//         ChunkCounter {
//             chunk: DetailCounter::new(name),
//             detail: DetailCounter::new(detail),
//         }
//     }
// }
// impl fmt::Display for ChunkCounter {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(f, "{}", self.chunk)?;
//         write!(f, " {}", self.detail)?;
//         Ok(())
//     }
// }
