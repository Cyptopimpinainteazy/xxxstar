//! Base Chain Adapter
//!
//! Adapter for Base (Coinbase L2) - an OP Stack rollup
//! Chain ID: 8453

use crate::adapter::*;
use crate::error::ExternalChainError;
use crate::evm_rpc::LogEntry;
use crate::ChainType;
use sp_core::{H160, H256, U256};
use sp_std::vec::Vec;

/// Sourced lookback for one `receive_messages` call, in blocks.
///
/// Bounded on purpose: `eth_getLogs` over an open range is refused by most
/// public nodes, and a request that cannot succeed is not a query.
pub const MESSAGE_LOOKBACK_BLOCKS: u64 = 5_000;

/// Refuse a batch larger than this rather than returning a partial list. A
/// caller that silently receives the first N of N+M messages believes it has
/// them all.
pub const MAX_MESSAGES_PER_CALL: usize = 256;

/// Base is an L2: a `SentMessage` it emits is bound for Ethereum.
///
/// The event carries no destination chain, so the decoder states the one the
/// messenger's counterpart lives on rather than leaving the field to be guessed
/// by a consumer. If Base ever relays elsewhere, this is the constant to change
/// and the reason the field is not derived from the log.
pub const L1_CHAIN_ID: u64 = 1;

/// The OP-Stack `L2CrossDomainMessenger` predeploy, which emits `SentMessage`.
///
/// A constant rather than `config.bridge_contract`: on Base this contract is
/// part of the chain's genesis (the OP-Stack predeploy at `0x4200…0007`), not
/// something an operator deploys, and `ChainConfig`'s default bridge address is
/// derived from a hash — so filtering by config answered an empty queue for a
/// chain that has messages. The same reasoning as `ARBSYS_ADDRESS` in the
/// Arbitrum adapter.
pub const L2_CROSS_DOMAIN_MESSENGER: H160 = H160(hex_literal::hex!(
    "4200000000000000000000000000000000000007"
));

/// Headroom over `eth_estimateGas`, in percent, before a send is signed.
///
/// State can change between the estimate and inclusion, and a transaction that
/// runs out of gas is recorded as a failed message — worse than paying a little
/// for headroom.
pub const GAS_ESTIMATE_MARGIN_PERCENT: u64 = 25;

/// An EIP-155 signer for this adapter's chain.
///
/// Key material stays here and never goes into [`ChainConfig`] (which is
/// SCALE-encoded, logged and serialised). The crypto is the workspace's single
/// EIP-155 implementation, in `x3-atomic-swap`'s `ethereum_tx`; this type exists so
/// an adapter can hold a key without growing a second implementation.
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

    fn private_key_hex(&self) -> &str {
        &self.private_key_hex
    }
}

/// The canonical OP-Stack message event:
///
/// ```solidity
/// event SentMessage(address indexed target, address sender, uint256 value,
///                   uint256 messageNonce, uint256 gasLimit, bytes message);
/// ```
///
/// Topic 0 is the keccak of that signature; `target` is the only indexed
/// parameter, so it is the only field that lives in `topics` — everything else
/// is ABI-encoded in `data`, which is what `decode_sent_message` walks.
pub fn sent_message_topic() -> [u8; 32] {
    sp_io::hashing::keccak_256(b"SentMessage(address,address,uint256,uint256,uint256,bytes)")
}

/// Base chain adapter
pub struct BaseAdapter {
    config: ChainConfig,
    #[allow(dead_code)]
    nonce: u64,
    /// Present only when the caller supplied a key; `send_message` refuses without
    /// one rather than returning a hash for a transaction nobody signed.
    #[cfg(feature = "std")]
    signer: Option<EvmSigner>,
}

impl BaseAdapter {
    /// Create new Base adapter
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
    pub fn with_signer(config: ChainConfig, signer: EvmSigner) -> Self {
        Self {
            config,
            nonce: 0,
            signer: Some(signer),
        }
    }

    /// Get chain-specific bridge ABI
    pub fn bridge_abi() -> &'static [u8] {
        // L2StandardBridge ABI for OP Stack
        include_bytes!("../../abi/l2_standard_bridge.json")
    }

    /// Decode one `SentMessage` log into a `ChainMessage`.
    ///
    /// Every field is taken from the log; nothing is defaulted. A log whose
    /// data is short, whose `message` offset is not where the ABI puts it, or
    /// whose declared `message` length runs past the data is *refused* — a
    /// relayer acting on a half-decoded message is worse than one that stops.
    pub fn decode_sent_message(
        log: &LogEntry,
        timestamp: u64,
        source_chain: u64,
        dest_chain: u64,
    ) -> AdapterResult<ChainMessage> {
        let expected_topic = sent_message_topic();
        if log.topics.len() != 2 {
            return Err(ExternalChainError::rpc_error(&format!(
                "SentMessage log has {} topics, expected 2 (signature + indexed target)",
                log.topics.len()
            )));
        }
        if log.topics[0].as_bytes() != expected_topic {
            return Err(ExternalChainError::rpc_error(
                "log is not a SentMessage event: topic 0 does not match the OP-Stack signature",
            ));
        }

        // `target` is indexed: the ABI pads an address to 32 bytes, so the
        // address is the low 20.
        let mut recipient = [0u8; 20];
        recipient.copy_from_slice(&log.topics[1].as_bytes()[12..]);

        // data = abi.encode(sender, value, messageNonce, gasLimit, message)
        // Five head words, then the tail the last word points at.
        const HEAD_WORDS: usize = 5;
        let head_bytes = HEAD_WORDS * 32;
        if log.data.len() < head_bytes {
            return Err(ExternalChainError::rpc_error(&format!(
                "SentMessage data is {} bytes, shorter than the {head_bytes}-byte head",
                log.data.len()
            )));
        }
        let word = |index: usize| -> &[u8] { &log.data[index * 32..(index + 1) * 32] };

        let mut sender = [0u8; 20];
        sender.copy_from_slice(&word(0)[12..]);
        let value = U256::from_big_endian(word(1));
        let message_nonce = u64::from_be_bytes(word(2)[24..].try_into().unwrap_or([0u8; 8]));
        let gas_limit = u64::from_be_bytes(word(3)[24..].try_into().unwrap_or([0u8; 8]));

        // The offset of `bytes message` is fixed by the ABI for this signature:
        // five head words. Anything else is a log whose shape this decoder does
        // not understand, and guessing a different offset is how a decoder reads
        // one field as another.
        let offset = U256::from_big_endian(word(4));
        if offset != U256::from(head_bytes as u64) {
            return Err(ExternalChainError::rpc_error(&format!(
                "SentMessage message offset is {offset}, expected {head_bytes}"
            )));
        }
        if log.data.len() < head_bytes + 32 {
            return Err(ExternalChainError::rpc_error(
                "SentMessage data has no length word for `message`",
            ));
        }
        let length_word = &log.data[head_bytes..head_bytes + 32];
        let length = U256::from_big_endian(length_word);
        let length: usize = length.try_into().map_err(|_| {
            ExternalChainError::rpc_error("SentMessage message length does not fit a usize")
        })?;
        let start = head_bytes + 32;
        let padded = length
            .checked_add(31)
            .map(|v| v / 32 * 32)
            .ok_or_else(|| ExternalChainError::rpc_error("SentMessage message length overflows"))?;
        if log.data.len() < start + padded {
            return Err(ExternalChainError::rpc_error(&format!(
                "SentMessage declares a {length}-byte message but carries only {} bytes of it",
                log.data.len().saturating_sub(start)
            )));
        }
        let payload = log.data[start..start + length].to_vec();

        Ok(ChainMessage {
            source_chain,
            dest_chain,
            sender: H160(sender),
            recipient: H160(recipient),
            nonce: message_nonce,
            payload,
            value,
            gas_limit,
            timestamp,
        })
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

    async fn send_message(&self, message: ChainMessage) -> AdapterResult<H256> {
        #[cfg(not(feature = "std"))]
        {
            let _ = message;
            return Err(ExternalChainError::adapter_unimplemented(
                "base: send_message needs a signer and this build has no std feature",
            ));
        }

        #[cfg(feature = "std")]
        {
            // Refused without a signer, not simulated. This used to build the
            // sendMessage calldata, run it through `eth_call` (which changes no
            // state) and return `message.hash()` as though a transaction had been
            // broadcast — a caller could not tell that from a real send.
            let Some(signer) = self.signer.as_ref() else {
                return Err(ExternalChainError::adapter_unimplemented(
                    "base: send_message needs a signed L2CrossDomainMessenger transaction and this \
                     adapter has no signer; build it with `BaseAdapter::with_signer` rather than \
                     receiving a hash for an unsent message",
                ));
            };

            let url = crate::evm_rpc::url(&self.config);
            let data =
                Self::encode_send_message(message.recipient, &message.payload, message.gas_limit);

            // Nonce, price and limit all come from the chain. A guessed nonce
            // collides with anything already in the mempool from this key, and a
            // guessed limit is a transaction that may run out of gas.
            let nonce = crate::evm_rpc::transaction_count(&url, signer.address()).await?;
            let gas_price = crate::evm_rpc::gas_price(&url).await?;
            let gas_price: u128 = gas_price.try_into().map_err(|_| {
                ExternalChainError::rpc_error("eth_gasPrice does not fit in the transaction field")
            })?;
            let estimate = crate::evm_rpc::estimate_gas(
                &url,
                signer.address(),
                L2_CROSS_DOMAIN_MESSENGER,
                &data,
            )
            .await?;
            let gas_limit = estimate + estimate * GAS_ESTIMATE_MARGIN_PERCENT / 100;

            let transaction = x3_atomic_swap::ethereum_tx::Transaction {
                nonce,
                gas_price,
                gas_limit,
                to: Some(format!(
                    "0x{}",
                    hex::encode(L2_CROSS_DOMAIN_MESSENGER.as_bytes())
                )),
                value: 0,
                data: format!("0x{}", hex::encode(&data)),
                chain_id: self.config.chain_type,
            };
            let signed = transaction.sign(signer.private_key_hex()).map_err(|e| {
                ExternalChainError::internal(&format!("could not sign the transaction: {e}"))
            })?;

            crate::evm_rpc::send_raw_transaction(&url, &signed).await
        }
    }

    async fn receive_messages(&self) -> AdapterResult<Vec<ChainMessage>> {
        // Decoded, not answered with an empty list. This used to run
        // `eth_getLogs`, throw the logs away and return `Ok(vec![])` — which
        // reads as "the chain has no pending messages" no matter what it said.
        let url = crate::evm_rpc::url(&self.config);

        let latest = crate::evm_rpc::block_number(&url).await?;
        let confirmations = u64::from(self.config.confirmations);
        // Nothing is final until the chain has that many blocks on top.
        let Some(to_block) = latest.checked_sub(confirmations) else {
            return Ok(Vec::new());
        };
        let from_block = to_block.saturating_sub(MESSAGE_LOOKBACK_BLOCKS);

        // The emitter is the OP-Stack `L2CrossDomainMessenger` predeploy, so the
        // filter is that constant — not `config.bridge_contract`, whose default is
        // a hash-derived placeholder that would make this query answer an empty
        // queue for a chain that has messages.
        let entries = crate::evm_rpc::logs(
            &url,
            from_block,
            to_block,
            L2_CROSS_DOMAIN_MESSENGER,
            H256::from(sent_message_topic()),
        )
        .await?;

        if entries.len() > MAX_MESSAGES_PER_CALL {
            return Err(ExternalChainError::rpc_error(&format!(
                "{} SentMessage logs in blocks {from_block}..={to_block}, more than the \
                 {MAX_MESSAGES_PER_CALL} this call returns; narrow the range rather than \
                 receiving a truncated queue",
                entries.len()
            )));
        }

        // One timestamp per block, fetched once. `eth_getLogs` does not carry
        // one, and inventing a timestamp for a message is the same class of
        // error as inventing its nonce.
        let mut timestamps: Vec<(u64, u64)> = Vec::new();
        let mut messages = Vec::with_capacity(entries.len());
        for entry in &entries {
            let timestamp = match timestamps.iter().find(|(b, _)| *b == entry.block_number) {
                Some((_, t)) => *t,
                None => {
                    let t = crate::evm_rpc::block_timestamp(&url, entry.block_number).await?;
                    timestamps.push((entry.block_number, t));
                    t
                }
            };
            messages.push(Self::decode_sent_message(
                entry,
                timestamp,
                self.config.chain_type,
                L1_CHAIN_ID,
            )?);
        }
        Ok(messages)
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
        // `receive_messages` is deliberately *not* asserted here any more: it now
        // queries the chain, and this adapter's default config points at the real
        // Base endpoint, so calling it in a unit test would be a network
        // dependency. Its behaviour — decoding a `SentMessage`, filtering by the
        // L2CrossDomainMessenger predeploy, refusing a malformed log — is covered
        // by `crates/external-chains/tests/receive_messages_decodes_logs.rs`.
        assert!(matches!(
            adapter.check_transfer_status(H256::zero()).await,
            Err(ExternalChainError::AdapterUnimplemented(_))
        ));
        assert!(matches!(
            adapter.finalize_transfer(H256::zero(), vec![1, 2, 3]).await,
            Err(ExternalChainError::AdapterUnimplemented(_))
        ));
    }

    /// A `SentMessage` log exactly as the OP-Stack messenger emits it.
    fn sent_message_log(
        target: H160,
        sender: H160,
        value: U256,
        nonce: u64,
        gas_limit: u64,
        message: &[u8],
    ) -> LogEntry {
        let mut data = Vec::new();
        let mut word = [0u8; 32];
        word[12..].copy_from_slice(sender.as_bytes());
        data.extend_from_slice(&word);

        data.extend_from_slice(&value.to_big_endian());

        let mut word = [0u8; 32];
        word[24..].copy_from_slice(&nonce.to_be_bytes());
        data.extend_from_slice(&word);

        let mut word = [0u8; 32];
        word[24..].copy_from_slice(&gas_limit.to_be_bytes());
        data.extend_from_slice(&word);

        // Offset of `bytes message`: five head words.
        let mut word = [0u8; 32];
        word[31] = 160;
        data.extend_from_slice(&word);

        let mut word = [0u8; 32];
        word[24..].copy_from_slice(&(message.len() as u64).to_be_bytes());
        data.extend_from_slice(&word);

        data.extend_from_slice(message);
        let padding = (32 - message.len() % 32) % 32;
        data.extend_from_slice(&vec![0u8; padding]);

        let mut target_topic = [0u8; 32];
        target_topic[12..].copy_from_slice(target.as_bytes());

        LogEntry {
            address: H160([0x11; 20]),
            topics: vec![H256::from(sent_message_topic()), H256::from(target_topic)],
            data,
            block_number: 100,
            transaction_hash: H256::from([0x22; 32]),
            log_index: 3,
        }
    }

    #[test]
    fn a_sent_message_log_decodes_into_every_field_it_carries() {
        let target = H160([0xaa; 20]);
        let sender = H160([0xbb; 20]);
        let log = sent_message_log(
            target,
            sender,
            U256::from(7_000_000_000u64),
            42,
            200_000,
            b"hello bridge",
        );

        let message = BaseAdapter::decode_sent_message(&log, 1_700_000_000, 8453, 1)
            .expect("a well-formed SentMessage log decodes");
        assert_eq!(message.source_chain, 8453);
        assert_eq!(message.dest_chain, 1);
        assert_eq!(message.sender, sender);
        assert_eq!(message.recipient, target);
        assert_eq!(message.nonce, 42);
        assert_eq!(message.gas_limit, 200_000);
        assert_eq!(message.value, U256::from(7_000_000_000u64));
        assert_eq!(message.payload, b"hello bridge".to_vec());
        assert_eq!(message.timestamp, 1_700_000_000);
    }

    #[test]
    fn a_log_with_a_different_event_signature_is_refused() {
        let mut log =
            sent_message_log(H160([0xaa; 20]), H160([0xbb; 20]), U256::zero(), 1, 1, b"x");
        log.topics[0] = H256::from([0x99; 32]);
        assert!(BaseAdapter::decode_sent_message(&log, 0, 8453, 1).is_err());
    }

    #[test]
    fn a_log_missing_the_indexed_target_is_refused() {
        let mut log =
            sent_message_log(H160([0xaa; 20]), H160([0xbb; 20]), U256::zero(), 1, 1, b"x");
        log.topics.truncate(1);
        assert!(BaseAdapter::decode_sent_message(&log, 0, 8453, 1).is_err());
    }

    #[test]
    fn a_log_shorter_than_the_head_is_refused() {
        let mut log =
            sent_message_log(H160([0xaa; 20]), H160([0xbb; 20]), U256::zero(), 1, 1, b"x");
        log.data.truncate(96);
        assert!(BaseAdapter::decode_sent_message(&log, 0, 8453, 1).is_err());
    }

    #[test]
    fn a_log_whose_offset_is_not_where_the_abi_puts_it_is_refused() {
        let mut log =
            sent_message_log(H160([0xaa; 20]), H160([0xbb; 20]), U256::zero(), 1, 1, b"x");
        // Point at a different offset: reading the tail from wherever a log
        // claims is how one field gets read as another.
        log.data[159] = 128;
        assert!(BaseAdapter::decode_sent_message(&log, 0, 8453, 1).is_err());
    }

    #[test]
    fn a_log_whose_message_length_overruns_the_data_is_refused() {
        let mut log =
            sent_message_log(H160([0xaa; 20]), H160([0xbb; 20]), U256::zero(), 1, 1, b"x");
        // Claim 1000 bytes of message in a log that carries three.
        log.data[191] = 0xe8;
        log.data[190] = 0x03;
        assert!(BaseAdapter::decode_sent_message(&log, 0, 8453, 1).is_err());
    }
}
