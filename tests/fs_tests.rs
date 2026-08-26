// ego_platform/tests/fs_tests.rs

mod common;
use common::{async_test, test};

use ego_platform::fs::*;
use std::sync::atomic::{AtomicU64, Ordering};

// Counter to ensure unique test paths
static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Helper to get a platform-appropriate unique test path
fn test_path(filename: &str) -> String {
    let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let unique_name = format!("{}_{}", counter, filename);
    unique_name
}

#[test]
fn test_write_and_read() {
    let path = test_path("write_read.txt");
    let content = b"Hello, platform!";

    // Write
    write(&path, content).expect("Failed to write file");

    // Read back
    let read_content = read(&path).expect("Failed to read file");
    assert_eq!(content, read_content.as_slice());

    // Cleanup
    remove_file(&path).ok();
}

#[test]
fn test_exists() {
    let path = test_path("exists.txt");

    // Should not exist initially
    assert!(!exists(&path).expect("exists() check failed"));

    // Create it
    write(&path, b"test").expect("Failed to write file");

    // Now should exist
    assert!(exists(&path).expect("exists() check failed"));

    // Cleanup
    remove_file(&path).expect("Failed to remove file");

    // Should not exist after removal
    assert!(!exists(&path).expect("exists() check failed"));
}

#[test]
fn test_remove_file_idempotent() {
    let path = test_path("remove.txt");

    // Removing non-existent file should succeed (idempotent behavior)
    remove_file(&path).expect("First removal should succeed even if file doesn't exist");

    // Create file
    write(&path, b"test").expect("Failed to write file");

    // Verify it exists
    assert!(exists(&path).expect("exists() check failed"));

    // Remove it
    remove_file(&path).expect("Failed to remove existing file");

    // Verify it's gone
    assert!(!exists(&path).expect("exists() check failed"));

    // Remove again - should still succeed (idempotent)
    remove_file(&path).expect("Second removal should succeed");
}

#[test]
fn test_binary_content() {
    let path = test_path("binary.bin");

    // Write binary content (not valid UTF-8)
    let binary_data: Vec<u8> = (0..=255).collect();
    write(&path, &binary_data).expect("Failed to write binary data");

    // Read it back
    let read_data = read(&path).expect("Failed to read binary data");
    assert_eq!(binary_data, read_data);

    // Cleanup
    remove_file(&path).ok();
}

#[test]
fn test_metadata() {
    let path = test_path("metadata.txt");
    let content = b"Hello, metadata!";

    // Write file
    write(&path, content).expect("Failed to write file");

    // Get metadata
    let meta = metadata(&path).expect("Failed to get metadata");

    // Verify it's a file
    assert!(meta.is_file());
    assert!(!meta.is_dir());

    // Verify size matches
    assert_eq!(meta.len(), content.len() as u64);

    // Cleanup
    remove_file(&path).ok();
}
