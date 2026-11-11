//! Explicit, thread-local captures of synchronous work (elapsed wall time, not thread CPU time). No logging, global
//! locks, retained events, business keys, or background sampling. Disabled unless
//! `perf-stats` is enabled. Nested elapsed time is excluded from parent self time.
use std::{cell::RefCell, collections::HashMap, marker::PhantomData, rc::Rc, time::Instant};

const MAX_STAGES: usize = 128;
const MAX_DEPTH: usize = 256;

/// Aggregated timings for a static stage name, in nanoseconds.
#[derive(Clone, Debug, Default)]
pub struct Stage {
    name: &'static str,
    calls: u64,
    inclusive_ns: u128,
    self_ns: u128,
    max_ns: u128,
}
impl Stage {
    /// Static label; never a row key or user value.
    pub fn name(&self) -> &'static str {
        self.name
    }
    /// Completed invocations, including early returns.
    pub fn calls(&self) -> u64 {
        self.calls
    }
    /// Elapsed time including nested instrumented stages.
    pub fn inclusive_ns(&self) -> u128 {
        self.inclusive_ns
    }
    /// Elapsed time excluding nested instrumented stages.
    pub fn self_ns(&self) -> u128 {
        self.self_ns
    }
    /// Longest single invocation, including children.
    pub fn max_ns(&self) -> u128 {
        self.max_ns
    }
}

/// One completed capture. Only self times may be summed across stages.
#[derive(Debug)]
pub struct Snapshot {
    elapsed_ns: u128,
    stages: Vec<Stage>,
    dropped: u64,
}
impl Snapshot {
    /// Capture wall time; excludes snapshot construction.
    pub fn elapsed_ns(&self) -> u128 {
        self.elapsed_ns
    }
    /// Aggregates sorted by name, not a timeline or call tree.
    pub fn stages(&self) -> &[Stage] {
        &self.stages
    }
    /// Scopes omitted because the bounded stage/depth budget was exceeded.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }
    /// Capture time outside completed instrumented scopes, including overhead.
    pub fn unattributed_ns(&self) -> u128 {
        self.elapsed_ns
            .saturating_sub(self.stages.iter().map(|s| s.self_ns).sum())
    }
}

struct Active {
    start: Instant,
    children: u128,
    stage: usize,
    token: u64,
}
#[derive(Default)]
struct Recorder {
    generation: u64,
    enabled: bool,
    invalid: bool,
    sequence: u64,
    lookup: HashMap<&'static str, usize>,
    stages: Vec<Stage>,
    stack: Vec<Active>,
    dropped: u64,
}
thread_local! { static RECORDER: RefCell<Recorder> = RefCell::default(); }

/// A same-thread capture. Nested captures are rejected. Dropping cancels it.
pub struct Capture {
    generation: u64,
    start: Instant,
    _thread: PhantomData<Rc<()>>,
}
impl Capture {
    /// Starts an explicit capture. Does not enable unrelated frame profilers.
    pub fn start() -> Result<Self, &'static str> {
        RECORDER.with_borrow_mut(|r| {
            if r.enabled {
                return Err("a CPU capture is already active on this thread");
            }
            r.generation = r.generation.wrapping_add(1);
            r.lookup = HashMap::with_capacity(MAX_STAGES);
            r.stages = Vec::with_capacity(MAX_STAGES);
            r.stack = Vec::with_capacity(MAX_DEPTH);
            r.dropped = 0;
            r.invalid = false;
            r.sequence = 0;
            r.enabled = true;
            Ok(Self {
                generation: r.generation,
                start: Instant::now(),
                _thread: PhantomData,
            })
        })
    }
    /// Finishes with no open scopes. An error cancels this capture.
    pub fn finish(self) -> Result<Snapshot, &'static str> {
        let elapsed_ns = self.start.elapsed().as_nanos();
        RECORDER.with_borrow_mut(|r| {
            if r.generation != self.generation || r.invalid {
                return Err("CPU scopes ended out of stack order");
            }
            if !r.stack.is_empty() {
                return Err("CPU capture has unfinished scopes");
            }
            r.enabled = false;
            let mut stages = std::mem::take(&mut r.stages);
            stages.sort_by_key(|s| s.name);
            Ok(Snapshot {
                elapsed_ns,
                stages,
                dropped: r.dropped,
            })
        })
    }
}
impl Drop for Capture {
    fn drop(&mut self) {
        RECORDER.with_borrow_mut(|r| {
            if r.generation == self.generation {
                r.enabled = false;
                r.stack.clear();
            }
        });
    }
}

/// Same-thread RAII scope; end in stack order and never keep over an await/yield.
/// Misordered drops invalidate the capture without panicking during unwinding.
pub struct Scope {
    generation: Option<(u64, u64)>,
    _thread: PhantomData<Rc<()>>,
}
/// Instruments one synchronous stage. Inactive captures do not read the clock.
#[must_use]
pub fn scope(name: &'static str) -> Scope {
    let generation = RECORDER.with_borrow_mut(|r| {
        if !r.enabled || r.invalid {
            return None;
        }
        if r.stack.len() == MAX_DEPTH
            || (!r.lookup.contains_key(name) && r.stages.len() == MAX_STAGES)
        {
            r.dropped += 1;
            return None;
        }
        let stage = *r.lookup.entry(name).or_insert_with(|| {
            let index = r.stages.len();
            r.stages.push(Stage {
                name,
                ..Default::default()
            });
            index
        });
        r.sequence += 1;
        r.stack.push(Active {
            start: Instant::now(),
            children: 0,
            stage,
            token: r.sequence,
        });
        Some((r.generation, r.sequence))
    });
    Scope {
        generation,
        _thread: PhantomData,
    }
}
impl Drop for Scope {
    fn drop(&mut self) {
        let Some((generation, token)) = self.generation else {
            return;
        };
        RECORDER.with_borrow_mut(|r| {
            if !r.enabled || r.generation != generation {
                return;
            }
            if r.invalid {
                return;
            }
            if r.stack.last().is_none_or(|active| active.token != token) {
                r.invalid = true;
                r.stack.clear();
                return;
            }
            let active = r.stack.pop().unwrap();
            let elapsed = active.start.elapsed().as_nanos();
            let stage = &mut r.stages[active.stage];
            stage.calls += 1;
            stage.inclusive_ns += elapsed;
            stage.self_ns += elapsed.saturating_sub(active.children);
            stage.max_ns = stage.max_ns.max(elapsed);
            if let Some(parent) = r.stack.last_mut() {
                parent.children += elapsed;
            }
        });
    }
}
