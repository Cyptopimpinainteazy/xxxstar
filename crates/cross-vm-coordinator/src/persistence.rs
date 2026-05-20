//! Session persistence layer for SwapCoordinator.
//!
//! Provides abstractions for storing/retrieving SwapSessions so the coordinator
//! state survives node restarts. Two built-in implementations:
//!
//! - `InMemoryPersistence`: HashMap-backed, for tests and short-lived nodes.
//! - `OffchainPersistence`: OffchainStorage-backed, for production nodes.

use crate::types::SwapSession;
use std::collections::HashMap;
#[cfg(feature = "offchain")]
use std::sync::Arc;

/// Persistence abstraction for swap sessions.
///
/// All operations are synchronous. Implementations may internally use
/// locks or atomic writes as appropriate.
pub trait SessionPersistence: Send + Sync + 'static {
    /// Store or update a session.
    fn save(&self, session: &SwapSession);

    /// Load a session by ID. Returns None if not found.
    fn load(&self, session_id: &str) -> Option<SwapSession>;

    /// Remove a session from storage.
    fn remove(&self, session_id: &str);

    /// Load all sessions. Used on startup to restore state.
    fn load_all(&self) -> HashMap<String, SwapSession>;

    /// Return the number of stored sessions.
    fn count(&self) -> usize;

    /// Persist the global set of used HTLC secrets.
    ///
    /// This MUST be called after every secret insertion to prevent
    /// cross-session replay attacks surviving a node restart.
    fn save_used_secrets(&self, secrets: &Vec<[u8; 32]>);

    /// Load the persisted set of used HTLC secrets.
    /// Returns an empty set if nothing was previously persisted.
    fn load_used_secrets(&self) -> Vec<[u8; 32]>;
}

// ─── InMemoryPersistence ──────────────────────────────────────────────────────

/// In-memory persistence backed by a simple HashMap.
///
/// Use for tests or ephemeral nodes where durability isn't needed.
pub struct InMemoryPersistence {
    inner: std::sync::RwLock<HashMap<String, SwapSession>>,
    used_secrets: std::sync::RwLock<Vec<[u8; 32]>>,
}

impl Default for InMemoryPersistence {
    fn default() -> Self {
        Self::new()
    }
}

impl InMemoryPersistence {
    pub fn new() -> Self {
        Self {
            inner: std::sync::RwLock::new(HashMap::new()),
            used_secrets: std::sync::RwLock::new(Vec::new()),
        }
    }
}

impl SessionPersistence for InMemoryPersistence {
    fn save(&self, session: &SwapSession) {
        let mut guard = self.inner.write().unwrap();
        guard.insert(session.session_id.clone(), session.clone());
    }

    fn load(&self, session_id: &str) -> Option<SwapSession> {
        let guard = self.inner.read().unwrap();
        guard.get(session_id).cloned()
    }

    fn remove(&self, session_id: &str) {
        let mut guard = self.inner.write().unwrap();
        guard.remove(session_id);
    }

    fn load_all(&self) -> HashMap<String, SwapSession> {
        let guard = self.inner.read().unwrap();
        guard.clone()
    }

    fn count(&self) -> usize {
        let guard = self.inner.read().unwrap();
        guard.len()
    }

    fn save_used_secrets(&self, secrets: &Vec<[u8; 32]>) {
        let mut guard = self.used_secrets.write().unwrap();
        *guard = secrets.clone();
    }

    fn load_used_secrets(&self) -> Vec<[u8; 32]> {
        let guard = self.used_secrets.read().unwrap();
        guard.clone()
    }
}

// ─── OffchainPersistence ──────────────────────────────────────────────────────

/// Production-grade persistence using Substrate's OffchainStorage.
///
/// Sessions are SCALE-encoded and stored under prefix "x3sess:".
/// Requires `codec` feature on SwapSession (already present via serde).
///
/// # Thread Safety
/// OffchainStorage implementations are inherently thread-safe.
#[cfg(feature = "offchain")]
pub struct OffchainPersistence<O: OffchainStorageProvider> {
    storage_provider: Arc<O>,
}

#[cfg(feature = "offchain")]
pub trait OffchainStorageProvider: Send + Sync + 'static {
    fn set(&self, key: &[u8], value: &[u8]);
    fn get(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn remove(&self, key: &[u8]);
    fn keys_with_prefix(&self, prefix: &[u8]) -> Vec<Vec<u8>>;
}

#[cfg(feature = "offchain")]
impl<O: OffchainStorageProvider> OffchainPersistence<O> {
    const PREFIX: &'static [u8] = b"x3sess:";

    pub fn new(storage_provider: Arc<O>) -> Self {
        Self { storage_provider }
    }

    fn session_key(session_id: &str) -> Vec<u8> {
        let mut key = Self::PREFIX.to_vec();
        key.extend_from_slice(session_id.as_bytes());
        key
    }
}

#[cfg(feature = "offchain")]
impl<O: OffchainStorageProvider> SessionPersistence for OffchainPersistence<O> {
    fn save(&self, session: &SwapSession) {
        let key = Self::session_key(&session.session_id);
        // Use JSON for now since SwapSession has serde. Could switch to SCALE.
        let value = serde_json::to_vec(session).expect("SwapSession serializes");
        self.storage_provider.set(&key, &value);
    }

    fn load(&self, session_id: &str) -> Option<SwapSession> {
        let key = Self::session_key(session_id);
        let bytes = self.storage_provider.get(&key)?;
        serde_json::from_slice(&bytes).ok()
    }

    fn remove(&self, session_id: &str) {
        let key = Self::session_key(session_id);
        self.storage_provider.remove(&key);
    }

    fn load_all(&self) -> HashMap<String, SwapSession> {
        let keys = self.storage_provider.keys_with_prefix(Self::PREFIX);
        let mut result = HashMap::new();
        for key in keys {
            if let Some(bytes) = self.storage_provider.get(&key) {
                if let Ok(session) = serde_json::from_slice::<SwapSession>(&bytes) {
                    result.insert(session.session_id.clone(), session);
                }
            }
        }
        result
    }

    fn count(&self) -> usize {
        self.storage_provider.keys_with_prefix(Self::PREFIX).len()
    }

    fn save_used_secrets(&self, secrets: &Vec<[u8; 32]>) {
        let key = b"x3secrets:used".to_vec();
        let value = serde_json::to_vec(secrets).expect("HashSet serializes");
        self.storage_provider.set(&key, &value);
    }

    fn load_used_secrets(&self) -> Vec<[u8; 32]> {
        let key = b"x3secrets:used".to_vec();
        self.storage_provider
            .get(&key)
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }
}

// ─── Adapter for sc_client_api::OffchainStorage ───────────────────────────────

/// Adapter that wraps `Arc<dyn sc_client_api::OffchainStorage>`.
///
/// Use this in the node service to wire real offchain DB.
#[cfg(feature = "offchain")]
pub struct SubstrateOffchainAdapter<Backend: sp_core::offchain::OffchainStorage> {
    inner: Arc<std::sync::RwLock<Backend>>,
}

#[cfg(feature = "offchain")]
impl<Backend: sp_core::offchain::OffchainStorage> SubstrateOffchainAdapter<Backend> {
    pub fn new(backend: Backend) -> Self {
        Self {
            inner: Arc::new(std::sync::RwLock::new(backend)),
        }
    }

    /// Key used to store the JSON-encoded list of session keys for a prefix.
    fn index_key(prefix: &[u8]) -> Vec<u8> {
        let mut k = b"__idx:".to_vec();
        k.extend_from_slice(prefix);
        k
    }

    /// Add `key` to the prefix-index stored in offchain DB.
    fn add_to_index(storage: &mut Backend, key: &[u8]) {
        let prefix = b"x3sess:" as &[u8]; // matches OffchainPersistence::PREFIX
        let index_key = Self::index_key(prefix);
        let mut keys: Vec<Vec<u8>> = storage
            .get(sp_core::offchain::STORAGE_PREFIX, &index_key)
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let key_vec = key.to_vec();
        if !keys.contains(&key_vec) {
            keys.push(key_vec);
            let encoded = serde_json::to_vec(&keys).expect("index serializes");
            storage.set(sp_core::offchain::STORAGE_PREFIX, &index_key, &encoded);
        }
    }

    /// Remove `key` from the prefix-index.
    fn remove_from_index(storage: &mut Backend, key: &[u8]) {
        let prefix = b"x3sess:" as &[u8];
        let index_key = Self::index_key(prefix);
        if let Some(bytes) = storage.get(sp_core::offchain::STORAGE_PREFIX, &index_key) {
            let mut keys: Vec<Vec<u8>> = serde_json::from_slice(&bytes).unwrap_or_default();
            keys.retain(|k| k != key);
            let encoded = serde_json::to_vec(&keys).expect("index serializes");
            storage.set(sp_core::offchain::STORAGE_PREFIX, &index_key, &encoded);
        }
    }
}

#[cfg(feature = "offchain")]
impl<Backend: sp_core::offchain::OffchainStorage + Send + Sync + 'static> OffchainStorageProvider
    for SubstrateOffchainAdapter<Backend>
{
    fn set(&self, key: &[u8], value: &[u8]) {
        let mut guard = self.inner.write().unwrap();
        // Use PERSISTENT storage so it survives reboots
        guard.set(sp_core::offchain::STORAGE_PREFIX, key, value);
        // Maintain an index of all stored keys under this prefix so that
        // `keys_with_prefix` can enumerate them after restart.
        Self::add_to_index(&mut *guard, key);
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        let guard = self.inner.read().unwrap();
        guard.get(sp_core::offchain::STORAGE_PREFIX, key)
    }

    fn remove(&self, key: &[u8]) {
        let mut guard = self.inner.write().unwrap();
        guard.remove(sp_core::offchain::STORAGE_PREFIX, key);
        Self::remove_from_index(&mut *guard, key);
    }

    fn keys_with_prefix(&self, prefix: &[u8]) -> Vec<Vec<u8>> {
        let guard = self.inner.read().unwrap();
        let index_key = Self::index_key(prefix);
        match guard.get(sp_core::offchain::STORAGE_PREFIX, &index_key) {
            Some(bytes) => serde_json::from_slice::<Vec<Vec<u8>>>(&bytes).unwrap_or_default(),
            None => vec![],
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HtlcHash, SwapPhase};

    fn make_test_session(id: &str) -> SwapSession {
        SwapSession {
            session_id: id.to_string(),
            hash_lock: HtlcHash([0u8; 32]),
            htlc_fast: None,
            htlc_slow: None,
            flash_legs: vec![],
            leg_outcomes: vec![],
            phase: SwapPhase::Setup,
            timelock_fast: 1000,
            timelock_slow: 2000,
            created_at: 123456,
            updated_at: 123456,
            requires_merkle_verification: false,
        }
    }

    #[test]
    fn inmemory_persistence_roundtrip() {
        let persistence = InMemoryPersistence::new();

        let session = make_test_session("swap-abc123");
        persistence.save(&session);

        assert_eq!(persistence.count(), 1);

        let loaded = persistence.load("swap-abc123").unwrap();
        assert_eq!(loaded.session_id, "swap-abc123");
        assert_eq!(loaded.timelock_fast, 1000);

        persistence.remove("swap-abc123");
        assert!(persistence.load("swap-abc123").is_none());
        assert_eq!(persistence.count(), 0);
    }

    #[test]
    fn inmemory_persistence_load_all() {
        let persistence = InMemoryPersistence::new();

        persistence.save(&make_test_session("swap-001"));
        persistence.save(&make_test_session("swap-002"));
        persistence.save(&make_test_session("swap-003"));

        let all = persistence.load_all();
        assert_eq!(all.len(), 3);
        assert!(all.contains_key("swap-001"));
        assert!(all.contains_key("swap-002"));
        assert!(all.contains_key("swap-003"));
    }

    #[test]
    fn inmemory_used_secrets_roundtrip() {
        let persistence = InMemoryPersistence::new();

        let secrets = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        persistence.save_used_secrets(&secrets);

        let loaded = persistence.load_used_secrets();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0], [1u8; 32]);
        assert_eq!(loaded[1], [2u8; 32]);
        assert_eq!(loaded[2], [3u8; 32]);
    }

    #[test]
    fn inmemory_used_secrets_empty_default() {
        let persistence = InMemoryPersistence::new();
        let loaded = persistence.load_used_secrets();
        assert!(loaded.is_empty());
    }

    // ─── OffchainPersistence tests (feature = "offchain") ─────────────────

    #[cfg(feature = "offchain")]
    mod offchain_tests {
        use super::*;
        use std::sync::RwLock;

        /// Mock OffchainStorageProvider backed by a HashMap with true prefix scan.
        struct MockOffchainStorage {
            store: RwLock<HashMap<Vec<u8>, Vec<u8>>>,
        }

        impl MockOffchainStorage {
            fn new() -> Self {
                Self {
                    store: RwLock::new(HashMap::new()),
                }
            }
        }

        impl OffchainStorageProvider for MockOffchainStorage {
            fn set(&self, key: &[u8], value: &[u8]) {
                let mut guard = self.store.write().unwrap();
                guard.insert(key.to_vec(), value.to_vec());
            }

            fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
                let guard = self.store.read().unwrap();
                guard.get(key).cloned()
            }

            fn remove(&self, key: &[u8]) {
                let mut guard = self.store.write().unwrap();
                guard.remove(key);
            }

            fn keys_with_prefix(&self, prefix: &[u8]) -> Vec<Vec<u8>> {
                let guard = self.store.read().unwrap();
                guard
                    .keys()
                    .filter(|k| k.starts_with(prefix))
                    .cloned()
                    .collect()
            }
        }

        fn make_persistence() -> OffchainPersistence<MockOffchainStorage> {
            OffchainPersistence::new(Arc::new(MockOffchainStorage::new()))
        }

        #[test]
        fn offchain_save_load_roundtrip() {
            let p = make_persistence();
            let session = make_test_session("oc-swap-001");
            p.save(&session);

            let loaded = p.load("oc-swap-001").unwrap();
            assert_eq!(loaded.session_id, "oc-swap-001");
            assert_eq!(loaded.timelock_fast, 1000);
            assert_eq!(loaded.timelock_slow, 2000);
        }

        #[test]
        fn offchain_load_nonexistent_returns_none() {
            let p = make_persistence();
            assert!(p.load("does-not-exist").is_none());
        }

        #[test]
        fn offchain_remove_session() {
            let p = make_persistence();
            p.save(&make_test_session("oc-swap-rm"));

            assert!(p.load("oc-swap-rm").is_some());
            p.remove("oc-swap-rm");
            assert!(p.load("oc-swap-rm").is_none());
        }

        #[test]
        fn offchain_remove_nonexistent_noop() {
            let p = make_persistence();
            p.remove("never-existed"); // should not panic
            assert_eq!(p.count(), 0);
        }

        #[test]
        fn offchain_count() {
            let p = make_persistence();
            assert_eq!(p.count(), 0);

            p.save(&make_test_session("oc-001"));
            assert_eq!(p.count(), 1);

            p.save(&make_test_session("oc-002"));
            p.save(&make_test_session("oc-003"));
            assert_eq!(p.count(), 3);

            p.remove("oc-002");
            assert_eq!(p.count(), 2);
        }

        #[test]
        fn offchain_load_all() {
            let p = make_persistence();
            p.save(&make_test_session("oc-a"));
            p.save(&make_test_session("oc-b"));
            p.save(&make_test_session("oc-c"));

            let all = p.load_all();
            assert_eq!(all.len(), 3);
            assert!(all.contains_key("oc-a"));
            assert!(all.contains_key("oc-b"));
            assert!(all.contains_key("oc-c"));
        }

        #[test]
        fn offchain_save_overwrites_existing() {
            let p = make_persistence();
            let mut session = make_test_session("oc-overwrite");
            session.timelock_fast = 5000;
            p.save(&session);

            session.timelock_fast = 9999;
            p.save(&session);

            let loaded = p.load("oc-overwrite").unwrap();
            assert_eq!(loaded.timelock_fast, 9999);
            assert_eq!(p.count(), 1);
        }

        #[test]
        fn offchain_used_secrets_roundtrip() {
            let p = make_persistence();
            let secrets = vec![[10u8; 32], [20u8; 32]];
            p.save_used_secrets(&secrets);

            let loaded = p.load_used_secrets();
            assert_eq!(loaded.len(), 2);
            assert_eq!(loaded[0], [10u8; 32]);
            assert_eq!(loaded[1], [20u8; 32]);
        }

        #[test]
        fn offchain_used_secrets_empty_default() {
            let p = make_persistence();
            let loaded = p.load_used_secrets();
            assert!(loaded.is_empty());
        }

        #[test]
        fn offchain_secrets_key_does_not_collide_with_sessions() {
            let p = make_persistence();
            p.save(&make_test_session("oc-secrets-test"));
            p.save_used_secrets(&vec![[42u8; 32]]);

            // Sessions and secrets should not interfere
            assert_eq!(p.count(), 1); // count only sessions, not secrets
            assert_eq!(p.load_used_secrets().len(), 1);
            let loaded = p.load("oc-secrets-test").unwrap();
            assert_eq!(loaded.session_id, "oc-secrets-test");
        }
    }
}
