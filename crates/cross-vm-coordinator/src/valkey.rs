//! Redis/Valkey production coordination backend.
//!
//! Uses server-side Lua so lease acquisition/takeover and secret ownership are
//! each decided atomically in a single round trip.

use crate::{CoordinatorError, SessionLease};

#[cfg(feature = "valkey")]
use redis::{Client, Script};

#[cfg(feature = "valkey")]
fn backend_error(context: &str, err: impl core::fmt::Display) -> CoordinatorError {
    CoordinatorError::Internal(format!("Valkey/Redis {context} failed: {err}"))
}

/// Single-round-trip Redis/Valkey lease/fencing authority.
#[cfg(feature = "valkey")]
#[derive(Clone)]
pub struct ValkeyLeaseAuthority {
    client: Client,
    namespace: String,
}

#[cfg(feature = "valkey")]
impl ValkeyLeaseAuthority {
    pub fn new(redis_url: &str) -> Result<Self, CoordinatorError> {
        Self::with_namespace(redis_url, "x3")
    }

    pub fn with_namespace(
        redis_url: &str,
        namespace: impl Into<String>,
    ) -> Result<Self, CoordinatorError> {
        let client = Client::open(redis_url).map_err(|e| backend_error("client creation", e))?;
        Ok(Self {
            client,
            namespace: namespace.into(),
        })
    }

    fn lease_key(&self, session_id: &str) -> String {
        // Hash tags keep one session's key stable on Redis Cluster.
        format!("{}:lease:{{{session_id}}}", self.namespace)
    }

    fn connection(&self) -> Result<redis::Connection, CoordinatorError> {
        self.client
            .get_connection()
            .map_err(|e| backend_error("connection", e))
    }

    pub fn acquire(
        &self,
        session_id: &str,
        owner_id: &str,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        if session_id.is_empty() || owner_id.is_empty() || ttl_secs == 0 {
            return Err(CoordinatorError::Internal(
                "Valkey lease session/owner must be non-empty and ttl non-zero".to_string(),
            ));
        }

        let expires_at = now_unix
            .checked_add(ttl_secs)
            .ok_or_else(|| CoordinatorError::Internal("lease expiry overflow".to_string()))?;
        let key = self.lease_key(session_id);
        let script = Script::new(
            r#"
local owner = redis.call('HGET', KEYS[1], 'owner')
local fence = tonumber(redis.call('HGET', KEYS[1], 'fence') or '0')
local expires = tonumber(redis.call('HGET', KEYS[1], 'expires_at') or '0')
local requested_owner = ARGV[1]
local now = tonumber(ARGV[2])
local new_expires = tonumber(ARGV[3])

if not owner then
  fence = 1
  redis.call('HSET', KEYS[1],
    'owner', requested_owner,
    'fence', fence,
    'expires_at', new_expires)
  return {1, fence, new_expires}
end

if owner == requested_owner and now < expires then
  redis.call('HSET', KEYS[1], 'expires_at', new_expires)
  return {1, fence, new_expires}
end

if now >= expires then
  fence = fence + 1
  redis.call('HSET', KEYS[1],
    'owner', requested_owner,
    'fence', fence,
    'expires_at', new_expires)
  return {1, fence, new_expires}
end

return {0, fence, expires}
"#,
        );

        let mut conn = self.connection()?;
        let result: Vec<i64> = script
            .key(&key)
            .arg(owner_id)
            .arg(now_unix)
            .arg(expires_at)
            .invoke(&mut conn)
            .map_err(|e| backend_error("lease acquire script", e))?;

        if result.len() != 3 {
            return Err(CoordinatorError::Internal(
                "Valkey lease acquire returned malformed response".to_string(),
            ));
        }
        if result[0] != 1 {
            return Err(CoordinatorError::Internal(format!(
                "session '{session_id}' already leased; current fence {}, expires {}",
                result[1], result[2]
            )));
        }

        Ok(SessionLease {
            session_id: session_id.to_string(),
            owner_id: owner_id.to_string(),
            fence: result[1] as u64,
            expires_at: result[2] as u64,
        })
    }

    pub fn validate(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        let key = self.lease_key(&lease.session_id);
        let script = Script::new(
            r#"
local owner = redis.call('HGET', KEYS[1], 'owner')
local fence = tonumber(redis.call('HGET', KEYS[1], 'fence') or '-1')
local expires = tonumber(redis.call('HGET', KEYS[1], 'expires_at') or '0')
if not owner then return 0 end
if owner ~= ARGV[1] then return 0 end
if fence ~= tonumber(ARGV[2]) then return 0 end
if tonumber(ARGV[3]) >= expires then return 0 end
return 1
"#,
        );

        let mut conn = self.connection()?;
        let valid: i64 = script
            .key(&key)
            .arg(&lease.owner_id)
            .arg(lease.fence)
            .arg(now_unix)
            .invoke(&mut conn)
            .map_err(|e| backend_error("lease validate script", e))?;

        if valid == 1 {
            Ok(())
        } else {
            Err(CoordinatorError::Internal(format!(
                "stale or expired Valkey fencing token for session '{}'",
                lease.session_id
            )))
        }
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
        let key = self.lease_key(&lease.session_id);
        let script = Script::new(
            r#"
local owner = redis.call('HGET', KEYS[1], 'owner')
local fence = tonumber(redis.call('HGET', KEYS[1], 'fence') or '-1')
if not owner then return 0 end
if owner ~= ARGV[1] then return 0 end
if fence ~= tonumber(ARGV[2]) then return 0 end

-- Preserve the fence forever; only clear ownership/expiry.
redis.call('HSET', KEYS[1], 'owner', '', 'expires_at', 0)
return 1
"#,
        );

        let mut conn = self.connection()?;
        let released: i64 = script
            .key(&key)
            .arg(&lease.owner_id)
            .arg(lease.fence)
            .invoke(&mut conn)
            .map_err(|e| backend_error("lease release script", e))?;

        if released == 1 {
            Ok(())
        } else {
            Err(CoordinatorError::Internal(format!(
                "stale Valkey fencing token cannot release session '{}'",
                lease.session_id
            )))
        }
    }
}

/// Atomic global HTLC secret ownership on Redis/Valkey.
#[cfg(feature = "valkey")]
#[derive(Clone)]
pub struct ValkeySecretRegistry {
    client: Client,
    namespace: String,
}

#[cfg(feature = "valkey")]
impl ValkeySecretRegistry {
    pub fn new(redis_url: &str) -> Result<Self, CoordinatorError> {
        Self::with_namespace(redis_url, "x3")
    }

    pub fn with_namespace(
        redis_url: &str,
        namespace: impl Into<String>,
    ) -> Result<Self, CoordinatorError> {
        let client = Client::open(redis_url).map_err(|e| backend_error("client creation", e))?;
        Ok(Self {
            client,
            namespace: namespace.into(),
        })
    }

    fn secret_key(&self, secret_hash: [u8; 32]) -> String {
        format!("{}:secret:{}", self.namespace, hex::encode(secret_hash))
    }

    pub fn claim(
        &self,
        secret_hash: [u8; 32],
        session_id: &str,
    ) -> Result<(), CoordinatorError> {
        let key = self.secret_key(secret_hash);
        let script = Script::new(
            r#"
local current = redis.call('GET', KEYS[1])
if not current then
  redis.call('SET', KEYS[1], ARGV[1])
  return 1
end
if current == ARGV[1] then
  return 1
end
return 0
"#,
        );

        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| backend_error("connection", e))?;
        let claimed: i64 = script
            .key(&key)
            .arg(session_id)
            .invoke(&mut conn)
            .map_err(|e| backend_error("secret claim script", e))?;

        if claimed == 1 {
            Ok(())
        } else {
            Err(CoordinatorError::Internal(
                "distributed secret replay rejected by Valkey".to_string(),
            ))
        }
    }

    pub fn owner(
        &self,
        secret_hash: [u8; 32],
    ) -> Result<Option<String>, CoordinatorError> {
        let key = self.secret_key(secret_hash);
        let mut conn = self
            .client
            .get_connection()
            .map_err(|e| backend_error("connection", e))?;
        redis::cmd("GET")
            .arg(key)
            .query(&mut conn)
            .map_err(|e| backend_error("secret owner lookup", e))
    }
}

/// Generic binary CAS store on Redis/Valkey for compatibility with the #169
/// `DistributedLeaseStore` contract. The specialized authorities above are
/// faster for the hot path because they combine read/decision/write in one Lua
/// script.
#[cfg(feature = "valkey")]
#[derive(Clone)]
pub struct ValkeyCasStore {
    client: Client,
    namespace: String,
}

#[cfg(feature = "valkey")]
impl ValkeyCasStore {
    pub fn new(redis_url: &str) -> Result<Self, CoordinatorError> {
        Self::with_namespace(redis_url, "x3:cas")
    }

    pub fn with_namespace(
        redis_url: &str,
        namespace: impl Into<String>,
    ) -> Result<Self, CoordinatorError> {
        Ok(Self {
            client: Client::open(redis_url)
                .map_err(|e| backend_error("client creation", e))?,
            namespace: namespace.into(),
        })
    }

    fn namespaced(&self, key: &[u8]) -> Vec<u8> {
        let mut out = self.namespace.as_bytes().to_vec();
        out.push(b':');
        out.extend_from_slice(key);
        out
    }
}

#[cfg(feature = "valkey")]
impl crate::DistributedLeaseStore for ValkeyCasStore {
    fn load(&self, key: &[u8]) -> Option<Vec<u8>> {
        let mut conn = self.client.get_connection().ok()?;
        redis::cmd("GET")
            .arg(self.namespaced(key))
            .query(&mut conn)
            .ok()
    }

    fn compare_and_set(
        &self,
        key: &[u8],
        old_value: Option<&[u8]>,
        new_value: &[u8],
    ) -> bool {
        let script = Script::new(
            r#"
local current = redis.call('GET', KEYS[1])
local expect_none = ARGV[1] == '1'
local expected = ARGV[2]
local replacement = ARGV[3]

if expect_none then
  if current then return 0 end
else
  if not current or current ~= expected then return 0 end
end

redis.call('SET', KEYS[1], replacement)
return 1
"#,
        );

        let Ok(mut conn) = self.client.get_connection() else {
            return false;
        };
        script
            .key(self.namespaced(key))
            .arg(if old_value.is_none() { "1" } else { "0" })
            .arg(old_value.unwrap_or_default())
            .arg(new_value)
            .invoke::<i64>(&mut conn)
            .map(|value| value == 1)
            .unwrap_or(false)
    }
}


/// Production multi-process coordinator using Valkey/Redis for the hot-path
/// lease/fencing and global secret-ownership decisions.
#[cfg(feature = "valkey")]
pub struct ValkeyDistributedCoordinator<P: crate::SessionPersistence> {
    coordinator: crate::ConcurrentSwapCoordinator<P>,
    leases: ValkeyLeaseAuthority,
    secrets: ValkeySecretRegistry,
}

#[cfg(feature = "valkey")]
impl<P: crate::SessionPersistence> Clone for ValkeyDistributedCoordinator<P> {
    fn clone(&self) -> Self {
        Self {
            coordinator: self.coordinator.clone(),
            leases: self.leases.clone(),
            secrets: self.secrets.clone(),
        }
    }
}

#[cfg(feature = "valkey")]
impl<P: crate::SessionPersistence> ValkeyDistributedCoordinator<P> {
    pub fn new(
        coordinator: crate::ConcurrentSwapCoordinator<P>,
        redis_url: &str,
    ) -> Result<Self, CoordinatorError> {
        Self::with_namespace(coordinator, redis_url, "x3")
    }

    pub fn with_namespace(
        coordinator: crate::ConcurrentSwapCoordinator<P>,
        redis_url: &str,
        namespace: &str,
    ) -> Result<Self, CoordinatorError> {
        Ok(Self {
            coordinator,
            leases: ValkeyLeaseAuthority::with_namespace(redis_url, namespace)?,
            secrets: ValkeySecretRegistry::with_namespace(redis_url, namespace)?,
        })
    }

    pub fn acquire_session_lease(
        &self,
        session_id: &str,
        owner_id: &str,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        let lease = self
            .leases
            .acquire(session_id, owner_id, now_unix, ttl_secs)?;
        self.coordinator
            .execute(|inner| inner.refresh_from_persistence(session_id))?;
        Ok(lease)
    }

    pub fn renew_session_lease(
        &self,
        lease: &SessionLease,
        now_unix: u64,
        ttl_secs: u64,
    ) -> Result<SessionLease, CoordinatorError> {
        self.leases.renew(lease, now_unix, ttl_secs)
    }

    pub fn release_session_lease(
        &self,
        lease: &SessionLease,
    ) -> Result<(), CoordinatorError> {
        self.leases.release(lease)
    }

    fn validate(&self, lease: &SessionLease, now_unix: u64) -> Result<(), CoordinatorError> {
        self.leases.validate(lease, now_unix)
    }

    pub fn record_htlc_fast(
        &self,
        lease: &SessionLease,
        record: crate::HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate(lease, now_unix)?;
        self.coordinator
            .record_htlc_fast(&lease.session_id, record, now_unix)
    }

    pub fn record_htlc_slow(
        &self,
        lease: &SessionLease,
        record: crate::HtlcRecord,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate(lease, now_unix)?;
        self.coordinator
            .record_htlc_slow(&lease.session_id, record, now_unix)
    }

    pub fn record_fast_claim(
        &self,
        lease: &SessionLease,
        secret: crate::HtlcSecret,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate(lease, now_unix)?;

        let session = self
            .coordinator
            .session(&lease.session_id)?
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: lease.session_id.clone(),
            })?;

        if secret.hash() != session.hash_lock {
            return Err(CoordinatorError::Internal(format!(
                "secret hash mismatch for Valkey claim on session '{}'",
                lease.session_id
            )));
        }

        let already_claimed = session.operation_journal.iter().any(|receipt| {
            receipt.operation == crate::CoordinatorOperation::FastClaim
        });
        if session.phase != crate::SwapPhase::ClaimingFast && !already_claimed {
            return Err(CoordinatorError::InvalidPhaseTransition {
                from: session.phase.to_string(),
                to: crate::SwapPhase::ClaimingFast.to_string(),
            });
        }

        let secret_hash = *blake3::hash(secret.as_bytes()).as_bytes();
        self.secrets.claim(secret_hash, &lease.session_id)?;
        self.coordinator
            .record_fast_claim(&lease.session_id, secret, now_unix)
    }

    pub fn record_slow_claim(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate(lease, now_unix)?;
        self.coordinator
            .record_slow_claim(&lease.session_id, now_unix)
    }

    pub fn record_refunds(
        &self,
        lease: &SessionLease,
        now_unix: u64,
    ) -> Result<(), CoordinatorError> {
        self.validate(lease, now_unix)?;
        self.coordinator
            .record_refunds(&lease.session_id, now_unix)
    }

    pub fn session(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::SwapSession>, CoordinatorError> {
        self.coordinator.session(session_id)
    }
}
