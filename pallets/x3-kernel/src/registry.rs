//! Asset registry: canonical metadata and lifecycle for all assets on X3 Chain.

use frame_support::pallet_prelude::*;

pub type AssetId = u32;

/// Canonical metadata for a registered asset.
#[derive(Clone, Debug, Default, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct AssetMeta {
    /// Human-readable ticker, at most 12 bytes.
    pub symbol: BoundedVec<u8, ConstU32<12>>,
    /// Decimal places (e.g. 18 for ETH-like assets).
    pub decimals: u8,
    /// Whether new supply can be minted.
    pub mintable: bool,
    /// Whether supply can be burned.
    pub burnable: bool,
    /// Whether the asset is currently paused (no transfers).
    pub paused: bool,
}

/// Errors returned by registry operations.
#[derive(Debug, PartialEq, Eq)]
pub enum RegistryError {
    /// Asset with this ID is already registered.
    AlreadyRegistered,
    /// Asset with this ID does not exist.
    NotFound,
    /// The provided symbol is too long.
    SymbolTooLong,
    /// Operation not permitted on a paused asset.
    AssetPaused,
    /// Operation requires the asset to be mintable.
    NotMintable,
    /// Operation requires the asset to be burnable.
    NotBurnable,
}

/// In-memory registry for unit tests.
#[cfg(test)]
pub struct InMemoryRegistry {
    assets: std::collections::BTreeMap<AssetId, AssetMeta>,
    next_id: AssetId,
}

#[cfg(test)]
impl InMemoryRegistry {
    pub fn new() -> Self {
        Self {
            assets: Default::default(),
            next_id: 1,
        }
    }

    pub fn register(
        &mut self,
        symbol: &[u8],
        decimals: u8,
        mintable: bool,
        burnable: bool,
    ) -> Result<AssetId, RegistryError> {
        let sym: BoundedVec<u8, ConstU32<12>> = symbol
            .to_vec()
            .try_into()
            .map_err(|_| RegistryError::SymbolTooLong)?;
        let id = self.next_id;
        self.next_id += 1;
        self.assets.insert(
            id,
            AssetMeta {
                symbol: sym,
                decimals,
                mintable,
                burnable,
                paused: false,
            },
        );
        Ok(id)
    }

    pub fn get(&self, id: AssetId) -> Result<&AssetMeta, RegistryError> {
        self.assets.get(&id).ok_or(RegistryError::NotFound)
    }

    pub fn pause(&mut self, id: AssetId) -> Result<(), RegistryError> {
        let meta = self.assets.get_mut(&id).ok_or(RegistryError::NotFound)?;
        meta.paused = true;
        Ok(())
    }

    pub fn unpause(&mut self, id: AssetId) -> Result<(), RegistryError> {
        let meta = self.assets.get_mut(&id).ok_or(RegistryError::NotFound)?;
        meta.paused = false;
        Ok(())
    }

    pub fn assert_mintable(&self, id: AssetId) -> Result<(), RegistryError> {
        let meta = self.get(id)?;
        if meta.paused {
            return Err(RegistryError::AssetPaused);
        }
        if !meta.mintable {
            return Err(RegistryError::NotMintable);
        }
        Ok(())
    }

    pub fn assert_burnable(&self, id: AssetId) -> Result<(), RegistryError> {
        let meta = self.get(id)?;
        if meta.paused {
            return Err(RegistryError::AssetPaused);
        }
        if !meta.burnable {
            return Err(RegistryError::NotBurnable);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let mut reg = InMemoryRegistry::new();
        let id = reg.register(b"X3T", 18, true, true).unwrap();
        let meta = reg.get(id).unwrap();
        assert_eq!(meta.decimals, 18);
        assert!(meta.mintable);
    }

    #[test]
    fn test_symbol_too_long_rejected() {
        let mut reg = InMemoryRegistry::new();
        assert_eq!(
            reg.register(b"WAYTOOLONGSYMBOL", 18, true, true),
            Err(RegistryError::SymbolTooLong)
        );
    }

    #[test]
    fn test_pause_blocks_mint() {
        let mut reg = InMemoryRegistry::new();
        let id = reg.register(b"TKN", 6, true, true).unwrap();
        reg.pause(id).unwrap();
        assert_eq!(reg.assert_mintable(id), Err(RegistryError::AssetPaused));
    }

    #[test]
    fn test_unpause_allows_mint() {
        let mut reg = InMemoryRegistry::new();
        let id = reg.register(b"TKN", 6, true, true).unwrap();
        reg.pause(id).unwrap();
        reg.unpause(id).unwrap();
        assert!(reg.assert_mintable(id).is_ok());
    }

    #[test]
    fn test_not_mintable_rejected() {
        let mut reg = InMemoryRegistry::new();
        let id = reg.register(b"RO", 0, false, false).unwrap();
        assert_eq!(reg.assert_mintable(id), Err(RegistryError::NotMintable));
    }

    #[test]
    fn test_unknown_asset_returns_not_found() {
        let reg = InMemoryRegistry::new();
        assert_eq!(reg.get(999), Err(RegistryError::NotFound));
    }
}
