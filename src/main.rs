use aloeplatform::{detect, init, spawn, sleep, Interval, Platform};
use std::time::Duration;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen::prelude::*;

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[cfg_attr(not(target_arch = "wasm32"), tokio::main(flavor = "multi_thread"))]
#[cfg_attr(target_arch = "wasm32", tokio::main(flavor = "current_thread"))]
async fn main() {
    run().await;
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn main() {
    // Browser: spawn_local requires setup through wasm-bindgen
    wasm_bindgen_futures::spawn_local(run());
}

async fn run() {
    // Initialize the platform
    init();

    // Detect and log the platform
    let platform = detect();
    log::info!("Running on platform: {:?}", platform);

    match platform {
        Platform::Native => {
            log::info!("Native platform detected - full tokio runtime available");
        }
        Platform::Wasi => {
            log::info!("WASI platform detected - tokio runtime with limited features");
        }
        Platform::Browser => {
            log::info!("Browser platform detected - using wasm_bindgen_futures");
        }
    }

    // Example: Spawn a background task
    spawn(async {
        log::info!("Background task started");
        sleep(Duration::from_millis(100)).await;
        log::info!("Background task completed");
    });

    // Example: Use sleep
    log::info!("Sleeping for 200ms...");
    sleep(Duration::from_millis(200)).await;
    log::info!("Sleep completed");

    // Example: Use interval
    log::info!("Starting interval (3 ticks)...");
    let mut interval = Interval::new(Duration::from_millis(100));
    for i in 0..3 {
        interval.tick().await;
        log::info!("Interval tick {}", i + 1);
    }

    log::info!("Example completed successfully!");
}