//! Spawn enforcement — validates INV-S-004 (spawn depth limits) and
//! INV-S-003 (per-class spawn caps).
//!
//! Call [`SpawnGuard::check`] before allowing any agent to spawn a child.
//! The guard consults [`GenesisStore`] for live counts and the static limits
//! defined in [`SPAWN_LIMITS`].

use crate::{
    genesis::{AgentId, BlockHeight, GenesisStore},
    AgentKind,
};

/// Maximum allowed spawn depth per agent class.
/// A depth of 0 means the agent may only be spawned directly by an operator
/// (root genesis); it cannot itself spawn children.
///
/// Values are drawn from AGENT_LAW.md §2.1.
pub fn max_spawn_depth(kind: &AgentKind) -> usize {
    match kind {
        AgentKind::RepoScanner => 2,
        AgentKind::FeatureMapper => 2,
        AgentKind::TestBuilder => 1,
        AgentKind::Integrator => 2,
        AgentKind::BuildFixer => 1,
        AgentKind::WiringInspector => 1,
        AgentKind::Auditor => 0,
        AgentKind::Breaker => 0,
        AgentKind::Fixer => 1,
        AgentKind::ReadinessReporter => 1,
        AgentKind::Benchmark => 1,
        AgentKind::Marketing => 1,
        AgentKind::Grant => 0,
        AgentKind::ApprovalGate => 0,
    }
}

/// Maximum number of active direct spawns per parent agent.
///
/// An agent of any class may not have more than this many live children at any
/// one time unless the class-specific override applies.
pub const DEFAULT_MAX_DIRECT_SPAWNS: usize = 5;

/// Error returned by spawn enforcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpawnError {
    /// The child's class is not allowed to be spawned at the requested depth.
    DepthExceeded {
        kind: AgentKind,
        requested_depth: usize,
        max: usize,
    },
    /// The parent already has too many active spawns.
    SpawnCapExceeded {
        parent_id: AgentId,
        active: usize,
        max: usize,
    },
    /// Parent agent is not registered in the genesis store.
    ParentNotFound(AgentId),
    /// Parent agent is terminated or expired.
    ParentInactive(AgentId),
}

impl core::fmt::Display for SpawnError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SpawnError::DepthExceeded {
                kind,
                requested_depth,
                max,
            } => write!(
                f,
                "{kind:?} cannot be spawned at depth {requested_depth} (max {max})"
            ),
            SpawnError::SpawnCapExceeded {
                parent_id,
                active,
                max,
            } => write!(
                f,
                "parent {:?} already has {active} active spawns (max {max})",
                parent_id
            ),
            SpawnError::ParentNotFound(id) => write!(f, "parent {:?} not found", id),
            SpawnError::ParentInactive(id) => {
                write!(f, "parent {:?} is terminated or expired", id)
            }
        }
    }
}

/// Stateless guard for spawn requests. Requires a reference to the
/// `GenesisStore` to check live spawn counts.
pub struct SpawnGuard<'a> {
    store: &'a GenesisStore,
    now: BlockHeight,
}

impl<'a> SpawnGuard<'a> {
    pub fn new(store: &'a GenesisStore, now: BlockHeight) -> Self {
        Self { store, now }
    }

    /// Validate a proposed spawn.
    ///
    /// `parent_id` — the agent requesting the spawn.
    /// `child_kind` — the class of the agent to be spawned.
    /// `child_lineage` — the lineage that the new child record will carry.
    ///   Typically `parent.lineage + [parent_id]`.
    ///
    /// Returns `Ok(())` if spawn is allowed.
    pub fn check(
        &self,
        parent_id: &AgentId,
        child_kind: &AgentKind,
        child_lineage: &[AgentId],
    ) -> Result<(), SpawnError> {
        // Validate parent exists and is active.
        let parent = self
            .store
            .get(parent_id)
            .ok_or(SpawnError::ParentNotFound(*parent_id))?;

        if !parent.is_active(self.now) {
            return Err(SpawnError::ParentInactive(*parent_id));
        }

        // Depth check: child_lineage.len() == requested spawn depth.
        let requested_depth = child_lineage.len();
        let max = max_spawn_depth(child_kind);
        if requested_depth > max {
            return Err(SpawnError::DepthExceeded {
                kind: child_kind.clone(),
                requested_depth,
                max,
            });
        }

        // Per-parent spawn cap.
        let active = self.store.active_spawn_count(parent_id, self.now);
        if active >= DEFAULT_MAX_DIRECT_SPAWNS {
            return Err(SpawnError::SpawnCapExceeded {
                parent_id: *parent_id,
                active,
                max: DEFAULT_MAX_DIRECT_SPAWNS,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        genesis::{GenesisRecord, GenesisStore},
        AgentKind, AgentPermissionTier,
    };

    fn store_with_parent(parent_id: AgentId) -> GenesisStore {
        let mut store = GenesisStore::new();
        let parent = GenesisRecord::new(
            parent_id,
            [0; 32],
            "parent",
            AgentKind::Integrator,
            AgentPermissionTier::TauriServiceWiring,
            vec![],
            1,
        );
        store.create(parent).unwrap();
        store
    }

    #[test]
    fn spawn_depth_zero_class_at_depth_1_rejected() {
        let parent_id = [1; 32];
        let store = store_with_parent(parent_id);
        let guard = SpawnGuard::new(&store, 10);
        // Auditor max_spawn_depth == 0; requesting depth 1 should fail.
        let lineage = vec![parent_id];
        let err = guard
            .check(&parent_id, &AgentKind::Auditor, &lineage)
            .unwrap_err();
        assert!(matches!(err, SpawnError::DepthExceeded { .. }));
    }

    #[test]
    fn spawn_within_depth_allowed() {
        let parent_id = [2; 32];
        let store = store_with_parent(parent_id);
        let guard = SpawnGuard::new(&store, 10);
        // Integrator max_spawn_depth == 2; depth 1 should pass.
        let lineage = vec![parent_id];
        guard
            .check(&parent_id, &AgentKind::Integrator, &lineage)
            .unwrap();
    }

    #[test]
    fn unknown_parent_rejected() {
        let store = GenesisStore::new();
        let guard = SpawnGuard::new(&store, 10);
        let err = guard
            .check(&[9; 32], &AgentKind::TestBuilder, &[])
            .unwrap_err();
        assert!(matches!(err, SpawnError::ParentNotFound(_)));
    }

    #[test]
    fn terminated_parent_rejected() {
        let parent_id = [3; 32];
        let mut store = store_with_parent(parent_id);
        store.terminate(&parent_id).unwrap();
        let guard = SpawnGuard::new(&store, 10);
        let err = guard
            .check(&parent_id, &AgentKind::TestBuilder, &[])
            .unwrap_err();
        assert!(matches!(err, SpawnError::ParentInactive(_)));
    }

    #[test]
    fn spawn_cap_exceeded_rejected() {
        let parent_id = [4; 32];
        let mut store = store_with_parent(parent_id);

        // Add DEFAULT_MAX_DIRECT_SPAWNS children that all list parent in lineage.
        for i in 0..DEFAULT_MAX_DIRECT_SPAWNS {
            let mut child = GenesisRecord::new(
                [(i + 10) as u8; 32],
                parent_id,
                "child",
                AgentKind::TestBuilder,
                AgentPermissionTier::DocsTestsReports,
                vec![parent_id],
                2,
            );
            child.lineage = vec![parent_id];
            store.create(child).unwrap();
        }

        let guard = SpawnGuard::new(&store, 10);
        let err = guard
            .check(&parent_id, &AgentKind::TestBuilder, &[parent_id])
            .unwrap_err();
        assert!(matches!(err, SpawnError::SpawnCapExceeded { .. }));
    }
}
