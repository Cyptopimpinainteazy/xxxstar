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


/// Immutable coordinator-session to canonical intent binding on Redis/Valkey.
#[cfg(all(feature = "valkey", feature = "canonical-proofs"))]
#[derive(Clone)]
pub struct ValkeyIntentBindingStore {
    client: Client,
    namespace: String,
}

#[cfg(all(feature = "valkey", feature = "canonical-proofs"))]
impl ValkeyIntentBindingStore {
    pub fn new(redis_url: &str) -> Result<Self, CoordinatorError> {
        Self::with_namespace(redis_url, "x3")
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

    fn key(&self, session_id: &str) -> String {
        format!("{}:intent-binding:{{{session_id}}}", self.namespace)
    }

    fn connection(&self) -> Result<redis::Connection, CoordinatorError> {
        self.client
            .get_connection()
            .map_err(|e| backend_error("connection", e))
    }
}

#[cfg(all(feature = "valkey", feature = "canonical-proofs"))]
impl crate::IntentBindingStore for ValkeyIntentBindingStore {
    fn bind(
        &self,
        binding: &crate::SessionIntentBinding,
    ) -> Result<(), CoordinatorError> {
        let payload = serde_json::to_string(binding).map_err(|e| {
            CoordinatorError::Internal(format!("intent binding serialize failed: {e}"))
        })?;
        let script = Script::new(
            r#"
local existing = redis.call('GET', KEYS[1])
if not existing then
  redis.call('SET', KEYS[1], ARGV[1])
  return 1
end
if existing == ARGV[1] then
  return 1
end
return 0
"#,
        );

        let mut conn = self.connection()?;
        let accepted: i64 = script
            .key(self.key(&binding.session_id))
            .arg(&payload)
            .invoke(&mut conn)
            .map_err(|e| backend_error("intent binding script", e))?;

        if accepted == 1 {
            Ok(())
        } else {
            Err(CoordinatorError::Internal(format!(
                "session '{}' is already bound to a different canonical intent",
                binding.session_id
            )))
        }
    }

    fn load(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::SessionIntentBinding>, CoordinatorError> {
        let mut conn = self.connection()?;
        let raw: Option<String> = redis::cmd("GET")
            .arg(self.key(session_id))
            .query(&mut conn)
            .map_err(|e| backend_error("intent binding lookup", e))?;

        raw.map(|entry| {
            serde_json::from_str(&entry).map_err(|e| {
                CoordinatorError::Internal(format!("invalid intent binding in Valkey: {e}"))
            })
        })
        .transpose()
    }
}


/// Content-addressed canonical proof-bundle vault on Redis/Valkey.
#[cfg(all(feature = "valkey", feature = "canonical-proofs"))]
#[derive(Clone)]
pub struct ValkeyProofBundleStore {
    client: Client,
    namespace: String,
}

#[cfg(all(feature = "valkey", feature = "canonical-proofs"))]
impl ValkeyProofBundleStore {
    pub fn new(redis_url: &str) -> Result<Self, CoordinatorError> {
        Self::with_namespace(redis_url, "x3")
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

    fn key(&self, proof_hash: [u8; 32]) -> String {
        format!("{}:proof:{}", self.namespace, hex::encode(proof_hash))
    }

    fn connection(&self) -> Result<redis::Connection, CoordinatorError> {
        self.client
            .get_connection()
            .map_err(|e| backend_error("connection", e))
    }
}

#[cfg(all(feature = "valkey", feature = "canonical-proofs"))]
impl crate::ProofBundleStore for ValkeyProofBundleStore {
    fn put_bundle(
        &self,
        bundle: &x3_atomic_swap::CrossDomainProofBundle,
    ) -> Result<(), CoordinatorError> {
        let payload = serde_json::to_string(bundle).map_err(|e| {
            CoordinatorError::Internal(format!("proof bundle serialize failed: {e}"))
        })?;
        let script = Script::new(
            r#"
local existing = redis.call('GET', KEYS[1])
if not existing then
  redis.call('SET', KEYS[1], ARGV[1])
  return 1
end
if existing == ARGV[1] then
  return 1
end
return 0
"#,
        );

        let mut conn = self.connection()?;
        let accepted: i64 = script
            .key(self.key(bundle.proof_hash))
            .arg(&payload)
            .invoke(&mut conn)
            .map_err(|e| backend_error("proof bundle put script", e))?;

        if accepted == 1 {
            Ok(())
        } else {
            Err(CoordinatorError::Internal(format!(
                "proof hash {} already maps to different bundle contents",
                hex::encode(bundle.proof_hash)
            )))
        }
    }

    fn get_bundle(
        &self,
        proof_hash: [u8; 32],
    ) -> Result<Option<x3_atomic_swap::CrossDomainProofBundle>, CoordinatorError> {
        let mut conn = self.connection()?;
        let raw: Option<String> = redis::cmd("GET")
            .arg(self.key(proof_hash))
            .query(&mut conn)
            .map_err(|e| backend_error("proof bundle lookup", e))?;

        raw.map(|entry| {
            serde_json::from_str(&entry).map_err(|e| {
                CoordinatorError::Internal(format!("invalid proof bundle in Valkey: {e}"))
            })
        })
        .transpose()
    }
}


/// Production multi-process coordinator using Valkey/Redis for the hot-path
/// lease/fencing and global secret-ownership decisions.
#[cfg(feature = "valkey")]
pub struct ValkeyDistributedCoordinator<P: crate::SessionPersistence> {
    coordinator: crate::ConcurrentSwapCoordinator<P>,
    leases: ValkeyLeaseAuthority,
    secrets: ValkeySecretRegistry,
    attempts: ValkeyAttemptStore,
    #[cfg(feature = "canonical-proofs")]
    proofs: ValkeyProofBundleStore,
    #[cfg(feature = "canonical-proofs")]
    bindings: ValkeyIntentBindingStore,
}

#[cfg(feature = "valkey")]
impl<P: crate::SessionPersistence> Clone for ValkeyDistributedCoordinator<P> {
    fn clone(&self) -> Self {
        Self {
            coordinator: self.coordinator.clone(),
            leases: self.leases.clone(),
            secrets: self.secrets.clone(),
            attempts: self.attempts.clone(),
            #[cfg(feature = "canonical-proofs")]
            proofs: self.proofs.clone(),
            #[cfg(feature = "canonical-proofs")]
            bindings: self.bindings.clone(),
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
            attempts: ValkeyAttemptStore::with_namespace(redis_url, namespace)?,
            #[cfg(feature = "canonical-proofs")]
            proofs: ValkeyProofBundleStore::with_namespace(redis_url, namespace)?,
            #[cfg(feature = "canonical-proofs")]
            bindings: ValkeyIntentBindingStore::with_namespace(redis_url, namespace)?,
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

    #[cfg(feature = "canonical-proofs")]
    pub fn bind_session_intent(
        &self,
        lease: &SessionLease,
        intent: &x3_atomic_swap::AtomicIntent,
        runtime_intent_id: [u8; 32],
        now_unix: u64,
    ) -> Result<crate::SessionIntentBinding, CoordinatorError> {
        self.validate(lease, now_unix)?;
        let session = self
            .coordinator
            .session(&lease.session_id)?
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: lease.session_id.clone(),
            })?;

        if session.hash_lock.0 != intent.hashlock {
            return Err(CoordinatorError::Internal(
                "coordinator session hashlock does not match AtomicIntent hashlock".into(),
            ));
        }

        let binding =
            crate::binding_from_intent(&lease.session_id, runtime_intent_id, intent)?;
        crate::IntentBindingStore::bind(&self.bindings, &binding)?;
        Ok(binding)
    }

    #[cfg(feature = "canonical-proofs")]
    fn assert_session_intent_binding(
        &self,
        session_id: &str,
        intent: &x3_atomic_swap::AtomicIntent,
        runtime_intent_id: [u8; 32],
    ) -> Result<crate::SessionIntentBinding, CoordinatorError> {
        let expected = crate::binding_from_intent(session_id, runtime_intent_id, intent)?;
        let actual = crate::IntentBindingStore::load(&self.bindings, session_id)?
            .ok_or_else(|| {
                CoordinatorError::Internal(format!(
                    "session '{session_id}' has no canonical runtime intent binding"
                ))
            })?;
        if actual != expected {
            return Err(CoordinatorError::Internal(format!(
                "session '{session_id}' canonical intent binding mismatch"
            )));
        }
        Ok(actual)
    }

    pub fn record_attempt_started(
        &self,
        lease: &SessionLease,
        operation: crate::CoordinatorOperation,
        attempt_id: &str,
        domain: &str,
        now_unix: u64,
    ) -> Result<crate::OperationAttempt, CoordinatorError> {
        self.validate(lease, now_unix)?;
        crate::OperationAttemptLedger::new(self.attempts.clone()).record_started(
            &lease.session_id,
            operation,
            attempt_id,
            &lease.owner_id,
            lease.fence,
            domain,
            now_unix,
        )
    }

    pub fn record_attempt_broadcast(
        &self,
        lease: &SessionLease,
        started: &crate::OperationAttempt,
        tx_id: &str,
        now_unix: u64,
    ) -> Result<crate::OperationAttempt, CoordinatorError> {
        self.validate(lease, now_unix)?;
        if started.session_id != lease.session_id
            || started.owner_id != lease.owner_id
            || started.fence != lease.fence
        {
            return Err(CoordinatorError::Internal(
                "attempt identity does not match active fencing lease".into(),
            ));
        }
        crate::OperationAttemptLedger::new(self.attempts.clone())
            .record_broadcast(started, tx_id, now_unix)
    }

    #[cfg(not(feature = "canonical-proofs"))]
    pub fn record_attempt_finalized(
        &self,
        lease: &SessionLease,
        prior: &crate::OperationAttempt,
        proof_hash: [u8; 32],
        now_unix: u64,
    ) -> Result<crate::CanonicalOperationResult, CoordinatorError> {
        self.validate(lease, now_unix)?;
        if prior.session_id != lease.session_id
            || prior.owner_id != lease.owner_id
            || prior.fence != lease.fence
        {
            return Err(CoordinatorError::Internal(
                "attempt identity does not match active fencing lease".into(),
            ));
        }
        crate::OperationAttemptLedger::new(self.attempts.clone())
            .record_finalized(prior, proof_hash, now_unix)
    }

    #[cfg(feature = "canonical-proofs")]
    pub fn record_attempt_finalized_with_bundle(
        &self,
        lease: &SessionLease,
        prior: &crate::OperationAttempt,
        intent: &x3_atomic_swap::AtomicIntent,
        runtime_intent_id: [u8; 32],
        bundle: &x3_atomic_swap::CrossDomainProofBundle,
        now_unix: u64,
    ) -> Result<crate::CanonicalOperationResult, CoordinatorError> {
        self.validate(lease, now_unix)?;
        if prior.session_id != lease.session_id
            || prior.owner_id != lease.owner_id
            || prior.fence != lease.fence
        {
            return Err(CoordinatorError::Internal(
                "attempt identity does not match active fencing lease".into(),
            ));
        }

        self.bind_session_intent(lease, intent, runtime_intent_id, now_unix)?;
        self.assert_session_intent_binding(
            &lease.session_id,
            intent,
            runtime_intent_id,
        )?;
        let proof_hash =
            crate::verify_bundle_for_attempt(prior, intent, runtime_intent_id, bundle)?;
        crate::ProofBundleStore::put_bundle(&self.proofs, bundle)?;
        crate::OperationAttemptLedger::new(self.attempts.clone())
            .record_finalized(prior, proof_hash, now_unix)
    }

    #[cfg(feature = "canonical-proofs")]
    pub fn assemble_canonical_proof_set(
        &self,
        session_id: &str,
        intent: &x3_atomic_swap::AtomicIntent,
        runtime_intent_id: [u8; 32],
    ) -> Result<x3_atomic_swap::CrossDomainProofSet, CoordinatorError> {
        self.assert_session_intent_binding(session_id, intent, runtime_intent_id)?;
        let mut canonical_results = Vec::new();
        for operation in [
            crate::CoordinatorOperation::FastHtlcLock,
            crate::CoordinatorOperation::SlowHtlcLock,
            crate::CoordinatorOperation::FastClaim,
            crate::CoordinatorOperation::SlowClaim,
            crate::CoordinatorOperation::RefundBoth,
        ] {
            if let Some(result) =
                crate::AttemptStore::canonical(&self.attempts, session_id, operation)?
            {
                canonical_results.push(result);
            }
        }
        crate::assemble_proof_set(
            intent,
            runtime_intent_id,
            &canonical_results,
            &self.proofs,
        )
    }

    #[cfg(feature = "canonical-proofs")]
    pub fn canonical_proof_bundle(
        &self,
        proof_hash: [u8; 32],
    ) -> Result<Option<x3_atomic_swap::CrossDomainProofBundle>, CoordinatorError> {
        crate::ProofBundleStore::get_bundle(&self.proofs, proof_hash)
    }

    #[cfg(feature = "canonical-proofs")]
    pub fn assemble_verified_claim_proof_set(
        &self,
        session_id: &str,
        intent: &x3_atomic_swap::AtomicIntent,
        runtime_intent_id: [u8; 32],
        required_domains: &[(x3_atomic_swap::ChainId, x3_atomic_swap::VmType)],
    ) -> Result<x3_atomic_swap::CrossDomainProofSet, CoordinatorError> {
        let set =
            self.assemble_canonical_proof_set(session_id, intent, runtime_intent_id)?;
        set.verify_runtime_binding(runtime_intent_id).map_err(|e| {
            CoordinatorError::Internal(format!(
                "assembled claim proof set failed runtime binding: {e}"
            ))
        })?;
        set.verify_claim_set(intent, required_domains).map_err(|e| {
            CoordinatorError::Internal(format!(
                "assembled claim proof set is incomplete or invalid: {e}"
            ))
        })?;
        Ok(set)
    }

    #[cfg(feature = "canonical-proofs")]
    pub fn assemble_verified_refund_proof_set(
        &self,
        session_id: &str,
        intent: &x3_atomic_swap::AtomicIntent,
        runtime_intent_id: [u8; 32],
        required_domains: &[(x3_atomic_swap::ChainId, x3_atomic_swap::VmType)],
    ) -> Result<x3_atomic_swap::CrossDomainProofSet, CoordinatorError> {
        let set =
            self.assemble_canonical_proof_set(session_id, intent, runtime_intent_id)?;
        set.verify_runtime_binding(runtime_intent_id).map_err(|e| {
            CoordinatorError::Internal(format!(
                "assembled refund proof set failed runtime binding: {e}"
            ))
        })?;
        set.verify_refund_set(intent, required_domains).map_err(|e| {
            CoordinatorError::Internal(format!(
                "assembled refund proof set is incomplete or invalid: {e}"
            ))
        })?;
        Ok(set)
    }

    pub fn attempt_history(
        &self,
        session_id: &str,
    ) -> Result<Vec<crate::OperationAttempt>, CoordinatorError> {
        crate::AttemptStore::attempts(&self.attempts, session_id)
    }

    pub fn canonical_operation_result(
        &self,
        session_id: &str,
        operation: crate::CoordinatorOperation,
    ) -> Result<Option<crate::CanonicalOperationResult>, CoordinatorError> {
        crate::AttemptStore::canonical(&self.attempts, session_id, operation)
    }

    pub fn recovery_action(
        &self,
        session_id: &str,
    ) -> Result<crate::RecoveryAction, CoordinatorError> {
        let session = self
            .coordinator
            .session(session_id)?
            .ok_or_else(|| CoordinatorError::SessionNotFound {
                session_id: session_id.to_string(),
            })?;

        let attempts = crate::AttemptStore::attempts(&self.attempts, session_id)?;
        let mut canonical_results = Vec::new();
        for operation in [
            crate::CoordinatorOperation::FastHtlcLock,
            crate::CoordinatorOperation::SlowHtlcLock,
            crate::CoordinatorOperation::FastClaim,
            crate::CoordinatorOperation::SlowClaim,
            crate::CoordinatorOperation::RefundBoth,
        ] {
            if let Some(result) =
                crate::AttemptStore::canonical(&self.attempts, session_id, operation)?
            {
                canonical_results.push(result);
            }
        }

        let evidence = crate::RecoveryEvidence {
            attempts: &attempts,
            canonical_results: &canonical_results,
        };
        Ok(crate::RecoveryReconciler::decide(&session, &evidence))
    }

    pub fn session(
        &self,
        session_id: &str,
    ) -> Result<Option<crate::SwapSession>, CoordinatorError> {
        self.coordinator.session(session_id)
    }
}


/// Append-only operation-attempt store on Redis/Valkey.
///
/// Each attempt snapshot is immutable. One canonical result key exists per
/// (session, operation) and may only be created once or re-set identically.
#[cfg(feature = "valkey")]
#[derive(Clone)]
pub struct ValkeyAttemptStore {
    client: Client,
    namespace: String,
}

#[cfg(feature = "valkey")]
impl ValkeyAttemptStore {
    pub fn new(redis_url: &str) -> Result<Self, CoordinatorError> {
        Self::with_namespace(redis_url, "x3")
    }

    pub fn with_namespace(
        redis_url: &str,
        namespace: impl Into<String>,
    ) -> Result<Self, CoordinatorError> {
        Ok(Self {
            client: Client::open(redis_url).map_err(|e| backend_error("client creation", e))?,
            namespace: namespace.into(),
        })
    }

    fn list_key(&self, session_id: &str) -> String {
        format!("{}:attempts:{{{session_id}}}", self.namespace)
    }

    fn event_key(&self, attempt: &crate::OperationAttempt) -> String {
        format!(
            "{}:attempt-event:{{{}}}:{}",
            self.namespace, attempt.session_id, attempt.attempt_id
        )
    }

    fn canonical_key(
        &self,
        session_id: &str,
        operation: crate::CoordinatorOperation,
    ) -> String {
        format!(
            "{}:canonical:{{{session_id}}}:{:?}",
            self.namespace, operation
        )
    }

    fn connection(&self) -> Result<redis::Connection, CoordinatorError> {
        self.client
            .get_connection()
            .map_err(|e| backend_error("connection", e))
    }
}

#[cfg(feature = "valkey")]
impl crate::AttemptStore for ValkeyAttemptStore {
    fn append_attempt(
        &self,
        attempt: &crate::OperationAttempt,
    ) -> Result<(), CoordinatorError> {
        let payload = serde_json::to_string(attempt)
            .map_err(|e| CoordinatorError::Internal(format!("attempt serialize failed: {e}")))?;
        let event_key = self.event_key(attempt);
        let list_key = self.list_key(&attempt.session_id);
        let script = Script::new(
            r#"
local existing = redis.call('GET', KEYS[1])
if existing then
  if existing == ARGV[1] then
    return 1
  end
  return 0
end
redis.call('SET', KEYS[1], ARGV[1])
redis.call('RPUSH', KEYS[2], ARGV[1])
return 1
"#,
        );

        let mut conn = self.connection()?;
        let accepted: i64 = script
            .key(event_key)
            .key(list_key)
            .arg(&payload)
            .invoke(&mut conn)
            .map_err(|e| backend_error("attempt append script", e))?;
        if accepted == 1 {
            Ok(())
        } else {
            Err(CoordinatorError::Internal(format!(
                "attempt id '{}' reused with different immutable contents",
                attempt.attempt_id
            )))
        }
    }

    fn attempts(
        &self,
        session_id: &str,
    ) -> Result<Vec<crate::OperationAttempt>, CoordinatorError> {
        let mut conn = self.connection()?;
        let raw: Vec<String> = redis::cmd("LRANGE")
            .arg(self.list_key(session_id))
            .arg(0)
            .arg(-1)
            .query(&mut conn)
            .map_err(|e| backend_error("attempt history lookup", e))?;

        raw.into_iter()
            .map(|entry| {
                serde_json::from_str(&entry).map_err(|e| {
                    CoordinatorError::Internal(format!(
                        "invalid Valkey attempt history entry: {e}"
                    ))
                })
            })
            .collect()
    }

    fn canonical(
        &self,
        session_id: &str,
        operation: crate::CoordinatorOperation,
    ) -> Result<Option<crate::CanonicalOperationResult>, CoordinatorError> {
        let mut conn = self.connection()?;
        let raw: Option<String> = redis::cmd("GET")
            .arg(self.canonical_key(session_id, operation))
            .query(&mut conn)
            .map_err(|e| backend_error("canonical result lookup", e))?;

        raw.map(|entry| {
            serde_json::from_str(&entry).map_err(|e| {
                CoordinatorError::Internal(format!("invalid canonical result: {e}"))
            })
        })
        .transpose()
    }

    fn set_canonical(
        &self,
        result: &crate::CanonicalOperationResult,
    ) -> Result<(), CoordinatorError> {
        let payload = serde_json::to_string(result).map_err(|e| {
            CoordinatorError::Internal(format!("canonical result serialize failed: {e}"))
        })?;
        let key = self.canonical_key(&result.session_id, result.operation);
        let script = Script::new(
            r#"
local existing = redis.call('GET', KEYS[1])
if not existing then
  redis.call('SET', KEYS[1], ARGV[1])
  return 1
end
if existing == ARGV[1] then
  return 1
end
return 0
"#,
        );

        let mut conn = self.connection()?;
        let accepted: i64 = script
            .key(key)
            .arg(&payload)
            .invoke(&mut conn)
            .map_err(|e| backend_error("canonical result script", e))?;

        if accepted == 1 {
            Ok(())
        } else {
            let existing = self
                .canonical(&result.session_id, result.operation)?
                .ok_or_else(|| {
                    CoordinatorError::Internal(
                        "canonical result conflict disappeared during readback".into(),
                    )
                })?;
            Err(CoordinatorError::IdempotencyConflict {
                operation: result.operation,
                existing: hex::encode(existing.proof_hash),
                incoming: hex::encode(result.proof_hash),
            })
        }
    }
}
