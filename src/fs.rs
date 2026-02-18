// aloeplatform/src/fs.rs

use std::path::Path;
use std::io;

// === Browser-specific implementation ===
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod browser_fs {
    use std::path::Path;
    use std::io::{self, ErrorKind};
    use web_sys::window;
    
    const STORAGE_PREFIX: &str = "ego2_fs_";
    
    fn get_storage() -> io::Result<web_sys::Storage> {
        window()
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "No window object"))?
            .local_storage()
            .map_err(|_| io::Error::new(ErrorKind::Other, "localStorage not available"))?
            .ok_or_else(|| io::Error::new(ErrorKind::Other, "localStorage is null"))
    }
    
    fn path_to_key(path: &Path) -> String {
        format!("{}{}", STORAGE_PREFIX, path.to_string_lossy())
    }
    
    pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
        let storage = get_storage()?;
        let key = path_to_key(path.as_ref());
        
        let value = storage
            .get_item(&key)
            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to read from localStorage"))?
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "File not found"))?;
        
        // Decode from base64 to support binary data
        base64::decode(&value)
            .map_err(|e| io::Error::new(ErrorKind::InvalidData, format!("Base64 decode error: {}", e)))
    }
    
    pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
        let path = path.as_ref();
        
        // Create parent "directories" by just storing a marker
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                create_dir_all(parent)?;
            }
        }
        
        let storage = get_storage()?;
        let key = path_to_key(path);
        
        // Encode as base64 to support binary data
        let encoded = base64::encode(contents.as_ref());
        
        storage
            .set_item(&key, &encoded)
            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to write to localStorage"))
    }
    
    pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
        let storage = get_storage()?;
        let key = format!("{}__dir__", path_to_key(path.as_ref()));
        
        // Just mark it as a directory
        storage
            .set_item(&key, "")
            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to create directory marker"))?;
        
        Ok(())
    }
    
    pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
        let storage = get_storage()?;
        let key = path_to_key(path.as_ref());
        
        storage
            .remove_item(&key)
            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to remove from localStorage"))?;
        
        Ok(())
    }
    
    pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
        let storage = get_storage()?;
        let key = path_to_key(path.as_ref());
        
        let exists = storage
            .get_item(&key)
            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to check localStorage"))?
            .is_some();
        
        Ok(exists)
    }
    
    pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<BrowserMetadata> {
        let storage = get_storage()?;
        let key = path_to_key(path.as_ref());
        
        let value = storage
            .get_item(&key)
            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to read from localStorage"))?
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "File not found"))?;
        
        // Decode to get actual size
        let decoded = base64::decode(&value)
            .map_err(|e| io::Error::new(ErrorKind::InvalidData, format!("Base64 decode error: {}", e)))?;
        
        Ok(BrowserMetadata {
            len: decoded.len() as u64,
            is_file: !key.ends_with("__dir__"),
        })
    }
    
    pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
        let storage = get_storage()?;
        let prefix = path_to_key(path.as_ref());
        let prefix_with_slash = if prefix.is_empty() {
            STORAGE_PREFIX.to_string()
        } else {
            format!("{}/", prefix)
        };
        
        let mut entries = Vec::new();
        let len = storage
            .length()
            .map_err(|_| io::Error::new(ErrorKind::Other, "Failed to get storage length"))?;
        
        for i in 0..len {
            if let Ok(Some(key)) = storage.key(i) {
                if key.starts_with(&prefix_with_slash) {
                    let remainder = &key[prefix_with_slash.len()..];
                    // Only include direct children (not nested)
                    if !remainder.contains('/') && !remainder.ends_with("__dir__") {
                        entries.push(remainder.to_string());
                    }
                }
            }
        }
        
        Ok(entries)
    }
    
    // Simple metadata struct for browser
    pub struct BrowserMetadata {
        len: u64,
        is_file: bool,
    }
    
    impl BrowserMetadata {
        pub fn len(&self) -> u64 {
            self.len
        }
        
        pub fn is_file(&self) -> bool {
            self.is_file
        }
        
        pub fn is_dir(&self) -> bool {
            !self.is_file
        }
    }
}

// === Native and WASI implementation ===
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
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

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    match std::fs::create_dir_all(path.as_ref()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    match std::fs::remove_file(path.as_ref()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    match std::fs::metadata(path.as_ref()) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<std::fs::Metadata> {
    std::fs::metadata(path)
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<std::fs::ReadDir> {
    std::fs::read_dir(path)
}

/// Returns the filenames (not full paths) of direct children in a directory.
/// Returns an empty Vec if the directory does not exist or cannot be read.
/// Available on all targets.
pub fn read_dir_names<P: AsRef<Path>>(path: P) -> Vec<String> {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        browser_fs::read_dir(path).unwrap_or_default()
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        match std::fs::read_dir(path) {
            Ok(iter) => iter
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect(),
            Err(_) => Vec::new(),
        }
    }
}

// Re-export platform-specific types
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use browser_fs::{BrowserMetadata as Metadata, read_dir};

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use browser_fs::{read, write, create_dir_all, remove_file, exists, metadata};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use std::fs::{OpenOptions, File, Metadata, ReadDir, DirEntry};

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
        
        #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
        {
            // Browser: Use simple paths (stored in localStorage)
            unique_name
        }
        
        #[cfg(all(target_arch = "wasm32", target_env = "p2"))]
        {
            // WASI: Use relative paths in current directory
            unique_name
        }
        
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Native: Use /tmp for test isolation
            format!("/tmp/ego2_test_{}", unique_name)
        }
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
}