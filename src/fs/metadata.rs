use std::io;
use std::path::Path;

pub struct Metadata {
    pub len: u64,
    pub is_file: bool,
    pub created_ms: Option<u64>,
    pub modified_ms: Option<u64>,
}

impl Metadata {
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_file(&self) -> bool {
        self.is_file
    }
    pub fn is_dir(&self) -> bool {
        !self.is_file
    }
    pub fn created_ms(&self) -> Option<u64> {
        self.created_ms
    }
    pub fn modified_ms(&self) -> Option<u64> {
        self.modified_ms
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl From<super::browser_fs::BrowserMetadata> for Metadata {
    fn from(m: super::browser_fs::BrowserMetadata) -> Self {
        Metadata {
            len: m.len(),
            is_file: m.is_file(),
            created_ms: Some(m.created_ms()),
            modified_ms: Some(m.modified_ms()),
        }
    }
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl From<std::fs::Metadata> for Metadata {
    fn from(m: std::fs::Metadata) -> Self {
        Metadata {
            len: m.len(),
            is_file: m.is_file(),
            created_ms: m
                .created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64),
            modified_ms: m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64),
        }
    }
}

pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<Metadata> {
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    {
        super::browser_fs::metadata(path).map(Into::into)
    }

    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    {
        std::fs::metadata(path).map(Into::into)
    }
}
