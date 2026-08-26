#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
use std::io;
use std::path::Path;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod browser_fs;

pub mod metadata;
pub use metadata::{Metadata, metadata};

// === Native and WASI implementation ===

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        create_dir_all(parent)?;
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
pub fn read_dir<P: AsRef<Path>>(path: P) -> io::Result<std::fs::ReadDir> {
    std::fs::read_dir(path)
}

/// Returns filenames (not full paths) of direct children. Empty vec on error.
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

// === Browser re-exports ===
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use browser_fs::{create_dir_all, exists, read, read_dir, remove_file, write};

// === Native re-exports ===
#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub use std::fs::{DirEntry, File, OpenOptions, ReadDir};
