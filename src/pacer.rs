//! Cooperative pacing for CPU-bound work, so long computations share the
//! thread — above all the browser main thread, where an unyielding loop
//! freezes the page and, under some watchdogs, gets the tab killed.
//!
//! [`spawn`](crate::spawn) and friends make *async* work cooperative; this
//! module is for the other kind — a loop that is genuinely computing.
//! Sprinkle [`Pacer::checkpoint`] through it and the loop runs in slices:
//! each checkpoint is nearly free until the slice budget is spent, then it
//! yields to the event loop once and starts the next slice.
//!
//! ```no_run
//! use ego_platform::pacer::Pacer;
//! use std::time::Duration;
//!
//! # async fn compute() {
//! let mut pacer = Pacer::new(Duration::from_millis(8));
//! for _work_item in 0..1_000_000 {
//!     // ... real work ...
//!     pacer.checkpoint().await; // yields only when the slice is spent
//! }
//! log::debug!("{}", pacer.stats());
//! # }
//! ```
//!
//! Everything here is plain safe library code: no threads, no signals, no
//! unsafe. The pacer cannot *preempt* — a stretch of code that never
//! checkpoints still hogs the thread — so its second job is making exactly
//! that visible: [`PacerStats::worst_slice`] records the longest gap between
//! yields, and a slice that overruns the budget by
//! [`overrun_warning_factor`](Pacer::set_overrun_warning) is reported
//! through [`log::warn!`] with enough context to find the loop that forgot
//! its checkpoint. Starvation debugging is reading one number instead of
//! profiling a frozen tab.

use std::fmt;
use std::time::Duration;

use crate::time::Instant;

/// Yield the thread once, to whatever schedules it.
///
/// - **Native / WASI**: `tokio::task::yield_now`.
/// - **Browser**: a zero-delay timer, which is a real macrotask yield — the
///   event loop runs (rendering, input, other timers) before we resume.
pub async fn yield_now() {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        tokio::task::yield_now().await;
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        crate::time::sleep(Duration::ZERO).await;
    }
}

/// Counters a [`Pacer`] keeps about how its loop actually behaved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PacerStats {
    /// Checkpoints reached.
    pub checkpoints: u64,
    /// Checkpoints that actually yielded.
    pub yields: u64,
    /// The longest slice observed between yields — the number to look at
    /// when something feels frozen. If it dwarfs the budget, some stretch of
    /// the loop is not checkpointing.
    pub worst_slice: Duration,
}

impl fmt::Display for PacerStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "pacer: {} checkpoints, {} yields, worst slice {:?}",
            self.checkpoints, self.yields, self.worst_slice
        )
    }
}

/// Slices a CPU-bound loop by yielding when a time budget is spent.
pub struct Pacer {
    budget: Duration,
    slice_start: Option<Instant>,
    stats: PacerStats,
    overrun_factor: Option<u32>,
}

impl Pacer {
    /// A pacer that yields once roughly every `budget` of computation.
    ///
    /// 4–10ms is a good browser budget (a 60Hz frame is ~16ms and the rest
    /// of the page needs some of it). Overrun warnings default on at 4×;
    /// see [`Pacer::set_overrun_warning`].
    pub fn new(budget: Duration) -> Self {
        Pacer {
            budget,
            slice_start: None,
            stats: PacerStats::default(),
            overrun_factor: Some(4),
        }
    }

    /// Warn (via [`log::warn!`]) whenever a slice reaches `factor` × budget
    /// — the signature of a stretch of code that is not checkpointing.
    /// `None` disables the warning; the stats still record it.
    pub fn set_overrun_warning(&mut self, factor: Option<u32>) {
        self.overrun_factor = factor;
    }

    /// Cheap when the slice has budget left; yields once when it is spent.
    /// Returns whether it yielded.
    pub async fn checkpoint(&mut self) -> bool {
        let now = Instant::now();
        let start = *self.slice_start.get_or_insert(now);
        let elapsed = now.duration_since(start);

        self.stats.checkpoints += 1;
        if elapsed > self.stats.worst_slice {
            self.stats.worst_slice = elapsed;
        }
        if let Some(factor) = self.overrun_factor {
            let limit = self.budget.saturating_mul(factor);
            if !limit.is_zero() && elapsed >= limit {
                log::warn!(
                    "pacer slice ran {elapsed:?} against a {:?} budget \
                     (over {factor}x): some stretch of this loop is not \
                     checkpointing ({})",
                    self.budget,
                    self.stats
                );
            }
        }

        if elapsed >= self.budget {
            yield_now().await;
            self.stats.yields += 1;
            self.slice_start = Some(Instant::now());
            true
        } else {
            false
        }
    }

    /// How long the current slice has been running.
    pub fn slice_elapsed(&self) -> Duration {
        self.slice_start
            .map(|s| s.elapsed())
            .unwrap_or(Duration::ZERO)
    }

    /// What actually happened, for logs and tests.
    pub fn stats(&self) -> PacerStats {
        self.stats
    }
}

// Async behavior is exercised in tests/pacer_tests.rs through the shared
// `async_test` alias, so the same tests run native, under wasmtime, and in
// a headless browser.
