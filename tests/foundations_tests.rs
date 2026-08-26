//! Entropy, pacing, sync, shutdown and env, on all three targets through the
//! shared test aliases. Blob stores get their own file.

mod common;
use common::{async_test, test};

use std::time::Duration;

// --- entropy ---

use ego_platform::entropy::{CryptoRngCore, SeededEntropy, SystemEntropy};
use rand_core::Rng;

#[test]
fn system_entropy_draws_differ() {
    let mut a = [0u8; 32];
    let mut b = [0u8; 32];
    SystemEntropy.fill_bytes(&mut a);
    SystemEntropy.fill_bytes(&mut b);
    assert_ne!(a, [0u8; 32]);
    assert_ne!(a, b);
}

#[test]
fn seeded_entropy_replays_and_system_satisfies_crypto() {
    fn crypto_user(rng: &mut impl CryptoRngCore) -> u64 {
        rng.next_u64()
    }
    fn plain_user(rng: &mut impl Rng) -> u64 {
        rng.next_u64()
    }

    let _ = crypto_user(&mut SystemEntropy);
    let mut x = SeededEntropy::from_u64(42);
    let mut y = SeededEntropy::from_u64(42);
    assert_eq!(plain_user(&mut x), plain_user(&mut y));
    // crypto_user(&mut x) is the compile error the module exists to cause.
}

// --- pacer ---

use ego_platform::pacer::Pacer;

#[async_test]
async fn a_zero_budget_yields_every_checkpoint() {
    let mut p = Pacer::new(Duration::ZERO);
    p.set_overrun_warning(None);
    assert!(p.checkpoint().await);
    assert!(p.checkpoint().await);
    let s = p.stats();
    assert_eq!((s.checkpoints, s.yields), (2, 2));
}

#[async_test]
async fn a_generous_budget_makes_checkpoints_free() {
    let mut p = Pacer::new(Duration::from_secs(3600));
    for _ in 0..500 {
        assert!(!p.checkpoint().await);
    }
    let s = p.stats();
    assert_eq!((s.checkpoints, s.yields), (500, 0));
}

#[async_test]
async fn the_worst_slice_is_visible() {
    let mut p = Pacer::new(Duration::from_millis(1));
    p.set_overrun_warning(None);
    p.checkpoint().await;
    // Busy-wait rather than thread::sleep: there is no thread to sleep on
    // the browser, and burning time is what a slice does anyway.
    let start = ego_platform::Instant::now();
    while start.elapsed() < Duration::from_millis(15) {
        std::hint::black_box(0u64);
    }
    assert!(p.checkpoint().await, "an overrun slice still yields");
    assert!(p.stats().worst_slice >= Duration::from_millis(10));
}

// --- sync: broadcast everywhere, browser included ---

#[async_test]
async fn broadcast_works_on_this_target() {
    let (tx, mut rx) = ego_platform::broadcast::channel::<u32>(8);
    let mut rx2 = tx.subscribe();
    tx.send(7).unwrap();
    assert_eq!(rx.recv().await.unwrap(), 7);
    assert_eq!(rx2.recv().await.unwrap(), 7);
}

// --- shutdown ---
// One test: the signal is process-global, so its facets are asserted
// together rather than racing across parallel tests.

use ego_platform::shutdown;

#[async_test]
async fn shutdown_is_level_not_edge() {
    assert!(!shutdown::is_requested());
    shutdown::request();
    assert!(shutdown::is_requested());
    assert_eq!(shutdown::requested().await, shutdown::Reason::Requested);
    // A late subscriber still sees it, and the first reason held.
    shutdown::request();
    assert_eq!(shutdown::requested().await, shutdown::Reason::Requested);
}

// --- env ---

use ego_platform::env::{EnvVar, var};

#[test]
fn env_answers_honestly() {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        assert!(!ego_platform::env::supported());
        assert_eq!(var("PATH"), EnvVar::Unsupported);
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        assert!(ego_platform::env::supported());
        assert_eq!(var("EGO_PLATFORM_SURELY_UNSET"), EnvVar::Unset);
        match var("PATH") {
            EnvVar::Set(_) | EnvVar::Unset => {}
            EnvVar::Unsupported => panic!("this platform has an environment"),
        }
    }
}
