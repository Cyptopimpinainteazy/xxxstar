//! SDK entry point — `UniversalContract` builder and execution planner.
//!
//! `UniversalContract` is the primary developer-facing type.  It lets callers
//! declare an ordered list of actions, compile them into an IXL program, build
//! an intent, and emit the packet commitment — all in one fluent API.

use crate::actions::Action;
use crate::compiler::{Compiler, IxlBundle};
use crate::error::UcError;
use crate::intents::IntentBuilder;
use serde::{Deserialize, Serialize};
use x3_intent::intent::ArbIntent;
use x3_packet_standard::packet::Packet;
use x3_proof::types::BlockHeight;

/// The primary SDK type.  Callers add actions, then call `.compile()` to get
/// a `CompiledContract` ready for submission to the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalContract {
    actions: Vec<Action>,
    submitter: [u8; 32],
    fee_cap: u128,
    submitted_at: BlockHeight,
    finality_window: u64,
    bond_amount: u128,
    intent_seq: u64,
}

impl UniversalContract {
    /// Create a new contract.
    pub fn new(submitter: [u8; 32]) -> Self {
        Self {
            actions: Vec::new(),
            submitter,
            fee_cap: 1_000_000,
            submitted_at: 1,
            finality_window: 100,
            bond_amount: 0,
            intent_seq: 0,
        }
    }

    pub fn fee_cap(mut self, cap: u128) -> Self {
        self.fee_cap = cap;
        self
    }

    pub fn submitted_at(mut self, block: BlockHeight) -> Self {
        self.submitted_at = block;
        self
    }

    pub fn finality_window(mut self, blocks: u64) -> Self {
        self.finality_window = blocks;
        self
    }

    pub fn bond(mut self, amount: u128) -> Self {
        self.bond_amount = amount;
        self
    }

    /// Set the intent sequence number (used as the low bits of the intent ID).
    pub fn sequence(mut self, seq: u64) -> Self {
        self.intent_seq = seq;
        self
    }

    /// Append an action.
    pub fn action(mut self, a: Action) -> Self {
        self.actions.push(a);
        self
    }

    /// Compile the contract into an `IxlProgram` and `ArbIntent`.
    ///
    /// Fails if the action list is empty, any action fails validation, or
    /// the intent builder rejects the configuration.
    pub fn compile(self) -> Result<CompiledContract, UcError> {
        let program = Compiler::compile(&self.actions)?;
        let action_count = self.actions.len() as u32;
        let has_cross_vm = self.actions.iter().any(|a| a.is_cross_vm());
        let packet_commitment = Self::compute_packet_commitment(&self.actions, &program);

        // Intent ID: mix submitter bytes with sequence to produce a unique u64.
        let id_raw = {
            use sha2::{Digest, Sha256};
            let mut h = Sha256::new();
            h.update(b"x3:uc:intent-id:v1:");
            h.update(&self.submitter);
            h.update(&self.intent_seq.to_le_bytes());
            let out = h.finalize();
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&out[..16]);
            u128::from_le_bytes(bytes)
        };

        let intent = IntentBuilder::new([id_raw], self.submitter, program.program_hash)
            .fee_cap(self.fee_cap)
            .submitted_at(self.submitted_at)
            .finality_window(self.finality_window)
            .bond(self.bond_amount)
            .build()?;

        Ok(CompiledContract {
            program,
            intent,
            action_count,
            has_cross_vm,
            packet_commitment,
        })
    }

    fn compute_packet_commitment(actions: &[Action], program: &IxlBundle) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(b"x3:uc:packet-commit:v1:");
        h.update(program.program_hash);
        for a in actions.iter().filter(|a| a.is_cross_vm()) {
            h.update(a.commitment());
        }
        let out = h.finalize();
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&out);
        arr
    }
}

/// A compiled, ready-to-submit Universal Contract.
#[derive(Debug)]
pub struct CompiledContract {
    /// The IXL bundle produced by the compiler.
    pub program: IxlBundle,
    /// The corresponding intent (in `Submitted` state).
    pub intent: ArbIntent,
    /// Total number of actions.
    pub action_count: u32,
    /// Whether any action crosses VM boundaries (requires packet routing).
    pub has_cross_vm: bool,
    /// Commitment hash covering cross-VM packet actions.
    pub packet_commitment: [u8; 32],
}

impl CompiledContract {
    /// Build a `Packet` for the first cross-VM action, if any.
    ///
    /// Used by the relayer to emit the IBC-style packet to the destination.
    pub fn to_packet(
        &self,
        dst_chain: [u8; 32],
        sequence: u64,
        timeout_block: u64,
    ) -> Option<Packet> {
        if !self.has_cross_vm {
            return None;
        }
        Packet::try_new(
            [0u8; 32], // src_chain (this chain)
            [0u8; 32], // src_port  (default port)
            dst_chain,
            [0u8; 32], // dst_port  (default port)
            sequence,
            timeout_block,
            0, // no timestamp timeout
            self.packet_commitment.to_vec(),
        )
        .ok()
    }
}

/// Re-export `AgentIdentity` for SDK consumers.
pub use x3_proof::types::AgentIdentity as SdkAgent;
