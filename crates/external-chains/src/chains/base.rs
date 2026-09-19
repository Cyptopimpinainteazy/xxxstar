//! Base Chain Adapter
//!
//! Adapter for Base (Coinbase L2) - an OP Stack rollup
//! Chain ID: 8453

use crate::adapter::*;
use crate::error::ExternalChainError;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Base chain adapter
pub struct BaseAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
}

impl BaseAdapter {
    /// Create new Base adapter
    pub fn new(config: ChainConfig) -> Self {
        Self { config, nonce: 0 }
    }

    /// Get chain-specific bridge ABI
    pub fn bridge_abi() -> &'static [u8] {
        // L2StandardBridge ABI for OP Stack
        include_bytes!("../../abi/l2_standard_bridge.json")
    }

    /// Encode bridge deposit call
    pub fn encode_deposit(to: H160, amount: U256, gas_limit: u64, data: Vec<u8>) -> Vec<u8> {
        // depositETH(address _to, uint32 _minGasLimit, bytes _extraData)
        let mut calldata = Vec::with_capacity(4 + 32 + 32 + 32 + data.len());

        // Function selector: depositETH(address,uint32,bytes)
        calldata.extend_from_slice(&[0xb1, 0xa1, 0xa8, 0x82]);

        // Pad address to 32 bytes
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(to.as_bytes());

        // Gas limit
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&(gas_limit as u32).to_be_bytes());

        // Data offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x60);

        // Data length
        let data_len = data.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&data_len.to_be_bytes());

        // Data (padded to 32 bytes)
        calldata.extend_from_slice(&data);
        let padding = (32 - data.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);

        calldata
    }

    /// Encode L2CrossDomainMessenger sendMessage call
    fn encode_send_message(target: H160, message: &[u8], gas_limit: u64) -> Vec<u8> {
        let mut calldata = Vec::with_capacity(4 + 32 + 32 + 32 + 32 + message.len());
        // sendMessage(address,bytes,uint32) selector: 0x3dbb202b
        calldata.extend_from_slice(&[0x3d, 0xbb, 0x20, 0x2b]);
        // target address
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(target.as_bytes());
        // data offset (96 = 0x60)
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x60);
        // gas limit
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&(gas_limit as u32).to_be_bytes());
        // data length
        let len = message.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&len.to_be_bytes());
        // data
        calldata.extend_from_slice(message);
        let padding = (32 - message.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);
        calldata
    }
}

#[async_trait::async_trait]
impl ChainAdapter for BaseAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Base
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        // A real `eth_chainId` probe: "connected" means the endpoint answered
        // *as Base*, not merely that it answered. Before this returned `true`
        // unconditionally, so every dead endpoint looked healthy.
        matches!(
            crate::evm_rpc::chain_id(&crate::evm_rpc::url(&self.config)).await,
            Ok(8453)
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
        // Refused, not simulated. This used to build the sendMessage calldata,
        // run it through `eth_call` (which changes no state) and return
        // `message.hash()` as though a transaction had been broadcast — a
        // caller could not tell that from a real send. Broadcasting one needs a
        // signed transaction, and this adapter holds no signer.
        Err(ExternalChainError::adapter_unimplemented(
            "base: send_message needs a signed L2CrossDomainMessenger transaction and this \
             adapter has no signer; refusing rather than returning a hash for an unsent message",
        ))
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Refused, not answered with an empty list. This used to run
        // `eth_getLogs`, throw the logs away and return `Ok(vec![])` — which
        // reads as "the chain has no pending messages" no matter what it said.
        // Decoding a `SentMessage` event into a `ChainMessage` is not
        // implemented (see docs/reports/SECURITY_BLOCKERS.md finding 6).
        Err(ExternalChainError::adapter_unimplemented(
            "base: receive_messages cannot decode SentMessage logs yet; refusing rather than \
             reporting an empty message queue",
        ))
    }

    async fn initiate_transfer(&self, _transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Refused, not simulated. This built the L2StandardBridge deposit
        // calldata, ran it through `eth_call` — a dry run that cannot move a
        // token — and returned `transfer.id` as though the deposit had been
        // broadcast. A caller would then wait for a transfer that does not
        // exist.
        Err(ExternalChainError::adapter_unimplemented(
            "base: initiate_transfer needs a signed L2StandardBridge deposit and this adapter \
             has no signer; refusing rather than returning an id for an uninitiated transfer",
        ))
    }

    async fn check_transfer_status(&self, transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Refused, not inferred. This used to read the *source* transaction
        // receipt and report `Completed` whenever that transaction succeeded —
        // but a successful deposit on the source chain says nothing about
        // whether the message was relayed on the destination. A relayer acting
        // on that answer would release funds for a message that never arrived.
        let _ = transfer_id;
        Err(ExternalChainError::adapter_unimplemented(
            "base: check_transfer_status needs the destination relay state; a source-chain \
             receipt is not evidence that the message arrived",
        ))
    }

    async fn verify_message_proof(
        &self,
        _message: &ChainMessage,
        _proof: &[u8],
    ) -> AdapterResult<bool> {
        // Refused, not shape-checked. The previous body accepted any proof of at
        // least 64 bytes containing one non-zero byte — it never touched the L1
        // state root it claims to verify against (the comment said so: "In
        // production: verify Merkle-Patricia trie proof against L1 state root").
        // Verifying an OP-Stack message means walking the output-root proof to
        // the L1 `L2OutputOracle`, which is not implemented here.
        Err(ExternalChainError::VerificationUnavailable)
    }

    async fn finalize_transfer(&self, _transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Refused, not simulated: `eth_call` to `relayMessage` executes nothing.
        // It used to return the transfer id as though the message had been
        // relayed on the destination.
        Err(ExternalChainError::adapter_unimplemented(
            "base: finalize_transfer needs a signed relayMessage transaction and a proof this \
             adapter cannot verify; refusing rather than returning an id for an unfinalized \
             transfer",
        ))
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        crate::evm_rpc::gas_price(&crate::evm_rpc::url(&self.config)).await
    }

    async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> AdapterResult<Option<TransactionReceipt>> {
        // Every field comes from the response or the call fails. The previous
        // body defaulted a missing block number to 0, a missing gas figure to
        // 21_000 and a bad block hash to zero, and it panicked through
        // `H256::from_slice` on a short hash.
        crate::evm_rpc::receipt(&crate::evm_rpc::url(&self.config), tx_hash).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_adapter() {
        let adapter = BaseAdapter::new(ChainConfig::for_chain(ChainType::Base));
        assert_eq!(adapter.chain_type(), ChainType::Base);
        assert_eq!(adapter.config().chain_type, 8453);
    }

    #[test]
    fn test_encode_deposit() {
        let calldata = BaseAdapter::encode_deposit(
            H160::zero(),
            U256::from(1_000_000_000_000_000_000u64),
            200_000,
            vec![],
        );
        // Check function selector
        assert_eq!(&calldata[0..4], &[0xb1, 0xa1, 0xa8, 0x82]);
    }

    #[test]
    fn test_encode_balance_of() {
        let calldata = crate::evm_rpc::encode_balance_of(H160::zero());
        assert_eq!(&calldata[0..4], &[0x70, 0xa0, 0x82, 0x31]);
        assert_eq!(calldata.len(), 36);
    }

    #[test]
    fn test_parse_hex_u64() {
        assert_eq!(crate::evm_rpc::parse_hex_u64("0x1").unwrap(), 1);
        assert_eq!(crate::evm_rpc::parse_hex_u64("0xff").unwrap(), 255);
        assert_eq!(crate::evm_rpc::parse_hex_u64("0x2105").unwrap(), 8453);
    }

    #[test]
    fn test_parse_hex_u256() {
        let val = crate::evm_rpc::parse_hex_u256("0x1").unwrap();
        assert_eq!(val, U256::from(1));
    }

    #[test]
    fn test_encode_send_message() {
        let calldata = BaseAdapter::encode_send_message(H160::zero(), &[1, 2, 3], 200_000);
        // Check function selector: sendMessage
        assert_eq!(&calldata[0..4], &[0x3d, 0xbb, 0x20, 0x2b]);
    }

    #[test]
    fn test_build_rpc_request() {
        let req = crate::evm_rpc::request("eth_blockNumber", "[]");
        let text = String::from_utf8(req).unwrap();
        assert!(text.contains("eth_blockNumber"));
        assert!(text.contains("jsonrpc"));
    }

    /// The operations this adapter has no signer or proof verifier for must
    /// refuse. Each of these used to return a plausible-looking `Ok`: a hash
    /// for a message that was never broadcast, an id for a transfer that was
    /// never initiated, `Completed` for a message that was never relayed.
    #[tokio::test]
    async fn operations_this_adapter_cannot_perform_are_refused() {
        let adapter = BaseAdapter::new(ChainConfig::for_chain(ChainType::Base));
        let message = ChainMessage::new(8453, 1, H160::zero(), H160::zero(), vec![1], U256::zero());
        let transfer = CrossChainTransfer {
            id: H256::zero(),
            source_chain: 8453,
            dest_chain: 1,
            source_token: H160::zero(),
            dest_token: H160::zero(),
            sender: H160::zero(),
            recipient: H160::zero(),
            amount: U256::from(1u64),
            fee: U256::zero(),
            status: TransferStatus::Pending,
            source_tx: None,
            dest_tx: None,
        };

        assert!(matches!(
            adapter.send_message(message).await,
            Err(ExternalChainError::AdapterUnimplemented(_))
        ));
        assert!(matches!(
            adapter.initiate_transfer(transfer).await,
            Err(ExternalChainError::AdapterUnimplemented(_))
        ));
        assert!(matches!(
            adapter.receive_messages().await,
            Err(ExternalChainError::AdapterUnimplemented(_))
        ));
        assert!(matches!(
            adapter.check_transfer_status(H256::zero()).await,
            Err(ExternalChainError::AdapterUnimplemented(_))
        ));
        assert!(matches!(
            adapter.finalize_transfer(H256::zero(), vec![1, 2, 3]).await,
            Err(ExternalChainError::AdapterUnimplemented(_))
        ));
    }
}
