//! Platform-specific time utilities.

use std::time::Duration;

// --- SystemTime ---

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod system_time {
    pub use instant::SystemTime;
    pub const UNIX_EPOCH: SystemTime = SystemTime::UNIX_EPOCH;
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
mod system_time {
    pub use std::time::{SystemTime, UNIX_EPOCH};
}

pub use system_time::*;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use instant::Instant;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use std::time::Instant;

/// Asynchronously  for the specified duration.
///
/// Uses the appropriate  implementation for the current platform:
/// - **Native/WASI**: `tokio::time::`
/// - **Browser**: `gloo_timers::future::`
///
/// # Examples
///
/// ```no_run
/// use std::time::Duration;
///
/// # async {
/// (Duration::from_secs(1));
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
    next_tick: Option<instant::Instant>, // None = Not started yet
    behavior: MissedTickBehavior,
}

// SAFETY: On Native, we wrap tokio::time::Interval which is thread-safe.
// On Browser, we use a Duration which is a primitive. We manually implement
// Send to satisfy the Service harness's global requirements.
unsafe impl Send for Interval {}
unsafe impl Sync for Interval {}

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
            let now = instant::Instant::now();

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
                        self.state.next_tick = Some(instant::Instant::now() + self.state.duration);
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
        use futures::future::{select, Either};
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








#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    #[tokio::test]
    async fn test_sleep() {
        let start = std::time::Instant::now();
        sleep(Duration::from_millis(100)).await;
        let elapsed = start.elapsed();

        // Should sleep for at least 100ms (with some tolerance)
        assert!(elapsed >= Duration::from_millis(90));
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_sleep_browser() {
        let start = instant::Instant::now();
        sleep(Duration::from_millis(100)).await;
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(90));
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    #[tokio::test]
    async fn test_interval() {
        let mut interval = Interval::new(Duration::from_millis(50));

        // First tick should complete immediately
        let start = std::time::Instant::now();
        interval.tick().await;
        let first_tick = start.elapsed();
        assert!(first_tick < Duration::from_millis(10));

        // Second tick should wait
        let start = std::time::Instant::now();
        interval.tick().await;
        let second_tick = start.elapsed();
        assert!(second_tick >= Duration::from_millis(40));
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_interval_browser() {
        let mut interval = Interval::new(Duration::from_millis(50));

        // In browser implementation, first tick also waits
        interval.tick().await;

        let start = instant::Instant::now();
        interval.tick().await;
        let elapsed = start.elapsed();

        assert!(elapsed >= Duration::from_millis(40));
    }

    #[test]
    fn test_system_time() {
        let now = SystemTime::now();
        let duration_since_epoch = now.duration_since(UNIX_EPOCH);

        // Should be successful (we're past the epoch)
        assert!(duration_since_epoch.is_ok());

        // Should be a reasonable time (after year 2020)
        let duration = duration_since_epoch.unwrap();
        assert!(duration.as_secs() > 1_600_000_000);
    }

    #[test]
    fn test_system_time_ordering() {
        let t1 = SystemTime::now();
        std::thread::sleep(Duration::from_millis(10));
        let t2 = SystemTime::now();

        // t2 should be after t1
        assert!(t2 > t1);
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    #[tokio::test]
    async fn test_multiple_intervals() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        tokio::spawn(async move {
            let mut interval = Interval::new(Duration::from_millis(25));
            for _ in 0..3 {
                interval.tick().await;
                counter_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }
}
