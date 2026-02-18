//! Platform abstraction layer for Rust applications targeting native, WASI, and browser environments.
//!
//! This library provides a unified API for common operations across different platforms:
//! - **Logging**: Platform-appropriate logging initialization
//! - **Async spawn**: Task spawning with correct bounds for each platform
//! - **Time**: Sleep, intervals, and system time
//! - **Sync**: Broadcast channels (native only, stub on WASM)
//!
//! # Platform Detection
//!
//! ```
//! use aloeplatform::{Platform, detect};
//!
//! match detect() {
//!     Platform::Native => println!("Running on native platform"),
//!     Platform::Wasi => println!("Running on WASI"),
//!     Platform::Browser => println!("Running in browser"),
//! }
//! ```

pub mod logging;
pub mod spawn;
pub mod sync;
pub mod time;
pub mod io;
pub mod fs;

pub use spawn::{spawn, TaskHandle};
pub use time::{sleep, Interval, SystemTime, UNIX_EPOCH, Instant};
pub use io::stdin;
pub use sync::broadcast;

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
pub fn init() {
    println!("[aloeplatform lib.rs] init");
    logging::init();
}

// Backward compatibility alias
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
        assert!(debug_str.len() > 0);
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