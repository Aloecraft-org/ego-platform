//! Platform-specific IO utilities.
//!
//! This module provides async stdin/stdout that work correctly across:
//! - Native platforms (Linux, macOS, Windows)
//! - WASI Preview 2 (wasm32-wasip2)
//! - Browser (wasm32-unknown-unknown)
//!
//! The key challenge on WASI P2 is that stdin.read() blocks the single-threaded
//! runtime, starving other async tasks (like timers). This implementation uses
//! WASI P2's native polling APIs to wait for stdin readiness without blocking.

use std::io::Result;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

// --- Public Interface ---

pub use impl_platform::{Stdin, Stdout, stdin, stdout};

// --- Native Implementation (Threaded) ---
// Uses a background thread for blocking stdin reads, communicating via channel.
#[cfg(not(target_arch = "wasm32"))]
mod impl_platform {
    use super::*;

    pub struct Stdin {
        receiver: tokio::sync::mpsc::Receiver<Result<Vec<u8>>>,
        buffer: Vec<u8>,
    }

    pub struct Stdout {
        inner: tokio::io::Stdout,
    }

    pub fn stdin() -> Stdin {
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        // Native: Spawn a detached thread for blocking reads.
        // This is necessary because std::io::Stdin::read() blocks,
        // and we need the async runtime to remain responsive.
        std::thread::spawn(move || {
            let mut input = std::io::stdin();
            let mut buf = [0u8; 1024];
            use std::io::Read;

            loop {
                match input.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if tx.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Err(e) => {
                        let _ = tx.blocking_send(Err(e));
                        break;
                    }
                }
            }
        });

        Stdin {
            receiver: rx,
            buffer: Vec::new(),
        }
    }

    pub fn stdout() -> Stdout {
        Stdout {
            inner: tokio::io::stdout(),
        }
    }

    impl AsyncRead for Stdin {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<Result<()>> {
            loop {
                // First, drain any buffered data
                if !self.buffer.is_empty() {
                    let len = std::cmp::min(buf.remaining(), self.buffer.len());
                    buf.put_slice(&self.buffer[..len]);
                    self.buffer.drain(..len);
                    return Poll::Ready(Ok(()));
                }

                // Then try to receive more data from the background thread
                match self.receiver.poll_recv(cx) {
                    Poll::Ready(Some(Ok(chunk))) => {
                        self.buffer = chunk;
                        continue;
                    }
                    Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e)),
                    Poll::Ready(None) => return Poll::Ready(Ok(())), // Channel closed = EOF
                    Poll::Pending => return Poll::Pending,
                }
            }
        }
    }

    impl AsyncWrite for Stdout {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            Pin::new(&mut self.inner).poll_write(cx, buf)
        }
        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            Pin::new(&mut self.inner).poll_flush(cx)
        }
        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
            Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }
}

// --- WASI P2 Implementation ---
//
// CRITICAL ARCHITECTURE NOTE:
// ==========================
// WASI P2 on a single-threaded tokio runtime presents a fundamental challenge:
// std::io::Stdin::read() BLOCKS the entire runtime until input arrives.
// This means timers, spawned tasks, and everything else gets starved.
//
// The WASI P2 component model has non-blocking primitives (wasi:io/streams),
// but Rust's std::io doesn't expose them. The `wasi` crate provides bindings
// that let us use the native WASI P2 polling APIs.
//
// SOLUTION: Timeout-based polling with wasi:io/poll
// =================================================
// We use wasi:io/poll with a very short timeout to check for stdin readiness.
// If data isn't ready, we yield back to tokio and re-poll later.
// This allows timers and other tasks to make progress between polls.
//
// This approach has a tradeoff: slightly higher latency for stdin input
// (up to POLL_TIMEOUT_NS), but guarantees the runtime stays responsive.
#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
mod impl_platform {
    use super::*;

    // How long to wait for stdin in each poll cycle (in nanoseconds).
    // Shorter = more responsive timers, but more CPU overhead.
    // 10ms is a good balance for interactive shells.
    const POLL_TIMEOUT_NS: u64 = 10_000_000; // 10ms

    /// Async stdin for WASI P2 that cooperates with the tokio runtime.
    ///
    /// Uses wasi:io/poll with short timeouts to avoid blocking the runtime.
    pub struct Stdin {
        buffer: Vec<u8>,
    }

    pub struct Stdout;

    pub fn stdin() -> Stdin {
        Stdin { buffer: Vec::new() }
    }

    pub fn stdout() -> Stdout {
        Stdout
    }

    impl AsyncRead for Stdin {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<Result<()>> {
            // First, drain any buffered data from previous reads
            if !self.buffer.is_empty() {
                let len = std::cmp::min(buf.remaining(), self.buffer.len());
                buf.put_slice(&self.buffer[..len]);
                self.buffer.drain(..len);
                return Poll::Ready(Ok(()));
            }

            // Get stdin stream - note: in WASI P2, get_stdin() returns a fresh handle each time
            let stream = wasi::cli::stdin::get_stdin();

            // Get a pollable for the stdin stream
            let stdin_pollable = stream.subscribe();

            // Also create a timer pollable for our timeout
            let timer_pollable = wasi::clocks::monotonic_clock::subscribe_duration(POLL_TIMEOUT_NS);

            // Poll both: stdin readiness OR timeout
            // This is the key: poll() will return when EITHER is ready,
            // so we won't block forever waiting for stdin.
            let ready_indices = wasi::io::poll::poll(&[&stdin_pollable, &timer_pollable]);

            // Check if stdin is ready (index 0)
            let stdin_ready = ready_indices.iter().any(|&i| i == 0);

            if stdin_ready {
                // Stdin has data! Read it non-blocking.
                // WASI streams return whatever is available (may be less than requested).
                match stream.read(buf.remaining() as u64) {
                    Ok(bytes) => {
                        if bytes.is_empty() {
                            // EOF
                            Poll::Ready(Ok(()))
                        } else {
                            buf.put_slice(&bytes);
                            Poll::Ready(Ok(()))
                        }
                    }
                    Err(wasi::io::streams::StreamError::Closed) => {
                        // Stream closed = EOF
                        Poll::Ready(Ok(()))
                    }
                    Err(_e) => Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "WASI stream read error",
                    ))),
                }
            } else {
                // Timeout fired, stdin not ready.
                // Yield back to tokio so other tasks can run.
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        }
    }

    impl AsyncWrite for Stdout {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            // Get stdout stream
            let stream = wasi::cli::stdout::get_stdout();

            // Check how much we can write without blocking
            match stream.check_write() {
                Ok(0) => {
                    // Can't write right now, would need to wait
                    // For simplicity, just report we wrote 0 bytes
                    Poll::Ready(Ok(0))
                }
                Ok(n) => {
                    let to_write = std::cmp::min(n as usize, buf.len());
                    match stream.write(&buf[..to_write]) {
                        Ok(()) => Poll::Ready(Ok(to_write)),
                        Err(_) => Poll::Ready(Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "WASI stream write error",
                        ))),
                    }
                }
                Err(_) => Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "WASI stream check_write error",
                ))),
            }
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            let stream = wasi::cli::stdout::get_stdout();

            // flush() is non-blocking, just requests a flush.
            // blocking_flush() will wait for it to complete.
            match stream.flush() {
                Ok(()) => match stream.blocking_flush() {
                    Ok(()) => Poll::Ready(Ok(())),
                    Err(_) => Poll::Ready(Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "WASI stream flush error",
                    ))),
                },
                Err(_) => Poll::Ready(Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "WASI stream flush error",
                ))),
            }
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
}

// --- Browser Implementation (Stubs) ---
// Browser doesn't have traditional stdin/stdout. These are no-ops.
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod impl_platform {
    use super::*;

    pub struct Stdin {}
    pub struct Stdout {}

    pub fn stdin() -> Stdin {
        Stdin {}
    }
    pub fn stdout() -> Stdout {
        Stdout {}
    }

    impl AsyncRead for Stdin {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &mut ReadBuf<'_>,
        ) -> Poll<Result<()>> {
            // Browser stdin is always EOF
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for Stdout {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            // Pretend we wrote everything (browser uses console.log instead)
            Poll::Ready(Ok(buf.len()))
        }
        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
}
