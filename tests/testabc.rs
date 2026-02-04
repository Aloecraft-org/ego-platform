#[cfg(target_arch = "wasm32")]
mod testabc {
    // use std::time::SystemTime;

    use aloeplatform::time::{sleep, SystemTime, UNIX_EPOCH};
    #[tokio::test]
    async fn test_wasi() {

        println!("Starting Test");

        println!("tick {}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()); 

        sleep(std::time::Duration::from_secs(1)).await;

        println!("tick {}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()); 

        sleep(std::time::Duration::from_secs(1)).await;

        println!("tick {}", SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()); 

        sleep(std::time::Duration::from_secs(1)).await;

        println!("done!"); 

        assert!(true); 
    }
}