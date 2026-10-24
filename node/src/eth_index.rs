//! Node-side index of Ethereum transactions, for `eth_getTransactionByHash` and
//! `eth_getTransactionReceipt` (frontier builds).
//!
//! `pallet-ethereum` keeps a block's transactions, receipts and statuses only in its `Current*`
//! storage, which the next block overwrites, and state itself is pruned after a few hundred
//! blocks. So as each block becomes the best block, this task copies its Ethereum records into the
//! node's aux store, keyed by transaction hash. A lookup returns an entry only while its block is
//! still canonical at that height, so a reorg cannot surface a transaction from a retracted fork.
//!
//! Coverage: every canonical block whose state is still readable when the task reaches it —
//! new best blocks as they are imported, and, on a timer, blocks a major sync imported without
//! announcing them. History older than the state pruning window needs `--state-pruning archive`.

use codec::{Decode, Encode};
use futures::StreamExt;
use sc_client_api::{AuxStore, BlockchainEvents, StorageProvider};
use sp_blockchain::HeaderBackend;
use sp_core::{storage::StorageKey, H256, U256};
use std::sync::Arc;
use x3_chain_runtime::ethereum_tx::{
    current_block_storage_keys, ethereum, Receipt, Transaction, TransactionStatus,
};

use crate::service::FullClient;

const LOG_TARGET: &str = "eth-index";
const TX_PREFIX: &[u8] = b"x3/eth-index/tx/";
const BLOCK_PREFIX: &[u8] = b"x3/eth-index/block/";
/// Height the catch-up walk has covered contiguously. Only `catch_up` moves it: a new best block
/// indexed from the import stream must not let the walk skip the blocks below it.
const LAST_INDEXED: &[u8] = b"x3/eth-index/last";
/// How far back the startup backfill reaches: the SDK's default state pruning window.
const BACKFILL_WINDOW: u32 = 256;
/// How often the canonical chain is walked for blocks the import stream did not announce.
const CATCH_UP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(6);
/// Most blocks one catch-up pass indexes, so a long sync does not starve the import handler.
const CATCH_UP_BATCH: u32 = 512;

/// One Ethereum transaction as included in a block.
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct IndexedTransaction {
    /// Substrate hash of the block that included it.
    pub block_hash: H256,
    /// Number of that block.
    pub block_number: u32,
    /// The signed transaction.
    pub transaction: Transaction,
    /// Sender, recipient, created contract and logs, as pallet-ethereum recorded them.
    pub status: TransactionStatus,
    /// Status code, cumulative gas and bloom.
    pub receipt: Receipt,
    /// Gas used by this transaction alone; the receipt carries the block's running total.
    pub gas_used: U256,
    /// Index, within the block, of this transaction's first log.
    pub first_log_index: u32,
}

fn tx_key(hash: &H256) -> Vec<u8> {
    [TX_PREFIX, hash.as_bytes()].concat()
}

/// Keyed by block hash, not number, so a fork's record never overwrites the canonical one.
fn block_key(hash: &H256) -> Vec<u8> {
    [BLOCK_PREFIX, hash.as_bytes()].concat()
}

fn receipt_data(receipt: &Receipt) -> &ethereum::EIP658ReceiptData {
    match receipt {
        Receipt::Legacy(d) | Receipt::EIP2930(d) | Receipt::EIP1559(d) | Receipt::EIP7702(d) => d,
    }
}

fn read<T: Decode>(client: &FullClient, at: H256, key: Vec<u8>) -> Result<Option<T>, String> {
    let Some(data) = client
        .storage(at, &StorageKey(key))
        .map_err(|e| format!("state at {at:?}: {e}"))?
    else {
        return Ok(None);
    };
    T::decode(&mut &data.0[..])
        .map(Some)
        .map_err(|e| format!("decode at {at:?}: {e}"))
}

/// The Ethereum transactions of block `hash`, read from its post-state.
pub fn block_entries(client: &FullClient, hash: H256) -> Result<Vec<IndexedTransaction>, String> {
    let number = client
        .number(hash)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("unknown block {hash:?}"))?;
    let (block_key, receipts_key, statuses_key) = current_block_storage_keys();
    let Some(block) = read::<ethereum::BlockV3>(client, hash, block_key)? else {
        return Ok(Vec::new());
    };
    // `Current*` is written in every block's `on_finalize`; a record for another height is stale.
    if block.header.number != U256::from(number) {
        return Ok(Vec::new());
    }
    let receipts: Vec<Receipt> = read(client, hash, receipts_key)?.unwrap_or_default();
    let statuses: Vec<TransactionStatus> = read(client, hash, statuses_key)?.unwrap_or_default();
    if receipts.len() != block.transactions.len() || statuses.len() != block.transactions.len() {
        return Err(format!(
            "block {number}: {} transactions, {} receipts, {} statuses",
            block.transactions.len(),
            receipts.len(),
            statuses.len()
        ));
    }

    let mut previous_gas = U256::zero();
    let mut log_index = 0u32;
    let mut entries = Vec::with_capacity(statuses.len());
    for ((transaction, receipt), status) in
        block.transactions.into_iter().zip(receipts).zip(statuses)
    {
        let cumulative = receipt_data(&receipt).used_gas;
        let logs = status.logs.len() as u32;
        entries.push(IndexedTransaction {
            block_hash: hash,
            block_number: number,
            transaction,
            status,
            receipt,
            gas_used: cumulative.saturating_sub(previous_gas),
            first_log_index: log_index,
        });
        previous_gas = cumulative;
        log_index = log_index.saturating_add(logs);
    }
    Ok(entries)
}

/// Index block `hash`; returns how many Ethereum transactions it held.
pub fn index_block(client: &FullClient, hash: H256) -> Result<usize, String> {
    let entries = block_entries(client, hash)?;
    let mut writes: Vec<(Vec<u8>, Vec<u8>)> = entries
        .iter()
        .map(|e| (tx_key(&e.status.transaction_hash), e.encode()))
        .collect();
    // Written even when empty: "indexed, no Ethereum transactions" differs from "not indexed".
    writes.push((block_key(&hash), entries.encode()));
    let pairs: Vec<(&[u8], &[u8])> = writes.iter().map(|(k, v)| (&k[..], &v[..])).collect();
    let deletes: [&[u8]; 0] = [];
    client
        .insert_aux(&pairs, &deletes)
        .map_err(|e| format!("aux write: {e}"))?;
    Ok(entries.len())
}

fn last_indexed(client: &FullClient) -> Result<Option<u32>, String> {
    let raw = client.get_aux(LAST_INDEXED).map_err(|e| e.to_string())?;
    Ok(raw.and_then(|raw| u32::decode(&mut &raw[..]).ok()))
}

/// The indexed transaction `hash`, if its block is canonical at its height.
pub fn lookup<C>(client: &C, hash: &H256) -> Result<Option<IndexedTransaction>, String>
where
    C: AuxStore + HeaderBackend<x3_chain_runtime::opaque::Block>,
{
    let Some(raw) = client.get_aux(&tx_key(hash)).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let entry = IndexedTransaction::decode(&mut &raw[..]).map_err(|e| e.to_string())?;
    let canonical = client.hash(entry.block_number).map_err(|e| e.to_string())?;
    Ok((canonical == Some(entry.block_hash)).then_some(entry))
}

/// The Ethereum transactions of block `hash`, or `None` if the block was never indexed.
pub fn block_transactions<C: AuxStore>(
    client: &C,
    hash: &H256,
) -> Result<Option<Vec<IndexedTransaction>>, String> {
    let Some(raw) = client
        .get_aux(&block_key(hash))
        .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    Vec::<IndexedTransaction>::decode(&mut &raw[..])
        .map(Some)
        .map_err(|e| e.to_string())
}

/// An `eth_getLogs` address/topics filter.
#[derive(Debug, Default, PartialEq)]
pub struct LogFilter {
    /// Empty matches every address.
    pub addresses: Vec<sp_core::H160>,
    /// Position `i` constrains topic `i`: `None` is a wildcard, `Some(set)` matches any member.
    pub topics: Vec<Option<Vec<H256>>>,
}

fn parse_hex<T: From<[u8; N]>, const N: usize>(value: &serde_json::Value) -> Result<T, String> {
    let text = value.as_str().ok_or("expected a hex string")?;
    let bytes = ::hex::decode(text.trim_start_matches("0x")).map_err(|e| e.to_string())?;
    let array: [u8; N] = bytes
        .try_into()
        .map_err(|_| format!("expected {N} bytes: {text}"))?;
    Ok(T::from(array))
}

impl LogFilter {
    /// Parse the `address` and `topics` members of an `eth_getLogs` filter object.
    pub fn from_json(filter: &serde_json::Value) -> Result<Self, String> {
        let addresses = match filter.get("address") {
            None | Some(serde_json::Value::Null) => Vec::new(),
            Some(serde_json::Value::Array(list)) => list
                .iter()
                .map(parse_hex::<_, 20>)
                .collect::<Result<_, _>>()?,
            Some(one) => vec![parse_hex::<_, 20>(one)?],
        };
        let topics = match filter.get("topics") {
            None | Some(serde_json::Value::Null) => Vec::new(),
            Some(serde_json::Value::Array(positions)) => positions
                .iter()
                .map(|position| match position {
                    serde_json::Value::Null => Ok(None),
                    serde_json::Value::Array(any) => any
                        .iter()
                        .map(parse_hex::<_, 32>)
                        .collect::<Result<Vec<H256>, _>>()
                        .map(Some),
                    one => parse_hex::<_, 32>(one).map(|t| Some(vec![t])),
                })
                .collect::<Result<_, String>>()?,
            Some(_) => return Err("topics must be an array".to_string()),
        };
        Ok(Self { addresses, topics })
    }

    /// Whether `log` passes the filter.
    pub fn matches(&self, log: &ethereum::Log) -> bool {
        if !self.addresses.is_empty() && !self.addresses.contains(&log.address) {
            return false;
        }
        self.topics
            .iter()
            .enumerate()
            .all(|(i, wanted)| match wanted {
                None => true,
                Some(set) => log.topics.get(i).is_some_and(|t| set.contains(t)),
            })
    }
}

/// Logs of the canonical blocks `from..=to` that pass `filter`. A block in the range that was
/// never indexed is an error, not an empty result: the caller would otherwise read "no logs".
pub fn logs_in_range<C>(
    client: &C,
    from: u32,
    to: u32,
    filter: &LogFilter,
) -> Result<Vec<serde_json::Value>, String>
where
    C: AuxStore + HeaderBackend<x3_chain_runtime::opaque::Block>,
{
    let mut out = Vec::new();
    for number in from..=to {
        let Some(hash) = client.hash(number).map_err(|e| e.to_string())? else {
            break;
        };
        out.extend(logs_in_block(client, &hash, filter)?);
    }
    Ok(out)
}

/// Logs of block `hash` that pass `filter`.
pub fn logs_in_block<C: AuxStore>(
    client: &C,
    hash: &H256,
    filter: &LogFilter,
) -> Result<Vec<serde_json::Value>, String> {
    let entries = block_transactions(client, hash)?
        .ok_or_else(|| format!("block {hash:?} is not in the Ethereum index"))?;
    Ok(entries
        .iter()
        .flat_map(|entry| {
            entry
                .status
                .logs
                .iter()
                .enumerate()
                .filter(|(_, log)| filter.matches(log))
                .map(move |(i, log)| log_json(entry, i, log))
        })
        .collect())
}

/// Ethereum fields for a block's JSON: its transactions (hashes, or objects when `full`),
/// gas used, the OR of its logs blooms, and the EVM block gas limit. `None` if not indexed.
pub fn block_fields<C: AuxStore>(
    client: &C,
    hash: &H256,
    full: bool,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, String> {
    let Some(entries) = block_transactions(client, hash)? else {
        return Ok(None);
    };
    let gas_used = entries
        .iter()
        .fold(U256::zero(), |acc, e| acc.saturating_add(e.gas_used));
    let mut bloom = [0u8; 256];
    for entry in &entries {
        for (b, x) in bloom.iter_mut().zip(entry.status.logs_bloom.as_bytes()) {
            *b |= x;
        }
    }
    let transactions: Vec<serde_json::Value> = entries
        .iter()
        .map(|e| {
            if full {
                transaction_json(e)
            } else {
                hex(e.status.transaction_hash.as_bytes()).into()
            }
        })
        .collect();
    let mut fields = serde_json::Map::new();
    fields.insert("transactions".into(), transactions.into());
    fields.insert("gasUsed".into(), quantity(gas_used).into());
    fields.insert("logsBloom".into(), hex(&bloom).into());
    fields.insert(
        "gasLimit".into(),
        quantity(x3_chain_runtime::ethereum_tx::block_gas_limit()).into(),
    );
    Ok(Some(fields))
}

fn index_logged(client: &FullClient, hash: H256) {
    match index_block(client, hash) {
        Ok(0) => {}
        Ok(n) => log::debug!(target: LOG_TARGET, "indexed {n} Ethereum transactions in {hash:?}"),
        Err(e) => log::warn!(target: LOG_TARGET, "could not index {hash:?}: {e}"),
    }
}

/// Backfill what was missed, then index every new best block (and, on a reorg, every block the
/// new best chain enacts).
pub async fn run(client: Arc<FullClient>) {
    // Subscribe before backfilling: a block imported between the two would otherwise be missed.
    let mut imports = client.import_notification_stream();
    let floor = client.info().best_number.saturating_sub(BACKFILL_WINDOW);
    catch_up(&client, floor, u32::MAX);

    // Blocks imported during a major sync are not announced on the import stream, so the
    // canonical chain is also walked on a timer.
    let mut tick = tokio::time::interval(CATCH_UP_INTERVAL);
    loop {
        tokio::select! {
            notification = imports.next() => {
                let Some(notification) = notification else { break };
                if !notification.is_new_best {
                    continue;
                }
                if let Some(route) = &notification.tree_route {
                    for enacted in route.enacted() {
                        index_logged(&client, enacted.hash);
                    }
                }
                index_logged(&client, notification.hash);
            }
            _ = tick.tick() => catch_up(&client, 0, CATCH_UP_BATCH),
        }
    }
}

/// Index up to `limit` canonical blocks after the last indexed one (and not below `floor`).
///
/// A block whose state is already pruned cannot be read; it is logged and passed over, so it stays
/// unindexed and the lookups report it as such instead of as "no transactions". Run the node with
/// `--state-pruning archive` to index history a sync imports faster than this task reads it.
fn catch_up(client: &FullClient, floor: u32, limit: u32) {
    let best = client.info().best_number;
    let start = match last_indexed(client) {
        Ok(Some(last)) => last.saturating_add(1),
        _ => 0,
    }
    .max(floor);
    if start > best {
        return;
    }
    let end = best.min(start.saturating_add(limit.saturating_sub(1)));
    for number in start..=end {
        if let Ok(Some(hash)) = client.hash(number) {
            index_logged(client, hash);
        }
    }
    // Move past blocks that could not be read, so one pruned block does not stall the index.
    if let Err(e) = client.insert_aux(&[(LAST_INDEXED, &end.encode()[..])], &[] as &[&[u8]]) {
        log::warn!(target: LOG_TARGET, "could not record catch-up progress: {e}");
    }
}

fn hex(bytes: &[u8]) -> String {
    format!("0x{}", ::hex::encode(bytes))
}

fn quantity(value: U256) -> String {
    format!("0x{value:x}")
}

fn to_address(action: &ethereum::TransactionAction) -> serde_json::Value {
    match action {
        ethereum::TransactionAction::Call(to) => hex(to.as_bytes()).into(),
        ethereum::TransactionAction::Create => serde_json::Value::Null,
    }
}

fn access_list(list: &[ethereum::AccessListItem]) -> serde_json::Value {
    list.iter()
        .map(|item| {
            serde_json::json!({
                "address": hex(item.address.as_bytes()),
                "storageKeys": item.storage_keys.iter().map(|k| hex(k.as_bytes())).collect::<Vec<_>>(),
            })
        })
        .collect()
}

/// What the sender paid per gas. EIP-1559 transactions pay `min(max_fee, base + priority)`, with
/// the runtime's fixed minimum gas price as the base fee (pallet-ethereum's `FeeCalculator`).
fn effective_gas_price(transaction: &Transaction) -> U256 {
    let base_fee = U256::from(x3_chain_runtime::NATIVE_GAS_PRICE);
    match transaction {
        Transaction::Legacy(t) => t.gas_price,
        Transaction::EIP2930(t) => t.gas_price,
        Transaction::EIP1559(t) => t
            .max_fee_per_gas
            .min(base_fee.saturating_add(t.max_priority_fee_per_gas)),
        Transaction::EIP7702(t) => t
            .max_fee_per_gas
            .min(base_fee.saturating_add(t.max_priority_fee_per_gas)),
    }
}

fn type_id(transaction: &Transaction) -> u64 {
    match transaction {
        Transaction::Legacy(_) => 0,
        Transaction::EIP2930(_) => 1,
        Transaction::EIP1559(_) => 2,
        Transaction::EIP7702(_) => 4,
    }
}

/// `eth_getTransactionByHash` result.
pub fn transaction_json(entry: &IndexedTransaction) -> serde_json::Value {
    let mut tx = serde_json::json!({
        "hash": hex(entry.status.transaction_hash.as_bytes()),
        "blockHash": hex(entry.block_hash.as_bytes()),
        "blockNumber": quantity(entry.block_number.into()),
        "transactionIndex": quantity(entry.status.transaction_index.into()),
        "from": hex(entry.status.from.as_bytes()),
        "type": quantity(type_id(&entry.transaction).into()),
        "gasPrice": quantity(effective_gas_price(&entry.transaction)),
    });
    let fields = match &entry.transaction {
        Transaction::Legacy(t) => serde_json::json!({
            "nonce": quantity(t.nonce),
            "gas": quantity(t.gas_limit),
            "to": to_address(&t.action),
            "value": quantity(t.value),
            "input": hex(&t.input),
            "chainId": t.signature.chain_id().map(|id| quantity(id.into())),
            "v": quantity(t.signature.v().into()),
            "r": hex(t.signature.r().as_bytes()),
            "s": hex(t.signature.s().as_bytes()),
        }),
        Transaction::EIP2930(t) => serde_json::json!({
            "nonce": quantity(t.nonce),
            "gas": quantity(t.gas_limit),
            "to": to_address(&t.action),
            "value": quantity(t.value),
            "input": hex(&t.input),
            "chainId": quantity(t.chain_id.into()),
            "accessList": access_list(&t.access_list),
            "v": quantity(u64::from(t.signature.odd_y_parity()).into()),
            "yParity": quantity(u64::from(t.signature.odd_y_parity()).into()),
            "r": hex(t.signature.r().as_bytes()),
            "s": hex(t.signature.s().as_bytes()),
        }),
        Transaction::EIP1559(t) => serde_json::json!({
            "nonce": quantity(t.nonce),
            "gas": quantity(t.gas_limit),
            "to": to_address(&t.action),
            "value": quantity(t.value),
            "input": hex(&t.input),
            "chainId": quantity(t.chain_id.into()),
            "maxFeePerGas": quantity(t.max_fee_per_gas),
            "maxPriorityFeePerGas": quantity(t.max_priority_fee_per_gas),
            "accessList": access_list(&t.access_list),
            "v": quantity(u64::from(t.signature.odd_y_parity()).into()),
            "yParity": quantity(u64::from(t.signature.odd_y_parity()).into()),
            "r": hex(t.signature.r().as_bytes()),
            "s": hex(t.signature.s().as_bytes()),
        }),
        Transaction::EIP7702(t) => serde_json::json!({
            "nonce": quantity(t.nonce),
            "gas": quantity(t.gas_limit),
            "to": to_address(&t.destination),
            "value": quantity(t.value),
            "input": hex(&t.data),
            "chainId": quantity(t.chain_id.into()),
            "maxFeePerGas": quantity(t.max_fee_per_gas),
            "maxPriorityFeePerGas": quantity(t.max_priority_fee_per_gas),
            "accessList": access_list(&t.access_list),
            "v": quantity(u64::from(t.signature.odd_y_parity()).into()),
            "yParity": quantity(u64::from(t.signature.odd_y_parity()).into()),
            "r": hex(t.signature.r().as_bytes()),
            "s": hex(t.signature.s().as_bytes()),
        }),
    };
    if let (Some(tx), serde_json::Value::Object(fields)) = (tx.as_object_mut(), fields) {
        tx.extend(fields);
    }
    tx
}

fn log_json(entry: &IndexedTransaction, i: usize, log: &ethereum::Log) -> serde_json::Value {
    serde_json::json!({
        "address": hex(log.address.as_bytes()),
        "topics": log.topics.iter().map(|t| hex(t.as_bytes())).collect::<Vec<_>>(),
        "data": hex(&log.data),
        "blockHash": hex(entry.block_hash.as_bytes()),
        "blockNumber": quantity(entry.block_number.into()),
        "transactionHash": hex(entry.status.transaction_hash.as_bytes()),
        "transactionIndex": quantity(entry.status.transaction_index.into()),
        "logIndex": quantity((entry.first_log_index + i as u32).into()),
        "removed": false,
    })
}

/// `eth_getTransactionReceipt` result.
pub fn receipt_json(entry: &IndexedTransaction) -> serde_json::Value {
    let data = receipt_data(&entry.receipt);
    let block_hash = hex(entry.block_hash.as_bytes());
    let block_number = quantity(entry.block_number.into());
    let tx_hash = hex(entry.status.transaction_hash.as_bytes());
    let tx_index = quantity(entry.status.transaction_index.into());
    let logs: Vec<serde_json::Value> = entry
        .status
        .logs
        .iter()
        .enumerate()
        .map(|(i, log)| log_json(entry, i, log))
        .collect();
    serde_json::json!({
        "transactionHash": tx_hash,
        "transactionIndex": tx_index,
        "blockHash": block_hash,
        "blockNumber": block_number,
        "from": hex(entry.status.from.as_bytes()),
        "to": entry.status.to.map(|to| hex(to.as_bytes())),
        "contractAddress": entry.status.contract_address.map(|a| hex(a.as_bytes())),
        "cumulativeGasUsed": quantity(data.used_gas),
        "gasUsed": quantity(entry.gas_used),
        "effectiveGasPrice": quantity(effective_gas_price(&entry.transaction)),
        "logs": logs,
        "logsBloom": hex(data.logs_bloom.as_bytes()),
        "status": quantity(data.status_code.into()),
        "type": quantity(type_id(&entry.transaction).into()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethereum::{legacy::TransactionSignature, EIP658ReceiptData, LegacyTransaction};

    fn entry(status_code: u8, cumulative: u64, gas_used: u64) -> IndexedTransaction {
        let signature = TransactionSignature::new(
            650_000 * 2 + 35,
            H256::repeat_byte(0x11),
            H256::repeat_byte(0x22),
        )
        .expect("canonical test signature");
        let transaction = Transaction::Legacy(LegacyTransaction {
            nonce: U256::from(7),
            gas_price: U256::from(1_000_000_000u64),
            gas_limit: U256::from(100_000u64),
            action: ethereum::TransactionAction::Call(sp_core::H160::repeat_byte(0x33)),
            value: U256::from(5u64),
            input: vec![0xab],
            signature,
        });
        let log = ethereum::Log {
            address: sp_core::H160::repeat_byte(0x44),
            topics: vec![H256::repeat_byte(0x55)],
            data: vec![1, 2],
        };
        IndexedTransaction {
            block_hash: H256::repeat_byte(0x66),
            block_number: 42,
            status: TransactionStatus {
                transaction_hash: transaction.hash(),
                transaction_index: 1,
                from: sp_core::H160::repeat_byte(0x77),
                to: Some(sp_core::H160::repeat_byte(0x33)),
                contract_address: None,
                logs: vec![log.clone()],
                logs_bloom: Default::default(),
            },
            receipt: Receipt::Legacy(EIP658ReceiptData {
                status_code,
                used_gas: U256::from(cumulative),
                logs_bloom: Default::default(),
                logs: vec![log],
            }),
            transaction,
            gas_used: U256::from(gas_used),
            first_log_index: 3,
        }
    }

    fn log(address: u8, topics: &[u8]) -> ethereum::Log {
        ethereum::Log {
            address: sp_core::H160::repeat_byte(address),
            topics: topics.iter().map(|t| H256::repeat_byte(*t)).collect(),
            data: vec![],
        }
    }

    #[test]
    fn log_filter_follows_eth_get_logs_semantics() {
        let a = hex(&[0x44; 20]);
        let t1 = hex(&[0x55; 32]);
        let t2 = hex(&[0x56; 32]);
        let f = |json| LogFilter::from_json(&json).unwrap();

        assert!(f(serde_json::json!({})).matches(&log(0x01, &[0x99])));
        assert!(f(serde_json::json!({ "address": a })).matches(&log(0x44, &[])));
        assert!(!f(serde_json::json!({ "address": a })).matches(&log(0x45, &[])));
        assert!(f(serde_json::json!({ "address": [hex(&[0x45; 20]), a] })).matches(&log(0x44, &[])));
        // Position 0 wildcard, position 1 either of two topics.
        let either = f(serde_json::json!({ "topics": [null, [t1, t2]] }));
        assert!(either.matches(&log(0x01, &[0x00, 0x56])));
        assert!(!either.matches(&log(0x01, &[0x00, 0x57])));
        assert!(
            !either.matches(&log(0x01, &[0x00])),
            "a missing topic is not a match"
        );
        assert!(LogFilter::from_json(&serde_json::json!({ "address": "0x12" })).is_err());
        assert!(LogFilter::from_json(&serde_json::json!({ "topics": "0x12" })).is_err());
    }

    #[test]
    fn entry_round_trips_through_scale() {
        let e = entry(1, 43_200, 21_600);
        assert_eq!(IndexedTransaction::decode(&mut &e.encode()[..]).unwrap(), e);
    }

    #[test]
    fn transaction_json_reports_the_signed_fields() {
        let e = entry(1, 43_200, 21_600);
        let json = transaction_json(&e);
        assert_eq!(json["hash"], hex(e.status.transaction_hash.as_bytes()));
        assert_eq!(json["blockNumber"], "0x2a");
        assert_eq!(json["transactionIndex"], "0x1");
        assert_eq!(json["nonce"], "0x7");
        assert_eq!(json["value"], "0x5");
        assert_eq!(json["gas"], "0x186a0");
        assert_eq!(json["gasPrice"], "0x3b9aca00");
        assert_eq!(json["input"], "0xab");
        assert_eq!(json["chainId"], "0x9eb10");
        assert_eq!(json["type"], "0x0");
        assert_eq!(json["to"], hex(&[0x33; 20]));
    }

    #[test]
    fn receipt_json_separates_own_gas_from_cumulative_and_numbers_logs() {
        let json = receipt_json(&entry(1, 43_200, 21_600));
        assert_eq!(json["status"], "0x1");
        assert_eq!(json["gasUsed"], "0x5460");
        assert_eq!(json["cumulativeGasUsed"], "0xa8c0");
        assert_eq!(json["logs"][0]["logIndex"], "0x3");
        assert_eq!(json["logs"][0]["removed"], false);
        assert_eq!(json["contractAddress"], serde_json::Value::Null);

        assert_eq!(receipt_json(&entry(0, 21_000, 21_000))["status"], "0x0");
    }
}
