//! Platform-specific IO utilities.

use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, ReadBuf};

// --- Stdin ---

/// A handle to the standard input stream.
pub struct Stdin {
    // Native: Receiver from the blocking thread
    #[cfg(not(target_arch = "wasm32"))]
    receiver: tokio::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>,
    #[cfg(not(target_arch = "wasm32"))]
    buffer: Vec<u8>,

    // WASI: Direct handle to std::io::Stdin
    #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
    inner: std::io::Stdin,
}

/// Constructs a new handle to the standard input of the current process.
pub fn stdin() -> Stdin {
    // --- NATIVE IMPLEMENTATION ---
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (tx, rx) = tokio::sync::mpsc::channel(32);

        // FIX: Use std::thread::spawn instead of tokio::task::spawn_blocking.
        // spawn_blocking tasks are managed by the runtime, which waits for them
        // to finish on shutdown. Since this thread blocks forever on read(),
        // it prevents the app from exiting.
        // std::thread::spawn creates a detached thread that dies immediately 
        // when the main process exits.
        std::thread::spawn(move || {
            let mut input = std::io::stdin();
            let mut buf = [0u8; 1024];
            use std::io::Read;

            loop {
                match input.read(&mut buf) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        if tx.blocking_send(Ok(buf[..n].to_vec())).is_err() {
                            break;
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

    // --- WASI IMPLEMENTATION ---
    #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
    {
        Stdin {
            inner: std::io::stdin(),
        }
    }

    // --- BROWSER IMPLEMENTATION ---
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        Stdin {}
    }
}

// --- Native Trait Impl ---
#[cfg(not(target_arch = "wasm32"))]
impl AsyncRead for Stdin {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        loop {
            if !self.buffer.is_empty() {
                let len = std::cmp::min(buf.remaining(), self.buffer.len());
                buf.put_slice(&self.buffer[..len]);
                self.buffer.drain(..len);
                return Poll::Ready(Ok(()));
            }

            match self.receiver.poll_recv(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    self.buffer = chunk;
                    continue;
                }
                Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e)),
                Poll::Ready(None) => return Poll::Ready(Ok(())), // EOF
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

// --- WASI Trait Impl ---
#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
impl AsyncRead for Stdin {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        use std::io::Read;
        // NOTE: On WASI P2 (single-threaded), this read is synchronous.
        // It will block the async executor until data is available.
        let slice = buf.initialize_unfilled();
        match self.inner.read(slice) {
            Ok(n) => {
                buf.advance(n);
                Poll::Ready(Ok(()))
            }
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

// --- Browser Trait Impl ---
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl AsyncRead for Stdin {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(())) 
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    use tokio::io::AsyncReadExt;

    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    #[wasm_bindgen_test::wasm_bindgen_test]
    async fn test_stdin_browser_noop() {
        let mut stdin = stdin();
        let mut buf = [0u8; 10];
        let n = stdin.read(&mut buf).await.unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn test_stdin_creation() {
        // This will now pass on Native even without a runtime!
        // (std::thread::spawn doesn't require a runtime context like spawn_blocking did)
        let _stdin = stdin();
    }
}