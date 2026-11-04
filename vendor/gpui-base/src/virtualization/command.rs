//! Bounded convergence shared by one-dimensional and masonry positioning.
use std::time::{Duration, Instant};

#[derive(Default)]
pub(super) struct CommandProgress {
    started: Option<Instant>,
    attempts: usize,
    last: Option<f64>,
    stalled: usize,
}
impl CommandProgress {
    pub fn advance(&mut self, now: Instant) -> bool {
        let started = *self.started.get_or_insert(now);
        self.attempts += 1;
        self.attempts > 32 || now.saturating_duration_since(started) > Duration::from_secs(2)
    }
    pub fn started(&self) -> Option<Instant> {
        self.started
    }
    pub fn confirm(&mut self, applied: f64, settled: bool) -> bool {
        if !settled && self.last.is_some_and(|old| (old - applied).abs() < 0.01) {
            self.stalled += 1;
        } else {
            self.stalled = 0;
        }
        self.last = Some(applied);
        self.stalled >= 8
    }
}
