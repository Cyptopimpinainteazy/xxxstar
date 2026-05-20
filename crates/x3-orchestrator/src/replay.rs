//! Replay protection backed by an in-memory `HashSet`.
//!
//! The orchestrator records every executed message id. If the same id is
//! submitted again, [`ReplayGuard::check_and_mark`] returns
//! [`OrchestratorError::ReplayDetected`].

use std::collections::HashSet;

use parking_lot::RwLock;

use crate::{OrchestratorError, Result};

#[derive(Default)]
pub struct ReplayGuard {
    executed: RwLock<HashSet<String>>,
}

impl ReplayGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Atomically check that `message_id` has not been seen and record it.
    pub fn check_and_mark(&self, message_id: &str) -> Result<()> {
        let mut executed = self.executed.write();
        if executed.contains(message_id) {
            return Err(OrchestratorError::ReplayDetected(message_id.to_string()));
        }
        executed.insert(message_id.to_string());
        Ok(())
    }

    pub fn has_seen(&self, message_id: &str) -> bool {
        self.executed.read().contains(message_id)
    }
}
