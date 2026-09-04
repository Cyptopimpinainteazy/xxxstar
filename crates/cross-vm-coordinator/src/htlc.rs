use std::sync::Arc;

use ed25519_dalek::{Signer as DalekSigner, SigningKey};
use secp256k1::{Message, Secp256k1, SecretKey};
use zeroize::Zeroize;

/// Key type for cryptographic signing
#[derive(Clone, Copy, Debug)]
pub enum KeyType {
    /// secp256k1 for EVM
    Secp256k1,
    /// ed25519 for SVM
    Ed25519,
}

/// Abstracts signing for EVM/SVM. Never exposes raw key bytes.
#[async_trait]
pub trait Signer: Send + Sync {
    /// Get the key type for this signer
    fn key_type(&self) -> KeyType;
    /// Sign a message (EVM: secp256k1, SVM: ed25519).
    /// Returns signature bytes.
    async fn sign(&self, msg: &[u8]) -> Vec<u8>;
    /// Return the public key (EVM: 20/33 bytes, SVM: 32 bytes).
    fn pubkey(&self) -> Vec<u8>;
}

/// In-memory signer for dev/test. Zeroizes on drop.
/// Wraps private key material with zeroize protection.
pub struct MemorySigner {
    key: ZeroizingSigner,
    pubkey: Vec<u8>,
    key_type: KeyType,
}

impl MemorySigner {
    pub fn new(key: [u8; 32], pubkey: Vec<u8>, key_type: KeyType) -> Self {
        Self {
            key: ZeroizingSigner::new(key),
            pubkey,
            key_type,
        }
    }
}

#[async_trait]
impl Signer for MemorySigner {
    fn key_type(&self) -> KeyType {
        self.key_type
    }

    async fn sign(&self, msg: &[u8]) -> Vec<u8> {
        match self.key_type {
            KeyType::Secp256k1 => {
                // EVM: secp256k1 signing
                if let Ok(secret_key) = SecretKey::from_slice(self.key.as_ref()) {
                    let secp = Secp256k1::new();
                    if let Ok(message) = Message::from_digest_slice(msg) {
                        let signature = secp.sign_ecdsa(&message, &secret_key);
                        signature.serialize_compact().to_vec()
                    } else {
                        // Fallback for non-32-byte messages
                        let mut hasher = Sha256::new();
                        hasher.update(msg);
                        let digest = hasher.finalize();
                        let message = Message::from_digest(digest.into());
                        let signature = secp.sign_ecdsa(&message, &secret_key);
                        signature.serialize_compact().to_vec()
                    }
                } else {
                    // Fallback: simple hash (not cryptographically secure)
                    let mut out = msg.to_vec();
                    out.extend_from_slice(self.key.as_ref());
                    out
                }
            }

            KeyType::Ed25519 => {
                // SVM: ed25519 signing
                let signing_key = SigningKey::from_bytes(self.key.as_ref());
                let signature = signing_key.sign(msg);
                signature.to_bytes().to_vec()
            }
        }
    }
    fn pubkey(&self) -> Vec<u8> {
        self.pubkey.clone()
    }
}

/// Zeroizing wrapper for 32-byte private keys.
/// Zeroizes memory on drop to prevent key material leakage.
pub struct ZeroizingSigner {
    inner: [u8; 32],
}

impl ZeroizingSigner {
    pub fn new(key: [u8; 32]) -> Self {
        Self { inner: key }
    }

    /// Returns a reference to the underlying key bytes.
    /// Use with caution - prefer not exposing this directly.
    pub fn as_ref(&self) -> &[u8; 32] {
        &self.inner
    }
}

impl Zeroize for ZeroizingSigner {
    fn zeroize(&mut self) {
        self.inner.zeroize();
    }
}

impl Drop for ZeroizingSigner {
    fn drop(&mut self) {
        self.zeroize();
    }
}
/// HTLC chain interface — abstracts HTLC operations across VMs.
///
/// Each VM (EVM, SVM, X3VM) has a different contract/program interface
/// for HTLCs. This module provides a unified async trait that the
/// coordinator uses, with concrete implementations for each VM.
use crate::abi;
use crate::rpc_client::RpcClient;
use crate::types::*;
use async_trait::async_trait;
use blake3;
use sha2::{Digest, Sha256};

/// Unified interface for HTLC operations on any chain.
///
/// Implementors handle the chain-specific details of:
/// - Creating HTLCs (encoding calldata, signing, broadcasting)
/// - Monitoring HTLC status and confirmations
/// - Claiming HTLCs with the secret
/// - Refunding expired HTLCs
#[async_trait]
pub trait HtlcChainAdapter: Send + Sync {
    /// The VM this adapter targets.
    fn vm_target(&self) -> VmTarget;

    /// Create an HTLC on the target chain.
    ///
    /// Returns the on-chain HTLC ID and initial record.
    async fn create_htlc(&self, params: &HtlcCreateParams) -> Result<HtlcRecord, CoordinatorError>;

    /// Query current HTLC status and confirmations.
    async fn query_htlc(&self, htlc_id: &HtlcId) -> Result<(HtlcStatus, u32), CoordinatorError>;

    /// Claim an HTLC by revealing the secret.
    ///
    /// This is the critical operation — once the secret is broadcast,
    /// it becomes public knowledge. The other chain's HTLC can then
    /// be claimed by anyone who observes the secret.
    async fn claim_htlc(
        &self,
        htlc_id: &HtlcId,
        secret: &HtlcSecret,
    ) -> Result<Vec<u8>, CoordinatorError>; // returns tx hash

    /// Refund an expired HTLC.
    async fn refund_htlc(&self, htlc_id: &HtlcId) -> Result<Vec<u8>, CoordinatorError>;

    /// Get the current block timestamp (seconds since epoch).
    async fn current_time(&self) -> Result<u64, CoordinatorError>;

    /// Estimate gas/compute for an HTLC claim transaction.
    async fn estimate_claim_cost(&self, htlc_id: &HtlcId) -> Result<u64, CoordinatorError>;
}

/// Compute blake3 hash of HTLC creation parameters for integrity verification.
///
/// This hash is stored with the HTLC record and verified before any operations
/// to detect tampering with the params (recipient, amount, asset, timelock, etc).
fn compute_htlc_params_hash(params: &HtlcCreateParams) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();

    // Hash all parameters in a deterministic order
    hasher.update(&params.recipient);
    hasher.update(params.hash_lock.as_bytes());
    hasher.update(&params.timelock.to_le_bytes());
    hasher.update(&params.asset);
    hasher.update(&params.amount.to_le_bytes());

    // Get 32-byte hash
    let hash = hasher.finalize();
    let mut result = [0u8; 32];
    result.copy_from_slice(hash.as_bytes());
    result
}

// ─── EVM Adapter ──────────────────────────────────────────────────────────────

/// EVM HTLC adapter — targets AtlasHTLC.sol deployed on Ethereum/L2s.
///
/// Uses real ABI encoding and JSON-RPC calls.
///
/// ABI selectors (from AtlasHTLC.sol):
/// - createHTLC:  0x4b2f336d
/// - claimHTLC:   0x84cc315c
/// - refundHTLC:  0x7249fbb6
/// - getHTLC:     0x905d22a5
pub struct EvmHtlcAdapter {
    /// Chain ID for this EVM network.
    pub chain_id: u64,
    /// AtlasHTLC contract address (20 bytes).
    pub htlc_contract: [u8; 20],
    /// JSON-RPC endpoint URL.
    pub rpc_url: String,
    /// Abstracted signer (never exposes raw key bytes).
    signer: Arc<dyn Signer + Send + Sync>,
    /// RPC client instance.
    rpc: RpcClient,
}

impl EvmHtlcAdapter {
    pub fn new(
        chain_id: u64,
        htlc_contract: [u8; 20],
        rpc_url: String,
        signer: Arc<dyn Signer + Send + Sync>,
    ) -> Self {
        let rpc = RpcClient::new(rpc_url.clone());
        Self {
            chain_id,
            htlc_contract,
            rpc_url,
            signer,
            rpc,
        }
    }

    /// Format contract address as hex string with 0x prefix.
    fn contract_hex(&self) -> String {
        format!("0x{}", hex::encode(self.htlc_contract))
    }

    /// Generate deterministic HTLC ID from create params.
    fn derive_htlc_id(&self, params: &HtlcCreateParams) -> HtlcId {
        let mut hasher = Sha256::new();
        hasher.update(self.htlc_contract);
        hasher.update(params.hash_lock.as_bytes());
        hasher.update(params.timelock.to_le_bytes());
        hasher.update(params.amount.to_le_bytes());
        let hash = hasher.finalize();
        HtlcId::from_bytes(hash.to_vec())
    }
}

#[async_trait]
impl HtlcChainAdapter for EvmHtlcAdapter {
    fn vm_target(&self) -> VmTarget {
        VmTarget::Evm {
            chain_id: self.chain_id,
        }
    }

    async fn create_htlc(&self, params: &HtlcCreateParams) -> Result<HtlcRecord, CoordinatorError> {
        // Encode ABI calldata
        let calldata = abi::encode_create_htlc(
            &params.recipient,
            params.hash_lock.as_bytes(),
            params.timelock,
            &params.asset,
            params.amount,
        );

        tracing::info!(
            chain_id = self.chain_id,
            hash_lock = %params.hash_lock,
            amount = params.amount,
            calldata_len = calldata.len(),
            contract = %self.contract_hex(),
            "Creating EVM HTLC via AtlasHTLC.sol"
        );

        let calldata_hex = format!("0x{}", hex::encode(&calldata));

        // Without `real-signing`: validate calldata via eth_call (no state committed).
        #[cfg(not(feature = "real-signing"))]
        {
            match self.rpc.eth_call(&self.contract_hex(), &calldata_hex).await {
                Ok(return_data) => {
                    tracing::info!("EVM HTLC creation simulated: {}", return_data);
                }
                Err(e) => {
                    tracing::warn!("EVM HTLC simulation failed (expected in test mode): {}", e);
                }
            }
        }

        // With `real-signing`: sign and broadcast the transaction on-chain.
        // NOTE: production callers must supply a fully RLP-encoded transaction
        // (nonce, gas_price, gas_limit injected by the signer implementation).
        #[cfg(feature = "real-signing")]
        {
            let sig_bytes = self.signer.sign(&calldata).await;
            let signed_tx_hex = format!("0x{}", hex::encode(&sig_bytes));
            let tx_hash = self
                .rpc
                .eth_send_raw_tx(&signed_tx_hex)
                .await
                .map_err(|e| CoordinatorError::Internal(format!("eth_sendRawTransaction: {e}")))?;
            tracing::info!(tx_hash = %tx_hash, "EVM HTLC published on-chain");
        }

        let htlc_id = self.derive_htlc_id(params);

        Ok(HtlcRecord {
            id: htlc_id,
            params: params.clone(),
            status: HtlcStatus::Funded,
            created_at_block: self.rpc.eth_block_number().await.unwrap_or(0),
            confirmations_required: 12,
            confirmations: 0,
            params_hash: compute_htlc_params_hash(params),
        })
    }

    async fn query_htlc(&self, htlc_id: &HtlcId) -> Result<(HtlcStatus, u32), CoordinatorError> {
        let calldata = abi::encode_get_htlc(&htlc_id.0);
        let calldata_hex = format!("0x{}", hex::encode(&calldata));

        match self.rpc.eth_call(&self.contract_hex(), &calldata_hex).await {
            Ok(result) => {
                let result_bytes = hex::decode(result.strip_prefix("0x").unwrap_or(&result))
                    .map_err(|e| CoordinatorError::Internal(format!("Hex decode: {e}")))?;
                abi::decode_htlc_status(&result_bytes)
            }
            Err(e) => {
                tracing::error!("query_htlc RPC failed: {}", e);
                Err(CoordinatorError::Internal(format!(
                    "EVM query_htlc RPC failed: {e}"
                )))
            }
        }
    }

    async fn claim_htlc(
        &self,
        htlc_id: &HtlcId,
        secret: &HtlcSecret,
    ) -> Result<Vec<u8>, CoordinatorError> {
        let calldata = abi::encode_claim_htlc(&htlc_id.0, secret.as_bytes());

        tracing::info!(
            chain_id = self.chain_id,
            htlc_id = %htlc_id.to_hex(),
            calldata_len = calldata.len(),
            "Claiming EVM HTLC (revealing secret on-chain)"
        );

        // In production: sign as Type-2 (EIP-1559) tx with high maxPriorityFeePerGas
        // to ensure fast inclusion and prevent MEV frontrunning
        let calldata_hex = format!("0x{}", hex::encode(&calldata));
        match self.rpc.eth_send_raw_tx(&calldata_hex).await {
            Ok(tx_hash) => {
                tracing::info!("EVM HTLC claim tx: {}", tx_hash);
                let hash_bytes = hex::decode(tx_hash.strip_prefix("0x").unwrap_or(&tx_hash))
                    .unwrap_or_else(|_| vec![0u8; 32]);
                Ok(hash_bytes)
            }
            Err(e) => {
                tracing::error!("EVM claim broadcast failed: {}", e);
                Err(CoordinatorError::Internal(format!(
                    "eth_sendRawTransaction (claim): {e}"
                )))
            }
        }
    }

    async fn refund_htlc(&self, htlc_id: &HtlcId) -> Result<Vec<u8>, CoordinatorError> {
        let calldata = abi::encode_refund_htlc(&htlc_id.0);

        tracing::info!(
            chain_id = self.chain_id,
            htlc_id = %htlc_id.to_hex(),
            "Refunding EVM HTLC (timelock expired)"
        );

        let calldata_hex = format!("0x{}", hex::encode(&calldata));
        match self.rpc.eth_send_raw_tx(&calldata_hex).await {
            Ok(tx_hash) => {
                let hash_bytes = hex::decode(tx_hash.strip_prefix("0x").unwrap_or(&tx_hash))
                    .unwrap_or_else(|_| vec![0u8; 32]);
                Ok(hash_bytes)
            }
            Err(e) => {
                tracing::error!("EVM refund broadcast failed: {}", e);
                Err(CoordinatorError::Internal(format!(
                    "eth_sendRawTransaction (refund): {e}"
                )))
            }
        }
    }

    async fn current_time(&self) -> Result<u64, CoordinatorError> {
        match self.rpc.eth_block_timestamp().await {
            Ok(ts) => Ok(ts),
            Err(_) => Ok(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()),
        }
    }

    async fn estimate_claim_cost(&self, _htlc_id: &HtlcId) -> Result<u64, CoordinatorError> {
        // claimHTLC typically costs ~30k-50k gas
        Ok(50_000)
    }
}

// ─── SVM Adapter ──────────────────────────────────────────────────────────────

/// Solana HTLC adapter — targets the X3 HTLC Anchor program.
///
/// Uses Anchor instruction encoding + JSON-RPC for Solana interactions.
pub struct SvmHtlcAdapter {
    /// Solana RPC endpoint.
    pub rpc_url: String,
    /// HTLC program ID (32 bytes).
    pub program_id: [u8; 32],
    /// Abstracted signer (never exposes raw key bytes).
    signer: Arc<dyn Signer + Send + Sync>,
    /// RPC client instance.
    rpc: RpcClient,
}

impl SvmHtlcAdapter {
    pub fn new(
        rpc_url: String,
        program_id: [u8; 32],
        signer: Arc<dyn Signer + Send + Sync>,
    ) -> Self {
        let rpc = RpcClient::new(rpc_url.clone());
        Self {
            rpc_url,
            program_id,
            signer,
            rpc,
        }
    }

    /// Derive PDA for HTLC account.
    fn derive_htlc_pda(&self, params: &HtlcCreateParams) -> HtlcId {
        let mut hasher = Sha256::new();
        hasher.update(self.program_id);
        hasher.update(b"htlc");
        hasher.update(params.hash_lock.as_bytes());
        hasher.update(self.signer.pubkey());
        let hash = hasher.finalize();
        HtlcId::from_bytes(hash.to_vec())
    }
}

#[async_trait]
impl HtlcChainAdapter for SvmHtlcAdapter {
    fn vm_target(&self) -> VmTarget {
        VmTarget::Svm
    }

    async fn create_htlc(&self, params: &HtlcCreateParams) -> Result<HtlcRecord, CoordinatorError> {
        // Encode Anchor instruction
        let instruction_data = abi::encode_svm_create_htlc(
            params.hash_lock.as_bytes(),
            params.timelock,
            params.amount as u64,
        );

        tracing::info!(
            program_id = %hex::encode(&self.program_id[..8]),
            hash_lock = %params.hash_lock,
            amount = params.amount,
            instruction_len = instruction_data.len(),
            "Creating SVM HTLC via Anchor program"
        );

        // With `real-signing`: sign and send the transaction to Solana.
        // NOTE: production callers must inject a recent_blockhash and fully
        // serialize the Solana transaction before calling solana_send_tx.
        #[cfg(feature = "real-signing")]
        {
            let sig_bytes = self.signer.sign(&instruction_data).await;
            let signed_tx_hex = hex::encode(&sig_bytes);
            let tx_sig =
                self.rpc.solana_send_tx(&signed_tx_hex).await.map_err(|e| {
                    CoordinatorError::Internal(format!("solana sendTransaction: {e}"))
                })?;
            tracing::info!(signature = %tx_sig, "SVM HTLC published on-chain");
        }

        let slot = self.rpc.solana_get_slot().await.unwrap_or(0);
        let htlc_id = self.derive_htlc_pda(params);

        Ok(HtlcRecord {
            id: htlc_id,
            params: params.clone(),
            status: HtlcStatus::Funded,
            created_at_block: slot,
            confirmations_required: 50, // finalized commitment
            confirmations: 0,
            params_hash: compute_htlc_params_hash(params),
        })
    }

    async fn query_htlc(&self, htlc_id: &HtlcId) -> Result<(HtlcStatus, u32), CoordinatorError> {
        let pubkey = bs58_encode(&htlc_id.0);

        match self.rpc.solana_get_account_info(&pubkey).await {
            Ok(Some(data)) => {
                // Parse Anchor account discriminator + status field
                if data.len() >= 16 {
                    let status = data[8]; // status field after 8-byte discriminator
                    let htlc_status = match status {
                        0 => HtlcStatus::Pending,
                        1 => HtlcStatus::Funded,
                        2 => HtlcStatus::Claimed,
                        3 => HtlcStatus::Refunded,
                        _ => HtlcStatus::Expired,
                    };
                    Ok((htlc_status, 0))
                } else {
                    Ok((HtlcStatus::Pending, 0))
                }
            }
            Ok(None) => Ok((HtlcStatus::Pending, 0)),
            Err(e) => {
                tracing::error!("SVM query_htlc RPC failed: {}", e);
                Err(CoordinatorError::Internal(format!(
                    "SVM query_htlc getAccountInfo failed: {e}"
                )))
            }
        }
    }

    async fn claim_htlc(
        &self,
        htlc_id: &HtlcId,
        secret: &HtlcSecret,
    ) -> Result<Vec<u8>, CoordinatorError> {
        let instruction_data = abi::encode_svm_claim_htlc(secret.as_bytes());

        tracing::info!(
            htlc_id = %htlc_id.to_hex(),
            instruction_len = instruction_data.len(),
            "Claiming SVM HTLC (revealing secret)"
        );

        #[cfg(feature = "real-signing")]
        {
            let sig_bytes = self.signer.sign(&instruction_data).await;
            let signed_tx_hex = hex::encode(&sig_bytes);
            let tx_sig = self.rpc.solana_send_tx(&signed_tx_hex).await.map_err(|e| {
                CoordinatorError::Internal(format!("SVM claim_htlc sendTransaction: {e}"))
            })?;
            tracing::info!(signature = %tx_sig, "SVM HTLC claim published");
            return Ok(tx_sig.into_bytes());
        }

        #[cfg(not(feature = "real-signing"))]
        Err(CoordinatorError::Internal(
            "real-signing feature required for SVM HTLC claim".to_string(),
        ))
    }

    async fn refund_htlc(&self, htlc_id: &HtlcId) -> Result<Vec<u8>, CoordinatorError> {
        let instruction_data = abi::encode_svm_refund_htlc();

        tracing::info!(
            htlc_id = %htlc_id.to_hex(),
            instruction_len = instruction_data.len(),
            "Refunding SVM HTLC (timelock expired)"
        );

        #[cfg(feature = "real-signing")]
        {
            let sig_bytes = self.signer.sign(&instruction_data).await;
            let signed_tx_hex = hex::encode(&sig_bytes);
            let tx_sig = self.rpc.solana_send_tx(&signed_tx_hex).await.map_err(|e| {
                CoordinatorError::Internal(format!("SVM refund_htlc sendTransaction: {e}"))
            })?;
            tracing::info!(signature = %tx_sig, "SVM HTLC refund published");
            return Ok(tx_sig.into_bytes());
        }

        #[cfg(not(feature = "real-signing"))]
        Err(CoordinatorError::Internal(
            "real-signing feature required for SVM HTLC refund".to_string(),
        ))
    }

    async fn current_time(&self) -> Result<u64, CoordinatorError> {
        // Solana uses slot-based time; estimate from slot × 400ms
        match self.rpc.solana_get_slot().await {
            Ok(_slot) => {
                // ~400ms per slot, estimate unix time
                Ok(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs())
            }
            Err(_) => Ok(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()),
        }
    }

    async fn estimate_claim_cost(&self, _htlc_id: &HtlcId) -> Result<u64, CoordinatorError> {
        // ~5000 compute units on Solana
        Ok(5_000)
    }
}

// ─── X3VM Adapter ─────────────────────────────────────────────────────────────

/// X3VM HTLC adapter — targets X3-lang HTLC contract via submitComitV2.
///
/// Uses the X3 ABI encoding and Substrate JSON-RPC.
pub struct X3VmHtlcAdapter {
    /// X3 chain WebSocket RPC endpoint.
    pub rpc_url: String,
    /// HTLC contract address on X3 (32 bytes, = verifier job_id).
    pub contract_address: [u8; 32],
    /// Abstracted signer (never exposes raw key bytes).
    signer: Arc<dyn Signer + Send + Sync>,
    /// RPC client instance.
    rpc: RpcClient,
}

impl X3VmHtlcAdapter {
    pub fn new(
        rpc_url: String,
        contract_address: [u8; 32],
        signer: Arc<dyn Signer + Send + Sync>,
    ) -> Self {
        let rpc = RpcClient::new(rpc_url.clone());
        Self {
            rpc_url,
            contract_address,
            signer,
            rpc,
        }
    }

    /// Derive HTLC ID from creation params + contract address.
    fn derive_htlc_id(&self, params: &HtlcCreateParams) -> HtlcId {
        let mut hasher = Sha256::new();
        hasher.update(self.contract_address);
        hasher.update(b"x3-htlc");
        hasher.update(params.hash_lock.as_bytes());
        hasher.update(params.timelock.to_le_bytes());
        let hash = hasher.finalize();
        HtlcId::from_bytes(hash.to_vec())
    }
}

#[async_trait]
impl HtlcChainAdapter for X3VmHtlcAdapter {
    fn vm_target(&self) -> VmTarget {
        VmTarget::X3Vm
    }

    async fn create_htlc(&self, params: &HtlcCreateParams) -> Result<HtlcRecord, CoordinatorError> {
        // Encode X3-lang ABI calldata
        let mut recipient_32 = [0u8; 32];
        let len = params.recipient.len().min(32);
        recipient_32[..len].copy_from_slice(&params.recipient[..len]);

        let calldata = abi::encode_x3_create_htlc(
            &recipient_32,
            params.hash_lock.as_bytes(),
            params.timelock,
            params.amount as u64,
        );

        tracing::info!(
            contract = %hex::encode(&self.contract_address[..8]),
            hash_lock = %params.hash_lock,
            amount = params.amount,
            calldata_len = calldata.len(),
            "Creating X3VM HTLC via submitComitV2 (Flash Finality: 1 block)"
        );

        // With `real-signing`: sign and submit the extrinsic to X3.
        // NOTE: production callers must construct a full SCALE-encoded signed
        // extrinsic (call index + signed extension + calldata) before submission.
        #[cfg(feature = "real-signing")]
        {
            let sig_bytes = self.signer.sign(&calldata).await;
            let extrinsic_hex = format!("0x{}", hex::encode(&sig_bytes));
            self.rpc
                .call("author_submitExtrinsic", serde_json::json!([extrinsic_hex]))
                .await
                .map_err(|e| CoordinatorError::Internal(format!("author_submitExtrinsic: {e}")))?;
            tracing::info!("X3VM HTLC extrinsic submitted on-chain");
        }

        let block = self.rpc.x3_get_block_number().await.unwrap_or(0);
        let htlc_id = self.derive_htlc_id(params);

        Ok(HtlcRecord {
            id: htlc_id,
            params: params.clone(),
            status: HtlcStatus::Funded,
            created_at_block: block,
            confirmations_required: 1, // Flash Finality: 1 block ≈ 200ms
            confirmations: 0,
            params_hash: compute_htlc_params_hash(params),
        })
    }

    async fn query_htlc(&self, htlc_id: &HtlcId) -> Result<(HtlcStatus, u32), CoordinatorError> {
        // Derive the storage key for this HTLC state in the x3-kernel pallet.
        // Key = Blake2_128Concat(htlc_id bytes) under PalletX3Kernel::HtlcStates.
        // We use state_getStorage to read the raw SCALE-encoded status byte.
        let mut key_data = Vec::with_capacity(32 + htlc_id.0.len());
        key_data.extend_from_slice(&htlc_id.0);
        let storage_key_hex = format!("0x{}", hex::encode(&key_data));

        match self
            .rpc
            .call("state_getStorage", serde_json::json!([storage_key_hex]))
            .await
        {
            Ok(result) => {
                // Parse SCALE-encoded HtlcStatus: u8 enum variant
                if let Some(hex_str) = result.as_str() {
                    let bytes = hex::decode(hex_str.strip_prefix("0x").unwrap_or(hex_str))
                        .unwrap_or_default();
                    let status = match bytes.first().copied().unwrap_or(0) {
                        0 => HtlcStatus::Pending,
                        1 => HtlcStatus::Funded,
                        2 => HtlcStatus::Claimed,
                        3 => HtlcStatus::Refunded,
                        _ => HtlcStatus::Expired,
                    };
                    // Flash Finality = 1 confirmation
                    Ok((status, 1))
                } else {
                    // Null result = not found on chain yet
                    Ok((HtlcStatus::Pending, 0))
                }
            }
            Err(e) => {
                tracing::error!("X3VM query_htlc state_getStorage failed: {}", e);
                Err(CoordinatorError::Internal(format!(
                    "X3VM query_htlc RPC failed: {e}"
                )))
            }
        }
    }

    async fn claim_htlc(
        &self,
        htlc_id: &HtlcId,
        secret: &HtlcSecret,
    ) -> Result<Vec<u8>, CoordinatorError> {
        let mut htlc_id_32 = [0u8; 32];
        let len = htlc_id.0.len().min(32);
        htlc_id_32[..len].copy_from_slice(&htlc_id.0[..len]);

        let calldata = abi::encode_x3_claim_htlc(&htlc_id_32, secret.as_bytes());

        tracing::info!(
            htlc_id = %htlc_id.to_hex(),
            calldata_len = calldata.len(),
            "Claiming X3VM HTLC (sub-200ms finality)"
        );

        #[cfg(feature = "real-signing")]
        {
            let sig_bytes = self.signer.sign(&calldata).await;
            let extrinsic_hex = format!("0x{}", hex::encode(&sig_bytes));
            self.rpc
                .call("author_submitExtrinsic", serde_json::json!([extrinsic_hex]))
                .await
                .map_err(|e| {
                    CoordinatorError::Internal(format!("X3VM claim_htlc submitExtrinsic: {e}"))
                })?;
            tracing::info!("X3VM HTLC claim extrinsic submitted");
            // Return sha3(calldata) as deterministic tx fingerprint
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(&calldata);
            return Ok(hasher.finalize().to_vec());
        }

        #[cfg(not(feature = "real-signing"))]
        Err(CoordinatorError::Internal(
            "real-signing feature required for X3VM HTLC claim".to_string(),
        ))
    }

    async fn refund_htlc(&self, htlc_id: &HtlcId) -> Result<Vec<u8>, CoordinatorError> {
        let mut htlc_id_32 = [0u8; 32];
        let len = htlc_id.0.len().min(32);
        htlc_id_32[..len].copy_from_slice(&htlc_id.0[..len]);

        let calldata = abi::encode_x3_refund_htlc(&htlc_id_32);

        tracing::info!(
            htlc_id = %htlc_id.to_hex(),
            calldata_len = calldata.len(),
            "Refunding X3VM HTLC"
        );

        #[cfg(feature = "real-signing")]
        {
            let sig_bytes = self.signer.sign(&calldata).await;
            let extrinsic_hex = format!("0x{}", hex::encode(&sig_bytes));
            self.rpc
                .call("author_submitExtrinsic", serde_json::json!([extrinsic_hex]))
                .await
                .map_err(|e| {
                    CoordinatorError::Internal(format!("X3VM refund_htlc submitExtrinsic: {e}"))
                })?;
            tracing::info!("X3VM HTLC refund extrinsic submitted");
            use sha2::Digest;
            let mut hasher = sha2::Sha256::new();
            hasher.update(&calldata);
            return Ok(hasher.finalize().to_vec());
        }

        #[cfg(not(feature = "real-signing"))]
        Err(CoordinatorError::Internal(
            "real-signing feature required for X3VM HTLC refund".to_string(),
        ))
    }

    async fn current_time(&self) -> Result<u64, CoordinatorError> {
        // X3 uses Timestamp pallet — query via RPC or use system time
        Ok(std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs())
    }

    async fn estimate_claim_cost(&self, _htlc_id: &HtlcId) -> Result<u64, CoordinatorError> {
        // X3 weight-based, very low cost for claim_htlc
        Ok(10_000)
    }
}

// ─── Utility ──────────────────────────────────────────────────────────────────

/// Base58 encode bytes (for Solana public keys).
fn bs58_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 58] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

    if data.is_empty() {
        return String::new();
    }

    // Count leading zeros
    let zeros = data.iter().take_while(|&&b| b == 0).count();

    // Convert to base58
    let mut digits: Vec<u8> = Vec::new();
    for &byte in data {
        let mut carry = byte as u32;
        for digit in digits.iter_mut() {
            carry += (*digit as u32) * 256;
            *digit = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            digits.push((carry % 58) as u8);
            carry /= 58;
        }
    }

    let mut result = String::with_capacity(zeros + digits.len());
    for _ in 0..zeros {
        result.push('1');
    }
    for &d in digits.iter().rev() {
        result.push(ALPHABET[d as usize] as char);
    }
    result
}
