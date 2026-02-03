// src/platform/mod.rs
pub mod logging;
pub mod spawn;
pub mod sync;
pub mod time;

// Re-export commonly used items at the platform level
pub use spawn::spawn;
pub use sync::broadcast;
pub use time::{sleep, Interval, SystemTime, UNIX_EPOCH};

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

pub fn init() {
    logging::init();
}

// Backward compatibility alias
pub use time::sleep as wait_for_timeout;