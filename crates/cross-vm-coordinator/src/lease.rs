//! Per-session lease sharding and fencing tokens for concurrent relayers.
//!
//! External work may outlive a local mutex guard. A fencing token lets the
//! coordinator reject a stale worker after lease expiry/takeover.

use crate::CoordinatorError;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};

const DEFAULT_SHARDS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
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
