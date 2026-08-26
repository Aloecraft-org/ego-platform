//! Named binary blobs with atomic writes, on every target.
//!
//! The [`fs`](crate::fs) module gives file *semantics* — paths, directories,
//! whatever the platform meant by them. This module gives a smaller, harder
//! promise for state that must survive: a flat namespace of keys to bytes
//! where **a put is all-or-nothing**. A reader sees the old blob or the new
//! blob, never a half-written one, on every backend — which is the property
//! checkpoint/snapshot state needs and plain `write` does not give.
//!
//! Backends:
//! - [`MemStore`] — in memory, every target. The test double and the cache.
//! - [`DirStore`] — a directory, native and WASI: write-to-temp,
//!   `sync_all`, rename into place.
//! - [`IdbStore`] — IndexedDB, browser: one transaction per put. (The
//!   `fs` module's browser backend is `localStorage` — string-typed and
//!   quota-bound at a few MB; IndexedDB is the browser's real home for
//!   binary state.)
//!
//! Keys are `[A-Za-z0-9._-]`, not starting with `.`, at most 255 bytes —
//! validated identically everywhere so a key that works on one backend
//! works on all. Blobs are read and written whole; this is deliberately not
//! a filesystem, a database, or a streaming interface.

use std::future::Future;
use std::io;
use std::pin::Pin;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod idb;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use idb::IdbStore;

/// The future every store method returns. Boxed for object safety; `Send`
/// holds on the browser by the same single-thread argument the rest of the
/// crate uses.
pub type BlobFuture<T> = Pin<Box<dyn Future<Output = io::Result<T>> + Send + 'static>>;

/// A flat namespace of keys to bytes, with atomic replacement.
pub trait BlobStore: Send + Sync {
    /// Store `bytes` under `key`, replacing atomically: a concurrent reader
    /// gets the old value or the new one, never a torn one.
    fn put(&self, key: &str, bytes: Vec<u8>) -> BlobFuture<()>;

    /// The bytes under `key`, or `None`.
    fn get(&self, key: &str) -> BlobFuture<Option<Vec<u8>>>;

    /// Remove `key`. Removing an absent key is fine.
    fn delete(&self, key: &str) -> BlobFuture<()>;

    /// Every key in the store, in no particular order.
    fn list(&self) -> BlobFuture<Vec<String>>;
}

/// Whether `key` is legal on every backend: `[A-Za-z0-9._-]`, nonempty, no
/// leading `.` (reserved for the directory backend's temp files), ≤ 255
/// bytes.
pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 255
        && !key.starts_with('.')
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
}

fn key_error(key: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("invalid blob key {key:?}: keys are [A-Za-z0-9._-], no leading dot, <=255 bytes"),
    )
}

// --- MemStore ---

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// An in-memory store. Clones share contents.
#[derive(Debug, Clone, Default)]
pub struct MemStore {
    map: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MemStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl BlobStore for MemStore {
    fn put(&self, key: &str, bytes: Vec<u8>) -> BlobFuture<()> {
        let map = Arc::clone(&self.map);
        let key = key.to_string();
        Box::pin(async move {
            if !valid_key(&key) {
                return Err(key_error(&key));
            }
            map.lock().expect("blob store poisoned").insert(key, bytes);
            Ok(())
        })
    }

    fn get(&self, key: &str) -> BlobFuture<Option<Vec<u8>>> {
        let map = Arc::clone(&self.map);
        let key = key.to_string();
        Box::pin(async move {
            if !valid_key(&key) {
                return Err(key_error(&key));
            }
            Ok(map.lock().expect("blob store poisoned").get(&key).cloned())
        })
    }

    fn delete(&self, key: &str) -> BlobFuture<()> {
        let map = Arc::clone(&self.map);
        let key = key.to_string();
        Box::pin(async move {
            if !valid_key(&key) {
                return Err(key_error(&key));
            }
            map.lock().expect("blob store poisoned").remove(&key);
            Ok(())
        })
    }

    fn list(&self) -> BlobFuture<Vec<String>> {
        let map = Arc::clone(&self.map);
        Box::pin(async move {
            Ok(map
                .lock()
                .expect("blob store poisoned")
                .keys()
                .cloned()
                .collect())
        })
    }
}

// --- DirStore (native and WASI) ---

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
mod dir {
    use super::{BlobFuture, BlobStore, key_error, valid_key};
    use crate::entropy;
    use std::io::{self, Write};
    use std::path::PathBuf;

    /// One directory, one blob per key.
    ///
    /// A put writes `.{key}.{random}.tmp` beside the destination, syncs it,
    /// and renames it into place — atomic on POSIX. (On Windows, where
    /// rename refuses to replace, the destination is removed first; the
    /// gap where the key briefly reads absent is the platform's best
    /// approximation, not a torn blob.) Operations run synchronously inside
    /// the returned future, which is the honest shape for
    /// checkpoint-sized blobs; a store for huge values wants streaming this
    /// trait deliberately does not offer.
    #[derive(Debug, Clone)]
    pub struct DirStore {
        root: PathBuf,
    }

    impl DirStore {
        /// Open (creating if needed) `root` as a blob directory.
        pub fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
            let root = root.into();
            std::fs::create_dir_all(&root)?;
            Ok(DirStore { root })
        }

        fn atomic_put(&self, key: &str, bytes: &[u8]) -> io::Result<()> {
            let dest = self.root.join(key);
            let mut suffix = [0u8; 8];
            entropy::fill(&mut suffix);
            let tmp = self.root.join(format!(".{key}.{}.tmp", hex(&suffix)));
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(bytes)?;
            f.sync_all()?;
            drop(f);
            match std::fs::rename(&tmp, &dest) {
                Ok(()) => Ok(()),
                Err(_) if dest.exists() => {
                    // Windows: rename refuses to replace. Remove and retry
                    // once; the temp file still carries the new bytes, so
                    // no outcome is a torn blob.
                    std::fs::remove_file(&dest)?;
                    std::fs::rename(&tmp, &dest)
                }
                Err(e) => {
                    let _ = std::fs::remove_file(&tmp);
                    Err(e)
                }
            }
        }
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    impl BlobStore for DirStore {
        fn put(&self, key: &str, bytes: Vec<u8>) -> BlobFuture<()> {
            let store = self.clone();
            let key = key.to_string();
            Box::pin(async move {
                if !valid_key(&key) {
                    return Err(key_error(&key));
                }
                store.atomic_put(&key, &bytes)
            })
        }

        fn get(&self, key: &str) -> BlobFuture<Option<Vec<u8>>> {
            let path = self.root.join(key);
            let key = key.to_string();
            Box::pin(async move {
                if !valid_key(&key) {
                    return Err(key_error(&key));
                }
                match std::fs::read(&path) {
                    Ok(bytes) => Ok(Some(bytes)),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
                    Err(e) => Err(e),
                }
            })
        }

        fn delete(&self, key: &str) -> BlobFuture<()> {
            let path = self.root.join(key);
            let key = key.to_string();
            Box::pin(async move {
                if !valid_key(&key) {
                    return Err(key_error(&key));
                }
                match std::fs::remove_file(&path) {
                    Ok(()) => Ok(()),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                    Err(e) => Err(e),
                }
            })
        }

        fn list(&self) -> BlobFuture<Vec<String>> {
            let root = self.root.clone();
            Box::pin(async move {
                let mut keys = Vec::new();
                for entry in std::fs::read_dir(&root)? {
                    let entry = entry?;
                    if let Ok(name) = entry.file_name().into_string()
                        && valid_key(&name)
                    {
                        keys.push(name);
                    }
                }
                Ok(keys)
            })
        }
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use dir::DirStore;
