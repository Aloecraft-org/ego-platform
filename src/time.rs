//! Platform-specific time utilities.

use std::time::Duration;

// --- SystemTime ---

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use web_time::{Instant, SystemTime, UNIX_EPOCH};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// Asynchronously sleep for the specified duration.
///
/// Uses the appropriate timer implementation for the current platform:
/// - **Native/WASI**: `tokio::time::sleep`
/// - **Browser**: `gloo_timers::future::sleep`
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
/// use ego_platform::sleep;
///
/// # async {
/// sleep(Duration::from_secs(1)).await;
/// # };
/// ```
pub async fn sleep(duration: Duration) {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        tokio::time::sleep(duration).await;
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        // wrapper implements Send for the !Send gloo future.
        struct SendFuture<F>(F);

        // SAFETY: WASM is single-threaded. We aren't actually sending this across threads.
        unsafe impl<F> Send for SendFuture<F> {}

        impl<F: std::future::Future> std::future::Future for SendFuture<F> {
            type Output = F::Output;
            fn poll(
                self: std::pin::Pin<&mut Self>,
                cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Self::Output> {
                // Project Pin<&mut Wrapper> to Pin<&mut Inner>
                unsafe { self.map_unchecked_mut(|s| &mut s.0).poll(cx) }
            }
        }

        SendFuture(gloo_timers::future::sleep(duration)).await;
    }
}

// --- Interval ---

/// Defines the behavior of an `Interval` when it misses a tick.
///
/// This mirrors `tokio::time::MissedTickBehavior`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissedTickBehavior {
    /// Ticks happen as fast as possible until caught up.
    Burst,
    /// Ticks usually happen after the specified duration, but will "skip"
    /// missed ticks to prevent burstiness, resetting the schedule to the current time.
    Skip,
    /// The interval is effectively restarted. The next tick will happen
    /// `duration` after the current tick completes.
    Delay,
}

/// A periodic timer that ticks at a fixed interval.
///
/// Uses the appropriate interval implementation for the current platform:
/// - **Native/WASI**: `tokio::time::Interval`
/// - **Browser**: Manual sleep-based implementation
pub struct Interval {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    state: BrowserIntervalState,
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    inner: tokio::time::Interval,
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
struct BrowserIntervalState {
    duration: Duration,
    next_tick: Option<Instant>, // None = Not started yet
    behavior: MissedTickBehavior,
}

impl Interval {
    /// Create a new interval with the specified duration.
    pub fn new(duration: Duration) -> Self {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            Self {
                inner: tokio::time::interval(duration),
            }
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            Self {
                state: BrowserIntervalState {
                    duration,
                    next_tick: None,
                    behavior: MissedTickBehavior::Burst, // Default to Burst to match Tokio
                },
            }
        }
    }

    /// Configures the behavior for missed ticks.
    pub fn set_missed_tick_behavior(&mut self, behavior: MissedTickBehavior) {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            let tokio_behavior = match behavior {
                MissedTickBehavior::Burst => tokio::time::MissedTickBehavior::Burst,
                MissedTickBehavior::Skip => tokio::time::MissedTickBehavior::Skip,
                MissedTickBehavior::Delay => tokio::time::MissedTickBehavior::Delay,
            };
            self.inner.set_missed_tick_behavior(tokio_behavior);
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            self.state.behavior = behavior;
        }
    }

    /// Wait for the next tick of the interval.
    pub async fn tick(&mut self) {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            self.inner.tick().await;
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            let now = Instant::now();

            // 1. Initialize logic (First tick fires immediately)
            let next_tick = match self.state.next_tick {
                Some(t) => t,
                None => {
                    self.state.next_tick = Some(now + self.state.duration);
                    return;
                }
            };

            // 2. Schedule logic
            if now < next_tick {
                // We are early (normal case). Wait for the deadline.
                sleep(next_tick - now).await;
                self.state.next_tick = Some(next_tick + self.state.duration);
            } else {
                // We are late (Missed Tick)
                match self.state.behavior {
                    MissedTickBehavior::Burst => {
                        // Catch up mode: Schedule relative to the OLD deadline.
                        // We return immediately to "fire" this delayed tick.
                        self.state.next_tick = Some(next_tick + self.state.duration);
                    }
                    MissedTickBehavior::Skip => {
                        // Skip mode: Abandon old schedule, reset to NOW + Duration.
                        // We return immediately for *this* tick, but the *next* one is pushed back.
                        self.state.next_tick = Some(now + self.state.duration);
                    }
                    MissedTickBehavior::Delay => {
                        // Delay mode: Wait full duration starting NOW.
                        // This introduces drift.
                        sleep(self.state.duration).await;
                        self.state.next_tick = Some(Instant::now() + self.state.duration);
                    }
                }
            }
        }
    }
}

// --- Error Type ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elapsed;

impl std::fmt::Display for Elapsed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "deadline has elapsed")
    }
}

impl std::error::Error for Elapsed {}

// --- Timeout Implementation ---

/// Require a `Future` to complete before the specified duration has elapsed.
///
/// If the future completes before the duration has elapsed, then the completed
/// value is returned. Otherwise, an error is returned and the future is
/// canceled.
pub async fn timeout<F>(duration: Duration, future: F) -> Result<F::Output, Elapsed>
where
    F: Future,
{
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        // Native/WASI: Wrapper around Tokio
        match tokio::time::timeout(duration, future).await {
            Ok(val) => Ok(val),
            Err(_) => Err(Elapsed),
        }
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        use futures::future::{Either, select};
        use gloo_timers::future::TimeoutFuture;

        // 1. Create the sleep future (The "Bomb")
        let delay = TimeoutFuture::new(duration.as_millis() as u32);

        // 2. Pin them both (Select requires pinning)
        futures::pin_mut!(future);
        futures::pin_mut!(delay);

        // 3. Race them!
        match select(future, delay).await {
            Either::Left((val, _)) => Ok(val), // Future finished first
            Either::Right(_) => Err(Elapsed),  // Timer finished first
        }
    }
}
