/// X3 Relayer Service Library
///
/// Provides components for watching EVM and SVM headers, acquiring finalized proofs,
/// and submitting them to the X3 runtime for cross-chain verification.
pub mod relayer;
pub mod submitter;
pub mod types;
pub mod watchers;

pub use relayer::RelayerService;
pub use submitter::RpcSubmitter;
pub use types::*;
pub use watchers::{EvmHeaderWatcher, SvmHeaderWatcher};
