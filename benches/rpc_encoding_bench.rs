//! Criterion benchmarks for X3 RPC serialization / encoding hot paths.
//!
//! Benchmarks:
//! - SCALE encode/decode for common RPC response types
//! - JSON-RPC response serialization (batch responses, single responses)
//! - H256 / Address hex encoding
//! - Large payload serialization (event logs, block bodies)

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rand::RngCore;
use serde::{Deserialize, Serialize};

// ─── RPC Response Types ──────────────────────────────────────────────────

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
struct RpcResponse {
    jsonrpc: String,
    id: u64,
    result: serde_json::Value,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
struct RpcBatchResponse {
    jsonrpc: String,
    responses: Vec<RpcResponse>,
}

#[derive(Clone, Serialize)]
struct TransactionReceipt {
    tx_hash: String,
    block_hash: String,
    block_number: u64,
    from: String,
    to: Option<String>,
    gas_used: u64,
    gas_price: u64,
    status: u8,
    logs: Vec<EventLog>,
    logs_bloom: String,
}

#[derive(Clone, Serialize)]
struct EventLog {
    address: String,
    topics: Vec<String>,
    data: String,
    block_number: u64,
    tx_index: u32,
    log_index: u32,
    removed: bool,
}

#[derive(Clone, Serialize)]
struct BlockHeader {
    number: String,
    hash: String,
    parent_hash: String,
    state_root: String,
    extrinsics_root: String,
    timestamp: u64,
    author: String,
    gas_limit: u64,
    gas_used: u64,
    size: u64,
    extrinsics_count: u32,
}

// ─── Helper Functions ────────────────────────────────────────────────────

fn rand_hex<const N: usize>(rng: &mut impl RngCore) -> String {
    let mut bytes = [0u8; N];
    rng.fill_bytes(&mut bytes);
    format!("0x{}", hex::encode(bytes))
}

fn rand_hex_20(rng: &mut impl RngCore) -> String {
    rand_hex::<20>(rng)
}

fn rand_hex_32(rng: &mut impl RngCore) -> String {
    rand_hex::<32>(rng)
}

fn make_receipt(rng: &mut impl RngCore) -> TransactionReceipt {
    let log_count = rng.next_u32() as usize % 10;
    let logs: Vec<EventLog> = (0..log_count)
        .map(|_| EventLog {
            address: rand_hex_20(rng),
            topics: (0..((rng.next_u32() as usize % 3) + 1))
                .map(|_| rand_hex_32(rng))
                .collect(),
            data: format!("0x{}", hex::encode((0..64).map(|_| rng.next_u32() as u8).collect::<Vec<u8>>())),
            block_number: rng.next_u64(),
            tx_index: rng.next_u32(),
            log_index: rng.next_u32(),
            removed: false,
        })
        .collect();

    TransactionReceipt {
        tx_hash: rand_hex_32(rng),
        block_hash: rand_hex_32(rng),
        block_number: rng.next_u64(),
        from: rand_hex_20(rng),
        to: Some(rand_hex_20(rng)),
        gas_used: rng.next_u64() % 1_000_000,
        gas_price: rng.next_u64() % 100_000_000_000,
        status: 1,
        logs,
        logs_bloom: format!("0x{}", hex::encode((0..256).map(|_| rng.next_u32() as u8).collect::<Vec<u8>>())),
    }
}

fn make_block(rng: &mut impl RngCore) -> BlockHeader {
    BlockHeader {
        number: format!("0x{:x}", rng.next_u64()),
        hash: rand_hex_32(rng),
        parent_hash: rand_hex_32(rng),
        state_root: rand_hex_32(rng),
        extrinsics_root: rand_hex_32(rng),
        timestamp: rng.next_u64() % 1_700_000_000,
        author: rand_hex_20(rng),
        gas_limit: 30_000_000,
        gas_used: rng.next_u64() % 30_000_000,
        size: 15000,
        extrinsics_count: rng.next_u32() % 200,
    }
}

// ─── Benchmarks ──────────────────────────────────────────────────────────

fn bench_rpc_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("rpc_serialization");
    let mut rng = rand::thread_rng();

    // Single receipt
    let receipt = make_receipt(&mut rng);
    group.bench_function("serialize_receipt", |b| {
        b.iter(|| serde_json::to_string(black_box(&receipt)).unwrap())
    });

    // Block header
    let block = make_block(&mut rng);
    group.bench_function("serialize_block_header", |b| {
        b.iter(|| serde_json::to_string(black_box(&block)).unwrap())
    });

    // Batch of 100 receipts
    let receipts: Vec<TransactionReceipt> = (0..100)
        .map(|_| make_receipt(&mut rng))
        .collect();
    group.bench_function("serialize_100_receipts", |b| {
        b.iter(|| serde_json::to_string(black_box(&receipts)).unwrap())
    });

    // RPC batch response
    let batch = RpcBatchResponse {
        jsonrpc: "2.0".into(),
        responses: (0..50)
            .map(|i| RpcResponse {
                jsonrpc: "2.0".into(),
                id: i,
                result: serde_json::json!(receipts[i as usize % 10]),
            })
            .collect(),
    };
    group.bench_function("serialize_batch_50", |b| {
        b.iter(|| serde_json::to_string(black_box(&batch)).unwrap())
    });

    group.finish();
}

fn bench_hex_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("hex_encoding");
    let mut rng = rand::thread_rng();

    let hash: [u8; 32] = {
        let mut h = [0u8; 32];
        rng.fill_bytes(&mut h);
        h
    };

    group.bench_function("hex_encode_h256", |b| {
        b.iter(|| hex::encode(black_box(&hash)))
    });

    let hash_str = format!("0x{}", hex::encode(hash));
    group.bench_function("hex_decode_h256", |b| {
        b.iter(|| hex::decode(black_box(hash_str.trim_start_matches("0x"))).unwrap())
    });

    // Address (20 bytes)
    let addr: [u8; 20] = {
        let mut a = [0u8; 20];
        rng.fill_bytes(&mut a);
        a
    };
    group.bench_function("hex_encode_address", |b| {
        b.iter(|| hex::encode(black_box(&addr)))
    });

    group.finish();
}

fn bench_data_encoding(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_encoding");
    let mut rng = rand::thread_rng();

    let sizes = [64usize, 256, 1024, 4096, 16384];
    for size in &sizes {
        let data: Vec<u8> = (0..*size).map(|_| rng.next_u32() as u8).collect();
        let hex_str = format!("0x{}", hex::encode(&data));

        group.bench_with_input(
            BenchmarkId::new("hex_encode_bytes", size),
            size,
            |b, _| b.iter(|| hex::encode(black_box(&data))),
        );

        group.bench_with_input(
            BenchmarkId::new("hex_decode_bytes", size),
            size,
            |b, _| b.iter(|| hex::decode(black_box(hex_str.trim_start_matches("0x"))).unwrap()),
        );
    }

    group.finish();
}

fn bench_json_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_parse");
    let mut rng = rand::thread_rng();

    let receipt = make_receipt(&mut rng);
    let receipt_json = serde_json::to_string(&receipt).unwrap();

    group.bench_function("parse_receipt", |b| {
        b.iter(|| serde_json::from_str::<TransactionReceipt>(black_box(&receipt_json)).unwrap())
    });

    // eth_getLogs response (array of 200 logs)
    let logs: Vec<EventLog> = (0..200)
        .map(|_| EventLog {
            address: rand_hex_20(&mut rng),
            topics: (0..2).map(|_| rand_hex_32(&mut rng)).collect(),
            data: format!("0x{}", hex::encode((0..32).map(|_| rng.next_u32() as u8).collect::<Vec<u8>>())),
            block_number: rng.next_u64(),
            tx_index: rng.next_u32(),
            log_index: rng.next_u32(),
            removed: false,
        })
        .collect();
    let logs_json = serde_json::to_string(&logs).unwrap();

    group.bench_function("parse_200_logs", |b| {
        b.iter(|| serde_json::from_str::<Vec<EventLog>>(black_box(&logs_json)).unwrap())
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_rpc_serialization,
    bench_hex_encoding,
    bench_data_encoding,
    bench_json_parse,
);
criterion_main!(benches);