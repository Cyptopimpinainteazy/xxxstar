//! Universal EVM Adapter
//!
//! One adapter to rule them all - works with ANY EVM chain from the registry.
//! Just give it a chain_id and GO!

use crate::adapter::*;
use crate::chains::registry::{get_chain, ChainInfo};
use crate::error::ExternalChainError;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Minimal onboarding payload for external EVM-compatible chains.
#[derive(Debug, Clone)]
pub struct ExternalEvmOnboarding {
    pub chain_id: u64,
    pub rpc_url: Vec<u8>,
    pub bridge_contract: H160,
    pub settlement_contract: H160,
    pub confirmations: u32,
}

/// Universal adapter that works with ANY EVM chain
pub struct UniversalEvmAdapter {
    chain_id: u64,
    config: ChainConfig,
    info: &'static ChainInfo,
}

impl UniversalEvmAdapter {
    /// Create adapter for any chain by ID
    pub fn new(chain_id: u64) -> Option<Self> {
        let info = get_chain(chain_id)?;
        let config = ChainConfig {
            chain_type: chain_id,
            rpc_url: info.rpc.as_bytes().to_vec(),
            ws_url: None,
            bridge_contract: Self::default_bridge(chain_id),
            settlement_contract: Self::default_bridge(chain_id),
            confirmations: info.confirmations,
            gas_price_multiplier: 100,
            max_gas_limit: 500_000,
        };
        Some(Self {
            chain_id,
            config,
            info,
        })
    }

    /// Create from ChainInfo directly
    pub fn from_info(info: &'static ChainInfo) -> Self {
        let config = ChainConfig {
            chain_type: info.chain_id,
            rpc_url: info.rpc.as_bytes().to_vec(),
            ws_url: None,
            bridge_contract: Self::default_bridge(info.chain_id),
            settlement_contract: Self::default_bridge(info.chain_id),
            confirmations: info.confirmations,
            gas_price_multiplier: 100,
            max_gas_limit: 500_000,
        };
        Self {
            chain_id: info.chain_id,
            config,
            info,
        }
    }

    /// Create adapter for an external EVM chain that is not present in registry.
    ///
    /// This is the fast onboarding path for partner chains:
    /// provide chain ID + RPC + contracts + confirmations and receive
    /// a validated, ready-to-use universal adapter.
    pub fn onboard_external_chain(input: ExternalEvmOnboarding) -> AdapterResult<Self> {
        if input.chain_id == 0 {
            return Err(ExternalChainError::InvalidChainId(input.chain_id));
        }

        if input.rpc_url.is_empty() {
            return Err(ExternalChainError::parse_error("rpc_url cannot be empty"));
        }

        if input.bridge_contract == H160::zero() || input.settlement_contract == H160::zero() {
            return Err(ExternalChainError::InvalidAddress);
        }

        if input.confirmations == 0 {
            return Err(ExternalChainError::parse_error(
                "confirmations must be greater than zero",
            ));
        }

        let rpc_text = String::from_utf8_lossy(&input.rpc_url);
        if !rpc_text.starts_with("http://") && !rpc_text.starts_with("https://") {
            return Err(ExternalChainError::parse_error(
                "rpc_url must start with http:// or https://",
            ));
        }

        let info = Box::leak(Box::new(ChainInfo {
            chain_id: input.chain_id,
            name: "External EVM",
            symbol: "EXT",
            rpc: "",
            explorer: "",
            is_l2: false,
            block_time_ms: 2_000,
            confirmations: input.confirmations,
        }));

        let config = ChainConfig {
            chain_type: input.chain_id,
            rpc_url: input.rpc_url,
            ws_url: None,
            bridge_contract: input.bridge_contract,
            settlement_contract: input.settlement_contract,
            confirmations: input.confirmations,
            gas_price_multiplier: 100,
            max_gas_limit: 500_000,
        };

        config.validate()?;

        Ok(Self {
            chain_id: input.chain_id,
            config,
            info,
        })
    }

    fn default_bridge(chain_id: u64) -> H160 {
        // Generate deterministic bridge address from chain_id
        let mut bytes = [0u8; 20];
        bytes[0..8].copy_from_slice(&chain_id.to_be_bytes());
        bytes[8..16].copy_from_slice(&chain_id.to_le_bytes());
        bytes[16..20].copy_from_slice(&[0xB0, 0x1D, 0x6E, 0x00]); // "BRIDGE"
        H160::from(bytes)
    }

    /// Get chain info
    pub fn info(&self) -> &'static ChainInfo {
        self.info
    }

    /// Get chain name
    pub fn name(&self) -> &'static str {
        self.info.name
    }

    /// Get native token symbol
    pub fn symbol(&self) -> &'static str {
        self.info.symbol
    }

    /// Is this an L2?
    pub fn is_l2(&self) -> bool {
        self.info.is_l2
    }

    /// Standard ERC20 transfer encoding
    pub fn encode_erc20_transfer(to: H160, amount: U256) -> Vec<u8> {
        let mut calldata = Vec::with_capacity(68);
        // transfer(address,uint256) selector
        calldata.extend_from_slice(&[0xa9, 0x05, 0x9c, 0xbb]);
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(to.as_bytes());
        let amount_bytes = amount.to_big_endian();
        calldata.extend_from_slice(&amount_bytes);
        calldata
    }

    /// Standard ERC20 approve encoding
    pub fn encode_erc20_approve(spender: H160, amount: U256) -> Vec<u8> {
        let mut calldata = Vec::with_capacity(68);
        // approve(address,uint256) selector
        calldata.extend_from_slice(&[0x09, 0x5e, 0xa7, 0xb3]);
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(spender.as_bytes());
        let amount_bytes = amount.to_big_endian();
        calldata.extend_from_slice(&amount_bytes);
        calldata
    }

    /// Standard native transfer
    pub fn encode_native_transfer() -> Vec<u8> {
        // Empty calldata for native transfer
        vec![]
    }
}

#[async_trait::async_trait]
impl ChainAdapter for UniversalEvmAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::from(self.chain_id)
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        // Real `eth_chainId` probe: the endpoint must answer *as this chain*.
        matches!(
            crate::evm_rpc::chain_id(&crate::evm_rpc::url(&self.config)).await,
            Ok(id) if id == self.info.chain_id
        )
    }

    async fn get_block_number(&self) -> AdapterResult<u64> {
        crate::evm_rpc::block_number(&crate::evm_rpc::url(&self.config)).await
    }

    async fn get_balance(&self, address: H160) -> AdapterResult<U256> {
        crate::evm_rpc::balance(&crate::evm_rpc::url(&self.config), address).await
    }

    async fn get_token_balance(&self, token: H160, address: H160) -> AdapterResult<U256> {
        crate::evm_rpc::token_balance(&crate::evm_rpc::url(&self.config), token, address).await
    }

    async fn send_message(&self, _message: ChainMessage) -> AdapterResult<H256> {
        // Refused, not invented: a universal adapter cannot know this chain's
        // messaging contract, so it cannot send anything. It used to return
        // `message.hash()` for a message that was never broadcast anywhere.
        Err(ExternalChainError::adapter_unimplemented(
            "universal: this chain's messaging contract is not configured, so no message can be \
             sent; refusing rather than returning a hash for an unsent message",
        ))
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Refused: an empty list reads as "no pending messages".
        Err(ExternalChainError::adapter_unimplemented(
            "universal: no message decoder is configured for this chain; refusing rather than \
             reporting an empty message queue",
        ))
    }

    async fn initiate_transfer(&self, _transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Refused: this returned `transfer.id` for a transfer never sent.
        Err(ExternalChainError::adapter_unimplemented(
            "universal: no bridge contract is configured for this chain, so no transfer can be \
             initiated; refusing rather than returning an id for an uninitiated transfer",
        ))
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Refused: this answered `Completed` for every transfer id, including
        // ids that do not exist.
        Err(ExternalChainError::adapter_unimplemented(
            "universal: nothing here can tell a relayed transfer from an unrelayed one; refusing \
             rather than reporting every transfer as complete",
        ))
    }

    async fn verify_message_proof(
        &self,
        _message: &ChainMessage,
        _proof: &[u8],
    ) -> AdapterResult<bool> {
        // Refused, not shape-checked: `!proof.is_empty()` accepts any byte
        // string, and no per-chain verifier exists here to do better.
        Err(ExternalChainError::VerificationUnavailable)
    }

    async fn finalize_transfer(&self, _transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Refused: this returned the transfer id as though finalization had run.
        Err(ExternalChainError::adapter_unimplemented(
            "universal: no finalization path is configured for this chain; refusing rather than \
             returning an id for an unfinalized transfer",
        ))
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        crate::evm_rpc::gas_price(&crate::evm_rpc::url(&self.config)).await
    }

    async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> AdapterResult<Option<TransactionReceipt>> {
        // Refused: this reported `success: true` at block 20_000_000 for every
        // transaction hash, including hashes that were never mined.
        crate::evm_rpc::receipt(&crate::evm_rpc::url(&self.config), tx_hash).await
    }
}

/// Create universal adapters for all chains in registry
pub fn create_all_universal_adapters() -> Vec<UniversalEvmAdapter> {
    crate::chains::registry::ALL_CHAINS
        .iter()
        .map(|info| UniversalEvmAdapter::from_info(info))
        .collect()
}

/// Quick adapter factory
pub fn adapter_for(chain_id: u64) -> Option<UniversalEvmAdapter> {
    UniversalEvmAdapter::new(chain_id)
}

/// Onboard and create a universal adapter for a non-registry EVM chain.
pub fn onboard_external_adapter(
    chain_id: u64,
    rpc_url: &str,
    bridge_contract: H160,
    settlement_contract: H160,
    confirmations: u32,
) -> AdapterResult<UniversalEvmAdapter> {
    UniversalEvmAdapter::onboard_external_chain(ExternalEvmOnboarding {
        chain_id,
        rpc_url: rpc_url.as_bytes().to_vec(),
        bridge_contract,
        settlement_contract,
        confirmations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_any_chain() {
        // Ethereum
        let eth = adapter_for(1).unwrap();
        assert_eq!(eth.name(), "Ethereum");
        assert_eq!(eth.symbol(), "ETH");
        assert!(!eth.is_l2());

        // zkSync Era
        let zksync = adapter_for(324).unwrap();
        assert_eq!(zksync.name(), "zkSync Era");
        assert!(zksync.is_l2());

        // Fantom
        let ftm = adapter_for(250).unwrap();
        assert_eq!(ftm.name(), "Fantom Opera");
        assert_eq!(ftm.symbol(), "FTM");
    }

    #[test]
    fn test_all_adapters() {
        let all = create_all_universal_adapters();
        assert!(all.len() > 100);
        println!("Created {} universal adapters", all.len());
    }

    #[test]
    fn test_encode_transfer() {
        let calldata = UniversalEvmAdapter::encode_erc20_transfer(H160::zero(), U256::from(1000));
        assert_eq!(&calldata[0..4], &[0xa9, 0x05, 0x9c, 0xbb]);
    }

    /// An endpoint that answers nothing must produce no answers.
    ///
    /// This replaces a test that asserted `is_connected().await == true` and
    /// `get_balance(zero) > 0` for a public RPC endpoint. It passed without a
    /// network because the adapter invented both: `is_connected` returned a
    /// constant `true` and `get_balance` a constant 1 ETH for every address.
    /// Pointing the adapter at a port nothing listens on is the honest version
    /// of the same check — a chain adapter must not be able to produce a
    /// balance, a block number or a connectivity claim out of nothing.
    #[tokio::test]
    async fn an_endpoint_that_answers_nothing_produces_no_answers() {
        let adapter = onboard_external_adapter(
            137,
            "http://127.0.0.1:1",
            H160::from_low_u64_be(0xBEEF),
            H160::from_low_u64_be(0xCAFE),
            1,
        )
        .unwrap();

        assert!(
            !adapter.is_connected().await,
            "an endpoint that is not answering is not connected"
        );
        assert!(
            adapter.get_balance(H160::zero()).await.is_err(),
            "a balance must come from the chain, not from a constant"
        );
        assert!(
            adapter.get_block_number().await.is_err(),
            "a block number must come from the chain"
        );
    }

    #[test]
    fn test_onboard_external_adapter_success() {
        let adapter = onboard_external_adapter(
            777_777,
            "https://rpc.partner-chain.io",
            H160::from_low_u64_be(0xBEEF),
            H160::from_low_u64_be(0xCAFE),
            3,
        )
        .unwrap();

        assert_eq!(adapter.chain_type(), ChainType::AtlasSphere);
        assert_eq!(adapter.config().chain_type, 777_777);
        assert_eq!(adapter.config().confirmations, 3);
    }

    #[test]
    fn test_onboard_external_adapter_rejects_bad_inputs() {
        let bad_rpc = onboard_external_adapter(
            777_777,
            "ws://not-allowed",
            H160::from_low_u64_be(1),
            H160::from_low_u64_be(2),
            1,
        );
        assert!(bad_rpc.is_err());

        let bad_addr = onboard_external_adapter(
            777_777,
            "https://rpc.partner-chain.io",
            H160::zero(),
            H160::from_low_u64_be(2),
            1,
        );
        assert!(bad_addr.is_err());
    }
}
