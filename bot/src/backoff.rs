use std::time::Duration;

/// Global maximum span delay for a retry.
const GLOBAL_MAX: Duration = Duration::from_secs(120);

/// An exponential backoff.
pub struct Exponential {
    initial: Duration,
    attempt: usize,
}

impl Exponential {
    /// Construct a new exponential backoff.
    pub fn new(initial: Duration) -> Self {
        Self {
            initial,
            attempt: 0,
        }
    }

    /// Get the next duration and increment the attempt counter.
    pub fn next(&mut self) -> Duration {
        let mut duration = self.initial;

        if self.attempt <= 4 {
            duration *= 2u32 << usize::min(self.attempt, 4);

            if duration > GLOBAL_MAX {
                duration = GLOBAL_MAX;
            }
        } else {
            duration = GLOBAL_MAX;
        }

        self.attempt += 1;
        duration
    }
}
