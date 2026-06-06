//! Agent subsystem — identity, on-chain agents, off-chain agents, lifecycle.

pub mod identity;
pub mod lifecycle;
pub mod off_chain;
pub mod on_chain;

pub use identity::{AgentIdentity, AlignmentScore};
pub use lifecycle::{AgentLifecycle, LifecycleEvent, SpawnParams};
pub use off_chain::{OffChainAgent, OffChainRole};
pub use on_chain::{OnChainAgent, OnChainStatus};
