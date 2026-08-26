//! Synchronization primitives.
//!
//! `broadcast` is tokio's, re-exported — and it works on **every** target,
//! the browser included: tokio's `sync` feature is runtime-free and
//! wasm-clean, so a `broadcast::channel` made on the browser main thread
//! behaves exactly as it does natively. (An earlier doc called this
//! "native only, stub on WASM"; that was never what the code did, and
//! `tests/sync_tests.rs` now pins the truth on all three targets.)

pub use tokio::sync::broadcast;
