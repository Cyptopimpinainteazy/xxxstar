//! X3 Swarm Core - Autonomous Agent System
//!
//! This crate provides the core infrastructure for X3's autonomous swarm agents
//! that can scan, build, test, audit, break, fix, and prove the system.

pub mod agent;
pub mod approval;
pub mod audit;
pub mod authority;
pub mod events;
pub mod genesis;
pub mod guard;
pub mod memory;
pub mod misconduct;
pub mod permissions;
pub mod policy;
pub mod report;
pub mod scheduler;
pub mod scoreboard;
pub mod spawn;
pub mod task;

pub use agent::{AgentKind, AgentPermissionTier};
pub use approval::ApprovalGate;
pub use audit::{AuditCategory, AuditEntry, AuditLog};
pub use authority::{AuthorityError, SwarmAuthority};
pub use events::SwarmEvent;
pub use genesis::{AgentId, BlockHeight, GenesisError, GenesisRecord, GenesisStore, SupervisionMode};
pub use guard::{evaluate_path, ForbiddenPathGuard, GuardAction};
pub use memory::SwarmMemoryEntry;
pub use misconduct::{MisconductEngine, MisconductError, Sanction, ViolationClass, ViolationRecord};
pub use policy::{default_agent_policies, AgentPolicy, ApprovalRequirement};
pub use report::SwarmReport;
pub use scheduler::SwarmScheduler;
pub use scoreboard::SwarmScoreboard;
pub use spawn::{max_spawn_depth, SpawnError, SpawnGuard, DEFAULT_MAX_DIRECT_SPAWNS};
pub use task::{AgentResult, AgentTask, TaskStatus};
