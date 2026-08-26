//! An injectable clock, beside the free functions in [`crate::time`].
//!
//! The free functions answer "what time is it" with the platform's truth,
//! which is right for most code and wrong for two kinds: tests that must not
//! sleep, and record/replay systems where time is an *input* — a replayed
//! run has to see the recorded clock, not the wall. Both need the clock to
//! be a value that can be handed in, so this module makes it one:
//!
//! - [`SystemClock`] — the platform clock, delegating to [`crate::time`].
//!   Zero-sized; the default.
//! - [`ManualClock`] — a clock that moves only when told. Sleeps against it
//!   park until [`ManualClock::advance`] reaches their deadline, so a test
//!   drives an hour of timeouts in a microsecond, deterministically.
//!
//! Code that should be virtualizable takes `&dyn Clock` (or a generic) and
//! calls the clock instead of the free functions. Code that has no reason to
//! be virtualized keeps calling [`crate::time::sleep`] and friends — the
//! free functions are not deprecated, they are the [`SystemClock`]'s own
//! implementation.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use crate::time::{Instant, SystemTime, UNIX_EPOCH};

/// The future a [`Clock::sleep`] returns.
///
/// Boxed so the trait stays object-safe; `Send` holds on every target (the
/// browser's timer future is single-thread-`Send` by the same argument
/// [`crate::time::sleep`] already relies on).
pub type SleepFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

/// A source of time that can be handed to the code that consumes it.
pub trait Clock: Send + Sync {
    /// Time elapsed since some fixed origin of this clock. Monotonic:
    /// successive calls never decrease. Origins differ between clocks, so
    /// only differences of two readings from the *same* clock mean anything.
    fn monotonic(&self) -> Duration;

    /// The wall-clock time this clock believes in.
    fn wall(&self) -> SystemTime;

    /// Sleep this clock's `duration`. On a [`ManualClock`] that is "until
    /// `advance` has moved the clock far enough", however long or little
    /// real time that takes.
    fn sleep(&self, duration: Duration) -> SleepFuture;
}

// --- SystemClock ---

/// The platform's own clock. What you get when nobody is virtualizing time.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

/// One origin per process, taken lazily on first use, so `monotonic` is a
/// plain `Duration` and two `SystemClock` values agree with each other.
fn system_origin() -> Instant {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    *ORIGIN.get_or_init(Instant::now)
}

impl Clock for SystemClock {
    fn monotonic(&self) -> Duration {
        system_origin().elapsed()
    }

    fn wall(&self) -> SystemTime {
        SystemTime::now()
    }

    fn sleep(&self, duration: Duration) -> SleepFuture {
        Box::pin(crate::time::sleep(duration))
    }
}

// --- ManualClock ---

/// A clock that moves only when told, for tests and replay.
///
/// Clones share the same time: hand one clone to the code under test, keep
/// one to [`advance`](ManualClock::advance). Wall time starts at
/// [`UNIX_EPOCH`] (fully deterministic) unless
/// [`set_wall`](ManualClock::set_wall) says otherwise, and advances with the
/// clock.
///
/// ```
/// use ego_platform::clock::{Clock, ManualClock};
/// use std::time::Duration;
///
/// let clock = ManualClock::new();
/// let sleeper = clock.clone();
/// let fut = sleeper.sleep(Duration::from_secs(3600));
/// clock.advance(Duration::from_secs(3600)); // the hour passes now
/// # let _ = fut;
/// ```
#[derive(Debug, Clone, Default)]
pub struct ManualClock {
    state: Arc<Mutex<ManualState>>,
}

#[derive(Debug)]
struct ManualState {
    now: Duration,
    wall_base: SystemTime,
    /// Sleepers awaiting a deadline. Repolls may register a waker more than
    /// once; `advance` drains everything due, and a spurious wake of a
    /// not-yet-due sleeper is allowed by the `Future` contract (it re-checks
    /// and re-registers).
    sleepers: Vec<(Duration, Waker)>,
}

impl Default for ManualState {
    fn default() -> Self {
        ManualState {
            now: Duration::ZERO,
            wall_base: UNIX_EPOCH,
            sleepers: Vec::new(),
        }
    }
}

impl ManualClock {
    /// A clock at zero, whose wall time is [`UNIX_EPOCH`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Move time forward, waking every sleep whose deadline is reached.
    pub fn advance(&self, by: Duration) {
        let woken: Vec<Waker> = {
            let mut s = self.state.lock().expect("manual clock poisoned");
            s.now += by;
            let now = s.now;
            let mut due = Vec::new();
            s.sleepers.retain(|(deadline, waker)| {
                if *deadline <= now {
                    due.push(waker.clone());
                    false
                } else {
                    true
                }
            });
            due
        };
        // Outside the lock: a woken task may immediately re-enter the clock.
        for w in woken {
            w.wake();
        }
    }

    /// Set the wall-clock reading for the clock's *current* moment; wall
    /// time keeps advancing with the clock from here.
    pub fn set_wall(&self, wall: SystemTime) {
        let mut s = self.state.lock().expect("manual clock poisoned");
        let now = s.now;
        // Store the base such that base + now == wall.
        s.wall_base = wall.checked_sub(now).unwrap_or(UNIX_EPOCH);
    }

    /// How many sleeps are currently parked on this clock. For tests that
    /// want to assert "the code under test is now waiting" before advancing.
    pub fn sleepers(&self) -> usize {
        self.state
            .lock()
            .expect("manual clock poisoned")
            .sleepers
            .len()
    }
}

impl Clock for ManualClock {
    fn monotonic(&self) -> Duration {
        self.state.lock().expect("manual clock poisoned").now
    }

    fn wall(&self) -> SystemTime {
        let s = self.state.lock().expect("manual clock poisoned");
        s.wall_base + s.now
    }

    fn sleep(&self, duration: Duration) -> SleepFuture {
        let state = Arc::clone(&self.state);
        let deadline = {
            let s = state.lock().expect("manual clock poisoned");
            s.now + duration
        };
        Box::pin(ManualSleep { state, deadline })
    }
}

struct ManualSleep {
    state: Arc<Mutex<ManualState>>,
    deadline: Duration,
}

impl Future for ManualSleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut s = self.state.lock().expect("manual clock poisoned");
        if s.now >= self.deadline {
            Poll::Ready(())
        } else {
            s.sleepers.push((self.deadline, cx.waker().clone()));
            Poll::Pending
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_clock_is_monotonic() {
        let c = SystemClock;
        let a = c.monotonic();
        let b = c.monotonic();
        assert!(b >= a);
    }

    #[test]
    fn manual_clock_moves_only_when_told() {
        let c = ManualClock::new();
        assert_eq!(c.monotonic(), Duration::ZERO);
        c.advance(Duration::from_secs(5));
        assert_eq!(c.monotonic(), Duration::from_secs(5));
        assert_eq!(
            c.wall().duration_since(UNIX_EPOCH).unwrap(),
            Duration::from_secs(5),
            "wall time advances with the clock"
        );
    }
}
