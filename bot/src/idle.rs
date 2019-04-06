//! Idle detection for incoming messages.

use parking_lot::RwLock;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Clone)]
pub struct Idle {
    /// Number of messages seen.
    seen: Arc<AtomicUsize>,
    /// Last time we saw enough messages to not be considered idle.
    last: Arc<AtomicUsize>,
    threshold: Arc<RwLock<u32>>,
}

impl Idle {
    /// Construct a new idle detector.
    pub fn new(threshold: Arc<RwLock<u32>>) -> Self {
        Idle {
            seen: Arc::new(AtomicUsize::new(0)),
            last: Arc::new(AtomicUsize::new(0)),
            threshold,
        }
    }

    /// Indicate that a message has been seen.
    pub fn seen(&self) {
        self.seen.fetch_add(1, Ordering::SeqCst);
    }

    /// Test if there is enough messages to not bee considered "idle".
    pub fn is_idle(&self) -> bool {
        let seen = self.seen.load(Ordering::SeqCst);

        if seen >= *self.threshold.read() as usize {
            self.seen.store(0, Ordering::SeqCst);
            return false;
        }

        true
    }
}
