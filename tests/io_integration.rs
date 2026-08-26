// --- Native Tests (Threaded) ---
#[cfg(not(target_arch = "wasm32"))]
mod native_tests {
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use std::time::Duration;
    use tokio::io::{AsyncRead, AsyncReadExt, ReadBuf};

    struct MockStdinBridge {
        receiver: tokio::sync::mpsc::Receiver<std::io::Result<Vec<u8>>>,
        buffer: Vec<u8>,
    }

    impl MockStdinBridge {
        fn new(data_chunks: Vec<Vec<u8>>) -> Self {
            let (tx, rx) = tokio::sync::mpsc::channel(32);

            // This is safe here because this module is guarded by cfg(not(wasm32))
            tokio::task::spawn_blocking(move || {
                for chunk in data_chunks {
                    std::thread::sleep(Duration::from_millis(10));
                    if tx.blocking_send(Ok(chunk)).is_err() {
                        break;
                    }
                }
            });

            Self {
                receiver: rx,
                buffer: Vec::new(),
            }
        }
    }

    impl AsyncRead for MockStdinBridge {
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
                    Poll::Ready(None) => return Poll::Ready(Ok(())),
                    Poll::Pending => return Poll::Pending,
                }
            }
        }
    }

    #[tokio::test]
    async fn test_blocking_bridge_correctness() {
        let input_data = vec![
            b"Chunk 1 ".to_vec(),
            b"Chunk 2 ".to_vec(),
            b"Chunk 3".to_vec(),
        ];

        let mut reader = MockStdinBridge::new(input_data);
        let mut buffer = String::new();

        reader.read_to_string(&mut buffer).await.unwrap();

        assert_eq!(buffer, "Chunk 1 Chunk 2 Chunk 3");
    }
}

// --- WASI Tests (Single Threaded) ---
#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
mod wasi_tests {
    #[tokio::test]
    async fn test_wasi_stdin_sanity() {
        // We just verify we can create the handle without crashing.
        // We cannot test actual reading without external piping.
        let _stdin = ego_platform::stdin();
    }
}
