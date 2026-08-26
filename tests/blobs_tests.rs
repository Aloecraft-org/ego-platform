mod common;
use common::{async_test, test};

use ego_platform::blobs::{BlobStore, MemStore, valid_key};

#[test]
fn keys_are_validated_identically_everywhere() {
    for good in ["a", "snapshot-42", "agent.7_x", "A-Z.0-9"] {
        assert!(valid_key(good), "{good:?} should be valid");
    }
    for bad in ["", ".hidden", "a/b", "a b", "über", &"x".repeat(256)] {
        assert!(!valid_key(bad), "{bad:?} should be invalid");
    }
}

async fn exercise(store: &dyn BlobStore) {
    // Empty to start (each test uses a fresh namespace).
    assert!(store.list().await.unwrap().is_empty());
    assert_eq!(store.get("k1").await.unwrap(), None);

    // Round trip.
    store.put("k1", vec![1, 2, 3]).await.unwrap();
    assert_eq!(store.get("k1").await.unwrap(), Some(vec![1, 2, 3]));

    // Atomic replacement: the new value, wholly.
    store.put("k1", vec![9; 4096]).await.unwrap();
    assert_eq!(store.get("k1").await.unwrap(), Some(vec![9; 4096]));

    // Listing sees exactly the live keys.
    store.put("k2", b"two".to_vec()).await.unwrap();
    let mut keys = store.list().await.unwrap();
    keys.sort();
    assert_eq!(keys, vec!["k1".to_string(), "k2".to_string()]);

    // Delete is idempotent.
    store.delete("k1").await.unwrap();
    store.delete("k1").await.unwrap();
    assert_eq!(store.get("k1").await.unwrap(), None);

    // Bad keys are refused, not mangled.
    assert!(store.put(".sneaky", vec![0]).await.is_err());
    assert!(store.get("a/b").await.is_err());
}

#[async_test]
async fn mem_store_honors_the_contract() {
    exercise(&MemStore::new()).await;
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
#[async_test]
async fn dir_store_honors_the_contract() {
    use ego_platform::blobs::DirStore;
    use rand_core::Rng;
    // Relative, like fs_tests: the WASI runner preopens only the cwd.
    let dir = std::path::PathBuf::from(format!(
        "ego-blobs-test-{:016x}",
        ego_platform::entropy::SystemEntropy.next_u64()
    ));
    let store = DirStore::open(&dir).unwrap();
    exercise(&store).await;

    // Contents survive reopening: it is a directory, not a session.
    store.put("kept", b"still here".to_vec()).await.unwrap();
    let reopened = DirStore::open(&dir).unwrap();
    assert_eq!(
        reopened.get("kept").await.unwrap(),
        Some(b"still here".to_vec())
    );
    // No temp litter left behind by the atomic writes.
    for name in ego_platform::fs::read_dir_names(&dir) {
        assert!(!name.ends_with(".tmp"), "leftover temp file {name:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[async_test]
async fn idb_store_honors_the_contract() {
    use ego_platform::blobs::IdbStore;
    use rand_core::Rng;
    // A fresh database per run, since IndexedDB outlives the page.
    let store = IdbStore::open(format!(
        "ego-blobs-test-{:016x}",
        ego_platform::entropy::SystemEntropy.next_u64()
    ));
    exercise(&store).await;
}
