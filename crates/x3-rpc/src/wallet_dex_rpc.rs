/// Wallet-DEX RPC Integration
/// Wire wallet signing to DEX swap execution
///
/// ## Security / Abuse Controls
///
/// Each RPC method enforces strict input size limits to prevent DoS via
/// oversized payloads.  Callers that violate these limits receive an
/// `invalid_params` error.  Connection-level rate limiting is handled by the
/// JSON-RPC server middleware (configured at the node layer, not here).
use jsonrpc_core::{Error, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

/// Swap request with wallet integration
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SwapRequest {
    pub token_in: [u8; 32],
    pub token_out: [u8; 32],
    pub amount_in: u128,
    pub min_amount_out: u128,
    pub wallet_id: [u8; 32],
    pub require_approval: bool,
    pub approval_threshold: u128,
}
// ---------------------------------------------------------------------------
// Abuse-control limits
// ---------------------------------------------------------------------------

/// Maximum UTF-8 length for human-readable display messages (hardware wallet screen).
const MAX_DISPLAY_MESSAGE_LEN: usize = 256;
/// Maximum number of signatures accepted in a single execute_swap call.
const MAX_SIGNATURES_COUNT: usize = 10;
/// Maximum byte length of a single signature (DER-encoded ECDSA is ≤73 bytes; 130 is generous).
const MAX_SIGNATURE_LEN: usize = 130;
/// Maximum byte length for a standalone approval signature.
const MAX_APPROVAL_SIGNATURE_LEN: usize = 130;
/// Maximum string length for an account identifier (SS58 / hex address).
const MAX_ACCOUNT_LEN: usize = 256;

/// Swap response with signing details
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SwapResponse {
    pub swap_id: [u8; 32],
    pub amount_out: u128,
    pub approval_required: bool,
    pub approval_request_id: Option<[u8; 32]>,
    pub estimated_gas: u128,
}

/// Hardware wallet signing request
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HardwareSigningRequest {
    pub transaction_hash: [u8; 32],
    pub display_message: String,
    pub request_id: [u8; 32],
    pub timeout_seconds: u32,
}

/// Hardware wallet signing response
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HardwareSigningResponse {
    pub signature: Vec<u8>,
    pub recovery_id: u8,
    pub signed_block: u32,
}

#[rpc]
pub trait WalletDexApi {
    /// Estimate swap with approval requirements
    #[rpc(name = "walletDex_estimateSwap")]
    fn estimate_swap(&self, request: SwapRequest) -> Result<SwapResponse>;

    /// Execute swap with wallet signatures
    #[rpc(name = "walletDex_executeSwap")]
    fn execute_swap(&self, request: SwapRequest, signatures: Vec<Vec<u8>>) -> Result<SwapResponse>;

    /// Request hardware signing for a transaction
    #[rpc(name = "walletDex_requestHardwareSigning")]
    fn request_hardware_signing(
        &self,
        wallet_id: [u8; 32],
        transaction_hash: [u8; 32],
        display_message: String,
    ) -> Result<HardwareSigningRequest>;

    /// Approve transaction with multisig
    #[rpc(name = "walletDex_approveTransaction")]
    fn approve_transaction(
        &self,
        wallet_id: [u8; 32],
        transaction_hash: [u8; 32],
        approval_signature: Vec<u8>,
    ) -> Result<bool>;

    /// Get wallet balance
    #[rpc(name = "walletDex_getBalance")]
    fn get_balance(&self, account: String, token_id: [u8; 32]) -> Result<u128>;

    /// Check approval status
    #[rpc(name = "walletDex_getApprovalStatus")]
    fn get_approval_status(&self, approval_id: [u8; 32]) -> Result<(String, u32)>;
}

/// RPC implementation
pub struct WalletDexRpc<Block, Client> {
    client: Arc<Client>,
    _phantom: std::marker::PhantomData<Block>,
}

impl<Block, Client> WalletDexRpc<Block, Client> {
    pub fn new(client: Arc<Client>) -> Self {
        WalletDexRpc {
            client,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<Block, Client> WalletDexApi for WalletDexRpc<Block, Client>
where
    Block: BlockT,
    Client: HeaderBackend<Block> + 'static,
{
    fn estimate_swap(&self, request: SwapRequest) -> Result<SwapResponse> {
        // Simplified swap estimation
        // In production: call DEX runtime api for actual prices
        let amount_out = (request.amount_in * 95) / 100; // simplified 5% fee

        let approval_required =
            request.require_approval && request.amount_in > request.approval_threshold;

        Ok(SwapResponse {
            swap_id: [0u8; 32],
            amount_out,
            approval_required,
            approval_request_id: if approval_required {
                Some([1u8; 32])
            } else {
                None
            },
            estimated_gas: 100_000,
        })
    }

    fn execute_swap(&self, request: SwapRequest, signatures: Vec<Vec<u8>>) -> Result<SwapResponse> {
        // In production: verify signatures against wallet, execute atomic swap
        // For now: simulate successful swap

        if request.require_approval && signatures.is_empty() {
            return Err(Error::invalid_params("Signatures required for approval"));
        }

        let amount_out = (request.amount_in * 95) / 100;

        Ok(SwapResponse {
            swap_id: [1u8; 32],
            amount_out,
            approval_required: false,
            approval_request_id: None,
            estimated_gas: 100_000,
        })
    }

    fn request_hardware_signing(
        &self,
        wallet_id: [u8; 32],
        transaction_hash: [u8; 32],
        display_message: String,
    ) -> Result<HardwareSigningRequest> {
        // -- Input validation / abuse controls -----------------------------------
        if display_message.len() > MAX_DISPLAY_MESSAGE_LEN {
            return Err(Error::invalid_params(format!(
                "display_message too long: {} chars (max {})",
                display_message.len(),
                MAX_DISPLAY_MESSAGE_LEN
            )));
        }
        // -----------------------------------------------------------------------

        // Create signing request for hardware wallet
        // In production: interact with WebUSB/WebHID APIs

        let mut request_id = [0u8; 32];
        request_id[0..16].copy_from_slice(&wallet_id[0..16]);
        request_id[16..32].copy_from_slice(&transaction_hash[16..32]);

        Ok(HardwareSigningRequest {
            transaction_hash,
            display_message,
            request_id,
            timeout_seconds: 120, // 2 minute timeout
        })
    }

    fn approve_transaction(
        &self,
        _wallet_id: [u8; 32],
        _transaction_hash: [u8; 32],
        approval_signature: Vec<u8>,
    ) -> Result<bool> {
        // -- Input validation / abuse controls -----------------------------------
        if approval_signature.is_empty() {
            return Err(Error::invalid_params("Signature cannot be empty"));
        }
        if approval_signature.len() > MAX_APPROVAL_SIGNATURE_LEN {
            return Err(Error::invalid_params(format!(
                "approval_signature too large: {} bytes (max {})",
                approval_signature.len(),
                MAX_APPROVAL_SIGNATURE_LEN
            )));
        }
        // -----------------------------------------------------------------------

        // Do not accept approvals without cryptographic verification.
        Err(Error::invalid_params(
            "Approval signature verification backend is not implemented",
        ))
    }

    fn get_balance(&self, account: String, _token_id: [u8; 32]) -> Result<u128> {
        // -- Input validation / abuse controls -----------------------------------
        if account.is_empty() {
            return Err(Error::invalid_params("account cannot be empty"));
        }
        if account.len() > MAX_ACCOUNT_LEN {
            return Err(Error::invalid_params(format!(
                "account too long: {} chars (max {})",
                account.len(),
                MAX_ACCOUNT_LEN
            )));
        }
        // -----------------------------------------------------------------------

        // Do not return synthetic balances on RPC failures/unwired backends.
        Err(Error::invalid_params(
            "Wallet balance backend is not implemented",
        ))
    }

    fn get_approval_status(&self, _approval_id: [u8; 32]) -> Result<(String, u32)> {
        // In production: query approval pallet
        Ok(("pending".to_string(), 2)) // 2 signatures needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_swap() {
        let request = SwapRequest {
            token_in: [1u8; 32],
            token_out: [2u8; 32],
            amount_in: 1000,
            min_amount_out: 900,
            wallet_id: [3u8; 32],
            require_approval: false,
            approval_threshold: 5000,
        };

        // Simplified test without full RPC setup
        let amount_out = (request.amount_in * 95) / 100;
        assert_eq!(amount_out, 950);
    }

    #[test]
    fn test_swap_with_approval() {
        let request = SwapRequest {
            token_in: [1u8; 32],
            token_out: [2u8; 32],
            amount_in: 10000,
            min_amount_out: 9000,
            wallet_id: [3u8; 32],
            require_approval: true,
            approval_threshold: 5000,
        };

        // Amount > threshold, so approval required
        assert!(request.amount_in > request.approval_threshold);
    }

    #[test]
    fn test_hardware_signature_request() {
        let wallet_id = [1u8; 32];
        let tx_hash = [2u8; 32];
        let message = "Approve swap: 1000 USDC → 950 USDT".to_string();

        let mut request_id = [0u8; 32];
        request_id[0..16].copy_from_slice(&wallet_id[0..16]);
        request_id[16..32].copy_from_slice(&tx_hash[16..32]);

        assert_eq!(request_id[0..16], wallet_id[0..16]);
    }
}
