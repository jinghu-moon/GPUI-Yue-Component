//! Bounded convergence shared by one-dimensional and masonry positioning.
use super::{VirtualizerMotion, VirtualizerScrollOptions};
use std::time::{Duration, Instant};

/// Sample once per frame time, even when measurement re-resolves geometry in
/// that frame. Callers keep the native owner as the only applied position.
pub(super) fn sample_offset(
    options: VirtualizerScrollOptions,
    elapsed: Duration,
    from: f64,
    destination: f64,
    current: f64,
    extent: f64,
) -> (f64, bool) {
    if let VirtualizerMotion::Smooth {
        duration,
        max_distance,
    } = options.motion
        && !options.reduced_motion
        && (destination - current).abs() > 0.01
        && (destination - from).abs() <= f64::from(max_distance).min(extent)
    {
        let t = (elapsed.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        (from + (destination - from) * eased, t >= 1.0)
    } else {
        (destination, true)
    }
}

#[derive(Default)]
pub(super) struct CommandProgress {
    started: Option<Instant>,
    attempts: u8,
    last: Option<f64>,
    stalled: u8,
}
impl CommandProgress {
    pub fn advance(&mut self, now: Instant) -> bool {
        self.advance_with_motion(now, false)
    }
    /// Timed animation samples are not failed geometry retries. The wall-clock
    /// deadline still bounds them, independently of the display refresh rate.
    pub fn advance_with_motion(&mut self, now: Instant, animating: bool) -> bool {
        let started = *self.started.get_or_insert(now);
        self.attempts = self.attempts.saturating_add(u8::from(!animating));
        self.attempts > 32 || now.saturating_duration_since(started) > Duration::from_secs(2)
    }
    pub fn started(&self) -> Option<Instant> {
        self.started
    }
    pub fn confirm(&mut self, applied: f64, settled: bool) -> bool {
        if !settled && self.last.is_some_and(|old| (old - applied).abs() < 0.01) {
            self.stalled = self.stalled.saturating_add(1);
        } else {
            self.stalled = 0;
        }
        self.last = Some(applied);
        self.stalled >= 8
    }
}
