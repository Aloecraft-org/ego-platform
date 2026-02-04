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

// --- Sleep ---

/// Asynchronously sleep for the specified duration.
///
/// Uses the appropriate sleep implementation for the current platform:
/// - **Native/WASI**: `tokio::time::sleep`
/// - **Browser**: `gloo_timers::future::sleep`
///
/// # Examples
///
/// ```no_run
/// use aloeplatform::sleep;
/// use std::time::Duration;
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
        gloo_timers::future::sleep(duration).await;
    }
}

// --- Interval ---

/// A periodic timer that ticks at a fixed interval.
///
/// Uses the appropriate interval implementation for the current platform:
/// - **Native/WASI**: `tokio::time::Interval`
/// - **Browser**: Manual sleep-based implementation
pub struct Interval {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    duration: Duration,
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    inner: tokio::time::Interval,
}

impl Interval {
    /// Create a new interval with the specified duration.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use aloeplatform::Interval;
    /// use std::time::Duration;
    ///
    /// # async {
    /// let mut interval = Interval::new(Duration::from_secs(1));
    /// interval.tick().await; // First tick completes immediately
    /// interval.tick().await; // Subsequent ticks wait for the interval
    /// # };
    /// ```
    pub fn new(duration: Duration) -> Self {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            Self {
                inner: tokio::time::interval(duration),
            }
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            Self { duration }
        }
    }

    /// Wait for the next tick of the interval.
    ///
    /// The first tick completes immediately. Subsequent ticks complete
    /// after the interval duration has elapsed.
    pub async fn tick(&mut self) {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            self.inner.tick().await;
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            gloo_timers::future::sleep(self.duration).await;
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