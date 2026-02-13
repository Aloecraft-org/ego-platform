#[deprecated(note = "Just wraps tokio::sync::broadcast, probably doesn't do that good a job of it")]
pub mod broadcast {
    use tokio::sync::broadcast as tokio_broadcast;
    pub use tokio_broadcast::error;

    #[derive(Clone)]
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
            // In the browser, this won't actually "broadcast" across threads 
            // (since there is only one), but it will broadcast to all 
            // async tasks listening to this bus.
            self.inner.send(value).map_err(|_| ())
        }

        pub fn subscribe(&self) -> Receiver<T> {
            Receiver { inner: self.inner.subscribe() }
        }
    }

    impl<T: Clone> Receiver<T> {
        pub async fn recv(&mut self) -> Result<T, tokio_broadcast::error::RecvError> {
            self.inner.recv().await
        }

        pub fn resubscribe(&self) -> Self {
            Self { inner: self.inner.resubscribe() }
        }
    }
}