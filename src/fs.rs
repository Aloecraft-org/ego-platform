// src/platform/fs.rs

use std::path::Path;
use std::io;

// Common wrapper functions that work on all platforms
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let path = path.as_ref();
    
    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            create_dir_all(parent)?;
        }
    }
    
    std::fs::write(path, contents)
}

pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    match std::fs::create_dir_all(path.as_ref()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    match std::fs::remove_file(path.as_ref()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    match std::fs::metadata(path.as_ref()) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<std::fs::Metadata> {
    std::fs::metadata(path)
}

pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<std::fs::ReadDir> {
    std::fs::read_dir(path)
}

// Re-export types
pub use std::fs::{OpenOptions, File, Metadata, ReadDir, DirEntry};


#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
pub mod wasi_fs {
    use std::path::Path;
    use std::io;
    
    /// Read entire file contents
    pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
        std::fs::read(path.as_ref()).map_err(|e| {
            log::debug!("WASI read failed for {:?}: {:?}", path.as_ref(), e);
            e
        })
    }
    
    /// Write entire file contents
    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
        let path = path.as_ref();
        
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                create_dir_all(parent)?;
            }
        }
        
        log::debug!("WASI writing to {:?}", path);
        std::fs::write(path, contents).map_err(|e| {
            log::error!("WASI write failed for {:?}: {:?}", path, e);
            e
        })
    }
    
    /// Create directory and all parents
    pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
        match std::fs::create_dir_all(path.as_ref()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                Ok(())
            }
            Err(e) => {
                log::debug!("WASI create_dir_all failed for {:?}: {:?}", path.as_ref(), e);
                Err(e)
            }
        }
    }
    
    /// Remove file
    pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
        match std::fs::remove_file(path.as_ref()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Ok(())
            }
            Err(e) => Err(e)
        }
    }
    
    /// Check if path exists (matches std::fs::exists signature from Rust 1.83+)
    pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
        match std::fs::metadata(path.as_ref()) {
            Ok(_) => Ok(true),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e),
        }
    }
    
    /// Get file metadata
    pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<std::fs::Metadata> {
        std::fs::metadata(path)
    }
    
    /// Read directory entries
    pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<std::fs::ReadDir> {
        std::fs::read_dir(path)
    }
    
    pub use std::fs::{OpenOptions, File, Metadata, ReadDir, DirEntry};
}

#[cfg(all(target_arch = "wasm32", target_env = "p2"))]
pub use wasi_fs::*;
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    
    // Counter to ensure unique test paths
    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);
    
    /// Helper to get a platform-appropriate unique test path
    fn test_path(filename: &str) -> String {
        let counter = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let unique_name = format!("{}_{}", counter, filename);
        
        #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
        {
            // WASI: Use relative paths in current directory
            unique_name
        }
        
        #[cfg(not(all(target_arch = "wasm32", target_env = "p2")))]
        {
            // Native: Use /tmp for test isolation
            format!("/tmp/ego2_test_{}", unique_name)
        }
    }
    
    #[test]
    fn test_write_and_read() {
        let path = test_path("write_read.txt");
        let content = b"Hello, WASI!";
        
        // Write
        write(&path, content).expect("Failed to write file");
        
        // Read back
        let read_content = read(&path).expect("Failed to read file");
        assert_eq!(content, read_content.as_slice());
        
        // Cleanup
        remove_file(&path).ok();
    }
    
    #[test]
    fn test_write_with_subdirectory() {
        let path = test_path("subdir/nested/file.txt");
        let content = b"Nested file content";
        
        // Write should auto-create parent directories
        write(&path, content).expect("Failed to write file with subdirs");
        
        // Verify it exists
        assert!(exists(&path).expect("exists() failed"));
        
        // Read back
        let read_content = read(&path).expect("Failed to read nested file");
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
        
        // Ensure it doesn't exist first
        remove_file(&path).ok();
        
        // Removing non-existent file should succeed
        remove_file(&path).expect("First removal should succeed even if file doesn't exist");
        
        // Create file
        write(&path, b"test").expect("Failed to write file");
        
        // Verify it exists
        assert!(exists(&path).expect("exists() check failed"));
        
        // Remove it
        remove_file(&path).expect("Failed to remove existing file");
        
        // Verify it's gone
        assert!(!exists(&path).expect("exists() check failed"));
        
        // Remove again - should still succeed
        remove_file(&path).expect("Second removal should succeed");
    }
    
    #[test]
    fn test_create_dir_all() {
        let dir_path = test_path("dirs/a/b/c");
        
        // Create nested directories
        create_dir_all(&dir_path).expect("Failed to create directories");
        
        // Should be idempotent
        create_dir_all(&dir_path).expect("Second create should succeed");
        
        // Verify by writing a file in it
        let file_path = format!("{}/test.txt", dir_path);
        write(&file_path, b"test").expect("Failed to write to created directory");
        
        // Cleanup
        remove_file(&file_path).ok();
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
    
    #[test]
    fn test_read_dir() {
        let dir_path = test_path("readdir");
        create_dir_all(&dir_path).expect("Failed to create directory");
        
        // Create some files
        let file1 = format!("{}/file1.txt", dir_path);
        let file2 = format!("{}/file2.txt", dir_path);
        write(&file1, b"content1").expect("Failed to write file1");
        write(&file2, b"content2").expect("Failed to write file2");
        
        // Read directory
        let entries: Vec<_> = read_dir(&dir_path)
            .expect("Failed to read directory")
            .filter_map(|e| e.ok())
            .collect();
        
        // Should have exactly 2 entries (our files)
        assert_eq!(entries.len(), 2, "Expected exactly 2 entries, got {}", entries.len());
        
        // Cleanup
        remove_file(&file1).ok();
        remove_file(&file2).ok();
    }
    
    #[test]
    fn test_overwrite_file() {
        let path = test_path("overwrite.txt");
        
        // Write initial content
        write(&path, b"initial").expect("Failed to write initial content");
        
        // Overwrite with new content
        write(&path, b"overwritten").expect("Failed to overwrite");
        
        // Verify new content
        let content = read(&path).expect("Failed to read");
        assert_eq!(content, b"overwritten");
        
        // Cleanup
        remove_file(&path).ok();
    }
    
    #[test]
    fn test_empty_file() {
        let path = test_path("empty.txt");
        
        // Write empty file
        write(&path, b"").expect("Failed to write empty file");
        
        // Read it back
        let content = read(&path).expect("Failed to read empty file");
        assert_eq!(content.len(), 0);
        
        // Metadata should show 0 size
        let meta = metadata(&path).expect("Failed to get metadata");
        assert_eq!(meta.len(), 0);
        
        // Cleanup
        remove_file(&path).ok();
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
    fn test_large_file() {
        let path = test_path("large.bin");
        
        // Write a larger file (1MB)
        let large_data = vec![0xAB; 1024 * 1024];
        write(&path, &large_data).expect("Failed to write large file");
        
        // Verify size
        let meta = metadata(&path).expect("Failed to get metadata");
        assert_eq!(meta.len(), 1024 * 1024);
        
        // Read it back
        let read_data = read(&path).expect("Failed to read large file");
        assert_eq!(large_data.len(), read_data.len());
        
        // Cleanup
        remove_file(&path).ok();
    }
}