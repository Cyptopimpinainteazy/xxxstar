//! X3 Autonomic Core
//!
//! Self-testing, self-auditing, and controlled self-improvement layer
//! for the X3 multi-VM blockchain.

pub mod prelude {
    pub use crate::*;
}

/// Workspace-level constants
pub const WORKSPACE_NAME: &str = "x3-autonomic-core";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");