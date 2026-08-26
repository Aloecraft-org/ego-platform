fn main() {
    // Manually build a current_thread runtime for WASI P2 compatibility
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            use tokio::io::AsyncReadExt;

            let mut buf = String::new();
            println!("Reading stdin...");

            // This blocks until EOF or data is received
            ego_platform::stdin()
                .read_to_string(&mut buf)
                .await
                .unwrap();

            assert_eq!(buf, "hello");
        });
}
