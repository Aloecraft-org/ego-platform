// Integration tests for aloeplatform
//
// These tests verify the library works correctly when used as a dependency

use aloeplatform::{detect, init, spawn, sleep, Interval, Platform};
use std::time::Duration;

#[test]
fn test_integration_platform_detection() {
    let platform = detect();
    
    // Should detect one of the three platforms
    match platform {
        Platform::Native | Platform::Wasi | Platform::Browser => (),
    }
}

#[test]
fn test_integration_init() {
    // Should not panic
    init();
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[tokio::test]
async fn test_integration_sleep() {
    init();
    
    let start = std::time::Instant::now();
    sleep(Duration::from_millis(50)).await;
    let elapsed = start.elapsed();
    
    assert!(elapsed >= Duration::from_millis(40));
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[tokio::test]
async fn test_integration_spawn() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    init();
    
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();
    
    spawn(async move {
        flag_clone.store(true, Ordering::SeqCst);
    });
    
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(flag.load(Ordering::SeqCst));
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[tokio::test]
async fn test_integration_interval() {
    init();
    
    let mut interval = Interval::new(Duration::from_millis(25));
    let mut count = 0;
    
    for _ in 0..3 {
        interval.tick().await;
        count += 1;
    }
    
    assert_eq!(count, 3);
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
async fn test_integration_broadcast() {
    use aloeplatform::broadcast;
    
    init();
    
    let (tx, mut rx) = broadcast::channel::<String>(10);
    
    tx.send("test message".to_string()).ok();
    let msg = rx.recv().await.unwrap();
    
    assert_eq!(msg, "test message");
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[tokio::test]
async fn test_integration_complex_workflow() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    init();
    
    let counter = Arc::new(AtomicUsize::new(0));
    
    // Spawn multiple tasks
    for i in 0..5 {
        let counter_clone = counter.clone();
        spawn(async move {
            sleep(Duration::from_millis(i * 10)).await;
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
    }
    
    // Wait for all tasks
    sleep(Duration::from_millis(100)).await;
    
    assert_eq!(counter.load(Ordering::SeqCst), 5);
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_integration_browser_sleep() {
    init();
    
    let start = instant::Instant::now();
    sleep(Duration::from_millis(50)).await;
    let elapsed = start.elapsed();
    
    assert!(elapsed >= Duration::from_millis(40));
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_integration_browser_spawn() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    init();
    
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();
    
    spawn(async move {
        flag_clone.store(true, Ordering::SeqCst);
    });
    
    gloo_timers::future::sleep(Duration::from_millis(50)).await;
    assert!(flag.load(Ordering::SeqCst));
}