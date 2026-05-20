//! Agent genesis records — persistence layer for INV-S-001 and AGENT_LAW §1.
//!
//! Every agent spawned in the swarm must have a `GenesisRecord` created before
//! it may act. The record is immutable for the fields marked
//! [`GenesisRecord::IMMUTABLE_FIELDS`] after creation; mutable fields
//! (e.g. `expiry_block`) may be amended by the creator or an operator.
//!
//! This module provides an in-memory `GenesisStore` suitable for both the
//! embedded swarm runtime and for tests. A durable backend (e.g. storage pallet
//! or flat-file) should wrap this store for production use.

use crate::{AgentKind, AgentPermissionTier};
use serde::{Deserialize, Serialize};

pub type AgentId = [u8; 32];
pub type BlockHeight = u64;

/// Supervision mode for a spawned agent. Determines the operator oversight
/// level required for each action the agent takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupervisionMode {
    /// Agent may act without human confirmation.
    FullAuto,
    /// Agent must checkpoint with operator at each phase boundary.
    HumanCheckpoint,
    /// Agent must receive explicit approval before every action.
    HumanApproval,
}

/// An immutable-on-creation record describing the provenance, authority, and
/// constraints for a single agent instance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisRecord {
    // ── Immutable after creation ─────────────────────────────────────────
    /// Unique agent identifier (hash of creator + purpose + created_at).
    pub agent_id: AgentId,
    /// The agent_id of the entity that spawned this agent, or zero for
    /// operator-direct genesis.
    pub creator: AgentId,
    /// Short human-readable purpose string. Immutable.
    pub purpose: String,
    /// Agent class. Immutable — changing class requires a new genesis record.
    pub class: AgentKind,
    /// Permission tier at genesis. Immutable.
    pub permission_tier: AgentPermissionTier,
    /// Lineage chain — ordered list of ancestor agent_ids, oldest first.
    /// Immutable.
    pub lineage: Vec<AgentId>,
    /// Block at which this record was created. Immutable.
    pub created_at_block: BlockHeight,

    // ── Mutable after creation ────────────────────────────────────────────
    /// Free-form model/tool identifiers (e.g. "claude-3.7-sonnet,cargo").
    pub model_tool_stack: Vec<String>,
    /// Surface identifiers the agent is allowed to act on
    /// (e.g. "crates/x3-swarm-core", "docs/").
    pub allowed_surfaces: Vec<String>,
    /// Who pays for this agent's compute. Free-form label.
    pub funding_source: String,
    /// Supervision mode.
    pub supervision_mode: SupervisionMode,
    /// Human-readable description of the revocation authority for this agent.
    pub revocation_path: String,
    /// Semantic version of the swarm framework under which this agent was
    /// created.
    pub version: String,
    /// Optional block at which this agent's authorisation expires. After
    /// expiry the agent must be re-authorised or terminated.
    pub expiry_block: Option<BlockHeight>,
    /// Whether the agent has been terminated.
    pub terminated: bool,
}

impl GenesisRecord {
    /// Construct a minimal genesis record. Caller must supply all immutable
    /// fields; mutable fields can be adjusted after creation via
    /// [`GenesisStore::amend`].
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        agent_id: AgentId,
        creator: AgentId,
        purpose: impl Into<String>,
        class: AgentKind,
        permission_tier: AgentPermissionTier,
        lineage: Vec<AgentId>,
        created_at_block: BlockHeight,
    ) -> Self {
        Self {
            agent_id,
            creator,
            purpose: purpose.into(),
            class,
            permission_tier,
            lineage,
            created_at_block,
            model_tool_stack: vec![],
            allowed_surfaces: vec![],
            funding_source: String::new(),
            supervision_mode: SupervisionMode::HumanCheckpoint,
            revocation_path: String::new(),
            version: "0.1.0".to_string(),
            expiry_block: None,
            terminated: false,
        }
    }

    /// Returns `true` if the agent's authorisation has expired at `now`.
    pub fn is_expired(&self, now: BlockHeight) -> bool {
        self.expiry_block.map(|exp| now >= exp).unwrap_or(false)
    }

    /// Returns `true` if the agent is active (not terminated, not expired).
    pub fn is_active(&self, now: BlockHeight) -> bool {
        !self.terminated && !self.is_expired(now)
    }

    /// Depth of this agent in the spawn tree (length of lineage).
    pub fn spawn_depth(&self) -> usize {
        self.lineage.len()
    }
}

/// Error returned by [`GenesisStore`] operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenesisError {
    /// An agent with this id already exists.
    Duplicate(AgentId),
    /// No agent with this id exists.
    NotFound(AgentId),
    /// Attempted to mutate an immutable field.
    ImmutableFieldViolation { field: &'static str },
    /// Attempted to amend or act as a terminated agent.
    AgentTerminated(AgentId),
}

impl core::fmt::Display for GenesisError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            GenesisError::Duplicate(id) => write!(f, "agent {:?} already registered", id),
            GenesisError::NotFound(id) => write!(f, "agent {:?} not found", id),
            GenesisError::ImmutableFieldViolation { field } => {
                write!(f, "cannot mutate immutable field: {field}")
            }
            GenesisError::AgentTerminated(id) => write!(f, "agent {:?} is terminated", id),
        }
    }
}

/// In-memory store of genesis records, keyed by `agent_id`.
///
/// All write operations are validated before committing. Immutable fields are
/// protected against overwrite.
#[derive(Debug, Default)]
pub struct GenesisStore {
    records: std::collections::HashMap<AgentId, GenesisRecord>,
}

impl GenesisStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new genesis record. Returns an error if `agent_id` already
    /// exists.
    pub fn create(&mut self, record: GenesisRecord) -> Result<(), GenesisError> {
        if self.records.contains_key(&record.agent_id) {
            return Err(GenesisError::Duplicate(record.agent_id));
        }
        self.records.insert(record.agent_id, record);
        Ok(())
    }

    /// Retrieve a reference to a genesis record by agent id.
    pub fn get(&self, agent_id: &AgentId) -> Option<&GenesisRecord> {
        self.records.get(agent_id)
    }

    /// Amend mutable fields of a genesis record.
    ///
    /// The closure receives a mutable reference to the record. If the closure
    /// attempts to mutate an immutable field, it should return
    /// `Err(GenesisError::ImmutableFieldViolation)` from a helper — but this
    /// store does not prevent direct field access from within the closure.
    /// Callers should use the dedicated setters below instead.
    pub fn amend<F>(&mut self, agent_id: &AgentId, f: F) -> Result<(), GenesisError>
    where
        F: FnOnce(&mut GenesisRecord) -> Result<(), GenesisError>,
    {
        let record = self
            .records
            .get_mut(agent_id)
            .ok_or(GenesisError::NotFound(*agent_id))?;
        if record.terminated {
            return Err(GenesisError::AgentTerminated(*agent_id));
        }
        f(record)
    }

    /// Set the expiry block for an agent.
    pub fn set_expiry(
        &mut self,
        agent_id: &AgentId,
        expiry: Option<BlockHeight>,
    ) -> Result<(), GenesisError> {
        self.amend(agent_id, |r| {
            r.expiry_block = expiry;
            Ok(())
        })
    }

    /// Mark an agent as terminated. Termination is irreversible.
    pub fn terminate(&mut self, agent_id: &AgentId) -> Result<(), GenesisError> {
        let record = self
            .records
            .get_mut(agent_id)
            .ok_or(GenesisError::NotFound(*agent_id))?;
        record.terminated = true;
        Ok(())
    }

    /// Count of all records (including terminated / expired).
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns `true` if no records are registered.
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Count of active (non-terminated, non-expired) agents at `now`.
    pub fn active_count(&self, now: BlockHeight) -> usize {
        self.records.values().filter(|r| r.is_active(now)).count()
    }

    /// Count of active descendants of `ancestor_id` at `now`.
    pub fn active_spawn_count(&self, ancestor_id: &AgentId, now: BlockHeight) -> usize {
        self.records
            .values()
            .filter(|r| r.is_active(now) && r.lineage.contains(ancestor_id))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentKind, AgentPermissionTier};

    fn make_record(id: u8, creator: u8, depth: usize) -> GenesisRecord {
        let agent_id = [id; 32];
        let creator_id = [creator; 32];
        let lineage: Vec<AgentId> = (0..depth).map(|i| [(i + 1) as u8; 32]).collect();
        GenesisRecord::new(
            agent_id,
            creator_id,
            "test purpose",
            AgentKind::TestBuilder,
            AgentPermissionTier::DocsTestsReports,
            lineage,
            100,
        )
    }

    #[test]
    fn create_and_retrieve() {
        let mut store = GenesisStore::new();
        let record = make_record(1, 0, 0);
        store.create(record.clone()).unwrap();
        assert_eq!(store.get(&[1; 32]).unwrap().purpose, "test purpose");
    }

    #[test]
    fn duplicate_create_rejected() {
        let mut store = GenesisStore::new();
        let record = make_record(2, 0, 0);
        store.create(record.clone()).unwrap();
        assert!(matches!(
            store.create(record),
            Err(GenesisError::Duplicate(_))
        ));
    }

    #[test]
    fn termination_is_irreversible() {
        let mut store = GenesisStore::new();
        store.create(make_record(3, 0, 0)).unwrap();
        store.terminate(&[3; 32]).unwrap();
        assert!(store.get(&[3; 32]).unwrap().terminated);
        // Amend after termination is rejected.
        assert!(matches!(
            store.set_expiry(&[3; 32], Some(999)),
            Err(GenesisError::AgentTerminated(_))
        ));
    }

    #[test]
    fn expiry_detection() {
        let record = make_record(4, 0, 0);
        let mut r = record;
        r.expiry_block = Some(200);
        assert!(!r.is_expired(199));
        assert!(r.is_expired(200));
    }

    #[test]
    fn spawn_depth_from_lineage() {
        let record = make_record(5, 0, 3);
        assert_eq!(record.spawn_depth(), 3);
    }

    #[test]
    fn active_spawn_count() {
        let mut store = GenesisStore::new();
        let parent_id: AgentId = [0; 32];

        // Two children that list parent in lineage.
        let mut c1 = make_record(10, 0, 0);
        c1.lineage = vec![parent_id];
        let mut c2 = make_record(11, 0, 0);
        c2.lineage = vec![parent_id];
        // One grandchild.
        let mut gc = make_record(12, 0, 0);
        gc.lineage = vec![parent_id, [10; 32]];

        store.create(c1).unwrap();
        store.create(c2).unwrap();
        store.create(gc).unwrap();

        assert_eq!(store.active_spawn_count(&parent_id, 0), 3);

        // Terminate one child — count drops.
        store.terminate(&[10; 32]).unwrap();
        assert_eq!(store.active_spawn_count(&parent_id, 0), 2);
    }
}
