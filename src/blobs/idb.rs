//! The browser blob store, on IndexedDB.
//!
//! IndexedDB is the browser's home for binary state: transactional (which is
//! where the atomic-put promise comes from), asynchronous, and quota'd in
//! the hundreds of MB where `localStorage` stops at a few. The cost is its
//! callback-shaped API; this file folds each request into a future once so
//! nobody else has to.

use super::{BlobFuture, BlobStore, key_error, valid_key};
use futures::channel::oneshot;
use std::future::Future;
use std::io;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{IdbDatabase, IdbOpenDbRequest, IdbRequest, IdbTransactionMode};

/// A named IndexedDB database holding one object store of blobs.
///
/// Two `IdbStore`s with the same name address the same data, across page
/// loads — this is the browser's durable state, subject to the browser's
/// own eviction policies (a private window or cleared site data comes back
/// empty; that is the platform, not the store).
#[derive(Debug, Clone)]
pub struct IdbStore {
    db_name: String,
}

const STORE: &str = "blobs";

impl IdbStore {
    /// A store backed by the IndexedDB database `db_name`.
    pub fn open(db_name: impl Into<String>) -> Self {
        IdbStore {
            db_name: db_name.into(),
        }
    }
}

fn js_err(context: &str, e: impl std::fmt::Debug) -> io::Error {
    io::Error::other(format!("indexeddb {context}: {e:?}"))
}

/// Open (and on first use create) the database. One open per operation:
/// an `IdbDatabase` is a JS handle that cannot be parked in a `Send + Sync`
/// struct, and for checkpoint-sized traffic the open is cheap against the
/// transaction it precedes.
async fn open_db(name: &str) -> io::Result<IdbDatabase> {
    let factory = web_sys::window()
        .ok_or_else(|| io::Error::other("no window: IdbStore is browser-main-thread only"))?
        .indexed_db()
        .map_err(|e| js_err("factory", e))?
        .ok_or_else(|| io::Error::other("indexedDB unavailable in this context"))?;
    let req: IdbOpenDbRequest = factory
        .open_with_u32(name, 1)
        .map_err(|e| js_err("open", e))?;

    // First-ever open: create the object store during the upgrade window,
    // the only moment IndexedDB allows it.
    let upgrade = Closure::once_into_js(move |event: web_sys::Event| {
        if let Some(target) = event.target()
            && let Ok(req) = target.dyn_into::<IdbOpenDbRequest>()
            && let Ok(result) = req.result()
            && let Ok(db) = result.dyn_into::<IdbDatabase>()
        {
            let _ = db.create_object_store(STORE);
        }
    });
    req.set_onupgradeneeded(Some(upgrade.unchecked_ref()));

    let value = await_request(req.clone().into()).await?;
    value
        .dyn_into::<IdbDatabase>()
        .map_err(|e| js_err("open result", e))
}

/// One request, one future: resolves with `result()` on success, errors on
/// failure. The success and error closures are `once_into_js`, so the fired
/// one is freed by the binding; the other leaks a few dozen bytes, which is
/// the accepted cost of the callback API.
fn await_request(req: IdbRequest) -> impl Future<Output = io::Result<JsValue>> {
    let (tx, rx) = oneshot::channel::<Result<JsValue, String>>();
    let (tx2, req2) = (std::cell::RefCell::new(Some(tx)), req.clone());
    let tx2b = std::rc::Rc::new(tx2);
    let tx_ok = std::rc::Rc::clone(&tx2b);
    let on_ok = Closure::once_into_js(move |_: web_sys::Event| {
        if let Some(tx) = tx_ok.borrow_mut().take() {
            let _ = tx.send(req2.result().map_err(|e| format!("{e:?}")));
        }
    });
    let on_err = Closure::once_into_js(move |_: web_sys::Event| {
        if let Some(tx) = tx2b.borrow_mut().take() {
            let _ = tx.send(Err("request failed".to_string()));
        }
    });
    req.set_onsuccess(Some(on_ok.unchecked_ref()));
    req.set_onerror(Some(on_err.unchecked_ref()));
    async move {
        match rx.await {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(io::Error::other(format!("indexeddb request: {e}"))),
            Err(_) => Err(io::Error::other("indexeddb request dropped")),
        }
    }
}

/// The browser future is `!Send` (it holds JS handles); the trait promises
/// `Send`. The same single-threaded-wasm argument `time::sleep` documents
/// applies: nothing ever crosses a thread, because there are no threads.
struct SendFuture<F>(F);

// SAFETY: wasm32-unknown-unknown is single-threaded; this future is never
// actually sent between threads.
unsafe impl<F> Send for SendFuture<F> {}

impl<F: Future> Future for SendFuture<F> {
    type Output = F::Output;
    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        // Project Pin<&mut Wrapper> to Pin<&mut Inner>.
        unsafe { self.map_unchecked_mut(|s| &mut s.0).poll(cx) }
    }
}

fn wrap<T: 'static>(fut: impl Future<Output = io::Result<T>> + 'static) -> BlobFuture<T> {
    Box::pin(SendFuture(fut))
}

impl BlobStore for IdbStore {
    fn put(&self, key: &str, bytes: Vec<u8>) -> BlobFuture<()> {
        let name = self.db_name.clone();
        let key = key.to_string();
        wrap(async move {
            if !valid_key(&key) {
                return Err(key_error(&key));
            }
            let db = open_db(&name).await?;
            let tx = db
                .transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)
                .map_err(|e| js_err("transaction", e))?;
            let store = tx.object_store(STORE).map_err(|e| js_err("store", e))?;
            let value = js_sys::Uint8Array::from(bytes.as_slice());
            let req = store
                .put_with_key(&value, &JsValue::from_str(&key))
                .map_err(|e| js_err("put", e))?;
            await_request(req).await?;
            Ok(())
        })
    }

    fn get(&self, key: &str) -> BlobFuture<Option<Vec<u8>>> {
        let name = self.db_name.clone();
        let key = key.to_string();
        wrap(async move {
            if !valid_key(&key) {
                return Err(key_error(&key));
            }
            let db = open_db(&name).await?;
            let tx = db
                .transaction_with_str(STORE)
                .map_err(|e| js_err("transaction", e))?;
            let store = tx.object_store(STORE).map_err(|e| js_err("store", e))?;
            let req = store
                .get(&JsValue::from_str(&key))
                .map_err(|e| js_err("get", e))?;
            let value = await_request(req).await?;
            if value.is_undefined() || value.is_null() {
                return Ok(None);
            }
            let arr = value
                .dyn_into::<js_sys::Uint8Array>()
                .map_err(|e| js_err("get result", e))?;
            Ok(Some(arr.to_vec()))
        })
    }

    fn delete(&self, key: &str) -> BlobFuture<()> {
        let name = self.db_name.clone();
        let key = key.to_string();
        wrap(async move {
            if !valid_key(&key) {
                return Err(key_error(&key));
            }
            let db = open_db(&name).await?;
            let tx = db
                .transaction_with_str_and_mode(STORE, IdbTransactionMode::Readwrite)
                .map_err(|e| js_err("transaction", e))?;
            let store = tx.object_store(STORE).map_err(|e| js_err("store", e))?;
            let req = store
                .delete(&JsValue::from_str(&key))
                .map_err(|e| js_err("delete", e))?;
            await_request(req).await?;
            Ok(())
        })
    }

    fn list(&self) -> BlobFuture<Vec<String>> {
        let name = self.db_name.clone();
        wrap(async move {
            let db = open_db(&name).await?;
            let tx = db
                .transaction_with_str(STORE)
                .map_err(|e| js_err("transaction", e))?;
            let store = tx.object_store(STORE).map_err(|e| js_err("store", e))?;
            let req = store.get_all_keys().map_err(|e| js_err("keys", e))?;
            let value = await_request(req).await?;
            let arr = value
                .dyn_into::<js_sys::Array>()
                .map_err(|e| js_err("keys result", e))?;
            Ok(arr.iter().filter_map(|v| v.as_string()).collect())
        })
    }
}
