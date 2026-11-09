//! Per-gallery CPU/queue timings. Enabled before startup with
//! YUE_IMAGE_PREVIEW_TIMING=1; disabled runs allocate no metrics or timestamps.
use std::{
    collections::BTreeMap,
    fmt,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

pub(super) type Clock = Option<Instant>;

#[derive(Clone, Default)]
pub(super) struct Profile(Option<Arc<Session>>);
struct Session {
    generation: u64,
    start: Instant,
    stages: Mutex<BTreeMap<&'static str, Stats>>,
}
#[derive(Default)]
struct Stats {
    total: Duration,
    count: usize,
    samples: Vec<Duration>,
    slowest: String,
    maximum: Duration,
}
impl Profile {
    pub fn new(generation: u64) -> Self {
        static ENABLED: OnceLock<bool> = OnceLock::new();
        if !*ENABLED.get_or_init(|| {
            std::env::var("YUE_IMAGE_PREVIEW_TIMING")
                .is_ok_and(|value| matches!(value.as_str(), "1" | "true"))
        }) {
            return Self::default();
        }
        Self(Some(Arc::new(Session {
            generation,
            start: Instant::now(),
            stages: Mutex::new(BTreeMap::new()),
        })))
    }
    pub fn clock(&self) -> Clock {
        self.0.as_ref().map(|_| Instant::now())
    }
    /// Timestamp on the worker, before the UI task resumes. Taking this after
    /// `.await` would omit scheduler latency from the publication measurement.
    pub fn handoff<T>(&self, work: impl FnOnce() -> T) -> (T, Clock) {
        let result = work();
        (result, self.clock())
    }
    pub fn run<T>(&self, stage: &'static str, name: &str, work: impl FnOnce() -> T) -> T {
        let start = self.clock();
        let result = work();
        self.record(stage, name, start);
        result
    }
    pub fn record(&self, stage: &'static str, name: &str, start: Clock) {
        let (Some(session), Some(start)) = (&self.0, start) else {
            return;
        };
        let elapsed = start.elapsed();
        if let Ok(mut stages) = session.stages.lock() {
            let stats = stages.entry(stage).or_default();
            stats.total += elapsed;
            // Bound diagnostics during long navigation sessions. P95 uses the
            // most recent 2048 samples; totals/average/max span the whole session.
            if stats.samples.len() < 2048 {
                stats.samples.push(elapsed);
            } else {
                stats.samples[stats.count % 2048] = elapsed;
            }
            stats.count += 1;
            if elapsed >= stats.maximum {
                stats.maximum = elapsed;
                stats.slowest = name.to_owned();
            }
        }
        eprintln!(
            "[ImagePreview timing] layer=host g={} thread={:?} stage={stage} name={name:?} elapsed_ms={:.3}",
            session.generation,
            std::thread::current().id(),
            elapsed.as_secs_f64() * 1000.
        );
    }
    pub fn note(&self, detail: impl fmt::Display) {
        if let Some(session) = &self.0 {
            eprintln!(
                "[ImagePreview timing] layer=host g={} {detail}",
                session.generation
            );
        }
    }
    /// Stage sums are worker time, not wall time; queues and nested phases overlap.
    pub fn summary(&self, reason: &str) {
        let Some(session) = &self.0 else {
            return;
        };
        self.note(format_args!(
            "summary={reason} wall_ms={:.3}",
            session.start.elapsed().as_secs_f64() * 1000.
        ));
        if let Ok(stages) = session.stages.lock() {
            for (stage, stats) in stages.iter() {
                let mut samples = stats.samples.clone();
                samples.sort_unstable();
                let p95 = samples[(samples.len() * 95).div_ceil(100) - 1];
                self.note(format_args!("summary_stage={stage} count={} sum_ms={:.3} avg_ms={:.3} p95_ms={:.3} p95_samples={} max_ms={:.3} slowest={:?}",
                    stats.count, stats.total.as_secs_f64() * 1000., stats.total.as_secs_f64() * 1000. / stats.count as f64,
                    p95.as_secs_f64() * 1000., samples.len(), stats.maximum.as_secs_f64() * 1000., stats.slowest));
            }
        }
    }
}

pub(super) fn prepare(
    profile: &Profile,
    bytes: &[u8],
    limits: &yue_component::ImagePreviewLimits,
    name: &str,
) -> Result<yue_component::ImagePreviewImage, yue_component::ImagePreviewError> {
    let pixels = profile.run("source_decode", name, || {
        yue_component::ImagePreviewPixels::decode(bytes, limits)
    })?;
    profile.run("source_resize", name, || {
        pixels.into_preview(limits, name.to_owned())
    })
}
