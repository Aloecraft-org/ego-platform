//! Platform-specific task spawning.

use std::future::Future;

/// Spawn a future onto the appropriate runtime for the current platform.
///
/// - **Browser**: Uses `wasm_bindgen_futures::spawn_local` (no Send required)
/// - **Native/WASI**: Uses `tokio::spawn` (requires Send)
///
/// # Examples
///
/// ```no_run
/// use aloeplatform::spawn;
///
/// spawn(async {
///     println!("Running in background");
/// });
/// ```
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

/// Spawn a future onto the appropriate runtime for the current platform.
///
/// - **Browser**: Uses `wasm_bindgen_futures::spawn_local` (no Send required)
/// - **Native/WASI**: Uses `tokio::spawn` (requires Send)
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    #[tokio::test]
    async fn test_spawn_executes() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        spawn(async move {
            flag_clone.store(true, Ordering::SeqCst);
        });

        // Give the spawned task time to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        assert!(flag.load(Ordering::SeqCst));
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    #[tokio::test]
    async fn test_spawn_multiple_tasks() {
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        for _ in 0..10 {
            let counter_clone = counter.clone();
            spawn(async move {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });
        }

        // Give tasks time to execute
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_spawn_browser() {
        let flag = Arc::new(AtomicBool::new(false));
        let flag_clone = flag.clone();

        spawn(async move {
            flag_clone.store(true, Ordering::SeqCst);
        });

        // Give the spawned task time to execute
        gloo_timers::future::sleep(std::time::Duration::from_millis(50)).await;

        assert!(flag.load(Ordering::SeqCst));
    }
}