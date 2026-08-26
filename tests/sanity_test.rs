mod common;
use ego_platform::time::{SystemTime, UNIX_EPOCH, sleep};
use common::async_test;

#[async_test]
async fn sanity_test() {
    println!("Starting Test");
    println!(
        "tick {}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    sleep(std::time::Duration::from_secs(1)).await;
    println!(
        "tick {}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    sleep(std::time::Duration::from_secs(1)).await;
    println!(
        "tick {}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    sleep(std::time::Duration::from_secs(1)).await;
    println!("done!");
    assert!(true);
}
