// src/platform/spawn.rs

use std::future::Future;

/// Spawn a future onto the runtime.
/// 
/// On native/WASI: Uses tokio::spawn (requires Send)
/// On browser: Uses wasm_bindgen_futures::spawn_local (no Send required)
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + 'static,
{
    wasm_bindgen_futures::spawn_local(future);
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn spawn<F>(future: F)
where
    F: Future<Output = ()> + Send + 'static,
{
    tokio::spawn(future);
}