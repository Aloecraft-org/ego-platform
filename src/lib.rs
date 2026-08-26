//! Platform abstraction layer for Rust applications targeting native, WASI, and browser environments.
//!
//! This library provides a unified API for common operations across different platforms:
//! - **Logging**: Platform-appropriate logging initialization
//! - **Async spawn**: Task spawning with correct bounds for each platform
//! - **Time**: Sleep, intervals, and system time — plus an injectable
//!   [`clock::Clock`] for tests and record/replay
//! - **Sync**: Broadcast channels, on every target including the browser
//!   (tokio's `sync` primitives are runtime-free and wasm-clean)
//! - **Entropy**: The platform CSPRNG and a deterministic stand-in, in the
//!   `rand_core` 0.10 vocabulary ([`entropy`])
//! - **Pacing**: Cooperative slicing for CPU-bound loops ([`pacer`])
//! - **Blobs**: Named binary state with atomic writes ([`blobs`])
//! - **Shutdown**: One awaitable end-of-process signal ([`shutdown`])
//! - **Env**: Environment variables with honest absence ([`env`])
//!
//! # Platform Detection
//!
//! ```
//! use ego_platform::{Platform, detect};
//!
//! match detect() {
//!     Platform::Native => println!("Running on native platform"),
//!     Platform::Wasi => println!("Running on WASI"),
//!     Platform::Browser => println!("Running in browser"),
//! }
//! ```

pub mod blobs;
pub mod clock;
pub mod entropy;
pub mod env;
pub mod fs;
pub mod io;
pub mod logging;
pub mod pacer;
pub mod shutdown;
pub mod spawn;
pub mod sync;
pub mod time;

pub use blobs::{BlobStore, MemStore};
pub use clock::{Clock, ManualClock, SystemClock};
pub use io::stdin;
pub use pacer::{Pacer, yield_now};
pub use spawn::{TaskHandle, spawn};
pub use sync::broadcast;
pub use time::{Instant, Interval, SystemTime, UNIX_EPOCH, sleep, timeout};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use blobs::DirStore;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use blobs::IdbStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Native,
    Wasi,
    Browser,
}

pub fn detect() -> Platform {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    return Platform::Browser;

    #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
    return Platform::Wasi;

    #[cfg(not(target_arch = "wasm32"))]
    return Platform::Native;
}

pub use logging::register_output_hook;

/// Initialize the platform layer (currently: logging). Idempotent.
pub fn init() {
    logging::init();
}

/// Backward-compatibility alias for [`time::sleep`].
pub use time::sleep as wait_for_timeout;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = detect();

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        assert_eq!(platform, Platform::Browser);

        #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
        assert_eq!(platform, Platform::Wasi);

        #[cfg(not(target_arch = "wasm32"))]
        assert_eq!(platform, Platform::Native);
    }

    #[test]
    fn test_platform_debug() {
        let platform = detect();
        let debug_str = format!("{:?}", platform);
        assert!(!debug_str.is_empty());
    }

    #[test]
    fn test_platform_equality() {
        let p1 = detect();
        let p2 = detect();
        assert_eq!(p1, p2);

        assert_eq!(Platform::Native, Platform::Native);
        assert_eq!(Platform::Wasi, Platform::Wasi);
        assert_eq!(Platform::Browser, Platform::Browser);

        assert_ne!(Platform::Native, Platform::Wasi);
        assert_ne!(Platform::Native, Platform::Browser);
        assert_ne!(Platform::Wasi, Platform::Browser);
    }

    #[test]
    fn test_init_does_not_panic() {
        // Init should be idempotent and not panic
        init();
    }
}
