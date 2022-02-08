//! Idle detection for incoming messages.

use crate::settings;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct Idle {
    /// Number of messages seen.
    seen: Arc<AtomicUsize>,
    threshold: settings::Var<u32>,
}

impl Idle {
    /// Construct a new idle detector.
    pub fn new(threshold: settings::Var<u32>) -> Self {
        Idle {
            seen: Arc::new(AtomicUsize::new(0)),
            threshold,
        }
    }

    /// Indicate that a message has been seen.
    pub fn seen(&self) {
        self.seen.fetch_add(1, Ordering::SeqCst);
    }

    /// Test if there is enough messages to not bee considered "idle".
    pub async fn is_idle(&self) -> bool {
        let seen = self.seen.load(Ordering::SeqCst);

        if seen >= self.threshold.load().await as usize {
            self.seen.store(0, Ordering::SeqCst);
            return false;
        }

        true
    }
}
