// src/platform/time.rs

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

pub struct Interval {
    duration: Duration,
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    inner: tokio::time::Interval,
}

impl Interval {
    pub fn new(duration: Duration) -> Self {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            Self {
                duration,
                inner: tokio::time::interval(duration),
            }
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            Self { duration }
        }
    }

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