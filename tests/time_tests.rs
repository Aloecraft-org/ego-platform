mod common;
use common::{test, async_test};

use aloeplatform::time::*;
use std::time::Duration;
use aloeplatform::time::{sleep, Interval, SystemTime, UNIX_EPOCH, Instant};

#[async_test]
async fn test_sleep() {
    let start = Instant::now();
    sleep(Duration::from_millis(100)).await;
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(90));
}


#[async_test]
async fn test_interval() {
    let mut interval = Interval::new(Duration::from_millis(50));
    let start = Instant::now();
    interval.tick().await;
    let first_tick = start.elapsed();
    assert!(first_tick < Duration::from_millis(10));
    let start = Instant::now();
    interval.tick().await;
    let second_tick = start.elapsed();
    assert!(second_tick >= Duration::from_millis(40));
}

#[test]
fn test_system_time() {
    let now = SystemTime::now();
    let duration_since_epoch = now.duration_since(UNIX_EPOCH);
    assert!(duration_since_epoch.is_ok());
    let duration = duration_since_epoch.unwrap();
    assert!(duration.as_secs() > 1_600_000_000);
}

#[test]
fn test_system_time_ordering() {
    let t1 = Instant::now();
    aloeplatform::sleep(Duration::from_millis(10));
    let t2 = Instant::now();
    assert!(t2 > t1);
}

#[async_test]
async fn test_multiple_intervals() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    aloeplatform::spawn(async move {
        let mut interval = Interval::new(Duration::from_millis(25));
        for _ in 0..3 {
            interval.tick().await;
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }
    });

    sleep(Duration::from_millis(150)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 3);
}