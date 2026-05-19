//! Asset mapping: cross-VM address/contract mapping for bridged assets.
//!
//! Maps canonical X3 asset IDs to their EVM contract addresses and SVM program IDs,
//! and enforces that a canonical asset cannot be double-mapped to conflicting contracts.

use frame_support::pallet_prelude::*;

pub type AssetId = u32;
pub type EvmAddress = [u8; 20];
pub type SvmProgramId = [u8; 32];

/// A single cross-VM mapping record.
#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct AssetMapping {
    /// Canonical X3 asset ID.
    pub asset_id: AssetId,
    /// EVM ERC-20 contract address (all-zero if not mapped).
    pub evm_address: EvmAddress,
    /// SVM SPL token program ID (all-zero if not mapped).
    pub svm_program_id: SvmProgramId,
    /// Whether this mapping is currently active.
    pub active: bool,
}

impl AssetMapping {
    pub fn has_evm(&self) -> bool {
        self.evm_address != [0u8; 20]
    }
    pub fn has_svm(&self) -> bool {
        self.svm_program_id != [0u8; 32]
    }
}

/// Errors returned by mapping operations.
#[derive(Debug, PartialEq, Eq)]
pub enum MappingError {
    /// Mapping for this asset already exists.
    AlreadyMapped,
    /// Mapping for this asset does not exist.
    NotMapped,
    /// The provided EVM address conflicts with an existing mapping.
    EvmAddressConflict,
    /// The provided SVM program ID conflicts with an existing mapping.
    SvmProgramConflict,
    /// Cannot modify an inactive mapping.
    InactiveMappingError,
}

#[cfg(test)]
pub struct InMemoryMappingStore {
    by_asset: std::collections::BTreeMap<AssetId, AssetMapping>,
    evm_to_asset: std::collections::BTreeMap<[u8; 20], AssetId>,
    svm_to_asset: std::collections::BTreeMap<[u8; 32], AssetId>,
}

#[cfg(test)]
impl InMemoryMappingStore {
    pub fn new() -> Self {
        Self {
            by_asset: Default::default(),
            evm_to_asset: Default::default(),
            svm_to_asset: Default::default(),
        }
    }

    pub fn register(
        &mut self,
        asset_id: AssetId,
        evm_address: EvmAddress,
        svm_program_id: SvmProgramId,
    ) -> Result<(), MappingError> {
        if self.by_asset.contains_key(&asset_id) {
            return Err(MappingError::AlreadyMapped);
        }
        if evm_address != [0u8; 20] && self.evm_to_asset.contains_key(&evm_address) {
            return Err(MappingError::EvmAddressConflict);
        }
        if svm_program_id != [0u8; 32] && self.svm_to_asset.contains_key(&svm_program_id) {
            return Err(MappingError::SvmProgramConflict);
        }
        if evm_address != [0u8; 20] {
            self.evm_to_asset.insert(evm_address, asset_id);
        }
        if svm_program_id != [0u8; 32] {
            self.svm_to_asset.insert(svm_program_id, asset_id);
        }
        self.by_asset.insert(
            asset_id,
            AssetMapping {
                asset_id,
                evm_address,
                svm_program_id,
                active: true,
            },
        );
        Ok(())
    }

    pub fn get(&self, asset_id: AssetId) -> Result<&AssetMapping, MappingError> {
        self.by_asset.get(&asset_id).ok_or(MappingError::NotMapped)
    }

    pub fn resolve_evm(&self, addr: &EvmAddress) -> Option<AssetId> {
        self.evm_to_asset.get(addr).copied()
    }

    pub fn resolve_svm(&self, prog: &SvmProgramId) -> Option<AssetId> {
        self.svm_to_asset.get(prog).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evm(b: u8) -> EvmAddress {
        [b; 20]
    }
    fn svm(b: u8) -> SvmProgramId {
        [b; 32]
    }

    #[test]
    fn test_register_and_resolve() {
        let mut store = InMemoryMappingStore::new();
        store.register(1, evm(1), svm(1)).unwrap();
        assert_eq!(store.resolve_evm(&evm(1)), Some(1));
        assert_eq!(store.resolve_svm(&svm(1)), Some(1));
    }

    #[test]
    fn test_double_register_rejected() {
        let mut store = InMemoryMappingStore::new();
        store.register(1, evm(1), svm(1)).unwrap();
        assert_eq!(
            store.register(1, evm(2), svm(2)),
            Err(MappingError::AlreadyMapped)
        );
    }

    #[test]
    fn test_evm_address_conflict_rejected() {
        let mut store = InMemoryMappingStore::new();
        store.register(1, evm(1), svm(1)).unwrap();
        assert_eq!(
            store.register(2, evm(1), svm(2)),
            Err(MappingError::EvmAddressConflict)
        );
    }

    #[test]
    fn test_svm_program_conflict_rejected() {
        let mut store = InMemoryMappingStore::new();
        store.register(1, evm(1), svm(1)).unwrap();
        assert_eq!(
            store.register(2, evm(2), svm(1)),
            Err(MappingError::SvmProgramConflict)
        );
    }

    #[test]
    fn test_get_not_found() {
        let store = InMemoryMappingStore::new();
        assert_eq!(store.get(99), Err(MappingError::NotMapped));
    }

    #[test]
    fn test_resolve_unknown_evm_returns_none() {
        let store = InMemoryMappingStore::new();
        assert_eq!(store.resolve_evm(&evm(255)), None);
    }
}
