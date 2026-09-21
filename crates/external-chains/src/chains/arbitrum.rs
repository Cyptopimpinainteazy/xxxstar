//! Arbitrum Chain Adapter
//!
//! Adapter for Arbitrum One - Optimistic rollup with Nitro stack
//! Chain ID: 42161

use crate::adapter::*;
use crate::evm_rpc::LogEntry;
use crate::ChainType;
use crate::ExternalChainError;
use alloc::{
    format,
    string::{String, ToString},
};
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// `ArbSys`, the Arbitrum system predeploy that emits `L2ToL1Tx`.
///
/// A constant, not `config.bridge_contract`: ArbSys is part of the chain, not a
/// contract an operator deploys, and filtering on a configurable address is how a
/// log query ends up answering "no messages" for a chain that has them.
pub const ARBSYS_ADDRESS: H160 = H160(hex_literal::hex!(
    "0000000000000000000000000000000000000064"
));

/// Sourced lookback for one `receive_messages` call, in blocks — the same bound
/// and the same reason as the Base adapter: `eth_getLogs` over an open range is
/// refused by most public nodes.
pub const MESSAGE_LOOKBACK_BLOCKS: u64 = 5_000;

/// Refuse a batch larger than this rather than returning a truncated list.
pub const MAX_MESSAGES_PER_CALL: usize = 256;

/// The canonical Arbitrum L2→L1 event:
///
/// ```solidity
/// event L2ToL1Tx(address caller, address indexed destination,
///                uint256 indexed hash, uint256 indexed position,
///                uint256 arbBlockNum, uint256 ethBlockNum, uint256 timestamp,
///                uint256 callvalue, bytes data);
/// ```
///
/// `destination`, `hash` and `position` are indexed, so they live in `topics`;
/// the rest is ABI-encoded in `data`.
pub fn l2_to_l1_tx_topic() -> [u8; 32] {
    sp_io::hashing::keccak_256(
        b"L2ToL1Tx(address,address,uint256,uint256,uint256,uint256,uint256,uint256,bytes)",
    )
}

/// Arbitrum is an L2: an `L2ToL1Tx` is bound for Ethereum.
pub const L1_CHAIN_ID: u64 = 1;

/// Headroom over `eth_estimateGas`, in percent, before a send is signed — the same
/// rule and the same reason as the Base adapter's.
pub const GAS_ESTIMATE_MARGIN_PERCENT: u64 = 25;

/// Selector for `ArbSys.sendTxToL1(address,bytes)`.
///
/// Computed rather than pasted: the sibling call in this adapter
/// (`arbBlockNumber()`) already derives its selector with
/// `sp_io::hashing::keccak_256`, and a hardcoded four-byte literal is a value
/// nobody can check by reading it.
pub fn send_tx_to_l1_selector() -> [u8; 4] {
    let digest = sp_io::hashing::keccak_256(b"sendTxToL1(address,bytes)");
    [digest[0], digest[1], digest[2], digest[3]]
}

/// Arbitrum chain adapter
pub struct ArbitrumAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
    /// Present only when the caller supplied a key; `send_message` refuses without
    /// one rather than returning a hash for a transaction nobody signed.
    #[cfg(feature = "std")]
    signer: Option<crate::signer::EvmSigner>,
}

impl ArbitrumAdapter {
    /// Create new Arbitrum adapter
    pub fn new(config: ChainConfig) -> Self {
        Self {
            config,
            nonce: 0,
            #[cfg(feature = "std")]
            signer: None,
        }
    }

    /// An adapter that can send, with the key its transactions are signed by.
    #[cfg(feature = "std")]
    pub fn with_signer(config: ChainConfig, signer: crate::signer::EvmSigner) -> Self {
        Self {
            config,
            nonce: 0,
            signer: Some(signer),
        }
    }

    /// `ArbSys.sendTxToL1(address,bytes)` calldata.
    ///
    /// Two head words (destination, offset to the bytes) and the bytes tail —
    /// the ABI shape for this signature, which is not the shape of the Inbox's
    /// opposite-direction `sendL2Message`.
    pub fn encode_send_tx_to_l1(destination: H160, data: &[u8]) -> Vec<u8> {
        let mut encoded = Vec::with_capacity(4 + 64 + 32 + data.len() + 31);
        encoded.extend_from_slice(&send_tx_to_l1_selector());
        // destination, padded to a word
        encoded.extend_from_slice(&[0u8; 12]);
        encoded.extend_from_slice(destination.as_bytes());
        // offset to `bytes data` = 64 (0x40), after the two head words
        encoded.extend_from_slice(&[0u8; 31]);
        encoded.push(0x40);
        // length, then the data padded to a word
        let len = data.len() as u64;
        encoded.extend_from_slice(&[0u8; 24]);
        encoded.extend_from_slice(&len.to_be_bytes());
        encoded.extend_from_slice(data);
        encoded.extend_from_slice(&vec![0u8; (32 - data.len() % 32) % 32]);
        encoded
    }

    /// Decode one `L2ToL1Tx` log into a `ChainMessage`.
    ///
    /// Every field comes from the log. The two that do not have an obvious
    /// counterpart are named rather than guessed: `nonce` is the outbox
    /// `position` (the unique identifier this event carries), and `gas_limit` is
    /// zero because `L2ToL1Tx` states no gas limit — a caller that needs one must
    /// not read this zero as a limit the chain chose.
    pub fn decode_l2_to_l1_tx(log: &LogEntry) -> AdapterResult<ChainMessage> {
        let expected_topic = l2_to_l1_tx_topic();
        if log.topics.len() != 4 {
            return Err(ExternalChainError::rpc_error(&format!(
                "L2ToL1Tx log has {} topics, expected 4 (signature + destination, hash, position)",
                log.topics.len()
            )));
        }
        if log.topics[0].as_bytes() != expected_topic {
            return Err(ExternalChainError::rpc_error(
                "log is not an L2ToL1Tx event: topic 0 does not match the ArbSys signature",
            ));
        }

        let mut recipient = [0u8; 20];
        recipient.copy_from_slice(&log.topics[1].as_bytes()[12..]);
        let position = U256::from_big_endian(log.topics[3].as_bytes());

        // data = abi.encode(caller, arbBlockNum, ethBlockNum, timestamp, callvalue, data)
        const HEAD_WORDS: usize = 6;
        let head_bytes = HEAD_WORDS * 32;
        if log.data.len() < head_bytes {
            return Err(ExternalChainError::rpc_error(&format!(
                "L2ToL1Tx data is {} bytes, shorter than the {head_bytes}-byte head",
                log.data.len()
            )));
        }
        let word = |index: usize| -> &[u8] { &log.data[index * 32..(index + 1) * 32] };

        let mut sender = [0u8; 20];
        sender.copy_from_slice(&word(0)[12..]);
        let timestamp = u64::from_be_bytes(word(3)[24..].try_into().unwrap_or([0u8; 8]));
        let callvalue = U256::from_big_endian(word(4));

        let offset = U256::from_big_endian(word(5));
        if offset != U256::from(head_bytes as u64) {
            return Err(ExternalChainError::rpc_error(&format!(
                "L2ToL1Tx data offset is {offset}, expected {head_bytes}"
            )));
        }
        if log.data.len() < head_bytes + 32 {
            return Err(ExternalChainError::rpc_error(
                "L2ToL1Tx data has no length word",
            ));
        }
        let length = U256::from_big_endian(&log.data[head_bytes..head_bytes + 32]);
        let length: usize = length
            .try_into()
            .map_err(|_| ExternalChainError::rpc_error("L2ToL1Tx data length overflows a usize"))?;
        let start = head_bytes + 32;
        let padded = length
            .checked_add(31)
            .map(|v| v / 32 * 32)
            .ok_or_else(|| ExternalChainError::rpc_error("L2ToL1Tx data length overflows"))?;
        if log.data.len() < start + padded {
            return Err(ExternalChainError::rpc_error(&format!(
                "L2ToL1Tx declares {length} bytes of data but carries {}",
                log.data.len().saturating_sub(start)
            )));
        }
        let payload = log.data[start..start + length].to_vec();

        if position > U256::from(u64::MAX) {
            return Err(ExternalChainError::rpc_error(
                "L2ToL1Tx position does not fit in the message nonce",
            ));
        }

        Ok(ChainMessage {
            source_chain: 42_161,
            dest_chain: L1_CHAIN_ID,
            sender: H160(sender),
            recipient: H160(recipient),
            nonce: position.as_u64(),
            payload,
            value: callvalue,
            gas_limit: 0,
            timestamp,
        })
    }

    /// Arbitrum Inbox contract for L1->L2 messages
    pub const INBOX_ADDRESS: H160 = H160(hex_literal::hex!(
        "4Dbd4fc535Ac27206064B68FfCf827b0A60BAB3f"
    ));

    /// Arbitrum Gateway Router for token bridging
    pub const GATEWAY_ROUTER: H160 = H160(hex_literal::hex!(
        "72Ce9c846789fdB6fC1f34aC4AD25Dd9ef7031ef"
    ));

    /// ArbSys precompile address
    pub const ARBSYS_ADDRESS: H160 = H160(hex_literal::hex!(
        "0000000000000000000000000000000000000064"
    ));

    /// Encode outboundTransfer call for token bridging
    pub fn encode_outbound_transfer(token: H160, to: H160, amount: U256, data: Vec<u8>) -> Vec<u8> {
        // outboundTransfer(address _token, address _to, uint256 _amount, bytes _data)
        let mut calldata = Vec::with_capacity(4 + 32 * 4 + data.len());

        // Function selector: outboundTransfer(address,address,uint256,bytes)
        calldata.extend_from_slice(&[0xd2, 0xce, 0x7d, 0x65]);

        // Token address (padded to 32 bytes)
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(token.as_bytes());

        // To address (padded to 32 bytes)
        calldata.extend_from_slice(&[0u8; 12]);
        calldata.extend_from_slice(to.as_bytes());

        // Amount (32 bytes)
        let amount_bytes = amount.to_big_endian();
        calldata.extend_from_slice(&amount_bytes);

        // Data offset
        calldata.extend_from_slice(&[0u8; 31]);
        calldata.push(0x80);

        // Data length
        let data_len = data.len() as u32;
        calldata.extend_from_slice(&[0u8; 28]);
        calldata.extend_from_slice(&data_len.to_be_bytes());

        // Data (padded)
        calldata.extend_from_slice(&data);
        let padding = (32 - data.len() % 32) % 32;
        calldata.extend_from_slice(&vec![0u8; padding]);

        calldata
    }

    /// Encode sendL2Message for cross-chain messaging
    pub fn encode_send_l2_message(target: H160, calldata: Vec<u8>) -> Vec<u8> {
        // sendL2Message(address _target, bytes _data)
        let mut encoded = Vec::with_capacity(4 + 32 + 32 + 32 + calldata.len() + 31);

        // Function selector
        encoded.extend_from_slice(&[0x67, 0x9a, 0xef, 0xce]);

        // Target address
        encoded.extend_from_slice(&[0u8; 12]);
        encoded.extend_from_slice(target.as_bytes());

        // Dynamic bytes offset = 0x40 (after two static args)
        encoded.extend_from_slice(&[0u8; 31]);
        encoded.push(0x40);

        // Dynamic bytes section: length + data + padding
        let mut len_word = [0u8; 32];
        let len = calldata.len() as u32;
        len_word[28..32].copy_from_slice(&len.to_be_bytes());
        encoded.extend_from_slice(&len_word);
        encoded.extend_from_slice(&calldata);
        let padding = (32 - calldata.len() % 32) % 32;
        encoded.extend_from_slice(&vec![0u8; padding]);

        encoded
    }

    /// Arbitrum-specific: Get L1 block info from ArbSys
    #[allow(dead_code)]
    async fn get_l1_block_number(&self) -> AdapterResult<u64> {
        // ArbSys.arbBlockNumber() selector = first 4 bytes of keccak256("arbBlockNumber()")
        let selector = sp_io::hashing::keccak_256(b"arbBlockNumber()");
        let call_data = &selector[0..4];

        let params = format!(
            r#"[{{"to":"0x{}","data":"0x{}"}},"latest"]"#,
            hex::encode(Self::ARBSYS_ADDRESS.as_bytes()),
            hex::encode(call_data)
        );

        let response =
            crate::evm_rpc::call(&crate::evm_rpc::url(&self.config), "eth_call", &params).await?;
        let result = crate::evm_rpc::extract_result(&response)?;
        crate::evm_rpc::parse_hex_u64(&result)
    }
}

#[async_trait::async_trait]
impl ChainAdapter for ArbitrumAdapter {
    fn chain_type(&self) -> ChainType {
        ChainType::Arbitrum
    }

    fn config(&self) -> &ChainConfig {
        &self.config
    }

    async fn is_connected(&self) -> bool {
        // Real `eth_chainId` probe: the endpoint must answer *as Arbitrum*.
        matches!(
            crate::evm_rpc::chain_id(&crate::evm_rpc::url(&self.config)).await,
            Ok(id) if id == self.config.chain_type
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

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        #[cfg(not(feature = "std"))]
        {
            let _ = message;
            return Err(ExternalChainError::adapter_unimplemented(
                "arbitrum: send_message needs a signer and this build has no std feature",
            ));
        }

        #[cfg(feature = "std")]
        {
            // Refused without a signer, not invented. This used to return
            // `message.hash()` for a message that was never broadcast.
            let Some(signer) = self.signer.as_ref() else {
                return Err(ExternalChainError::adapter_unimplemented(
                    "arbitrum: send_message needs a signed ArbSys.sendTxToL1 transaction and this \
                     adapter has no signer; build it with `ArbitrumAdapter::with_signer` rather \
                     than receiving a hash for an unsent message",
                ));
            };

            let url = crate::evm_rpc::url(&self.config);
            // The destination is the recipient on L1 and the payload is the
            // message body — the same two values `decode_l2_to_l1_tx` reads back
            // out of the resulting `L2ToL1Tx`.
            let data = Self::encode_send_tx_to_l1(message.recipient, &message.payload);

            let nonce = crate::evm_rpc::transaction_count(&url, signer.address()).await?;
            let gas_price = crate::evm_rpc::gas_price(&url).await?;
            let gas_price: u128 = gas_price.try_into().map_err(|_| {
                ExternalChainError::rpc_error("eth_gasPrice does not fit in the transaction field")
            })?;
            let estimate =
                crate::evm_rpc::estimate_gas(&url, signer.address(), ARBSYS_ADDRESS, &data).await?;
            let gas_limit = estimate + estimate * GAS_ESTIMATE_MARGIN_PERCENT / 100;

            let transaction = x3_atomic_swap::ethereum_tx::Transaction {
                nonce,
                gas_price,
                gas_limit,
                to: Some(format!("0x{}", hex::encode(ARBSYS_ADDRESS.as_bytes()))),
                value: 0,
                data: format!("0x{}", hex::encode(&data)),
                chain_id: self.config.chain_type,
            };
            let signed = signer.sign_transaction(transaction)?;
            crate::evm_rpc::send_raw_transaction(&url, &signed).await
        }
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Decoded, not answered with an empty list: an empty list used to read as
        // "no pending L2ToL1Tx events" no matter what the chain said.
        let url = crate::evm_rpc::url(&self.config);

        let latest = crate::evm_rpc::block_number(&url).await?;
        let confirmations = u64::from(self.config.confirmations);
        let Some(to_block) = latest.checked_sub(confirmations) else {
            return Ok(Vec::new());
        };
        let from_block = to_block.saturating_sub(MESSAGE_LOOKBACK_BLOCKS);

        // The emitter is ArbSys, a system predeploy — not a contract an operator
        // deploys — so the filter is this constant rather than a configured
        // address. `L2ToL1Tx` is emitted from nowhere else, and filtering on a
        // configurably-wrong address is how a log query answers "no messages"
        // for a chain that has them.
        let entries = crate::evm_rpc::logs(
            &url,
            from_block,
            to_block,
            ARBSYS_ADDRESS,
            H256::from(l2_to_l1_tx_topic()),
        )
        .await?;

        if entries.len() > MAX_MESSAGES_PER_CALL {
            return Err(ExternalChainError::rpc_error(&format!(
                "{} L2ToL1Tx events in blocks {from_block}..={to_block}, more than the \
                 {MAX_MESSAGES_PER_CALL} this call returns; narrow the range rather than \
                 receiving a truncated queue",
                entries.len()
            )));
        }

        let mut messages = Vec::with_capacity(entries.len());
        for entry in &entries {
            messages.push(Self::decode_l2_to_l1_tx(entry)?);
        }
        Ok(messages)
    }

    async fn initiate_transfer(&self, _transfer: CrossChainTransfer) -> AdapterResult<H256> {
        // Refused: this returned `transfer.id` for a Gateway Router transfer
        // that was never sent.
        Err(ExternalChainError::adapter_unimplemented(
            "arbitrum: initiate_transfer needs a signed Gateway Router transaction and this \
             adapter has no signer",
        ))
    }

    async fn check_transfer_status(&self, _transfer_id: H256) -> AdapterResult<TransferStatus> {
        // Refused: this answered `Completed` for every transfer id, including
        // ids that do not exist. A relayer acting on that releases funds for a
        // message that never arrived.
        Err(ExternalChainError::adapter_unimplemented(
            "arbitrum: check_transfer_status needs the destination Outbox state; nothing here \
             can tell a relayed message from an unrelayed one",
        ))
    }

    async fn verify_message_proof(
        &self,
        _message: &ChainMessage,
        _proof: &[u8],
    ) -> AdapterResult<bool> {
        // Refused, not shape-checked. Arbitrum proofs walk to Nitro's state
        // commitment through a 7-day challenge period; `!proof.is_empty()` is
        // not that, and returning `true` for it made any byte string a proof.
        Err(ExternalChainError::VerificationUnavailable)
    }

    async fn finalize_transfer(&self, _transfer_id: H256, _proof: Vec<u8>) -> AdapterResult<H256> {
        // Refused: this returned the transfer id as though the message had been
        // executed on the L1 Outbox.
        Err(ExternalChainError::adapter_unimplemented(
            "arbitrum: finalize_transfer needs a signed L1 Outbox execution and a proof this \
             adapter cannot verify",
        ))
    }

    async fn estimate_gas_price(&self) -> AdapterResult<U256> {
        crate::evm_rpc::gas_price(&crate::evm_rpc::url(&self.config)).await
    }

    async fn get_transaction_receipt(
        &self,
        tx_hash: H256,
    ) -> AdapterResult<Option<TransactionReceipt>> {
        // Refused: this reported `success: true` for *every* transaction hash,
        // including hashes that were never mined.
        crate::evm_rpc::receipt(&crate::evm_rpc::url(&self.config), tx_hash).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arbitrum_adapter() {
        let adapter = ArbitrumAdapter::new(ChainConfig::for_chain(ChainType::Arbitrum));
        assert_eq!(adapter.chain_type(), ChainType::Arbitrum);
        assert_eq!(adapter.config().chain_type, 42161);
    }

    #[test]
    fn test_constants() {
        // Verify well-known addresses
        assert_ne!(ArbitrumAdapter::INBOX_ADDRESS, H160::zero());
        assert_ne!(ArbitrumAdapter::GATEWAY_ROUTER, H160::zero());
    }

    #[test]
    fn test_encode_send_l2_message_dynamic_bytes_layout() {
        let target = H160::from_low_u64_be(0xBEEF);
        let payload = vec![1u8, 2, 3, 4, 5];
        let encoded = ArbitrumAdapter::encode_send_l2_message(target, payload.clone());

        // selector + address + offset + len + payload...
        assert!(encoded.len() >= 4 + 32 + 32 + 32 + payload.len());
        // offset word should end in 0x40
        assert_eq!(encoded[4 + 32 + 31], 0x40);
        // dynamic length word should match payload length
        assert_eq!(encoded[4 + 64 + 31], payload.len() as u8);
    }

    /// An `L2ToL1Tx` log as ArbSys emits it.
    fn l2_to_l1_tx_log(
        caller: H160,
        destination: H160,
        position: u64,
        timestamp: u64,
        callvalue: U256,
        data: &[u8],
    ) -> LogEntry {
        let mut payload = Vec::new();
        let mut word = [0u8; 32];
        word[12..].copy_from_slice(caller.as_bytes());
        payload.extend_from_slice(&word);
        payload.extend_from_slice(&[0u8; 32]); // arbBlockNum
        payload.extend_from_slice(&[0u8; 32]); // ethBlockNum
        let mut word = [0u8; 32];
        word[24..].copy_from_slice(&timestamp.to_be_bytes());
        payload.extend_from_slice(&word);
        payload.extend_from_slice(&callvalue.to_big_endian());
        let mut word = [0u8; 32];
        word[31] = 192; // six head words
        payload.extend_from_slice(&word);
        let mut word = [0u8; 32];
        word[24..].copy_from_slice(&(data.len() as u64).to_be_bytes());
        payload.extend_from_slice(&word);
        payload.extend_from_slice(data);
        payload.extend_from_slice(&vec![0u8; (32 - data.len() % 32) % 32]);

        let mut destination_topic = [0u8; 32];
        destination_topic[12..].copy_from_slice(destination.as_bytes());
        let mut position_topic = [0u8; 32];
        position_topic[24..].copy_from_slice(&position.to_be_bytes());

        LogEntry {
            address: ARBSYS_ADDRESS,
            topics: vec![
                H256::from(l2_to_l1_tx_topic()),
                H256::from(destination_topic),
                H256::from([0x77u8; 32]), // hash
                H256::from(position_topic),
            ],
            data: payload,
            block_number: 300,
            transaction_hash: H256::from([0x88u8; 32]),
            log_index: 1,
        }
    }

    #[test]
    fn an_l2_to_l1_tx_log_decodes_into_every_field_it_carries() {
        let caller = H160([0x11; 20]);
        let destination = H160([0x22; 20]);
        let log = l2_to_l1_tx_log(
            caller,
            destination,
            9_001,
            1_700_000_123,
            U256::from(12_345u64),
            b"escrowed",
        );

        let message = ArbitrumAdapter::decode_l2_to_l1_tx(&log).expect("decodes");
        assert_eq!(message.source_chain, 42_161);
        assert_eq!(message.dest_chain, 1);
        assert_eq!(message.sender, caller);
        assert_eq!(message.recipient, destination);
        assert_eq!(message.nonce, 9_001);
        assert_eq!(message.value, U256::from(12_345u64));
        assert_eq!(message.payload, b"escrowed".to_vec());
        assert_eq!(message.timestamp, 1_700_000_123);
        // The event states no gas limit, so the field is zero and the decoder
        // says so rather than inventing one.
        assert_eq!(message.gas_limit, 0);
    }

    #[test]
    fn a_log_with_a_different_signature_is_refused() {
        let mut log = l2_to_l1_tx_log(H160([0x11; 20]), H160([0x22; 20]), 1, 1, U256::zero(), b"x");
        log.topics[0] = H256::from([0x55; 32]);
        assert!(ArbitrumAdapter::decode_l2_to_l1_tx(&log).is_err());
    }

    #[test]
    fn a_log_without_its_indexed_fields_is_refused() {
        let mut log = l2_to_l1_tx_log(H160([0x11; 20]), H160([0x22; 20]), 1, 1, U256::zero(), b"x");
        log.topics.truncate(2);
        assert!(ArbitrumAdapter::decode_l2_to_l1_tx(&log).is_err());
    }

    #[test]
    fn a_log_shorter_than_the_head_is_refused() {
        let mut log = l2_to_l1_tx_log(H160([0x11; 20]), H160([0x22; 20]), 1, 1, U256::zero(), b"x");
        log.data.truncate(128);
        assert!(ArbitrumAdapter::decode_l2_to_l1_tx(&log).is_err());
    }

    #[test]
    fn a_log_with_a_shifted_data_offset_is_refused() {
        let mut log = l2_to_l1_tx_log(H160([0x11; 20]), H160([0x22; 20]), 1, 1, U256::zero(), b"x");
        log.data[191] = 128;
        assert!(ArbitrumAdapter::decode_l2_to_l1_tx(&log).is_err());
    }

    #[test]
    fn a_log_whose_length_overruns_the_data_is_refused() {
        let mut log = l2_to_l1_tx_log(H160([0x11; 20]), H160([0x22; 20]), 1, 1, U256::zero(), b"x");
        // Claim 255 bytes in a log whose payload is one byte plus padding. The
        // length word's low byte is the last byte of the sixth... seventh word:
        // head (192) + length word, so `data[223]` is that low byte.
        log.data[223] = 0xff;
        assert!(ArbitrumAdapter::decode_l2_to_l1_tx(&log).is_err());
    }
}
