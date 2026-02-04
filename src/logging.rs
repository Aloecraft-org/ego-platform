//! Platform-specific logging initialization.

/// Initialize logging for the current platform.
///
/// - **Browser**: Uses console_log with panic hook
/// - **WASI**: Uses a simple println-based logger
/// - **Native**: Uses env_logger with INFO level
pub fn init() {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        use log::Level;
        // Try to initialize, but don't panic if already initialized
        let _ = console_log::init_with_level(Level::Info);
        console_error_panic_hook::set_once();
    }

    #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
    {
        struct SimpleLogger;

        impl log::Log for SimpleLogger {
            fn enabled(&self, _metadata: &log::Metadata) -> bool {
                true
            }

            fn log(&self, record: &log::Record) {
                println!("[{}] {}", record.level(), record.args());
            }

            fn flush(&self) {}
        }

        static LOGGER: SimpleLogger = SimpleLogger;
        // Try to set logger, ignore error if already set
        let _ = log::set_logger(&LOGGER)
            .map(|()| log::set_max_level(log::LevelFilter::Info));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        // Try to initialize, but don't panic if already initialized
        let _ = env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .try_init();
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