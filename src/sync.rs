//! Platform-agnostic synchronization primitives.

/// Broadcast channel that works on all platforms.
///
/// On native/WASI, wraps `tokio::sync::broadcast`.
/// On browser, provides a stub (no shutdown signaling needed in browser context).
pub mod broadcast {
    #[cfg(not(target_arch = "wasm32"))]
    mod native {
        use tokio::sync::broadcast as tokio_broadcast;

        /// Sending half of a broadcast channel.
        pub struct Sender<T> {
            inner: tokio_broadcast::Sender<T>,
        }

        /// Receiving half of a broadcast channel.
        pub struct Receiver<T> {
            inner: tokio_broadcast::Receiver<T>,
        }

        /// Create a new broadcast channel with the specified capacity.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use aloeplatform::broadcast;
        ///
        /// # async {
        /// let (tx, mut rx) = broadcast::channel::<String>(10);
        /// tx.send("Hello".to_string()).ok();
        /// let msg = rx.recv().await.ok();
        /// # };
        /// ```
        pub fn channel<T: Clone>(capacity: usize) -> (Sender<T>, Receiver<T>) {
            let (tx, rx) = tokio_broadcast::channel(capacity);
            (Sender { inner: tx }, Receiver { inner: rx })
        }

        impl<T: Clone> Sender<T> {
            /// Send a value to all active receivers.
            ///
            /// Returns the number of receivers that received the value,
            /// or an error if there are no active receivers.
            pub fn send(&self, value: T) -> Result<usize, ()> {
                self.inner.send(value).map_err(|_| ())
            }

            /// Create a new receiver for this channel.
            pub fn subscribe(&self) -> Receiver<T> {
                Receiver {
                    inner: self.inner.subscribe(),
                }
            }
        }

        impl<T: Clone> Receiver<T> {
            /// Receive the next value from the channel.
            ///
            /// Returns an error if all senders have been dropped.
            pub async fn recv(&mut self) -> Result<T, ()> {
                self.inner.recv().await.map_err(|_| ())
            }

            /// Create a new receiver with the same position in the channel.
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

        /// Stub sender for WASM platforms.
        pub struct Sender<T> {
            _phantom: PhantomData<T>,
        }

        /// Stub receiver for WASM platforms.
        pub struct Receiver<T> {
            _phantom: PhantomData<T>,
        }

        /// Create a stub broadcast channel for WASM platforms.
        ///
        /// Note: This is a no-op on WASM. Broadcast channels are not needed
        /// in browser contexts as there's no shutdown signaling mechanism.
        pub fn channel<T: Clone>(_capacity: usize) -> (Sender<T>, Receiver<T>) {
            (
                Sender {
                    _phantom: PhantomData,
                },
                Receiver {
                    _phantom: PhantomData,
                },
            )
        }

        impl<T: Clone> Sender<T> {
            /// Stub send implementation (no-op on WASM).
            pub fn send(&self, _value: T) -> Result<usize, ()> {
                Ok(0)
            }

            /// Create a stub receiver.
            pub fn subscribe(&self) -> Receiver<T> {
                Receiver {
                    _phantom: PhantomData,
                }
            }
        }

        impl<T: Clone> Receiver<T> {
            /// Stub receive implementation.
            ///
            /// Note: This will never complete on WASM platforms.
            pub async fn recv(&mut self) -> Result<T, ()> {
                futures::future::pending::<()>().await;
                unreachable!()
            }

            /// Create a stub resubscribed receiver.
            pub fn resubscribe(&self) -> Self {
                Self {
                    _phantom: PhantomData,
                }
            }
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub use native::*;

    #[cfg(target_arch = "wasm32")]
    pub use wasm::*;
}

#[cfg(test)]
mod tests {
    use super::broadcast;

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_broadcast_send_recv() {
        let (tx, mut rx) = broadcast::channel::<i32>(10);

        tx.send(42).ok();
        let received = rx.recv().await.unwrap();

        assert_eq!(received, 42);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_broadcast_multiple_receivers() {
        let (tx, mut rx1) = broadcast::channel::<String>(10);
        let mut rx2 = tx.subscribe();

        tx.send("Hello".to_string()).ok();

        assert_eq!(rx1.recv().await.unwrap(), "Hello");
        assert_eq!(rx2.recv().await.unwrap(), "Hello");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_broadcast_multiple_messages() {
        let (tx, mut rx) = broadcast::channel::<i32>(10);

        tx.send(1).ok();
        tx.send(2).ok();
        tx.send(3).ok();

        assert_eq!(rx.recv().await.unwrap(), 1);
        assert_eq!(rx.recv().await.unwrap(), 2);
        assert_eq!(rx.recv().await.unwrap(), 3);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_broadcast_resubscribe() {
        let (tx, mut rx1) = broadcast::channel::<i32>(10);

        tx.send(1).ok();
        
        // rx1 receives the first message
        assert_eq!(rx1.recv().await.unwrap(), 1);

        // Create a new receiver at the current position
        let mut rx2 = rx1.resubscribe();

        tx.send(2).ok();

        // Both should receive the second message
        assert_eq!(rx1.recv().await.unwrap(), 2);
        assert_eq!(rx2.recv().await.unwrap(), 2);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_broadcast_channel_closed() {
        let (tx, mut rx) = broadcast::channel::<i32>(10);

        drop(tx);

        // Should get an error when the channel is closed
        assert!(rx.recv().await.is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_broadcast_send_return_value() {
        let (tx, _rx1) = broadcast::channel::<i32>(10);
        let _rx2 = tx.subscribe();
        let _rx3 = tx.subscribe();

        // Should return the number of receivers (3)
        let result = tx.send(42);
        assert_eq!(result.unwrap(), 3);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_broadcast_wasm_stub() {
        let (tx, _rx) = broadcast::channel::<i32>(10);

        // Send should succeed but do nothing
        assert_eq!(tx.send(42).unwrap(), 0);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_broadcast_wasm_subscribe() {
        let (tx, _rx1) = broadcast::channel::<i32>(10);
        let _rx2 = tx.subscribe();

        // Should not panic
        assert!(true);
    }

    #[cfg(target_arch = "wasm32")]
    #[test]
    fn test_broadcast_wasm_resubscribe() {
        let (_tx, rx) = broadcast::channel::<i32>(10);
        let _rx2 = rx.resubscribe();

        // Should not panic
        assert!(true);
    }
}