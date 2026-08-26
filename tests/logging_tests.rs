mod common;
use common::test;

use ego_platform::logging::*;

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
