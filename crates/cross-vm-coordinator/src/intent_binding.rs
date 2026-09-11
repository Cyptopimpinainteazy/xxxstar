//! Immutable coordinator-session to canonical atomic/runtime intent binding.
//!
//! A coordinator session may be bound once. Every later proof finalization,
//! recovery, and settlement package must match the same runtime H256 intent,
//! AtomicIntent id/hash, and hashlock.

use crate::CoordinatorError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionIntentBinding {
    pub session_id: String,
    pub runtime_intent_id: [u8; 32],
    pub atomic_intent_id: u64,
    pub atomic_intent_hash: [u8; 32],
    pub hashlock: [u8; 32],
}

pub trait IntentBindingStore: Send + Sync + 'static {
    /// Set the binding if absent. Repeating the exact same binding is
    /// idempotent. Any different binding for the same session must fail.
    fn bind(&self, binding: &SessionIntentBinding) -> Result<(), CoordinatorError>;

    fn load(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionIntentBinding>, CoordinatorError>;
}

#[derive(Default)]
pub struct InMemoryIntentBindingStore {
    bindings: RwLock<HashMap<String, SessionIntentBinding>>,
}

impl IntentBindingStore for InMemoryIntentBindingStore {
    fn bind(&self, binding: &SessionIntentBinding) -> Result<(), CoordinatorError> {
        let mut guard = self
            .bindings
            .write()
            .map_err(|_| CoordinatorError::Internal("intent-binding store poisoned".into()))?;

        if let Some(existing) = guard.get(&binding.session_id) {
            if existing == binding {
                return Ok(());
            }
            return Err(CoordinatorError::Internal(format!(
                "session '{}' is already bound to a different canonical intent",
                binding.session_id
            )));
        }

        guard.insert(binding.session_id.clone(), binding.clone());
        Ok(())
    }

    fn load(
        &self,
        session_id: &str,
    ) -> Result<Option<SessionIntentBinding>, CoordinatorError> {
        let guard = self
            .bindings
            .read()
            .map_err(|_| CoordinatorError::Internal("intent-binding store poisoned".into()))?;
        Ok(guard.get(session_id).cloned())
    }
}

#[cfg(feature = "canonical-proofs")]
pub fn binding_from_intent(
    session_id: &str,
    runtime_intent_id: [u8; 32],
    intent: &x3_atomic_swap::AtomicIntent,
) -> Result<SessionIntentBinding, CoordinatorError> {
    if !intent.verify_hash() {
        return Err(CoordinatorError::Internal(
            "AtomicIntent canonical hash verification failed".into(),
        ));
    }

    Ok(SessionIntentBinding {
        session_id: session_id.to_string(),
        runtime_intent_id,
        atomic_intent_id: intent.intent_id,
        atomic_intent_hash: intent.intent_hash,
        hashlock: intent.hashlock,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn binding(runtime_byte: u8) -> SessionIntentBinding {
        SessionIntentBinding {
            session_id: "swap-a".into(),
            runtime_intent_id: [runtime_byte; 32],
            atomic_intent_id: 7,
            atomic_intent_hash: [8u8; 32],
            hashlock: [9u8; 32],
        }
    }

    #[test]
    fn exact_rebind_is_idempotent() {
        let store = InMemoryIntentBindingStore::default();
        let b = binding(1);
        store.bind(&b).unwrap();
        store.bind(&b).unwrap();
        assert_eq!(store.load("swap-a").unwrap(), Some(b));
    }

    #[test]
    fn different_runtime_intent_cannot_rebind_session() {
        let store = InMemoryIntentBindingStore::default();
        store.bind(&binding(1)).unwrap();
        assert!(store.bind(&binding(2)).is_err());
    }

    #[test]
    fn different_atomic_hash_cannot_rebind_session() {
        let store = InMemoryIntentBindingStore::default();
        store.bind(&binding(1)).unwrap();
        let mut other = binding(1);
        other.atomic_intent_hash = [0x55u8; 32];
        assert!(store.bind(&other).is_err());
    }
}
