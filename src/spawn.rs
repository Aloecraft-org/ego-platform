//! Platform-specific task spawning.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::oneshot;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use std::sync::Arc;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use std::sync::atomic::{AtomicBool, Ordering};

/// Spawn a future onto the appropriate runtime for the current platform.
///
/// - **Browser**: Uses `wasm_bindgen_futures::spawn_local` (no `Send` required)
/// - **Native/WASI**: Uses `tokio::spawn` (requires `Send`)
///
/// # Examples
///
/// ```no_run
/// use ego_platform::spawn;
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
/// - **Browser**: Uses `wasm_bindgen_futures::spawn_local` (no `Send` required)
/// - **Native/WASI**: Uses `tokio::spawn` (requires `Send`)
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn spawn<F, T>(future: F) -> TaskHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = oneshot::channel();
    let inner = tokio::spawn(async move {
        let _ = tx.send(future.await);
    });
    TaskHandle { rx, inner }
}

/// A handle to a spawned task, loosely mirroring `tokio::task::JoinHandle`.
///
/// Can be awaited directly, or via [`TaskHandle::join`].
pub struct TaskHandle<T> {
    rx: oneshot::Receiver<T>,
    // On native/WASI we keep the actual JoinHandle to allow forceful aborts
    // and accurate is_finished() reporting.
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    inner: tokio::task::JoinHandle<()>,
    // The browser has no JoinHandle, so completion is tracked manually.
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    is_done: Arc<AtomicBool>,
}

impl<T> TaskHandle<T> {
    /// Await the completion of the task and return the result.
    pub async fn join(self) -> Result<T, oneshot::error::RecvError> {
        self.rx.await
    }

    /// Forcefully terminate the task.
    ///
    /// # Platform Behavior
    /// - **Native/WASI**: Aborts via the Tokio `JoinHandle`.
    /// - **Browser**: No-op; `spawn_local` tasks cannot be aborted.
    pub fn abort(&self) {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        self.inner.abort();
    }

    /// Returns `true` if the task has finished.
    pub fn is_finished(&self) -> bool {
        #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
        {
            self.inner.is_finished()
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            self.is_done.load(Ordering::Acquire)
        }
    }
}

impl<T> Future for TaskHandle<T> {
    type Output = Result<T, oneshot::error::RecvError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.rx).poll(cx)
    }
}
