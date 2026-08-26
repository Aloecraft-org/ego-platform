mod common;
use common::{async_test, test};

use ego_platform::clock::{Clock, ManualClock, SystemClock};
use ego_platform::time::UNIX_EPOCH;
use std::time::Duration;

#[test]
fn system_clock_marches() {
    let c = SystemClock;
    let a = c.monotonic();
    let b = c.monotonic();
    assert!(b >= a);
    assert!(c.wall().duration_since(UNIX_EPOCH).unwrap().as_secs() > 1_600_000_000);
}

#[async_test]
async fn system_clock_sleep_is_the_platform_sleep() {
    let c = SystemClock;
    let before = c.monotonic();
    c.sleep(Duration::from_millis(50)).await;
    assert!(c.monotonic() - before >= Duration::from_millis(40));
}

#[test]
fn manual_clock_only_moves_when_told() {
    let c = ManualClock::new();
    assert_eq!(c.monotonic(), Duration::ZERO);
    c.advance(Duration::from_secs(60));
    assert_eq!(c.monotonic(), Duration::from_secs(60));
    assert_eq!(
        c.wall().duration_since(UNIX_EPOCH).unwrap(),
        Duration::from_secs(60)
    );
}

#[async_test]
async fn an_hour_of_manual_sleep_takes_no_real_time() {
    let clock = ManualClock::new();
    let sleeper = clock.clone();
    let task = ego_platform::spawn(async move {
        sleeper.sleep(Duration::from_secs(3600)).await;
        "woke"
    });

    // Let the task park on the clock before advancing it.
    let mut tries = 0;
    while clock.sleepers() == 0 && tries < 200 {
        ego_platform::time::sleep(Duration::from_millis(5)).await;
        tries += 1;
    }
    assert_eq!(clock.sleepers(), 1, "the task parked on the manual clock");
    assert!(!task.is_finished(), "no real hour has passed");

    clock.advance(Duration::from_secs(3599));
    ego_platform::time::sleep(Duration::from_millis(20)).await;
    assert!(!task.is_finished(), "3599s is not the deadline");

    clock.advance(Duration::from_secs(1));
    assert_eq!(task.join().await.unwrap(), "woke");
    assert_eq!(clock.sleepers(), 0);
}

#[async_test]
async fn an_elapsed_deadline_resolves_immediately() {
    let clock = ManualClock::new();
    clock.advance(Duration::from_secs(10));
    // A sleep whose deadline is already in the clock's past.
    clock.sleep(Duration::ZERO).await;
}
