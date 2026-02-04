//! Platform-specific IO utilities.

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::io::Result;

// --- Public Interface ---

pub use impl_platform::{stdin, stdout, Stdin, Stdout};

// --- Native Implementation ---
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

        // use std::thread::spawn to avoid blocking the runtime on shutdown
        std::thread::spawn(move || {
            let mut input = std::io::stdin();
            let mut buf = [0u8; 1024];
            use std::io::Read;

            loop {
                match input.read(&mut buf) {
                    Ok(0) => break,
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
                    Poll::Ready(None) => return Poll::Ready(Ok(())),
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

// --- WASI Implementation ---
#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
mod impl_platform {
    use super::*;
    use std::io::{Read, Write};

    pub struct Stdin {
        inner: std::io::Stdin,
    }

    pub struct Stdout {
        inner: std::io::Stdout,
    }

    pub fn stdin() -> Stdin {
        Stdin {
            inner: std::io::stdin(),
        }
    }

    pub fn stdout() -> Stdout {
        Stdout {
            inner: std::io::stdout(),
        }
    }

    impl AsyncRead for Stdin {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<Result<()>> {
            // WASI P2: Synchronous blocking read.
            // We rely on the application to yield before calling this!
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

    impl AsyncWrite for Stdout {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize>> {
            match std::io::stdout().write(buf) {
                Ok(n) => Poll::Ready(Ok(n)),
                Err(e) => Poll::Ready(Err(e)),
            }
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            match std::io::stdout().flush() {
                Ok(_) => Poll::Ready(Ok(())),
                Err(e) => Poll::Ready(Err(e)),
            }
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }
    }
}

// --- Browser Implementation (Stubs) ---
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod impl_platform {
    use super::*;

    pub struct Stdin {}
    pub struct Stdout {}

    pub fn stdin() -> Stdin { Stdin {} }
    pub fn stdout() -> Stdout { Stdout {} }

    impl AsyncRead for Stdin {
        fn poll_read(self: Pin<&mut Self>, _cx: &mut Context<'_>, _buf: &mut ReadBuf<'_>) -> Poll<Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    impl AsyncWrite for Stdout {
        fn poll_write(self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize>> {
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
    fn test_io_creation() {
        let _stdin = stdin();
        let _stdout = stdout();
    }
}