//! Pluggable key storage backends.
//!
//! Provides a [`KeyStore`] trait for abstracting key persistence and two
//! implementations:
//! - [`InMemoryKeyStore`]: Thread-safe in-memory store with zeroize-on-drop.
//! - [`HsmKeyStore`]: Placeholder stub for future HSM integration (Phase 5).

use std::collections::HashMap;
use std::sync::RwLock;

use zeroize::Zeroize;

use crate::{CryptoError, Result};

/// Trait for key storage backends.
///
/// All implementations must be `Send + Sync` so they can be shared across
/// async tasks and threads.
pub trait KeyStore: Send + Sync {
    /// Stores key material under the given identifier.
    ///
    /// If a key with the same `id` already exists, it is overwritten.
    fn store_key(&self, id: &str, key_data: &[u8]) -> Result<()>;

    /// Loads key material by identifier.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::KeyNotFound` if no key exists with the given `id`.
    fn load_key(&self, id: &str) -> Result<Vec<u8>>;

    /// Deletes key material by identifier.
    ///
    /// # Errors
    ///
    /// Returns `CryptoError::KeyNotFound` if no key exists with the given `id`.
    fn delete_key(&self, id: &str) -> Result<()>;

    /// Lists all stored key identifiers.
    fn list_keys(&self) -> Result<Vec<String>>;
}

/// Thread-safe in-memory key store.
///
/// Keys are stored in a `HashMap` protected by a `RwLock`. All key material
/// is zeroized when entries are removed or when the store is dropped.
pub struct InMemoryKeyStore {
    keys: RwLock<HashMap<String, Vec<u8>>>,
}

impl InMemoryKeyStore {
    /// Creates a new, empty in-memory key store.
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for InMemoryKeyStore {
    fn drop(&mut self) {
        if let Ok(mut map) = self.keys.write() {
            for value in map.values_mut() {
                value.zeroize();
            }
            map.clear();
        }
    }
}

impl KeyStore for InMemoryKeyStore {
    fn store_key(&self, id: &str, key_data: &[u8]) -> Result<()> {
        let mut map = self
            .keys
            .write()
            .map_err(|e| CryptoError::KeyGenerationFailed(format!("lock poisoned: {e}")))?;

        // Zeroize any existing key material before overwriting.
        if let Some(old) = map.get_mut(id) {
            old.zeroize();
        }

        map.insert(id.to_string(), key_data.to_vec());
        Ok(())
    }

    fn load_key(&self, id: &str) -> Result<Vec<u8>> {
        let map = self
            .keys
            .read()
            .map_err(|e| CryptoError::KeyNotFound(format!("lock poisoned: {e}")))?;

        map.get(id)
            .cloned()
            .ok_or_else(|| CryptoError::KeyNotFound(id.to_string()))
    }

    fn delete_key(&self, id: &str) -> Result<()> {
        let mut map = self
            .keys
            .write()
            .map_err(|e| CryptoError::KeyNotFound(format!("lock poisoned: {e}")))?;

        match map.get_mut(id) {
            Some(key_data) => {
                key_data.zeroize();
                map.remove(id);
                Ok(())
            }
            None => Err(CryptoError::KeyNotFound(id.to_string())),
        }
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        let map = self
            .keys
            .read()
            .map_err(|e| CryptoError::KeyNotFound(format!("lock poisoned: {e}")))?;

        Ok(map.keys().cloned().collect())
    }
}

/// Placeholder HSM key store for future hardware security module integration.
///
/// All operations currently return `CryptoError::HsmUnavailable`.
/// This will be implemented in Phase 5 of the Califax VPN roadmap.
pub struct HsmKeyStore;

impl HsmKeyStore {
    /// Creates a new HSM key store stub.
    pub fn new() -> Self {
        Self
    }
}

impl Default for HsmKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyStore for HsmKeyStore {
    fn store_key(&self, _id: &str, _key_data: &[u8]) -> Result<()> {
        Err(CryptoError::HsmUnavailable(
            "HSM integration not yet implemented (Phase 5)".into(),
        ))
    }

    fn load_key(&self, _id: &str) -> Result<Vec<u8>> {
        Err(CryptoError::HsmUnavailable(
            "HSM integration not yet implemented (Phase 5)".into(),
        ))
    }

    fn delete_key(&self, _id: &str) -> Result<()> {
        Err(CryptoError::HsmUnavailable(
            "HSM integration not yet implemented (Phase 5)".into(),
        ))
    }

    fn list_keys(&self) -> Result<Vec<String>> {
        Err(CryptoError::HsmUnavailable(
            "HSM integration not yet implemented (Phase 5)".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_store_roundtrip() {
        let store = InMemoryKeyStore::new();
        let key_data = vec![0xDE, 0xAD, 0xBE, 0xEF];

        store.store_key("test-key", &key_data).expect("store failed");
        let loaded = store.load_key("test-key").expect("load failed");
        assert_eq!(loaded, key_data);
    }

    #[test]
    fn test_in_memory_store_delete() {
        let store = InMemoryKeyStore::new();
        store.store_key("ephemeral", &[1, 2, 3]).expect("store failed");
        store.delete_key("ephemeral").expect("delete failed");
        assert!(store.load_key("ephemeral").is_err());
    }

    #[test]
    fn test_in_memory_store_list() {
        let store = InMemoryKeyStore::new();
        store.store_key("key-a", &[1]).expect("store failed");
        store.store_key("key-b", &[2]).expect("store failed");

        let mut keys = store.list_keys().expect("list failed");
        keys.sort();
        assert_eq!(keys, vec!["key-a", "key-b"]);
    }

    #[test]
    fn test_key_not_found() {
        let store = InMemoryKeyStore::new();
        let result = store.load_key("nonexistent");
        assert!(matches!(result, Err(CryptoError::KeyNotFound(_))));
    }

    #[test]
    fn test_hsm_unavailable() {
        let store = HsmKeyStore::new();
        assert!(matches!(
            store.store_key("k", &[]),
            Err(CryptoError::HsmUnavailable(_))
        ));
        assert!(matches!(
            store.load_key("k"),
            Err(CryptoError::HsmUnavailable(_))
        ));
        assert!(matches!(
            store.delete_key("k"),
            Err(CryptoError::HsmUnavailable(_))
        ));
        assert!(matches!(
            store.list_keys(),
            Err(CryptoError::HsmUnavailable(_))
        ));
    }
}
