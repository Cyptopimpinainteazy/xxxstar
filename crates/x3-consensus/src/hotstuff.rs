//! X3 HotStuff Consensus Engine
//!
//! Chained HotStuff BFT with immediate finality (2/3 quorum certificates).
//! No probabilistic finality, no reorgs. Strict determinism.
//!
//! References:
//! - Yin et al., "HotStuff: BFT Consensus in the Lens of Blockchain" (2019)
//! - Diem/BetterCoin: linear view-change, pipelined 3-phase commit

use serde::{Deserialize, Serialize};
use sp_core::hashing::blake2_256;
use std::collections::HashSet;

/// 256-bit hash type
pub type Hash = [u8; 32];
/// 32-byte address type
pub type Address = [u8; 32];
/// Block height/epoch number
pub type BlockHeight = u64;
/// View/round number
pub type View = u64;

/// Quorum Certificate - proof that ≥2/3 validators accepted a block
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QC {
    /// View/round number when this QC was produced
    pub view: View,
    /// Block hash that was certified
    pub block_hash: Hash,
    /// Aggregate signature from the quorum (BLS or similar)
    pub aggregate_signature: Vec<u8>,
    /// List of validator indices that signed (for accountability)
    pub signer_indices: Vec<u32>,
    /// Validator set hash at time of QC
    pub validator_set_hash: Hash,
}

impl QC {
    /// Create a new QC
    pub fn new(
        view: View,
        block_hash: Hash,
        aggregate_signature: Vec<u8>,
        signer_indices: Vec<u32>,
        validator_set_hash: Hash,
    ) -> Self {
        Self {
            view,
            block_hash,
            aggregate_signature,
            signer_indices,
            validator_set_hash,
        }
    }
}

/// Block header with consensus metadata
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Hash of parent block
    pub parent_hash: Hash,
    /// Block height
    pub height: BlockHeight,
    /// View/round number
    pub round: View,
    /// Timestamp (milliseconds)
    pub timestamp: u64,
    /// Hash of current validator set
    pub validator_set_hash: Hash,
    /// Quorum certificate for this block (must be present for finality)
    pub qc: QC,
    /// Proposer/leader for this block
    pub proposer: Address,
    /// Round-specific random beacon (optional, for randomness)
    pub randomness: Option<Hash>,
}

impl BlockHeader {
    /// Compute hash of the header (excluding qc signatures to avoid recursion)
    pub fn hash(&self) -> Result<Hash, HashError> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&self.parent_hash);
        payload.extend_from_slice(&self.height.to_le_bytes());
        payload.extend_from_slice(&self.round.to_le_bytes());
        payload.extend_from_slice(&self.timestamp.to_le_bytes());
        payload.extend_from_slice(&self.validator_set_hash);
        // QC: include block hash and view but not the signature itself
        payload.extend_from_slice(&self.qc.block_hash);
        payload.extend_from_slice(&self.qc.view.to_le_bytes());
        payload.extend_from_slice(&self.qc.validator_set_hash);
        payload.extend_from_slice(&self.proposer);
        if let Some(ref rand) = self.randomness {
            payload.extend_from_slice(rand);
        }
        Ok(blake2_256(&payload))
    }
}

/// Slashing event recorded in a block
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashingEvent {
    /// Offender address
    pub offender: Address,
    /// Reason code (e.g., "InvalidBlock", "DoubleSign")
    pub reason: SlashReason,
    /// Amount slashed (in native token units)
    pub amount: u128,
    /// Block height where slash occurred
    pub height: BlockHeight,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlashReason {
    InvalidBlock,
    InvalidSignature,
    DoubleSign,
    InvalidExecution,
    InvalidDag,
    InvalidOrder,
    ReceiptMismatch,
    ResourceMismatch,
    ProposerEquivocation,
    AgentFraud,
    InvalidChallenge,
}

/// Full block structure
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    /// Committed actions/tasks (hashes only, payload off-chain)
    pub actions: Vec<ActionCommitment>,
    /// Merkle root of action dependency DAG
    pub action_dag_root: Hash,
    /// Hash of deterministic execution order
    pub execution_order_hash: Hash,
    /// State root before execution
    pub state_root_pre: Hash,
    /// State root after execution
    pub state_root_post: Hash,
    /// Merkle root of all receipts
    pub receipts_root: Hash,
    /// Slashing events in this block
    pub slashing_events: Vec<SlashingEvent>,
}

/// Action commitment: hash of task/transaction data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionCommitment {
    pub id: u64,
    pub hash: Hash,
    /// Optional resource bounds (CPU, GPU, memory) if known at commitment
    pub resource_bounds: Option<ResourceVector>,
}

/// Chain state (mutable)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChainState {
    /// Hash of last applied block
    pub last_block: Hash,
    /// Current state root (Merkle root of all accounts/storage)
    pub state_root: Hash,
    /// Current validator set
    pub validators: Vec<Validator>,
    /// Current view/round
    pub current_view: View,
    /// Current block height
    pub height: BlockHeight,
    /// Locked block (from safe/decide) for linear consistency
    pub locked_block: Option<Block>,
    /// Executed blocks (for state transitions)
    pub executed_blocks: Vec<Hash>,
}

impl ChainState {
    pub fn new(initial_state_root: Hash, validators: Vec<Validator>) -> Self {
        Self {
            last_block: [0u8; 32], // genesis parent
            state_root: initial_state_root,
            validators,
            current_view: 0,
            height: 0,
            locked_block: None,
            executed_blocks: Vec::new(),
        }
    }

    pub fn state_root(&self) -> Hash {
        self.state_root
    }

    /// Advance height after block application
    pub fn advance_height(&mut self) {
        self.height += 1;
    }

    /// Advance view (round)
    pub fn advance_view(&mut self) {
        self.current_view += 1;
    }
}

/// Validator info
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Validator {
    pub address: Address,
    pub stake: u128,
    /// Index in the validator set array
    pub index: u32,
}

/// Resource usage vector (multi-dimensional accounting)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceVector {
    pub cpu_cycles: u64,
    pub gpu_cycles: u64,
    pub memory_bytes: u64,
    pub io_ops: u64,
    pub storage_reads: u64,
    pub storage_writes: u64,
}

/// Hash error type
#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("Hash computation failed")]
    Failed,
}

/// Consensus errors
#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Invalid block parent")]
    InvalidParent,
    #[error("Quorum certificate verification failed")]
    InvalidQC,
    #[error("Validator set mismatch")]
    ValidatorSetMismatch,
    #[error("Proposer not in current set")]
    UnknownProposer,
    #[error("View/round mismatch")]
    ViewMismatch,
    #[error("Block already finalized")]
    AlreadyFinalized,
    #[error("Execution failed")]
    ExecutionFailure,
    #[error("State root mismatch")]
    StateMismatch,
    #[error("DAG validation failed")]
    InvalidDag,
    #[error("Execution order mismatch")]
    InvalidOrder,
    #[error("Timeout waiting for proposals")]
    Timeout,
}

/// Proposer selection: round-robin weighted by stake
pub fn select_proposer(view: View, validators: &[Validator]) -> Address {
    if validators.is_empty() {
        return [0u8; 32];
    }
    // Simple weighted round-robin: total stake determines cycle length
    let total_stake: u128 = validators.iter().map(|v| v.stake).sum();
    if total_stake == 0 {
        return validators[0].address.clone();
    }
    // Use view modulo number of validators for basic selection
    // In production, use Verifiable Random Function (VRF) for unpredictability
    let idx = (view as usize) % validators.len();
    validators[idx].address.clone()
}

/// Verify QC signature threshold: ≥2/3 of total stake
pub fn verify_quorum_threshold(
    qc: &QC,
    validators: &[Validator],
    validator_set_hash: Hash,
) -> Result<(), ConsensusError> {
    // Verify validator set hash matches
    let computed_set_hash = compute_validator_set_hash(validators);
    if computed_set_hash != validator_set_hash {
        return Err(ConsensusError::ValidatorSetMismatch);
    }
    if qc.validator_set_hash != validator_set_hash {
        return Err(ConsensusError::InvalidQC);
    }

    // Identify which validators are in the committee
    let mut total_stake = 0u128;
    let mut signer_stake = 0u128;
    for (idx, validator) in validators.iter().enumerate() {
        total_stake += validator.stake;
        if qc.signer_indices.contains(&(idx as u32)) {
            signer_stake += validator.stake;
        }
    }

    // Check 2/3 threshold by stake weight
    // threshold = ceil(2 * total_stake / 3)
    let threshold = (2 * total_stake + 2) / 3;
    if signer_stake >= threshold {
        Ok(())
    } else {
        Err(ConsensusError::InvalidQC)
    }
}

/// Compute validator set hash (for QC binding)
pub fn compute_validator_set_hash(validators: &[Validator]) -> Hash {
    let mut data = Vec::new();
    for v in validators {
        data.extend_from_slice(&v.address);
        data.extend_from_slice(&v.stake.to_le_bytes());
    }
    blake2_256(&data)
}

/// Verify block signature by proposer (pre-vote/pre-commit)
pub fn verify_block_signature(
    block_hash: Hash,
    signature: &[u8],
    proposer: &Address,
    validators: &[Validator],
) -> Result<(), ConsensusError> {
    // Find proposer
    let proposer_info = validators
        .iter()
        .find(|v| v.address == *proposer)
        .ok_or(ConsensusError::UnknownProposer)?;

    // In production: use actual signature verification (BLS/Ed25519/Schnorr)
    // Here: check placeholder
    if signature.is_empty() {
        return Err(ConsensusError::InvalidQC);
    }
    // Signature verification logic would go here using proposer's public key
    Ok(())
}

/// Core Block verification (independent of execution)
pub fn verify_block_header(
    header: &BlockHeader,
    validators: &[Validator],
    expected_height: BlockHeight,
) -> Result<(), ConsensusError> {
    // Height must match
    if header.height != expected_height {
        return Err(ConsensusError::ViewMismatch);
    }

    // Proposer must be in the current validator set
    if !validators.iter().any(|v| v.address == header.proposer) {
        return Err(ConsensusError::UnknownProposer);
    }

    // Validator set hash must match
    let computed_set_hash = compute_validator_set_hash(validators);
    if header.validator_set_hash != computed_set_hash {
        return Err(ConsensusError::ValidatorSetMismatch);
    }

    // QC must be valid
    verify_quorum_threshold(&header.qc, validators, header.validator_set_hash)?;

    // Check that the block hash inside the QC matches the parent or this block appropriately
    // In HotStuff, a block's QC certifies either the parent (for prepare) or the block itself (for pre-commit/commit)
    // Simplified: we require that QC.block_hash equals the block's parent hash (or for committed block, equals self)
    // This is simplified; real HotStuff has 3-phase locking logic.
    let qc_target_ok =
        header.qc.block_hash == header.parent_hash || header.qc.block_hash == header.hash()?;
    if !qc_target_ok {
        return Err(ConsensusError::InvalidQC);
    }

    Ok(())
}

/// Derive DAG from action commitments
fn derive_action_dag(actions: &[ActionCommitment]) -> Result<ActionDag, ConsensusError> {
    let mut dag = ActionDag::new();
    for action in actions {
        dag.add_node(action.id, action.hash)?;
    }
    // Edges would be computed based on declared dependencies; here we assume none for simplicity.
    dag.compute_root()
}

/// Deterministic execution order from DAG (topological sort, ties broken by ID)
fn derive_execution_order(dag: &ActionDAG) -> Vec<ActionCommitment> {
    let order_ids = dag.topological_order();
    // In real implementation, look up actions by ID
    // Here we just return as-is ( stub: actions already in order)
    Vec::new()
}

/// Compute hash of any serializable type
fn hash<T: Serialize + ?Sized>(value: &T) -> Result<Hash, HashError> {
    let serialized = bincode::serialize(value).map_err(|_| HashError::Failed)?;
    Ok(blake2_256(&serialized))
}

/// Simple Action DAG structure
#[derive(Debug, Clone)]
struct ActionDAG {
    nodes: Vec<(u64, Hash)>,
    edges: Vec<(u64, u64)>,
    root: Option<Hash>,
}

impl ActionDAG {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            root: None,
        }
    }

    fn add_node(&mut self, id: u64, hash: Hash) -> Result<(), ConsensusError> {
        if self.nodes.iter().any(|(nid, _)| *nid == id) {
            return Err(ConsensusError::InvalidDag);
        }
        self.nodes.push((id, hash));
        Ok(())
    }

    fn add_edge(&mut self, from: u64, to: u64) {
        self.edges.push((from, to));
    }

    fn compute_root(&mut self) -> Result<ActionDAG, ConsensusError> {
        // Simple Merkle root of sorted node hashes
        let mut sorted_nodes: Vec<Hash> = self.nodes.iter().map(|(_, h)| *h).collect();
        sorted_nodes.sort(); // deterministic sort by hash
        self.root = Some(compute_merkle_root(&sorted_nodes));
        Ok(self.clone())
    }

    fn root_hash(&self) -> Hash {
        self.root.unwrap_or([0u8; 32])
    }

    fn topological_sort(&self) -> Vec<u64> {
        // Build adjacency list
        let mut adj = std::collections::HashMap::new();
        let mut indegree = std::collections::HashMap::new();
        for (id, _) in &self.nodes {
            adj.insert(*id, Vec::new());
            indegree.insert(*id, 0);
        }
        for (from, to) in &self.edges {
            if let Some(list) = adj.get_mut(from) {
                list.push(*to);
            }
            if let Some(deg) = indegree.get_mut(to) {
                *deg += 1;
            }
        }

        // Kahn's algorithm with deterministic tie-breaking
        let mut ready: Vec<u64> = indegree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(&id, _)| id)
            .collect();
        ready.sort(); // deterministic order

        let mut order = Vec::new();
        while let Some(node) = ready.pop() {
            order.push(node);
            if let Some(neighbors) = adj.get(&node) {
                for &neighbor in neighbors {
                    if let Some(deg) = indegree.get_mut(&neighbor) {
                        *deg -= 1;
                        if *deg == 0 {
                            ready.push(neighbor);
                        }
                    }
                }
            }
            ready.sort(); // maintain determinism
        }

        order
    }
}

/// Compute Merkle root of a list of hashes
fn compute_merkle_root(hashes: &[Hash]) -> Hash {
    if hashes.is_empty() {
        return [0u8; 32];
    }
    if hashes.len() == 1 {
        return hashes[0];
    }
    let mut level = hashes.to_vec();
    while level.len() > 1 {
        let mut next = Vec::new();
        for chunk in level.chunks(2) {
            let mut combined = Vec::new();
            combined.extend_from_slice(&chunk[0]);
            if chunk.len() == 2 {
                combined.extend_from_slice(&chunk[1]);
            } else {
                // duplicate last if odd
                combined.extend_from_slice(&chunk[0]);
            }
            next.push(blake2_256(&combined));
        }
        level = next;
    }
    level[0]
}

/// Execute a single action (stub - integrates with X3 VM)
fn execute_action(
    state: &mut ChainState,
    action: &ActionCommitment,
) -> Result<Hash, ConsensusError> {
    // In full implementation: dispatch to VM adapter or agent execution
    // Produces a receipt whose hash is returned
    let receipt_hash = hash(&action)?;
    Ok(receipt_hash)
}

/// Apply slashing events to state
fn apply_slashing(state: &mut ChainState, events: &[SlashingEvent]) -> Result<(), ConsensusError> {
    // In full implementation: reduce stakes, update validator weights, record slash event
    for event in events {
        // Deduct from validator's stake, maybe remove if zero
        if let Some(v) = state
            .validators
            .iter_mut()
            .find(|v| v.address == event.offender)
        {
            v.stake = v.stake.saturating_sub(event.amount);
        }
    }
    Ok(())
}

/// Block application - the state transition function (from design booklet)
pub fn apply_block(
    state: &mut ChainState,
    block: &Block,
    verify_execution: bool,
) -> Result<(), ConsensusError> {
    // 1. Verify block header and QC
    verify_block_header(&block.header, &state.validators, state.height)?;

    ensure!(
        block.header.parent_hash == state.last_block,
        ConsensusError::InvalidParent
    );

    // 2. Check action commitments & derive DAG
    let dag = derive_action_dag(&block.actions)?;
    if dag.root_hash() != block.action_dag_root {
        return Err(ConsensusError::InvalidDag);
    }
    let order = derive_execution_order(&dag);
    let order_hash = hash(&order)?;
    if order_hash != block.execution_order_hash {
        return Err(ConsensusError::InvalidOrder);
    }

    // 3. Execute actions in order (or verify if replayed by Court)
    let mut receipts = Vec::new();
    let mut post_state = if verify_execution {
        Some(state.state_root)
    } else {
        None
    };
    for action in order {
        let receipt = execute_action(&mut ChainState { ..*state }, &action)?; // pure function
        receipts.push(receipt);
    }

    // 4. Verify receipts root (if provided)
    let computed_receipts_root = compute_merkle_root(&receipts);
    if computed_receipts_root != block.receipts_root {
        return Err(ConsensusError::ExecutionFailure);
    }

    // 5. Verify post-state root
    if let Some(expected_post) = block.state_root_post {
        if post_state.unwrap_or([0u8; 32]) != expected_post {
            return Err(ConsensusError::StateMismatch);
        }
    }

    // 6. Apply slashing events
    apply_slashing(state, &block.slashing_events)?;

    // 7. Update chain state
    state.last_block = hash(block)?;
    state.advance_height();
    state.executed_blocks.push(state.last_block);
    state.state_root = block.state_root_post.unwrap_or([0u8; 32]);

    // 8. Update locked block (for HotStuff prepare/commit phases)
    if !block.execution_order_hash.is_empty() {
        state.locked_block = Some(block.clone());
    }

    Ok(())
}

/// Ensure macro for simple error propagation
fn ensure(cond: bool, err: ConsensusError) -> Result<(), ConsensusError> {
    if cond {
        Ok(())
    } else {
        Err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_validator_set() -> Vec<Validator> {
        vec![
            Validator {
                address: [1u8; 32],
                stake: 1000,
                index: 0,
            },
            Validator {
                address: [2u8; 32],
                stake: 1000,
                index: 1,
            },
            Validator {
                address: [3u8; 32],
                stake: 1000,
                index: 2,
            },
        ]
    }

    fn make_block(height: u64, parent: Hash, proposer: Address) -> Block {
        let qc = QC {
            view: height.saturating_sub(1),
            block_hash: parent,
            aggregate_signature: vec![0u8; 64],
            signer_indices: vec![0, 1, 2],
            validator_set_hash: compute_validator_set_hash(&test_validator_set()),
        };
        let header = BlockHeader {
            parent_hash: parent,
            height,
            round: height,
            timestamp: 0,
            validator_set_hash: qc.validator_set_hash,
            qc,
            proposer,
            randomness: None,
        };
        Block {
            header,
            actions: Vec::new(),
            action_dag_root: [0u8; 32],
            execution_order_hash: [0u8; 32],
            state_root_pre: [0u8; 32],
            state_root_post: Some([0u8; 32]),
            receipts_root: [0u8; 32],
            slashing_events: Vec::new(),
        }
    }

    #[test]
    fn test_proposer_selection() {
        let validators = test_validator_set();
        let p0 = select_proposer(0, &validators);
        let p1 = select_proposer(1, &validators);
        assert_eq!(p0, validators[0].address);
        assert_eq!(p1, validators[1].address);
    }

    #[test]
    fn test_quorum_threshold() {
        let validators = test_validator_set();
        let qc = QC {
            view: 1,
            block_hash: [0u8; 32],
            aggregate_signature: vec![],
            signer_indices: vec![0, 1], // 2 out of 3 = 66.6% -> passes
            validator_set_hash: compute_validator_set_hash(&validators),
        };
        assert!(verify_quorum_threshold(&qc, &validators, qc.validator_set_hash).is_ok());
    }

    #[test]
    fn test_apply_block_success() {
        let mut state = ChainState::new([9u8; 32], test_validator_set());
        let parent_hash = [0u8; 32];
        let block = make_block(1, parent_hash, [1u8; 32]);
        state.last_block = parent_hash;
        state.state_root = [0u8; 32];

        assert!(apply_block(&mut state, &block, true).is_ok());
        assert_eq!(state.height, 1);
        assert_eq!(state.last_block, block.header.hash().unwrap());
    }

    #[test]
    fn test_apply_block_invalid_parent() {
        let mut state = ChainState::new([9u8; 32], test_validator_set());
        let block = make_block(1, [1u8; 32], [1u8; 32]);
        state.last_block = [0u8; 32]; // different parent

        assert!(matches!(
            apply_block(&mut state, &block, true),
            Err(ConsensusError::InvalidParent)
        ));
    }
}
