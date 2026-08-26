use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use std::io::{self, ErrorKind};
use std::path::Path;
use web_sys::window;

pub const STORAGE_PREFIX: &str = "ego2_fs_";

pub fn get_storage() -> io::Result<web_sys::Storage> {
    window()
        .ok_or_else(|| io::Error::other("No window object"))?
        .local_storage()
        .map_err(|_| io::Error::other("localStorage not available"))?
        .ok_or_else(|| io::Error::other("localStorage is null"))
}

pub fn path_to_key(path: &Path) -> String {
    format!("{}{}", STORAGE_PREFIX, path.to_string_lossy())
}

pub fn meta_key(path: &Path) -> String {
    format!("{}{}.fsmeta", STORAGE_PREFIX, path.to_string_lossy())
}

pub fn now_ms() -> u64 {
    js_sys::Date::now() as u64
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct StoredMeta {
    pub created: u64,
    pub modified: u64,
    pub len: u64,
}

pub fn write_meta(path: &Path, len: u64, created: Option<u64>) -> io::Result<()> {
    let storage = get_storage()?;
    let now = now_ms();
    let meta = StoredMeta {
        created: created.unwrap_or(now),
        modified: now,
        len,
    };
    let json = serde_json::to_string(&meta)
        .map_err(|e| io::Error::other(format!("Meta serialize error: {}", e)))?;
    storage
        .set_item(&meta_key(path), &json)
        .map_err(|_| io::Error::other("Failed to write metadata"))
}

pub struct BrowserMetadata {
    pub len: u64,
    pub created: u64,
    pub modified: u64,
    pub is_file: bool,
}

impl BrowserMetadata {
    pub fn len(&self) -> u64 {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn is_file(&self) -> bool {
        self.is_file
    }
    pub fn is_dir(&self) -> bool {
        !self.is_file
    }
    pub fn created_ms(&self) -> u64 {
        self.created
    }
    pub fn modified_ms(&self) -> u64 {
        self.modified
    }
}

pub fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let storage = get_storage()?;
    let key = path_to_key(path.as_ref());

    let value = storage
        .get_item(&key)
        .map_err(|_| io::Error::other("Failed to read from localStorage"))?
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "File not found"))?;

    BASE64.decode(&value).map_err(|e| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("Base64 decode error: {}", e),
        )
    })
}

pub fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        create_dir_all(parent)?;
    }

    let storage = get_storage()?;
    let key = path_to_key(path);
    let encoded = BASE64.encode(contents.as_ref());
    let len = contents.as_ref().len() as u64;

    let existing_created = storage
        .get_item(&meta_key(path))
        .ok()
        .flatten()
        .and_then(|json| serde_json::from_str::<StoredMeta>(&json).ok())
        .map(|m| m.created);

    storage
        .set_item(&key, &encoded)
        .map_err(|_| io::Error::other("Failed to write to localStorage"))?;

    write_meta(path, len, existing_created)
}

pub fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let storage = get_storage()?;
    let key = format!("{}__dir__", path_to_key(path.as_ref()));
    storage
        .set_item(&key, "")
        .map_err(|_| io::Error::other("Failed to create directory marker"))
}

pub fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let storage = get_storage()?;
    let key = path_to_key(path.as_ref());
    storage
        .remove_item(&key)
        .map_err(|_| io::Error::other("Failed to remove from localStorage"))
}

pub fn exists<P: AsRef<Path>>(path: P) -> io::Result<bool> {
    let storage = get_storage()?;
    let key = path_to_key(path.as_ref());
    storage
        .get_item(&key)
        .map(|v| v.is_some())
        .map_err(|_| io::Error::other("Failed to check localStorage"))
}

pub fn metadata<P: AsRef<Path>>(path: P) -> io::Result<BrowserMetadata> {
    let storage = get_storage()?;
    let path = path.as_ref();
    let key = path_to_key(path);

    storage
        .get_item(&key)
        .map_err(|_| io::Error::other("Failed to read from localStorage"))?
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "File not found"))?;

    let meta_json = storage
        .get_item(&meta_key(path))
        .map_err(|_| io::Error::other("Failed to read metadata"))?
        .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "Metadata not found"))?;

    let stored: StoredMeta = serde_json::from_str(&meta_json).map_err(|e| {
        io::Error::new(
            ErrorKind::InvalidData,
            format!("Meta deserialize error: {}", e),
        )
    })?;

    Ok(BrowserMetadata {
        len: stored.len,
        created: stored.created,
        modified: stored.modified,
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
        .map_err(|_| io::Error::other("Failed to get storage length"))?;

    for i in 0..len {
        if let Ok(Some(key)) = storage.key(i)
            && key.starts_with(&prefix_with_slash)
        {
            let remainder = &key[prefix_with_slash.len()..];
            if !remainder.contains('/')
                && !remainder.ends_with("__dir__")
                && !remainder.ends_with(".fsmeta")
            {
                entries.push(remainder.to_string());
            }
        }
    }

    Ok(entries)
}
