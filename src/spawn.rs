//! Platform-specific task spawning.

use std::future::Future;
use tokio::sync::oneshot;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use std::pin::Pin;
use std::task::{Context, Poll};

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
pub fn spawn<F, T>(future: F) -> TaskHandle<T>
where
    F: Future<Output = T> + 'static,
    T: Send + 'static,
{
    let is_done = Arc::new(AtomicBool::new(false));
    let is_done_clone = is_done.clone();

    let (tx, rx) = oneshot::channel();
    wasm_bindgen_futures::spawn_local(async move {
        let result = future.await;
        is_done_clone.store(true, Ordering::Release);
        let _ = tx.send(result);
    });
    TaskHandle { rx, is_done }
}

/// Spawn a future onto the appropriate runtime for the current platform.
///
/// - **Browser**: Uses `wasm_bindgen_futures::spawn_local` (no Send required)
/// - **Native/WASI**: Uses `tokio::spawn` (requires Send)
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn spawn<F, T>(future: F) -> TaskHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let is_done = Arc::new(AtomicBool::new(false));
    let is_done_clone = is_done.clone();

    let (tx, rx) = oneshot::channel();
    let inner = tokio::spawn(async move {
        let result = future.await;
        is_done_clone.store(true, Ordering::Release);
        let _ = tx.send(result);
    });
    TaskHandle { rx, inner, is_done }
}


pub struct TaskHandle<T> {
    rx: oneshot::Receiver<T>,
    // On native, we keep the actual JoinHandle to allow forceful aborts
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    inner: tokio::task::JoinHandle<()>,

    // Valid on all platforms
    is_done: Arc<AtomicBool>,
}

impl<T> TaskHandle<T> {
    /// Await the completion of the task and return the result.
    pub async fn join(self) -> Result<T, oneshot::error::RecvError> {
        self.rx.await
    }

    /// Forcefully terminate the task (Native only).
    pub fn abort(&self) {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        self.inner.abort();
        // Note: Browser/spawn_local does not support native aborting.
    }

    /// Returns `true` if the task has finished.
    ///
    /// # Platform Behavior
    /// - **Native**: Returns the accurate state from the Tokio JoinHandle.
    /// - **WASM**: Always returns `false` (limitation of `spawn_local`).
    pub fn is_finished(&self) -> bool {
        // CASE A: Native & WASI (Use the real handle)
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            return self.inner.is_finished();
        }

        // CASE B: Browser / Unknown OS (Use the manual flag)
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            return self.is_done.load(Ordering::Acquire);
        }
    }
}

// Allow the handle to be awaited directly!
impl<T> Future for TaskHandle<T> {
    type Output = Result<T, oneshot::error::RecvError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // We just proxy the poll call to the inner receiver (rx)
        Pin::new(&mut self.rx).poll(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn test_spawn_with_handle() {
        let handle = spawn(async { 42 });

        let result = handle.join().await.unwrap();
        assert_eq!(result, 42);
    }

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
