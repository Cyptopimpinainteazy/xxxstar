use serde::{Deserialize, Serialize};

/// 256-bit Hash
pub type Hash = [u8; 32];
pub type Address = [u8; 32];

// Stub representations built from Chapter 3 of Design Booklet

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceVector {
    pub cpu_cycles: u64,
    pub gpu_cycles: u64,
    pub memory_bytes: u64,
    pub io_ops: u64,
    pub storage_reads: u64,
    pub storage_writes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PriceVector {
    pub cpu: u128,
    pub gpu: u128,
    pub memory: u128,
    pub io: u128,
    pub storage_read: u128,
    pub storage_write: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Valid,
    InvalidExecution,
    InvalidDag,
    InvalidOrder,
    ReceiptMismatch,
    ResourceMismatch,
    ProposerEquivocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengeType {
    Execution,
    Dag,
    Resource,
    Receipt,
    Equivocation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Challenge {
    pub block_hash: Hash,
    pub challenge_type: ChallengeType,
    pub challenger: Address,
    pub bond: u128,
    pub payload: ChallengePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChallengePayload {
    ReceiptMismatch {
        action_id: u64,
        expected: Hash,
        observed: Hash,
    },
    ResourceMismatch {
        agent_id: u64,
        claimed: ResourceVector,
        actual: ResourceVector,
    },
    DagConflict {
        a: u64,
        b: u64,
    },
    Equivocation {
        block_a: Hash,
        block_b: Hash,
    },
}

#[derive(Debug, Clone)]
pub struct Action {
    pub id: u64,
    pub hash: Hash,
}

#[derive(Debug, Clone, Default)]
pub struct Receipt {
    pub hash: Hash,
}

#[derive(Debug, Clone)]
pub struct BlockHeader {
    pub proposer: Address,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub header: BlockHeader,
    pub actions: Vec<Action>,
    pub action_dag_root: Hash,
    pub execution_order_hash: Hash,
    pub receipts: Vec<Receipt>,
    pub resource_summary: ResourceVector,
}

#[derive(Debug, Clone)]
pub struct ChainState {
    pub dummy_state: u64,
}

impl ChainState {
    pub fn resource_summary(&self) -> ResourceVector {
        ResourceVector {
            cpu_cycles: 0,
            gpu_cycles: 0,
            memory_bytes: 0,
            io_ops: 0,
            storage_reads: 0,
            storage_writes: 0,
        }
    }
}

pub enum CourtVmError {
    BlockHashMismatch,
    InvalidDag,
    ExecutionFailure,
    InvalidEquivocationProof,
}

use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};

fn hash<T: serde::Serialize>(value: &T) -> Hash {
    let bytes = bincode::serialize(value).unwrap_or_default();
    let mut h = Sha256::new();
    h.update(&bytes);
    let result = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// A directed acyclic graph built from the action dependency matrix.
struct ActionDag {
    /// Adjacency list: action id → set of ids that depend on it.
    edges: HashMap<u64, Vec<u64>>,
    /// Reverse adjacency: action id → set of ids it depends on.
    reverse: HashMap<u64, Vec<u64>>,
    /// All action ids present in the DAG.
    all_ids: Vec<u64>,
    /// Actions keyed by id for constant-time lookup.
    by_id: HashMap<u64, Action>,
}

impl ActionDag {
    /// Merkle root of the DAG topology. We hash the canonical serialization
    /// of each action plus its dependency list, ordered by id.
    fn root_hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        let mut sorted_ids: Vec<u64> = self.all_ids.clone();
        sorted_ids.sort();
        for id in &sorted_ids {
            let action = &self.by_id[id];
            hasher.update(&action.id.to_le_bytes());
            hasher.update(&action.hash);
            let mut deps = self.reverse.get(id).cloned().unwrap_or_default();
            deps.sort();
            for dep in deps {
                hasher.update(&dep.to_le_bytes());
            }
        }
        let result = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    }
}

/// Build a DAG from a slice of actions.  Edge (A → B) exists if B's input
/// references A's output (deduced by hash).  This is a simplified model —
/// a full implementation would parse action payloads to extract explicit
/// dependency annotations.
fn derive_action_dag(actions: &[Action]) -> Result<ActionDag, CourtVmError> {
    if actions.is_empty() {
        return Err(CourtVmError::InvalidDag);
    }

    let by_id: HashMap<u64, Action> = actions.iter().map(|a| (a.id, a.clone())).collect();
    let all_ids: Vec<u64> = actions.iter().map(|a| a.id).collect();

    // Build edges based on hash dependencies.
    // For each pair (a, b), if b.hash has a detectable dependency on a.hash,
    // we add an edge a → b.
    let mut edges: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut reverse: HashMap<u64, Vec<u64>> = HashMap::new();

    for a in actions {
        for b in actions {
            if a.id == b.id {
                continue;
            }
            // Simple dependency detection: if b.hash's first 8 bytes match
            // a.id as little-endian, treat it as a dependency.
            // Real impl would parse SEEN/READS/WRITES annotations.
            let dep_pattern = a.id.to_le_bytes();
            if b.hash[..8] == dep_pattern {
                edges
                    .entry(a.id)
                    .or_insert_with(Vec::new)
                    .push(b.id);
                reverse
                    .entry(b.id)
                    .or_insert_with(Vec::new)
                    .push(a.id);
            }
        }
    }

    // Detect cycles via Kahn's algorithm.  If a cycle exists, return error.
    let mut in_degree: HashMap<u64, usize> = HashMap::new();
    for id in &all_ids {
        in_degree.insert(*id, reverse.get(id).map(|v| v.len()).unwrap_or(0));
    }

    let mut queue: VecDeque<u64> = all_ids
        .iter()
        .filter(|id| in_degree[id] == 0)
        .copied()
        .collect();

    let mut seen = 0usize;
    while let Some(node) = queue.pop_front() {
        seen += 1;
        if let Some(children) = edges.get(&node) {
            for child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(*child);
                    }
                }
            }
        }
    }

    if seen != all_ids.len() {
        return Err(CourtVmError::InvalidDag);
    }

    Ok(ActionDag {
        edges,
        reverse,
        all_ids,
        by_id,
    })
}

/// Topological sort (Kahn's algorithm) over the DAG to produce a
/// deterministic execution order.
fn derive_execution_order(dag: &ActionDag) -> Vec<Action> {
    let mut in_degree: HashMap<u64, usize> = HashMap::new();
    for id in &dag.all_ids {
        in_degree.insert(*id, dag.reverse.get(id).map(|v| v.len()).unwrap_or(0));
    }

    // Use a BTreeSet for deterministic tie-breaking when multiple nodes
    // have in-degree zero.
    let mut ready: std::collections::BTreeSet<u64> = dag
        .all_ids
        .iter()
        .filter(|id| in_degree[id] == 0)
        .copied()
        .collect();

    let mut order = Vec::with_capacity(dag.all_ids.len());

    while let Some(node) = ready.pop_first() {
        order.push(dag.by_id[&node].clone());
        if let Some(children) = dag.edges.get(&node) {
            for child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        ready.insert(*child);
                    }
                }
            }
        }
    }

    order
}

/// Execute a single action against the chain state.
/// Returns a receipt with a deterministic hash of the state delta.
fn execute_action(state: &mut ChainState, action: &Action) -> Result<Receipt, CourtVmError> {
    // Simulate execution: advance the dummy state counter by the action id.
    let old_state = state.dummy_state;
    state.dummy_state = state.dummy_state.wrapping_add(action.id);
    let receipt_hash = {
        let mut h = Sha256::new();
        h.update(&old_state.to_le_bytes());
        h.update(&action.id.to_le_bytes());
        h.update(&state.dummy_state.to_le_bytes());
        let result = h.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    };
    Ok(Receipt {
        hash: receipt_hash,
    })
}

fn slash_challenger(_state: &mut ChainState, _addr: &Address, _amount: u128) {}
fn slash_proposer(_state: &mut ChainState, _addr: &Address) {}
fn reward_challenger(_state: &mut ChainState, _addr: &Address, _amount: u128) {}

/// Apply Court Rules deterministic check
pub fn adjudicate(
    pre_state: &ChainState,
    block: &Block,
    chal: &Challenge,
) -> Result<Verdict, CourtVmError> {
    if hash(block) != chal.block_hash {
        return Err(CourtVmError::BlockHashMismatch);
    }
    // Derive DAG and order
    let dag = derive_action_dag(&block.actions).map_err(|_| CourtVmError::InvalidDag)?;
    if dag.root_hash() != block.action_dag_root {
        return Ok(Verdict::InvalidDag);
    }
    let order = derive_execution_order(&dag);
    if hash(&order) != block.execution_order_hash {
        return Ok(Verdict::InvalidOrder);
    }
    // Replay execution
    let mut state = pre_state.clone();
    let mut receipts = Vec::new();
    for action in order.iter() {
        let receipt =
            execute_action(&mut state, action).map_err(|_| CourtVmError::ExecutionFailure)?;
        receipts.push(receipt);
    }
    // Verify receipts
    if receipts.len() != block.receipts.len() {
        return Ok(Verdict::ReceiptMismatch);
    }
    for (r_local, r_comm) in receipts.iter().zip(block.receipts.iter()) {
        if hash(r_local) != hash(r_comm) {
            return Ok(Verdict::ReceiptMismatch);
        }
    }
    // Verify resources
    if state.resource_summary() != block.resource_summary {
        return Ok(Verdict::ResourceMismatch);
    }
    // Proposer equivocation check (distinct blocks at same height signed)
    if let ChallengePayload::Equivocation { block_a, block_b } = &chal.payload {
        if block_a == block_b {
            return Err(CourtVmError::InvalidEquivocationProof);
        }
        return Ok(Verdict::ProposerEquivocation);
    }
    Ok(Verdict::Valid)
}

pub fn apply_verdict(
    verdict: Verdict,
    block: &Block,
    challenge: &Challenge,
    state: &mut ChainState,
) {
    match verdict {
        Verdict::Valid => {
            // False challenge: slash challenger bond
            slash_challenger(state, &challenge.challenger, challenge.bond);
        }
        Verdict::InvalidDag
        | Verdict::InvalidOrder
        | Verdict::InvalidExecution
        | Verdict::ReceiptMismatch
        | Verdict::ResourceMismatch
        | Verdict::ProposerEquivocation => {
            // Valid challenge: slash proposer and reward challenger
            slash_proposer(state, &block.header.proposer);
            reward_challenger(state, &challenge.challenger, challenge.bond);
        }
    }
}
