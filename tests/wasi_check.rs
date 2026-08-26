//! Manual smoke test for async stdin, mainly aimed at WASI P2 where a naive
//! blocking read would starve the single-threaded runtime.
//!
//! Reading stdin requires externally piped input, so this test is ignored by
//! default. Run it manually with:
//!
//! ```sh
//! echo -n hello | cargo test --target wasm32-wasip2 --test wasi_check -- --ignored
//! ```

#[test]
#[ignore = "requires 'hello' piped on stdin; run manually"]
fn wasi_stdin_reads_piped_input() {
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
