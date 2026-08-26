//! One "we are shutting down" signal, however the platform says it.
//!
//! Every long-running consumer eventually wants the same thing: notice the
//! environment ending, flush state, exit cleanly. What "ending" looks like
//! differs — Ctrl-C or SIGTERM on native, `pagehide` in the browser, nothing
//! at all on WASI (a wasm guest is torn down by its host) — so the platform
//! layer folds them into one awaitable:
//!
//! ```no_run
//! # async fn run() {
//! let reason = ego_platform::shutdown::requested().await;
//! log::info!("shutting down: {reason:?}");
//! // flush, snapshot, close listeners...
//! # }
//! ```
//!
//! [`request`] raises the same signal programmatically on every target — the
//! only trigger WASI has, and the one tests use. The signal is level, not
//! edge: once raised it stays raised, so a subscriber that starts listening
//! late still sees it.
//!
//! **Interop note (ego-proc):** this stays a platform primitive on purpose —
//! no protobuf, no `ControlSignal` — because ego-platform sits *below* the
//! actor layer. The intended wiring is one line in the orchestrator's
//! owner: `shutdown::requested().await` then broadcast
//! `ControlSignal::Stop` to the actors. The mapping lives where the
//! vocabulary does.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

/// Why the shutdown was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// An interactive interrupt: Ctrl-C / SIGINT.
    Interrupt,
    /// A supervisor's terminate: SIGTERM (Unix only).
    Terminate,
    /// The page is being hidden or unloaded (browser only). The last
    /// reliable moment to write state; work quickly.
    PageHide,
    /// [`request`] was called.
    Requested,
}

struct Signal {
    raised: AtomicBool,
    reason: std::sync::Mutex<Option<Reason>>,
    notify: Notify,
}

fn signal() -> &'static Signal {
    static SIGNAL: OnceLock<Signal> = OnceLock::new();
    SIGNAL.get_or_init(|| Signal {
        raised: AtomicBool::new(false),
        reason: std::sync::Mutex::new(None),
        notify: Notify::new(),
    })
}

fn raise(reason: Reason) {
    let s = signal();
    {
        let mut r = s.reason.lock().expect("shutdown signal poisoned");
        if r.is_none() {
            *r = Some(reason);
        }
    }
    s.raised.store(true, Ordering::Release);
    s.notify.notify_waiters();
}

/// Request shutdown programmatically. Idempotent; the first reason wins.
pub fn request() {
    raise(Reason::Requested);
}

/// Has shutdown been requested? For loops that poll rather than await.
pub fn is_requested() -> bool {
    signal().raised.load(Ordering::Acquire)
}

/// Resolves when shutdown is requested — immediately, if it already was.
///
/// Any number of callers may await this; all of them resolve. The platform
/// triggers are installed lazily on first await:
///
/// - **Native**: Ctrl-C (and SIGTERM on Unix) listeners are spawned onto
///   the current tokio runtime.
/// - **Browser**: a `pagehide` listener goes on the window.
/// - **WASI**: no ambient trigger exists; [`request`] is the signal.
pub async fn requested() -> Reason {
    install_platform_triggers();
    let s = signal();
    loop {
        if s.raised.load(Ordering::Acquire) {
            return s
                .reason
                .lock()
                .expect("shutdown signal poisoned")
                .unwrap_or(Reason::Requested);
        }
        // Registered-then-rechecked, so a raise between the check and the
        // await is caught by notify_waiters' semantics with notified().
        let notified = s.notify.notified();
        if s.raised.load(Ordering::Acquire) {
            continue;
        }
        notified.await;
    }
}

fn install_platform_triggers() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        #[cfg(not(target_arch = "wasm32"))]
        {
            crate::spawn(async {
                if tokio::signal::ctrl_c().await.is_ok() {
                    raise(Reason::Interrupt);
                }
            });
            #[cfg(unix)]
            crate::spawn(async {
                use tokio::signal::unix::{SignalKind, signal};
                if let Ok(mut term) = signal(SignalKind::terminate())
                    && term.recv().await.is_some()
                {
                    raise(Reason::Terminate);
                }
            });
        }

        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            use wasm_bindgen::JsCast;
            use wasm_bindgen::prelude::Closure;
            if let Some(window) = web_sys::window() {
                let cb = Closure::<dyn FnMut(web_sys::Event)>::new(move |_: web_sys::Event| {
                    raise(Reason::PageHide)
                });
                let _ = window
                    .add_event_listener_with_callback("pagehide", cb.as_ref().unchecked_ref());
                // The listener lives for the page's lifetime.
                cb.forget();
            }
        }
    });
}
