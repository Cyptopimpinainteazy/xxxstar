//! The one EVM signer the adapters share.
//!
//! It moved out of `chains/base` when Arbitrum needed it too: two adapters holding
//! copies of a key type is how two signing paths start, and the workspace already
//! has exactly one EIP-155 implementation (`x3-atomic-swap`'s `ethereum_tx`) —
//! this type holds the key and that crate does the crypto.
//!
//! Only available with `std`: a `no_std` build has no signer, and every send path
//! refuses there.

#[cfg(feature = "std")]
use crate::adapter::AdapterResult;
#[cfg(feature = "std")]
use crate::error::ExternalChainError;
#[cfg(feature = "std")]
use sp_core::H160;

/// An EIP-155 signer for one chain.
///
/// Key material stays here and never goes into [`crate::adapter::ChainConfig`],
/// which is SCALE-encoded, logged and serialised.
#[cfg(feature = "std")]
#[derive(Debug, Clone)]
pub struct EvmSigner {
    private_key_hex: String,
    address: H160,
}

#[cfg(feature = "std")]
impl EvmSigner {
    /// Derive the sender's address from a 32-byte private key (`0x`-prefixed).
    ///
    /// A key that does not derive an address is refused here rather than at the
    /// first send: failing later puts the error somewhere harder to read.
    pub fn from_private_key(private_key_hex: &str) -> AdapterResult<Self> {
        let address_hex =
            x3_atomic_swap::ethereum_tx::Transaction::address_from_private_key(private_key_hex)
                .map_err(|e| ExternalChainError::internal(&format!("invalid private key: {e}")))?;
        let bytes = hex::decode(address_hex.trim_start_matches("0x")).map_err(|e| {
            ExternalChainError::internal(&format!("derived address is not hex: {e}"))
        })?;
        if bytes.len() != 20 {
            return Err(ExternalChainError::internal(&format!(
                "derived address is {} bytes, not 20",
                bytes.len()
            )));
        }
        let mut address = [0u8; 20];
        address.copy_from_slice(&bytes);
        Ok(Self {
            private_key_hex: private_key_hex.to_string(),
            address: H160(address),
        })
    }

    /// The address this signer sends from.
    pub fn address(&self) -> H160 {
        self.address
    }

    pub(crate) fn private_key_hex(&self) -> &str {
        &self.private_key_hex
    }

    /// Sign an EIP-155 transaction, returning the raw `0x…` payload for
    /// `eth_sendRawTransaction`.
    pub(crate) fn sign_transaction(
        &self,
        transaction: x3_atomic_swap::ethereum_tx::Transaction,
    ) -> AdapterResult<String> {
        transaction
            .sign(self.private_key_hex())
            .map_err(|e| ExternalChainError::internal(&format!("could not sign: {e}")))
    }
}
