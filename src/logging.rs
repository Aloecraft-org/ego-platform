//! Platform-specific logging initialization.

#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
use std::sync::atomic::AtomicBool;


use std::sync::atomic::{AtomicU64, Ordering};
// Track last output time - logs will update this
pub static LAST_OUTPUT_TIME: AtomicU64 = AtomicU64::new(0);

pub fn notify_output() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    LAST_OUTPUT_TIME.store(now, Ordering::Relaxed);
}

// Track if we're in command mode
#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
static COMMAND_MODE: AtomicBool = AtomicBool::new(false);

#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
pub fn set_command_mode(enabled: bool) {
    // eprintln!("DEBUG LOGGER: Setting command_mode to {}", enabled);
    COMMAND_MODE.store(enabled, Ordering::Relaxed);
    // eprintln!(
    //     "DEBUG LOGGER: Command_mode now = {}",
    //     COMMAND_MODE.load(Ordering::Relaxed)
    // );
}

/// Initialize logging for the current platform.
///
/// - **Browser**: Uses console_log with panic hook
/// - **WASI**: Uses a simple println-based logger
/// - **Native**: Uses env_logger with INFO level
pub fn init() {
    println!("[logging.rs] HELLO FROM INIT!!!");

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        // console_log::init_with_level(log::Level::Debug).expect("Failed to init logger");
        // Gracefully no-op if a logger (e.g. ShellLogger) was already installed
        let _ = console_log::init_with_level(log::Level::Info);
        console_error_panic_hook::set_once();
    }
    #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
    {
        println!("[logging.rs] INIT FOR WASI");


        struct SimpleLogger;

        impl log::Log for SimpleLogger {
            fn enabled(&self, _metadata: &log::Metadata) -> bool {
                true
            }

            fn log(&self, record: &log::Record) {
                let in_command_mode = COMMAND_MODE.load(Ordering::Relaxed);

                // Debug every log call
                // eprintln!(
                //     "DEBUG LOGGER: Attempting to log '{}', command_mode={}",
                //     record.args(),
                //     in_command_mode
                // );

                if in_command_mode {
                    // eprintln!("DEBUG LOGGER: SUPPRESSED (command mode active)");
                    return;
                }

                // Not in command mode - show logs
                println!("[{}] {}", record.level(), record.args());
            }

            fn flush(&self) {}
        }

        static LOGGER: SimpleLogger = SimpleLogger;
        
        if log::log_enabled!(log::Level::Debug) {
            println!("Debug logs are currently enabled!");
        } else {
            println!("Debug logs are NOT currently enabled. Current max level is lower.");
        }

        if log::log_enabled!(log::Level::Info) {
            println!("Info logs are currently enabled!");
        } else {
            println!("Info logs are NOT currently enabled. Current max level is lower.");
        }

        match log::set_logger(&LOGGER) {
            Ok(_) => {
                log::set_max_level(log::LevelFilter::Info);
                eprintln!("✅ SimpleLogger installed successfully");
            }
            Err(e) => {
                eprintln!("❌ FAILED to install SimpleLogger: {:?}", e);
                eprintln!("   Another logger was already installed!");
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        println!("[logging.rs] INIT FOR NATIVE");
        let _ = env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .try_init(); // don't panic if already initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_init() {
        // Should not panic
        init();

        // Test that we can log after initialization
        log::info!("Test log message");
        log::debug!("Debug message");
        log::warn!("Warning message");
    }

    #[test]
    fn test_logging_levels() {
        init();

        // These should all work without panicking
        log::error!("Error level");
        log::warn!("Warn level");
        log::info!("Info level");
        log::debug!("Debug level");
        log::trace!("Trace level");
    }
}
