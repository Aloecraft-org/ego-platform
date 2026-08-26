// ego_platform/tests/io_tests.rs

mod common;
use common::{async_test, test};

use ego_platform::io::*;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use tokio::io::AsyncReadExt;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen_test::wasm_bindgen_test]
async fn test_stdin_browser_noop() {
    let mut stdin = stdin();
    let mut buf = [0u8; 10];
    let n = stdin.read(&mut buf).await.unwrap();
    assert_eq!(n, 0);
}

#[test]
fn test_io_creation() {
    let _stdin = stdin();
    let _stdout = stdout();
}
