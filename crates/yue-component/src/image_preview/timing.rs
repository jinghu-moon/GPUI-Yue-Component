//! Opt-in CPU pipeline diagnostics. Never equate a paint submission with GPU completion.
use gpui_kit::ImageId;
use std::{
    collections::HashMap,
    fmt,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

pub(super) fn enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("YUE_IMAGE_PREVIEW_TIMING")
            .is_ok_and(|value| matches!(value.as_str(), "1" | "true"))
    })
}

pub(super) struct Timing {
    operation: &'static str,
    start: Option<Instant>,
    last: Option<Instant>,
    stages: Vec<(&'static str, Duration)>,
}
impl Timing {
    pub fn new(operation: &'static str) -> Self {
        let start = enabled().then(Instant::now);
        Self {
            operation,
            start,
            last: start,
            stages: Vec::new(),
        }
    }
    /// Repeated stages (PNG rows) accumulate instead of producing per-row logs.
    pub fn mark(&mut self, stage: &'static str) {
        let Some(last) = self.last else {
            return;
        };
        let now = Instant::now();
        let elapsed = now.duration_since(last);
        if let Some((_, total)) = self.stages.iter_mut().find(|(name, _)| *name == stage) {
            *total += elapsed;
        } else {
            self.stages.push((stage, elapsed));
        }
        self.last = Some(now);
    }
    pub fn finish(mut self, detail: impl fmt::Display) {
        if let Some(start) = self.start.take() {
            let phases = self
                .stages
                .iter()
                .map(|(name, elapsed)| format!("{name}_ms={:.3}", elapsed.as_secs_f64() * 1000.))
                .collect::<Vec<_>>()
                .join(" ");
            eprintln!(
                "[ImagePreview timing] layer=component thread={:?} operation={} total_ms={:.3} {phases} {detail}",
                std::thread::current().id(),
                self.operation,
                start.elapsed().as_secs_f64() * 1000.
            );
        }
    }
}
impl Drop for Timing {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            eprintln!(
                "[ImagePreview timing] layer=component thread={:?} operation={} status=incomplete total_ms={:.3}",
                std::thread::current().id(),
                self.operation,
                start.elapsed().as_secs_f64() * 1000.
            );
        }
    }
}

fn resources() -> &'static Mutex<HashMap<ImageId, Instant>> {
    static RESOURCES: OnceLock<Mutex<HashMap<ImageId, Instant>>> = OnceLock::new();
    RESOURCES.get_or_init(Default::default)
}
pub(super) fn resource_ready(id: ImageId) {
    if !enabled() {
        return;
    }
    if let Ok(mut entries) = resources().lock() {
        // Diagnostics must remain bounded even when prepared resources are never painted.
        if entries.len() >= 512
            && let Some(oldest) = entries.iter().min_by_key(|(_, at)| **at).map(|(id, _)| *id)
        {
            entries.remove(&oldest);
        }
        entries.insert(id, Instant::now());
    }
}
pub(super) fn first_paint(id: ImageId) -> Option<Instant> {
    if !enabled() {
        return None;
    }
    resources().lock().ok()?.remove(&id)
}
