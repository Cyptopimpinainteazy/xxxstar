/// X3 Relayer Service Library
///
/// Provides components for watching EVM and SVM headers, acquiring finalized proofs,
/// and submitting them to the X3 runtime for cross-chain verification.
///
/// The `types` module is always available (including in `no_std`) so the
/// runtime gateway pallet can consume `ValidatorSignature` / `SvmProof` /
/// `EvmProof` without pulling in tokio. The service modules (`relayer`,
/// `submitter`, `watchers`) require the `std` feature.
#[cfg(feature = "std")]
pub mod relayer;
#[cfg(feature = "std")]
pub mod submitter;
pub mod types;
#[cfg(feature = "std")]
pub mod watchers;

#[cfg(feature = "std")]
pub use relayer::RelayerService;
#[cfg(feature = "std")]
pub use submitter::RpcSubmitter;
pub use types::*;
#[cfg(feature = "std")]
pub use watchers::{EvmHeaderWatcher, SvmHeaderWatcher};
