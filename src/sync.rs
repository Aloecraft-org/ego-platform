// src/platform/sync.rs

//! Platform-agnostic synchronization primitives.

/// Broadcast channel that works on all platforms.
/// On native, wraps tokio::sync::broadcast.
/// On browser, provides a stub (no shutdown signaling needed).
pub mod broadcast {
    #[cfg(not(target_arch = "wasm32"))]
    mod native {
        use tokio::sync::broadcast as tokio_broadcast;

        pub struct Sender<T> {
            inner: tokio_broadcast::Sender<T>,
        }

        pub struct Receiver<T> {
            inner: tokio_broadcast::Receiver<T>,
        }

        pub fn channel<T: Clone>(capacity: usize) -> (Sender<T>, Receiver<T>) {
            let (tx, rx) = tokio_broadcast::channel(capacity);
            (Sender { inner: tx }, Receiver { inner: rx })
        }

        impl<T: Clone> Sender<T> {
            pub fn send(&self, value: T) -> Result<usize, ()> {
                self.inner.send(value).map_err(|_| ())
            }

            pub fn subscribe(&self) -> Receiver<T> {
                Receiver {
                    inner: self.inner.subscribe(),
                }
            }
        }

        impl<T: Clone> Receiver<T> {
            pub async fn recv(&mut self) -> Result<T, ()> {
                self.inner.recv().await.map_err(|_| ())
            }

            pub fn resubscribe(&self) -> Self {
                Self {
                    inner: self.inner.resubscribe(),
                }
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    mod wasm {
        use std::marker::PhantomData;

        pub struct Sender<T> {
            _phantom: PhantomData<T>,
        }

        pub struct Receiver<T> {
            _phantom: PhantomData<T>,
        }

        pub fn channel<T: Clone>(_capacity: usize) -> (Sender<T>, Receiver<T>) {
            (
                Sender { _phantom: PhantomData },
                Receiver { _phantom: PhantomData },
            )
        }

        impl<T: Clone> Sender<T> {
            pub fn send(&self, _value: T) -> Result<usize, ()> {
                Ok(0) // No-op on WASM
            }

            pub fn subscribe(&self) -> Receiver<T> {
                Receiver { _phantom: PhantomData }
            }
        }

        impl<T: Clone> Receiver<T> {
            pub async fn recv(&mut self) -> Result<T, ()> {
                // Never completes on WASM (no shutdown signaling)
                futures::future::pending::<()>().await;
                unreachable!()
            }

            pub fn resubscribe(&self) -> Self {
                Self { _phantom: PhantomData }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub use native::*;

    #[cfg(target_arch = "wasm32")]
    pub use wasm::*;
}