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

// Dummy hashes & execution logic
fn hash<T>(_: &T) -> Hash {
    [0u8; 32]
}

struct DagStub {
    root: Hash,
}
impl DagStub {
    fn root_hash(&self) -> Hash {
        self.root
    }
}

fn derive_action_dag(_: &[Action]) -> Result<DagStub, CourtVmError> {
    Ok(DagStub { root: [0u8; 32] })
}

fn derive_execution_order(_: &DagStub) -> Vec<Action> {
    vec![]
}

fn execute_action(_: &mut ChainState, _: &Action) -> Result<Receipt, CourtVmError> {
    Ok(Receipt::default())
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
