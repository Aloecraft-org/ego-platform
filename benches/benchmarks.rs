// Benchmarks for aloeplatform
//
// Run with: cargo bench

mod common;
use common::{test, async_test};
use aloeplatform::{sleep, spawn, Interval, Instant};
use std::time::Duration;

#[async_test]
async fn bench_sleep_accuracy() {
    let durations = vec![
        Duration::from_millis(1),
        Duration::from_millis(10),
        Duration::from_millis(50),
        Duration::from_millis(100),
    ];

    for target_duration in durations {
        let start = Instant::now();
        sleep(target_duration).await;
        let actual_duration = start.elapsed();

        let overhead = actual_duration.saturating_sub(target_duration);
        println!(
            "Sleep target: {:?}, actual: {:?}, overhead: {:?}",
            target_duration, actual_duration, overhead
        );

        // Sleep should be reasonably accurate (within 20ms overhead)
        assert!(overhead < Duration::from_millis(20));
    }
}

#[async_test]
async fn bench_spawn_throughput() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let counter = Arc::new(AtomicUsize::new(0));
    let num_tasks = 1000;

    let start = Instant::now();

    for _ in 0..num_tasks {
        let counter_clone = counter.clone();
        spawn(async move {
            counter_clone.fetch_add(1, Ordering::Relaxed);
        });
    }

    // Wait for all tasks
    while counter.load(Ordering::Relaxed) < num_tasks {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }

    let elapsed = start.elapsed();

    println!(
        "Spawned {} tasks in {:?} ({:.2} tasks/ms)",
        num_tasks,
        elapsed,
        num_tasks as f64 / elapsed.as_millis() as f64
    );

    assert_eq!(counter.load(Ordering::Relaxed), num_tasks);
}

#[async_test]
async fn bench_interval_accuracy() {
    let interval_duration = Duration::from_millis(10);
    let mut interval = Interval::new(interval_duration);
    let num_ticks = 10;

    // Skip first tick (completes immediately)
    interval.tick().await;

    let start = Instant::now();

    for _ in 0..num_ticks {
        interval.tick().await;
    }

    let elapsed = start.elapsed();
    let expected = interval_duration * num_ticks;
    let overhead_per_tick = elapsed
        .saturating_sub(expected)
        .as_micros() as f64 / num_ticks as f64;

    println!(
        "Interval: {} ticks of {:?}, expected: {:?}, actual: {:?}, overhead/tick: {:.2}µs",
        num_ticks, interval_duration, expected, elapsed, overhead_per_tick
    );

    // Should be within reasonable accuracy
    assert!(elapsed >= expected);
    assert!(elapsed < expected + Duration::from_millis(50));
}

#[async_test]
async fn bench_broadcast_latency() {
    use aloeplatform::broadcast;

    let (tx, mut rx) = broadcast::channel::<u64>(100);
    let num_messages = 100;

    let start = Instant::now();

    for i in 0..num_messages {
        tx.send(i).ok();
    }

    for expected in 0..num_messages {
        let received = rx.recv().await.unwrap();
        assert_eq!(received, expected);
    }

    let elapsed = start.elapsed();

    println!(
        "Broadcast: {} messages in {:?} ({:.2} µs/message)",
        num_messages,
        elapsed,
        elapsed.as_micros() as f64 / num_messages as f64
    );
}