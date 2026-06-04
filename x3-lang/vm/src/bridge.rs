//! Bridge adapter abstraction for cross-chain calls.
//! Production adapters must verify finality/proofs; dry-run is explicit.

use std::error::Error;
use std::fmt;

pub type BridgeResult = Result<Vec<u8>, Box<dyn Error>>;

#[derive(Debug)]
pub struct BridgeError {
    pub code: &'static str,
    pub message: String,
}
impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl Error for BridgeError {}

pub trait BridgeAdapter {
    fn evm_call(&self, data: &[u8]) -> BridgeResult;
    fn svm_call(&self, data: &[u8]) -> BridgeResult;
    fn gpu_dispatch(&self, kernel: &str, args: &[u8]) -> BridgeResult;
    fn simulate(&self, body: &[u8]) -> BridgeResult;
    fn scheduled_dispatch(&self, period: u32, entry: &[u8]) -> BridgeResult;
    fn intent_resolve(&self, constraints: &[u8]) -> BridgeResult;
    fn crdt_op(&self, kind: u8, key: &[u8], value: &[u8]) -> BridgeResult;
    fn proof_verify(&self, kind: u8, proof: &[u8], input: &[u8], key: &[u8]) -> BridgeResult;
    fn storage_op(&self, kind: u8, data: &[u8]) -> BridgeResult;
    fn pathfind(&self, from: &[u8], to: &[u8], max_depth: u32) -> BridgeResult;
    fn mempool_scan(&self, max_results: u32) -> BridgeResult;
    fn oracle_request(&self, token: &[u8], reward: u128) -> BridgeResult;
    fn emergency_control(&self, kind: u8) -> BridgeResult;
    fn lifecycle(&self, kind: u8, target: &[u8]) -> BridgeResult;
    fn serialize(&self, format: u8, data: &[u8]) -> BridgeResult;
    fn deserialize(&self, format: u8, data: &[u8]) -> BridgeResult;
    fn gas_estimate(&self, chain: &[u8], route: &[u8]) -> BridgeResult;
    fn chain_metric(&self, metric: u8) -> BridgeResult;
    fn event_provenance(&self, event_type: &[u8], data: &[u8]) -> BridgeResult;
    fn multi_hop_swap(&self, path: &[u8], amount: u128) -> BridgeResult;
    fn vector_math(&self, op: u8, a: &[u8], b: &[u8], size: u32) -> BridgeResult;
    fn role_check(&self, role: &[u8]) -> BridgeResult;
    fn multisig_check(&self, required: u32, total: u32) -> BridgeResult;
    fn vrf_seed(&self) -> BridgeResult;
    fn gas_adaptive_select(&self) -> BridgeResult;
    fn bounty_escrow(&self, amount: u128, condition: &[u8]) -> BridgeResult;
}

pub struct UnconfiguredBridge;
impl BridgeAdapter for UnconfiguredBridge {
    fn evm_call(&self, _data: &[u8]) -> BridgeResult {
        Err(Box::new(BridgeError {
            code: "X3_BACKEND_REQUIRED",
            message: "production EVM bridge backend is not configured".into(),
        }))
    }
    fn svm_call(&self, _data: &[u8]) -> BridgeResult {
        Err(Box::new(BridgeError {
            code: "X3_BACKEND_REQUIRED",
            message: "production SVM bridge backend is not configured".into(),
        }))
    }
    fn gpu_dispatch(&self, _kernel: &str, _args: &[u8]) -> BridgeResult {
        backend_required("GPU dispatch")
    }
    fn simulate(&self, _body: &[u8]) -> BridgeResult {
        backend_required("simulation")
    }
    fn scheduled_dispatch(&self, _period: u32, _entry: &[u8]) -> BridgeResult {
        backend_required("scheduled dispatch")
    }
    fn intent_resolve(&self, _constraints: &[u8]) -> BridgeResult {
        backend_required("intent resolver")
    }
    fn crdt_op(&self, _kind: u8, _key: &[u8], _value: &[u8]) -> BridgeResult {
        backend_required("CRDT")
    }
    fn proof_verify(&self, _kind: u8, _proof: &[u8], _input: &[u8], _key: &[u8]) -> BridgeResult {
        backend_required("proof verifier")
    }
    fn storage_op(&self, _kind: u8, _data: &[u8]) -> BridgeResult {
        backend_required("storage")
    }
    fn pathfind(&self, _from: &[u8], _to: &[u8], _max_depth: u32) -> BridgeResult {
        backend_required("pathfinder")
    }
    fn mempool_scan(&self, _max_results: u32) -> BridgeResult {
        backend_required("mempool scanner")
    }
    fn oracle_request(&self, _token: &[u8], _reward: u128) -> BridgeResult {
        backend_required("oracle")
    }
    fn emergency_control(&self, _kind: u8) -> BridgeResult {
        backend_required("emergency control")
    }
    fn lifecycle(&self, _kind: u8, _target: &[u8]) -> BridgeResult {
        backend_required("lifecycle")
    }
    fn serialize(&self, _format: u8, _data: &[u8]) -> BridgeResult {
        backend_required("serializer")
    }
    fn deserialize(&self, _format: u8, _data: &[u8]) -> BridgeResult {
        backend_required("deserializer")
    }
    fn gas_estimate(&self, _chain: &[u8], _route: &[u8]) -> BridgeResult {
        backend_required("gas estimator")
    }
    fn chain_metric(&self, _metric: u8) -> BridgeResult {
        backend_required("chain metrics")
    }
    fn event_provenance(&self, _event_type: &[u8], _data: &[u8]) -> BridgeResult {
        backend_required("event provenance")
    }
    fn multi_hop_swap(&self, _path: &[u8], _amount: u128) -> BridgeResult {
        backend_required("multi-hop swap")
    }
    fn vector_math(&self, _op: u8, _a: &[u8], _b: &[u8], _size: u32) -> BridgeResult {
        backend_required("vector math")
    }
    fn role_check(&self, _role: &[u8]) -> BridgeResult {
        backend_required("role check")
    }
    fn multisig_check(&self, _required: u32, _total: u32) -> BridgeResult {
        backend_required("multisig check")
    }
    fn vrf_seed(&self) -> BridgeResult {
        backend_required("VRF")
    }
    fn gas_adaptive_select(&self) -> BridgeResult {
        backend_required("gas adaptive selector")
    }
    fn bounty_escrow(&self, _amount: u128, _condition: &[u8]) -> BridgeResult {
        backend_required("bounty escrow")
    }
}

pub struct DryRunBridge;
impl BridgeAdapter for DryRunBridge {
    fn evm_call(&self, data: &[u8]) -> BridgeResult {
        Ok([b"dry-run-evm:".as_slice(), data].concat())
    }
    fn svm_call(&self, data: &[u8]) -> BridgeResult {
        Ok([b"dry-run-svm:".as_slice(), data].concat())
    }
    fn gpu_dispatch(&self, kernel: &str, args: &[u8]) -> BridgeResult {
        Ok([format!("dry-run-gpu_dispatch:{kernel}:").as_bytes(), args].concat())
    }
    fn simulate(&self, body: &[u8]) -> BridgeResult {
        dry_run("simulate", body)
    }
    fn scheduled_dispatch(&self, period: u32, entry: &[u8]) -> BridgeResult {
        Ok([
            format!("dry-run-scheduled_dispatch:{period}:").as_bytes(),
            entry,
        ]
        .concat())
    }
    fn intent_resolve(&self, constraints: &[u8]) -> BridgeResult {
        dry_run("intent_resolve", constraints)
    }
    fn crdt_op(&self, kind: u8, key: &[u8], value: &[u8]) -> BridgeResult {
        Ok([
            format!("dry-run-crdt_op:{kind}:").as_bytes(),
            key,
            b":",
            value,
        ]
        .concat())
    }
    fn proof_verify(&self, kind: u8, proof: &[u8], input: &[u8], key: &[u8]) -> BridgeResult {
        Ok([
            format!("dry-run-proof_verify:{kind}:").as_bytes(),
            proof,
            b":",
            input,
            b":",
            key,
        ]
        .concat())
    }
    fn storage_op(&self, kind: u8, data: &[u8]) -> BridgeResult {
        Ok([format!("dry-run-storage_op:{kind}:").as_bytes(), data].concat())
    }
    fn pathfind(&self, from: &[u8], to: &[u8], max_depth: u32) -> BridgeResult {
        Ok([
            format!("dry-run-pathfind:{max_depth}:").as_bytes(),
            from,
            b":",
            to,
        ]
        .concat())
    }
    fn mempool_scan(&self, max_results: u32) -> BridgeResult {
        Ok(format!("dry-run-mempool_scan:{max_results}").into_bytes())
    }
    fn oracle_request(&self, token: &[u8], reward: u128) -> BridgeResult {
        Ok([
            format!("dry-run-oracle_request:{reward}:").as_bytes(),
            token,
        ]
        .concat())
    }
    fn emergency_control(&self, kind: u8) -> BridgeResult {
        Ok(format!("dry-run-emergency_control:{kind}").into_bytes())
    }
    fn lifecycle(&self, kind: u8, target: &[u8]) -> BridgeResult {
        Ok([format!("dry-run-lifecycle:{kind}:").as_bytes(), target].concat())
    }
    fn serialize(&self, format: u8, data: &[u8]) -> BridgeResult {
        Ok([format!("dry-run-serialize:{format}:").as_bytes(), data].concat())
    }
    fn deserialize(&self, format: u8, data: &[u8]) -> BridgeResult {
        Ok([format!("dry-run-deserialize:{format}:").as_bytes(), data].concat())
    }
    fn gas_estimate(&self, chain: &[u8], route: &[u8]) -> BridgeResult {
        Ok([b"dry-run-gas_estimate:".as_slice(), chain, b":", route].concat())
    }
    fn chain_metric(&self, metric: u8) -> BridgeResult {
        Ok(format!("dry-run-chain_metric:{metric}").into_bytes())
    }
    fn event_provenance(&self, event_type: &[u8], data: &[u8]) -> BridgeResult {
        Ok([
            b"dry-run-event_provenance:".as_slice(),
            event_type,
            b":",
            data,
        ]
        .concat())
    }
    fn multi_hop_swap(&self, path: &[u8], amount: u128) -> BridgeResult {
        Ok([format!("dry-run-multi_hop_swap:{amount}:").as_bytes(), path].concat())
    }
    fn vector_math(&self, op: u8, a: &[u8], b: &[u8], size: u32) -> BridgeResult {
        Ok([
            format!("dry-run-vector_math:{op}:{size}:").as_bytes(),
            a,
            b":",
            b,
        ]
        .concat())
    }
    fn role_check(&self, role: &[u8]) -> BridgeResult {
        dry_run("role_check", role)
    }
    fn multisig_check(&self, required: u32, total: u32) -> BridgeResult {
        Ok(format!("dry-run-multisig_check:{required}:{total}").into_bytes())
    }
    fn vrf_seed(&self) -> BridgeResult {
        Ok(b"dry-run-vrf_seed:00000000000000000000000000000000".to_vec())
    }
    fn gas_adaptive_select(&self) -> BridgeResult {
        Ok(vec![0])
    }
    fn bounty_escrow(&self, amount: u128, condition: &[u8]) -> BridgeResult {
        Ok([
            format!("dry-run-bounty_escrow:{amount}:").as_bytes(),
            condition,
        ]
        .concat())
    }
}

#[deprecated(
    note = "Use DryRunBridge explicitly for simulations or a real verifier-backed adapter for production"
)]
pub type MockBridge = DryRunBridge;

fn backend_required(name: &str) -> BridgeResult {
    Err(Box::new(BridgeError {
        code: "X3_BACKEND_REQUIRED",
        message: format!("production {name} backend is not configured"),
    }))
}

fn dry_run(method: &str, data: &[u8]) -> BridgeResult {
    Ok([format!("dry-run-{method}:").as_bytes(), data].concat())
}
