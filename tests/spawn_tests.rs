mod common;
use common::async_test;

use ego_platform::spawn::spawn;
use ego_platform::time::sleep;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[async_test]
async fn test_spawn_with_handle() {
    let handle = spawn(async { 42 });
    let result = handle.join().await.unwrap();
    assert_eq!(result, 42);
}

#[async_test]
async fn test_spawn_executes() {
    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();

    spawn(async move {
        flag_clone.store(true, Ordering::SeqCst);
    });

    sleep(Duration::from_millis(50)).await;
    assert!(flag.load(Ordering::SeqCst));
}

#[async_test]
async fn test_spawn_multiple_tasks() {
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    for _ in 0..10 {
        let counter_clone = counter.clone();
        spawn(async move {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        });
    }

    sleep(tokio::time::Duration::from_millis(100)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}
