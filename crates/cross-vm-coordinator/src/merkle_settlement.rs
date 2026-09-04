//! Cross-VM coordinator settlement facade.
//!
//! Merkle proof validation lives in the bridge crate. The coordinator keeps a
//! thin facade so older orchestration code can import settlement types without
//! owning validation semantics.

pub use x3_cross_vm_bridge::merkle_settlement_bridge::{
    MerkleEnabledSettlement, MerkleSettlementExt,
};
