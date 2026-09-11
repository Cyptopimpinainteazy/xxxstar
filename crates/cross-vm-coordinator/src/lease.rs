//! Per-session lease sharding and fencing tokens for concurrent relayers.
//!
//! External work may outlive a local mutex guard. A fencing token lets the
//! coordinator reject a stale worker after lease expiry/takeover.

use crate::CoordinatorError;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

const DEFAULT_SHARDS: usize = 64;

/// Durable CAS store used by distributed lease authorities.
pub trait DistributedLeaseStore: Send + Sync + 'static {
    fn load(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn compare_and_set(
        &self,
        key: &[u8],
        old_value: Option<&[u8]>,
        new_value: &[u8],
    ) -> bool;
}

/// In-memory shared CAS store used by tests and single-process simulations.
#[derive(Default)]
pub struct InMemoryDistributedLeaseStore {
    inner: Mutex<HashMap<Vec<u8>, Vec<u8>>>,
}

impl DistributedLeaseStore for InMemoryDistributedLeaseStore {
    fn load(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.inner.lock().ok()?.get(key).cloned()
    }

    fn compare_and_set(
        &self,
        key: &[u8],
        old_value: Option<&[u8]>,
        new_value: &[u8],
    ) -> bool {
        let Ok(mut guard) = self.inner.lock() else {
            return false;
        };
        let current = guard.get(key).map(Vec::as_slice);
        if current != old_value {
            return false;
        }
        guard.insert(key.to_vec(), new_value.to_vec());
        true
    }
}

#[cfg(feature = "offchain")]
pub struct OffchainDistributedLeaseStore<O: crate::persistence::OffchainStorageProvider> {
    provider: Arc<O>,
}

#[cfg(feature = "offchain")]
impl<O: crate::persistence::OffchainStorageProvider> OffchainDistributedLeaseStore<O> {
    pub fn new(provider: Arc<O>) -> Self {
        Self { provider }
    }
}

#[cfg(feature = "offchain")]
impl<O: crate::persistence::OffchainStorageProvider> DistributedLeaseStore
    for OffchainDistributedLeaseStore<O>
{
    fn load(&self, key: &[u8]) -> Option<Vec<u8>> {
        self.provider.get(key)
    }

    fn compare_and_set(
        &self,
        key: &[u8],
        old_value: Option<&[u8]>,
        new_value: &[u8],
    ) -> bool {
        self.provider.compare_and_set(key, old_value, new_value)
    }
}


/// Distributed secret-ownership registry backed by the same CAS store as leases.
///
/// This closes the cross-session race where two independent coordinator
/// processes could otherwise update a last-write-wins used-secret vector.
#[derive(Clone)]
pub struct DurableSecretRegistry<S: DistributedLeaseStore> {
    store: Arc<S>,
}

impl<S: DistributedLeaseStore> DurableSecretRegistry<S> {
    const PREFIX: &'static [u8] = b"x3secret:";

    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    fn key(secret_hash: [u8; 32]) -> Vec<u8> {
        let mut key = Self::PREFIX.to_vec();
        key.extend_from_slice(&secret_hash);
        key
    }

    /// Claim a secret hash for exactly one session.
    ///
    /// Same-session retries are idempotent. Any other owner is rejected.
    pub fn claim(
        &self,
        secret_hash: [u8; 32],
        session_id: &str,
    ) -> Result<(), CoordinatorError> {
        let key = Self::key(secret_hash);
        let owner = session_id.as_bytes();

        loop {
            match self.store.load(&key) {
                Some(existing) if existing == owner => return Ok(()),
                Some(existing) => {
                    return Err(CoordinatorError::Internal(format!(
                        "distributed secret replay: hash already owned by session '{}'",
                        String::from_utf8_lossy(&existing)
                    )));
                }
                None => {
                    if self.store.compare_and_set(&key, None, owner) {
                        return Ok(());
                    }
                }
            }
        }
    }

    pub fn owner(&self, secret_hash: [u8; 32]) -> Option<String> {
        self.store
            .load(&Self::key(secret_hash))
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DurableLeaseState {
    owner_id: String,
    fence: u64,
    expires_at: u64,
}

/// Shared durable lease authority backed by atomic compare-and-swap storage.
///
/// Independent coordinator processes using the same store observe one
/// authoritative lease/fencing epoch per session.
#[derive(Clone)]
pub struct DurableLeaseAuthority<S: DistributedLeaseStore> {
    store: Arc<S>,
}

impl<S: DistributedLeaseStore> DurableLeaseAuthority<S> {
    const PREFIX: &'static [u8] = b"x3lease:";
    const MAX_CAS_RETRIES: usize = 64;

    pub fn new(store: Arc<S>) -> Self {
        Self { store }
    }

    fn key(session_id: &str) -> Vec<u8> {
        let mut key = Self::PREFIX.to_vec();
        key.extend_from_slice(session_id.as_bytes());
        key
    }

    fn decode(raw: &[u8]) -> Result<DurableLeaseState, CoordinatorError> {
        serde_json::from_slice(raw).map_err(|e| {
            CoordinatorError::Internal(format!("invalid durable lease state: {e}"))
        })
    }

    fn encode(state: &DurableLeaseState) -> Result<Vec<u8>, CoordinatorError> {
        serde_json::to_vec(state).map_err(|e| {
            CoordinatorError::Internal(format!("failed to encode durable lease state: {e}"))
        })
    }

    pub fn acquire(
        &self,
        session_id: &str,
        owner_id: &str,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        if owner_id.is_empty() || ttl_secs == 0 {
            return Err(CoordinatorError::Internal(
                "lease owner and ttl must be non-empty/non-zero".to_string(),
            ));
        }
        let expires_at = now_unix
            .checked_add(ttl_secs)
            .ok_or_else(|| CoordinatorError::Internal("lease expiry overflow".to_string()))?;
        let key = Self::key(session_id);

        for _ in 0..Self::MAX_CAS_RETRIES {
            let old = self.store.load(&key);
            let next = match old.as_deref() {
                None => DurableLeaseState {
                    owner_id: owner_id.to_string(),
                    fence: 1,
                    expires_at,
                },
                Some(raw) => {
                    let current = Self::decode(raw)?;
                    if current.owner_id == owner_id && now_unix < current.expires_at {
                        DurableLeaseState {
                            owner_id: owner_id.to_string(),
                            fence: current.fence,
                            expires_at,
                        }
                    } else if now_unix >= current.expires_at {
                        DurableLeaseState {
                            owner_id: owner_id.to_string(),
                            fence: current.fence.checked_add(1).ok_or_else(|| {
                                CoordinatorError::Internal("lease fence exhausted".to_string())
                            })?,
                            expires_at,
                        }
                    } else {
                        return Err(CoordinatorError::Internal(format!(
                            "session '{session_id}' leased by '{}' until {}",
                            current.owner_id, current.expires_at
                        )));
                    }
                }
            };

            let encoded = Self::encode(&next)?;
            if self
                .store
                .compare_and_set(&key, old.as_deref(), &encoded)
            {
                return Ok(SessionLease {
                    session_id: session_id.to_string(),
                    owner_id: owner_id.to_string(),
                    fence: next.fence,
                    expires_at: next.expires_at,
                });
            }
        }

        Err(CoordinatorError::Internal(format!(
            "durable lease CAS contention exceeded retry budget for session '{session_id}'"
        )))
    }

    pub fn validate(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        let key = Self::key(&lease.session_id);
        let raw = self.store.load(&key).ok_or_else(|| {
            CoordinatorError::Internal(format!(
                "no durable lease for session '{}'",
                lease.session_id
            ))
        })?;
        let current = Self::decode(&raw)?;
        if current.owner_id != lease.owner_id || current.fence != lease.fence {
            return Err(CoordinatorError::Internal(format!(
                "stale distributed fencing token for session '{}'",
                lease.session_id
            )));
        }
        if now_unix >= current.expires_at {
            return Err(CoordinatorError::Internal(format!(
                "durable lease expired for session '{}'",
                lease.session_id
            )));
        }
        Ok(())
    }

    pub fn renew(
        &self,
        lease: &SessionLease,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        self.validate(lease, now_unix)?;
        self.acquire(&lease.session_id, &lease.owner_id, now_unix, ttl_secs)
    }

    pub fn release(&self, lease: &SessionLease) -> Result<(), CoordinatorError> {
        let key = Self::key(&lease.session_id);

        for _ in 0..Self::MAX_CAS_RETRIES {
            let old = self.store.load(&key).ok_or_else(|| {
                CoordinatorError::Internal(format!(
                    "no durable lease for session '{}'",
                    lease.session_id
                ))
            })?;
            let current = Self::decode(&old)?;
            if current.owner_id != lease.owner_id || current.fence != lease.fence {
                return Err(CoordinatorError::Internal(format!(
                    "stale distributed fencing token cannot release session '{}'",
                    lease.session_id
                )));
            }

            let released = DurableLeaseState {
                owner_id: String::new(),
                fence: current.fence,
                expires_at: 0,
            };
            let encoded = Self::encode(&released)?;
            if self
                .store
                .compare_and_set(&key, Some(&old), &encoded)
            {
                return Ok(());
            }
        }

        Err(CoordinatorError::Internal(format!(
            "durable lease release CAS contention exceeded retry budget for session '{}'",
            lease.session_id
        )))
    }
}


#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionLease {
    pub session_id: String,
    pub owner_id: String,
    pub fence: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone)]
struct LeaseState {
    owner_id: String,
    fence: u64,
    expires_at: u64,
}

/// Lock-striped lease registry.
///
/// Each session hashes to one shard. Unrelated sessions on different shards can
/// acquire/renew/release leases concurrently.
#[derive(Clone)]
pub struct SessionLeaseManager {
    shards: Arc<Vec<Mutex<HashMap<String, LeaseState>>>>,
}

impl Default for SessionLeaseManager {
    fn default() -> Self {
        Self::new(DEFAULT_SHARDS)
    }
}

impl SessionLeaseManager {
    pub fn new(shard_count: usize) -> Self {
        assert!(shard_count > 0, "lease shard count must be non-zero");
        let shards = (0..shard_count)
            .map(|_| Mutex::new(HashMap::new()))
            .collect();
        Self {
            shards: Arc::new(shards),
        }
    }

    fn shard_index(&self, session_id: &str) -> usize {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        session_id.hash(&mut hasher);
        (hasher.finish() as usize) % self.shards.len()
    }

    fn shard(
        &self,
        session_id: &str,
    ) -> Result<std::sync::MutexGuard<'_, HashMap<String, LeaseState>>, CoordinatorError> {
        self.shards[self.shard_index(session_id)]
            .lock()
            .map_err(|_| {
                CoordinatorError::Internal(
                    "session lease shard poisoned; refusing lease operation".to_string(),
                )
            })
    }

    /// Acquire or take over an expired lease.
    ///
    /// A takeover always increments the fence. The same owner reacquiring an
    /// unexpired lease receives the existing fence and extends expiry.
    pub fn acquire(
        &self,
        session_id: &str,
        owner_id: &str,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        if owner_id.is_empty() || ttl_secs == 0 {
            return Err(CoordinatorError::Internal(
                "lease owner and ttl must be non-empty/non-zero".to_string(),
            ));
        }

        let mut shard = self.shard(session_id)?;
        let expires_at = now_unix
            .checked_add(ttl_secs)
            .ok_or_else(|| CoordinatorError::Internal("lease expiry overflow".to_string()))?;

        match shard.get_mut(session_id) {
            Some(state) if state.owner_id == owner_id && now_unix < state.expires_at => {
                state.expires_at = expires_at;
                Ok(SessionLease {
                    session_id: session_id.to_string(),
                    owner_id: owner_id.to_string(),
                    fence: state.fence,
                    expires_at,
                })
            }
            Some(state) if now_unix >= state.expires_at => {
                state.fence = state.fence.checked_add(1).ok_or_else(|| {
                    CoordinatorError::Internal("lease fence exhausted".to_string())
                })?;
                state.owner_id = owner_id.to_string();
                state.expires_at = expires_at;
                Ok(SessionLease {
                    session_id: session_id.to_string(),
                    owner_id: owner_id.to_string(),
                    fence: state.fence,
                    expires_at,
                })
            }
            Some(state) => Err(CoordinatorError::Internal(format!(
                "session '{session_id}' leased by '{}' until {}",
                state.owner_id, state.expires_at
            ))),
            None => {
                let state = LeaseState {
                    owner_id: owner_id.to_string(),
                    fence: 1,
                    expires_at,
                };
                shard.insert(session_id.to_string(), state.clone());
                Ok(SessionLease {
                    session_id: session_id.to_string(),
                    owner_id: owner_id.to_string(),
                    fence: state.fence,
                    expires_at,
                })
            }
        }
    }

    /// Validate that a worker still owns the latest, unexpired fence.
    pub fn validate(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        let shard = self.shard(&lease.session_id)?;
        let state = shard.get(&lease.session_id).ok_or_else(|| {
            CoordinatorError::Internal(format!(
                "no active lease for session '{}'",
                lease.session_id
            ))
        })?;

        if state.owner_id != lease.owner_id || state.fence != lease.fence {
            return Err(CoordinatorError::Internal(format!(
                "stale fencing token for session '{}': owner/fence no longer current",
                lease.session_id
            )));
        }
        if now_unix >= state.expires_at {
            return Err(CoordinatorError::Internal(format!(
                "lease expired for session '{}'",
                lease.session_id
            )));
        }
        Ok(())
    }

    /// Renew only the current fence holder.
    pub fn renew(
        &self,
        lease: &SessionLease,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        self.validate(lease, now_unix)?;
        self.acquire(&lease.session_id, &lease.owner_id, now_unix, ttl_secs)
    }

    /// Release only if the supplied token is still current.
    ///
    /// The state is retained with an empty owner and expired timestamp so the
    /// next acquisition increments the previous fence instead of reusing 1.
    pub fn release(&self, lease: &SessionLease) -> Result<(), CoordinatorError> {
        let mut shard = self.shard(&lease.session_id)?;
        let state = shard.get_mut(&lease.session_id).ok_or_else(|| {
            CoordinatorError::Internal(format!(
                "no active lease for session '{}'",
                lease.session_id
            ))
        })?;
        if state.owner_id != lease.owner_id || state.fence != lease.fence {
            return Err(CoordinatorError::Internal(format!(
                "stale fencing token cannot release session '{}'",
                lease.session_id
            )));
        }
        state.owner_id.clear();
        state.expires_at = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Barrier;
    use std::thread;

    #[test]
    fn distributed_secret_registry_allows_same_session_retry_only() {
        let store = Arc::new(InMemoryDistributedLeaseStore::default());
        let a = DurableSecretRegistry::new(store.clone());
        let b = DurableSecretRegistry::new(store);
        let hash = [0xabu8; 32];

        a.claim(hash, "swap-a").unwrap();
        b.claim(hash, "swap-a").expect("same-session retry");
        assert!(b.claim(hash, "swap-b").is_err());
        assert_eq!(a.owner(hash).as_deref(), Some("swap-a"));
    }

    #[test]
    fn concurrent_distributed_secret_claim_has_single_owner() {
        let store = Arc::new(InMemoryDistributedLeaseStore::default());
        let barrier = Arc::new(Barrier::new(3));
        let hash = [0xcdu8; 32];
        let mut handles = Vec::new();

        for owner in ["swap-a", "swap-b"] {
            let registry = DurableSecretRegistry::new(store.clone());
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                registry.claim(hash, owner)
            }));
        }

        barrier.wait();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1);
    }

    #[test]
    fn distributed_authorities_share_one_fencing_epoch() {
        let store = Arc::new(InMemoryDistributedLeaseStore::default());
        let a = DurableLeaseAuthority::new(store.clone());
        let b = DurableLeaseAuthority::new(store);

        let first = a.acquire("swap-d", "proc-a", 100, 10).unwrap();
        assert!(b.acquire("swap-d", "proc-b", 105, 10).is_err());

        let second = b.acquire("swap-d", "proc-b", 110, 10).unwrap();
        assert_eq!(second.fence, first.fence + 1);
        assert!(a.validate(&first, 111).is_err());
        b.validate(&second, 111).unwrap();
    }

    #[test]
    fn distributed_fence_survives_authority_restart() {
        let store = Arc::new(InMemoryDistributedLeaseStore::default());
        let first_authority = DurableLeaseAuthority::new(store.clone());
        let first = first_authority.acquire("swap-r", "proc-a", 100, 5).unwrap();
        drop(first_authority);

        let restarted = DurableLeaseAuthority::new(store);
        let second = restarted.acquire("swap-r", "proc-b", 105, 10).unwrap();
        assert_eq!(second.fence, first.fence + 1);
        assert!(restarted.validate(&first, 106).is_err());
    }

    #[test]
    fn concurrent_distributed_acquire_has_single_winner() {
        let store = Arc::new(InMemoryDistributedLeaseStore::default());
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();

        for owner in ["proc-a", "proc-b"] {
            let authority = DurableLeaseAuthority::new(store.clone());
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                authority.acquire("swap-race", owner, 100, 30)
            }));
        }

        barrier.wait();
        let results: Vec<_> = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
        assert_eq!(results.iter().filter(|r| r.is_err()).count(), 1);
    }

    #[test]
    fn distributed_release_preserves_monotonic_fence() {
        let store = Arc::new(InMemoryDistributedLeaseStore::default());
        let authority = DurableLeaseAuthority::new(store);
        let first = authority.acquire("swap-release", "a", 100, 30).unwrap();
        authority.release(&first).unwrap();
        let second = authority.acquire("swap-release", "b", 101, 30).unwrap();
        assert_eq!(second.fence, first.fence + 1);
        assert!(authority.validate(&first, 102).is_err());
    }

    #[test]
    fn takeover_increments_fence_and_rejects_stale_worker() {
        let manager = SessionLeaseManager::new(8);
        let first = manager.acquire("swap-a", "relayer-a", 100, 10).unwrap();
        assert_eq!(first.fence, 1);

        let second = manager.acquire("swap-a", "relayer-b", 110, 10).unwrap();
        assert_eq!(second.fence, 2);
        assert!(manager.validate(&first, 111).is_err());
        manager.validate(&second, 111).unwrap();
    }

    #[test]
    fn unexpired_competing_owner_is_rejected() {
        let manager = SessionLeaseManager::new(8);
        manager.acquire("swap-a", "relayer-a", 100, 20).unwrap();
        assert!(manager.acquire("swap-a", "relayer-b", 105, 20).is_err());
    }

    #[test]
    fn same_owner_renewal_keeps_fence() {
        let manager = SessionLeaseManager::new(8);
        let first = manager.acquire("swap-a", "relayer-a", 100, 10).unwrap();
        let renewed = manager.renew(&first, 105, 20).unwrap();
        assert_eq!(renewed.fence, first.fence);
        assert_eq!(renewed.expires_at, 125);
    }

    #[test]
    fn stale_token_cannot_release_new_owner() {
        let manager = SessionLeaseManager::new(8);
        let first = manager.acquire("swap-a", "a", 100, 5).unwrap();
        let second = manager.acquire("swap-a", "b", 105, 5).unwrap();
        assert!(manager.release(&first).is_err());
        manager.validate(&second, 106).unwrap();
    }

    #[test]
    fn release_does_not_reuse_fencing_epoch() {
        let manager = SessionLeaseManager::new(8);
        let first = manager.acquire("swap-a", "a", 100, 10).unwrap();
        manager.release(&first).unwrap();

        let second = manager.acquire("swap-a", "b", 101, 10).unwrap();
        assert_eq!(second.fence, first.fence + 1);
        assert!(manager.validate(&first, 102).is_err());
        manager.validate(&second, 102).unwrap();
    }

    #[test]
    fn unrelated_sessions_acquire_concurrently() {
        let manager = SessionLeaseManager::new(64);
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = Vec::new();

        for (session, owner) in [("swap-a", "a"), ("swap-b", "b")] {
            let manager = manager.clone();
            let barrier = barrier.clone();
            handles.push(thread::spawn(move || {
                barrier.wait();
                manager.acquire(session, owner, 100, 30)
            }));
        }

        barrier.wait();
        for handle in handles {
            handle.join().unwrap().unwrap();
        }
    }
}
