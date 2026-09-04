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

#[derive(Debug, Clone, Serialize)]
pub struct Action {
    pub id: u64,
    pub hash: Hash,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Receipt {
    pub hash: Hash,
}

#[derive(Debug, Clone, Serialize)]
pub struct BlockHeader {
    pub proposer: Address,
}

#[derive(Debug, Clone, Serialize)]
pub struct Block {
    pub header: BlockHeader,
    pub actions: Vec<Action>,
    pub action_dag_root: Hash,
    pub execution_order_hash: Hash,
    pub receipts: Vec<Receipt>,
    pub resource_summary: ResourceVector,
}

#[derive(Debug, Clone)]
pub struct CourtVmConfig {
    pub proposer_slash_penalty: u128,
}

impl Default for CourtVmConfig {
    fn default() -> Self {
        Self {
            proposer_slash_penalty: 10_000_000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChainState {
    pub dummy_state: u64,
    pub config: CourtVmConfig,
    pub balances: HashMap<Address, u128>,
    pub reputations: HashMap<Address, u64>,
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

#[derive(Debug)]
pub enum CourtVmError {
    BlockHashMismatch,
    InvalidDag,
    ExecutionFailure,
    InvalidEquivocationProof,
    BalanceInsufficient,
    AccountNotFound,
}

use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};

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
            hasher.update(action.id.to_le_bytes());
            hasher.update(action.hash);
            let mut deps = self.reverse.get(id).cloned().unwrap_or_default();
            deps.sort();
            for dep in deps {
                hasher.update(dep.to_le_bytes());
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
                edges.entry(a.id).or_default().push(b.id);
                reverse.entry(b.id).or_default().push(a.id);
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
        h.update(old_state.to_le_bytes());
        h.update(action.id.to_le_bytes());
        h.update(state.dummy_state.to_le_bytes());
        let result = h.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&result);
        out
    };
    Ok(Receipt { hash: receipt_hash })
}

fn slash_challenger(
    state: &mut ChainState,
    addr: &Address,
    amount: u128,
) -> Result<(), CourtVmError> {
    let balance = state.balances.entry(*addr).or_insert(0);
    *balance = balance.saturating_sub(amount);
    Ok(())
}

fn slash_proposer(state: &mut ChainState, addr: &Address) -> Result<(), CourtVmError> {
    let penalty = state.config.proposer_slash_penalty;
    let balance = state.balances.entry(*addr).or_insert(0);
    *balance = balance.saturating_sub(penalty);
    // Reduce reputation on slash
    let rep = state.reputations.entry(*addr).or_insert(0);
    *rep = rep.saturating_sub(1);
    Ok(())
}

fn reward_challenger(
    state: &mut ChainState,
    addr: &Address,
    amount: u128,
) -> Result<(), CourtVmError> {
    let balance = state.balances.entry(*addr).or_insert(0);
    *balance = balance.saturating_add(amount);
    Ok(())
}

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
) -> Result<(), CourtVmError> {
    match verdict {
        Verdict::Valid => {
            slash_challenger(state, &challenge.challenger, challenge.bond)?;
        }
        Verdict::InvalidDag
        | Verdict::InvalidOrder
        | Verdict::InvalidExecution
        | Verdict::ReceiptMismatch
        | Verdict::ResourceMismatch
        | Verdict::ProposerEquivocation => {
            slash_proposer(state, &block.header.proposer)?;
            reward_challenger(state, &challenge.challenger, challenge.bond)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn addr1() -> Address {
        [1u8; 32]
    }

    fn addr2() -> Address {
        [2u8; 32]
    }

    fn make_balance(balances: Vec<(Address, u128)>) -> HashMap<Address, u128> {
        let mut m = HashMap::new();
        for (addr, bal) in balances {
            m.insert(addr, bal);
        }
        m
    }

    fn default_state(balances: HashMap<Address, u128>) -> ChainState {
        ChainState {
            dummy_state: 0,
            config: CourtVmConfig::default(),
            balances,
            reputations: HashMap::new(),
        }
    }

    fn default_block(proposer: Address) -> Block {
        Block {
            header: BlockHeader { proposer },
            actions: vec![],
            action_dag_root: [0; 32],
            execution_order_hash: [0; 32],
            receipts: vec![],
            resource_summary: ResourceVector {
                cpu_cycles: 0,
                gpu_cycles: 0,
                memory_bytes: 0,
                io_ops: 0,
                storage_reads: 0,
                storage_writes: 0,
            },
        }
    }

    fn default_challenge(challenger: Address, bond: u128) -> Challenge {
        Challenge {
            block_hash: [0; 32],
            challenge_type: ChallengeType::Execution,
            challenger,
            bond,
            payload: ChallengePayload::ReceiptMismatch {
                action_id: 0,
                expected: [0; 32],
                observed: [0; 32],
            },
        }
    }

    #[test]
    fn test_slash_challenger_reduces_balance() {
        let mut state = default_state(make_balance(vec![(addr1(), 100_000)]));
        slash_challenger(&mut state, &addr1(), 10_000).unwrap();
        assert_eq!(state.balances[&addr1()], 90_000);
    }

    #[test]
    fn test_slash_challenger_saturates_to_zero() {
        let mut state = default_state(make_balance(vec![(addr1(), 5_000)]));
        slash_challenger(&mut state, &addr1(), 10_000).unwrap();
        assert_eq!(state.balances[&addr1()], 0);
    }

    #[test]
    fn test_slash_challenger_new_account() {
        let mut state = default_state(HashMap::new());
        slash_challenger(&mut state, &addr1(), 10_000).unwrap();
        assert_eq!(state.balances[&addr1()], 0);
    }

    #[test]
    fn test_reward_challenger_increases_balance() {
        let mut state = default_state(make_balance(vec![(addr1(), 50_000)]));
        reward_challenger(&mut state, &addr1(), 25_000).unwrap();
        assert_eq!(state.balances[&addr1()], 75_000);
    }

    #[test]
    fn test_reward_challenger_new_account() {
        let mut state = default_state(HashMap::new());
        reward_challenger(&mut state, &addr1(), 100_000).unwrap();
        assert_eq!(state.balances[&addr1()], 100_000);
    }

    #[test]
    fn test_slash_proposer_reduces_balance_and_reputation() {
        let mut state = default_state(make_balance(vec![(addr1(), 20_000_000)]));
        slash_proposer(&mut state, &addr1()).unwrap();
        assert_eq!(
            state.balances[&addr1()],
            20_000_000 - state.config.proposer_slash_penalty
        );
        assert_eq!(state.reputations[&addr1()], 0); // 0 stays 0 with saturating_sub
    }

    #[test]
    fn test_slash_proposer_saturates_balance_to_zero() {
        let mut state = default_state(make_balance(vec![(addr1(), 100)]));
        slash_proposer(&mut state, &addr1()).unwrap();
        assert_eq!(state.balances[&addr1()], 0);
    }

    #[test]
    fn test_apply_verdict_valid_slashes_challenger() {
        let challenger = addr1();
        let mut state = default_state(make_balance(vec![(challenger, 50_000)]));
        let block = default_block(addr2());
        let challenge = default_challenge(challenger, 10_000);
        apply_verdict(Verdict::Valid, &block, &challenge, &mut state).unwrap();
        assert_eq!(state.balances[&challenger], 40_000);
    }

    #[test]
    fn test_apply_verdict_invalid_slashes_proposer_rewards_challenger() {
        let proposer = addr1();
        let challenger = addr2();
        let mut state = default_state(make_balance(vec![
            (proposer, 20_000_000),
            (challenger, 50_000),
        ]));
        let block = default_block(proposer);
        let challenge = default_challenge(challenger, 10_000);
        apply_verdict(Verdict::InvalidDag, &block, &challenge, &mut state).unwrap();
        assert_eq!(
            state.balances[&proposer],
            20_000_000 - state.config.proposer_slash_penalty
        );
        assert_eq!(state.balances[&challenger], 60_000);
    }
}
