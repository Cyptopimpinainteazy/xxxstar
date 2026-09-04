//! Canonical cross-VM types (v1).
//!
//! This module defines the normalized, SCALE-encoded, `no_std`-safe types
//! used as the single source of truth for cross-VM calls between x3VM,
//! EVM, and SVM. It is the foundation for later patches that wire the
//! x3-kernel dispatcher, the 2PC coordinator, and the bridge settlement
//! path to a unified type.
//!
//! ## Scope of this module
//!
//! * Type definitions only.
//! * A deterministic `call_hash` helper with domain separation.
//! * Constants governing payload bounds, deadlines, proof freshness, and
//!   replay-map pruning horizons.
//!
//! Legacy types in this crate (`VmType`, `CrossVmOperation`, `CrossVmResult`)
//! are intentionally **left in place**. They will be migrated to these
//! canonical types in a subsequent, narrowly scoped patch so the
//! dispatcher/2PC/replay-storage work each lands as a separately reviewable
//! change.
//!
//! ## Invariants
//!
//! * `CrossVmCall::payload` is bounded to `MAX_CROSS_VM_PAYLOAD` bytes.
//!   Oversized payloads are rejected at construction time.
//! * `CrossVmCall::call_hash` is a domain-separated blake2_256 over the
//!   SCALE encoding of the call and the caller-supplied
//!   `source_finalized_hash`. The domain tag prevents cross-protocol hash
//!   reuse.
//! * `VmId` uses explicit discriminants so its SCALE encoding is stable
//!   across refactors. Never renumber these variants.
//!
//! ## Risk surface
//!
//! This file is on the cross-VM consensus path. Any change to the
//! encoding, the domain tag, or the set of hashed fields is a breaking
//! replay-protection change and requires a coordinated migration.

use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_io::hashing::blake2_256;
use sp_runtime::{BoundedVec, DispatchError, RuntimeDebug};
use sp_std::vec::Vec;

use frame_support::traits::ConstU32;

// ───────────────────────────── Constants ─────────────────────────────────────

/// Canonical cross-VM call protocol version.
///
/// Never decrement. Introduce a new constant (`CROSS_VM_CALL_VERSION_2`)
/// for future shapes and route on the `version` byte.
pub const CROSS_VM_CALL_VERSION: u8 = 1;

/// Maximum payload size carried inside a single `CrossVmCall` (bytes).
///
/// Chosen to be large enough for common EVM calldata and SVM instruction
/// data while small enough to bound worst-case storage/memory pressure
/// in the 2PC pipeline.
pub const MAX_CROSS_VM_PAYLOAD: u32 = 65_536;

/// Maximum horizon (in blocks) that `CrossVmCall::deadline` can be set
/// in the future relative to the block at which the call is admitted.
///
/// 100 blocks ≈ 10 minutes at a 6 s target block time. Calls with a
/// larger horizon MUST be rejected at admission to bound the 2PC
/// pipeline depth.
pub const MAX_CROSS_VM_DEADLINE_BLOCKS: u32 = 100;

/// Maximum age (in blocks, relative to the block containing the merkle
/// proof's claimed root) of a merkle inclusion proof accepted on the
/// **testnet** profile.
pub const MAX_PROOF_AGE_BLOCKS_TESTNET: u32 = 256;

/// Maximum age (in blocks) of a merkle inclusion proof accepted on the
/// **production** profile. Stricter than testnet to tighten the
/// adversarial window.
pub const MAX_PROOF_AGE_BLOCKS_PRODUCTION: u32 = 128;

/// Number of blocks after finalization during which a processed
/// `(VmId, call_hash)` entry must remain in the replay-protection map
/// before it may be pruned.
///
/// Chosen to comfortably exceed the worst-case cross-chain proof
/// propagation delay so that a legitimate late-arriving proof cannot
/// race the pruner.
pub const REPLAY_PRUNE_HORIZON_BLOCKS: u32 = 512;

/// Domain-separation tag prefixed to every `call_hash` preimage.
///
/// Including a domain tag prevents a `CrossVmCall` SCALE encoding from
/// colliding with any other SCALE-encoded structure in the system that
/// might be hashed under the same algorithm.
pub const CALL_HASH_DOMAIN: &[u8] = b"x3-cross-vm-call-v1";

/// Bounded byte payload carried by a `CrossVmCall`.
pub type CrossVmPayload = BoundedVec<u8, ConstU32<MAX_CROSS_VM_PAYLOAD>>;

// ───────────────────────────── VmId ──────────────────────────────────────────

/// Canonical identifier for each virtual machine participating in the
/// cross-VM protocol.
///
/// Explicit discriminants fix the SCALE encoding. Do not renumber.
#[derive(
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Encode,
    Decode,
    DecodeWithMemTracking,
    MaxEncodedLen,
    TypeInfo,
    RuntimeDebug,
)]
pub enum VmId {
    /// X3 native WASM VM. Acts as the cross-VM hub.
    X3Vm = 0,
    /// Ethereum Virtual Machine (Frontier pallet-evm).
    Evm = 1,
    /// Solana Virtual Machine (rbpf).
    Svm = 2,
}

impl VmId {
    /// Stable one-byte tag used in domain-separated hashing contexts.
    pub const fn tag(self) -> u8 {
        match self {
            VmId::X3Vm => 0,
            VmId::Evm => 1,
            VmId::Svm => 2,
        }
    }
}

// ───────────────────────────── CrossVmCall ───────────────────────────────────

/// A normalized cross-VM call.
///
/// A `CrossVmCall` is the input to the dispatcher. It is hashed together
/// with the source chain's finalized block hash to produce a unique
/// `call_hash` used for replay protection and receipt binding.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebug)]
pub struct CrossVmCall {
    /// Protocol version. Must equal `CROSS_VM_CALL_VERSION` at admission.
    pub version: u8,
    /// VM originating the call (where `source_finalized_hash` was
    /// finalized).
    pub source: VmId,
    /// VM where the call is to be executed.
    pub target: VmId,
    /// 4-byte selector identifying the target entrypoint.
    ///
    /// On EVM this is the function selector (first 4 bytes of
    /// `keccak256(signature)`). On SVM this identifies the instruction
    /// variant. On x3VM this identifies the host-call.
    pub selector: [u8; 4],
    /// SCALE- or ABI-encoded arguments, bounded to
    /// `MAX_CROSS_VM_PAYLOAD` bytes.
    pub payload: CrossVmPayload,
    /// Maximum gas / compute units the target VM may spend.
    pub gas_budget: u64,
    /// Monotonic nonce per `(source, target)` pair to prevent trivial
    /// duplicate submissions. Does **not** on its own guarantee replay
    /// safety — `call_hash` storage does that.
    pub nonce: u64,
    /// Absolute target-VM block number at which the call expires and
    /// must be refused admission.
    pub deadline: u64,
}

impl CrossVmCall {
    /// Construct a `CrossVmCall` from an unbounded payload, returning
    /// an error if the payload exceeds the protocol bound.
    ///
    /// Use this constructor at every protocol boundary (RPC, off-chain
    /// relayer, in-runtime dispatch) to guarantee the invariant holds
    /// before the value enters the 2PC pipeline.
    pub fn new(
        source: VmId,
        target: VmId,
        selector: [u8; 4],
        payload: Vec<u8>,
        gas_budget: u64,
        nonce: u64,
        deadline: u64,
    ) -> Result<Self, DispatchError> {
        let bounded = CrossVmPayload::try_from(payload).map_err(|_| {
            DispatchError::Other("CrossVmCall payload exceeds MAX_CROSS_VM_PAYLOAD")
        })?;
        Ok(Self {
            version: CROSS_VM_CALL_VERSION,
            source,
            target,
            selector,
            payload: bounded,
            gas_budget,
            nonce,
            deadline,
        })
    }

    /// Compute the canonical, domain-separated replay-protection hash.
    ///
    /// `call_hash = blake2_256( CALL_HASH_DOMAIN || source_finalized_hash || SCALE(self) )`
    ///
    /// The domain tag comes first so a SCALE-encoded `CrossVmCall` alone
    /// can never produce the same digest under any other domain. The
    /// source-finalized hash binds the call to a specific source-chain
    /// finalized state so the same logical call submitted against two
    /// different source histories produces two distinct `call_hash`
    /// values — desirable because those are semantically different
    /// calls.
    pub fn call_hash(&self, source_finalized_hash: &H256) -> H256 {
        let mut buf = Vec::with_capacity(CALL_HASH_DOMAIN.len() + 32 + 128);
        buf.extend_from_slice(CALL_HASH_DOMAIN);
        buf.extend_from_slice(source_finalized_hash.as_bytes());
        self.encode_to(&mut buf);
        H256::from(blake2_256(&buf))
    }

    /// Reject admission if the version byte does not match the current
    /// protocol version. Callers must invoke this before admission.
    pub fn ensure_current_version(&self) -> Result<(), DispatchError> {
        if self.version == CROSS_VM_CALL_VERSION {
            Ok(())
        } else {
            Err(DispatchError::Other("CrossVmCall version mismatch"))
        }
    }
}

// ───────────────────────────── CrossVmReceipt ────────────────────────────────

/// Canonical outcome of a single cross-VM call execution.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebug)]
pub enum CrossVmStatus {
    /// Target VM executed the call to completion and committed its
    /// state changes.
    Success,
    /// Target VM reverted — the optional payload carries the revert
    /// reason in VM-native form (already size-bounded upstream).
    Reverted,
    /// Target VM consumed all of `gas_budget` without completing.
    OutOfGas,
    /// Call admission refused: the call's `deadline` has already
    /// passed relative to the target-VM block number.
    DeadlineExpired,
    /// Call admission refused: this `(VmId, call_hash)` has already
    /// been processed.
    ReplayRejected,
    /// Non-VM error inside the dispatcher itself (bug, transient
    /// failure, or consensus-level refusal).
    InternalError,
}

/// Receipt produced by the dispatcher after attempting a cross-VM call.
///
/// `logs` is unbounded here because receipts cross the off-chain /
/// RPC boundary. Any pallet that stores receipts on-chain is
/// responsible for imposing its own storage-level bound.
#[derive(Clone, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo, RuntimeDebug)]
pub struct CrossVmReceipt {
    /// Call hash from `CrossVmCall::call_hash` — binds the receipt to
    /// the call's replay-protection entry.
    pub call_hash: H256,
    /// Source-chain state root at the block `source_finalized_hash`
    /// was finalized.
    pub source_state_root: H256,
    /// Target-VM state root after (attempting) execution.
    pub target_state_root: H256,
    /// Outcome classification.
    pub status: CrossVmStatus,
    /// Gas / compute units actually consumed.
    pub gas_used: u64,
    /// VM-native log entries produced during execution.
    pub logs: Vec<Vec<u8>>,
}

// ───────────────────────────── Tests ─────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use parity_scale_codec::{Decode, Encode};

    fn finalized_hash(byte: u8) -> H256 {
        H256::from([byte; 32])
    }

    fn sample_call(nonce: u64) -> CrossVmCall {
        CrossVmCall::new(
            VmId::X3Vm,
            VmId::Evm,
            [0xde, 0xad, 0xbe, 0xef],
            b"payload".to_vec(),
            1_000_000,
            nonce,
            42,
        )
        .expect("payload is within bound")
    }

    #[test]
    fn vmid_discriminants_are_stable() {
        assert_eq!(VmId::X3Vm.tag(), 0);
        assert_eq!(VmId::Evm.tag(), 1);
        assert_eq!(VmId::Svm.tag(), 2);

        // SCALE enum discriminant == tag for simple enums.
        assert_eq!(VmId::X3Vm.encode(), vec![0u8]);
        assert_eq!(VmId::Evm.encode(), vec![1u8]);
        assert_eq!(VmId::Svm.encode(), vec![2u8]);
    }

    #[test]
    fn cross_vm_call_roundtrip() {
        let call = sample_call(7);
        let encoded = call.encode();
        let decoded = CrossVmCall::decode(&mut &encoded[..]).expect("decode");
        assert_eq!(call, decoded);
    }

    #[test]
    fn new_rejects_oversized_payload() {
        let oversized = vec![0u8; (MAX_CROSS_VM_PAYLOAD as usize) + 1];
        let err = CrossVmCall::new(VmId::X3Vm, VmId::Evm, [0; 4], oversized, 1, 1, 1);
        assert!(err.is_err(), "expected oversized-payload rejection");
    }

    #[test]
    fn new_accepts_maximum_payload() {
        let at_bound = vec![0u8; MAX_CROSS_VM_PAYLOAD as usize];
        assert!(CrossVmCall::new(VmId::X3Vm, VmId::Evm, [0; 4], at_bound, 1, 1, 1).is_ok());
    }

    #[test]
    fn ensure_current_version_checks_byte() {
        let mut call = sample_call(1);
        assert!(call.ensure_current_version().is_ok());
        call.version = CROSS_VM_CALL_VERSION.wrapping_add(1);
        assert!(call.ensure_current_version().is_err());
    }

    #[test]
    fn call_hash_is_deterministic() {
        let call = sample_call(1);
        let source = finalized_hash(0xAA);
        let h1 = call.call_hash(&source);
        let h2 = call.call_hash(&source);
        assert_eq!(
            h1, h2,
            "call_hash must be deterministic for identical inputs"
        );
    }

    #[test]
    fn call_hash_binds_source_finalized_hash() {
        let call = sample_call(1);
        let h1 = call.call_hash(&finalized_hash(0x01));
        let h2 = call.call_hash(&finalized_hash(0x02));
        assert_ne!(
            h1, h2,
            "call_hash must differ when source_finalized_hash differs"
        );
    }

    #[test]
    fn call_hash_binds_every_field() {
        let source = finalized_hash(0xCC);
        let base = sample_call(1);
        let base_hash = base.call_hash(&source);

        // nonce
        let mut c = base.clone();
        c.nonce = 2;
        assert_ne!(
            base_hash,
            c.call_hash(&source),
            "nonce change must change hash"
        );

        // source VM
        let mut c = base.clone();
        c.source = VmId::Svm;
        assert_ne!(
            base_hash,
            c.call_hash(&source),
            "source change must change hash"
        );

        // target VM
        let mut c = base.clone();
        c.target = VmId::X3Vm;
        assert_ne!(
            base_hash,
            c.call_hash(&source),
            "target change must change hash"
        );

        // selector
        let mut c = base.clone();
        c.selector = [0, 0, 0, 0];
        assert_ne!(
            base_hash,
            c.call_hash(&source),
            "selector change must change hash"
        );

        // payload
        let c = CrossVmCall::new(
            base.source,
            base.target,
            base.selector,
            b"different".to_vec(),
            base.gas_budget,
            base.nonce,
            base.deadline,
        )
        .unwrap();
        assert_ne!(
            base_hash,
            c.call_hash(&source),
            "payload change must change hash"
        );

        // gas_budget
        let mut c = base.clone();
        c.gas_budget = base.gas_budget + 1;
        assert_ne!(
            base_hash,
            c.call_hash(&source),
            "gas_budget change must change hash"
        );

        // deadline
        let mut c = base.clone();
        c.deadline = base.deadline + 1;
        assert_ne!(
            base_hash,
            c.call_hash(&source),
            "deadline change must change hash"
        );

        // version
        let mut c = base.clone();
        c.version = CROSS_VM_CALL_VERSION.wrapping_add(1);
        assert_ne!(
            base_hash,
            c.call_hash(&source),
            "version change must change hash"
        );
    }

    #[test]
    fn call_hash_uses_domain_separation() {
        // Hashing a raw SCALE encoding without the domain tag must not
        // collide with the canonical call_hash for the same input. This
        // is what protects us from cross-protocol hash collisions.
        let call = sample_call(9);
        let source = finalized_hash(0x55);
        let canonical = call.call_hash(&source);

        let mut without_domain = Vec::new();
        without_domain.extend_from_slice(source.as_bytes());
        call.encode_to(&mut without_domain);
        let naive = H256::from(blake2_256(&without_domain));

        assert_ne!(canonical, naive, "domain tag must alter the digest");
    }

    #[test]
    fn constants_are_shipping_profile() {
        // Lock in the testnet/production profile so changes require a
        // deliberate edit that a reviewer will catch.
        assert_eq!(CROSS_VM_CALL_VERSION, 1);
        assert_eq!(MAX_CROSS_VM_PAYLOAD, 65_536);
        assert_eq!(MAX_CROSS_VM_DEADLINE_BLOCKS, 100);
        assert_eq!(MAX_PROOF_AGE_BLOCKS_TESTNET, 256);
        assert_eq!(MAX_PROOF_AGE_BLOCKS_PRODUCTION, 128);
        assert_eq!(REPLAY_PRUNE_HORIZON_BLOCKS, 512);
        assert_eq!(CALL_HASH_DOMAIN, b"x3-cross-vm-call-v1");
    }

    #[test]
    fn receipt_encoding_roundtrips() {
        let receipt = CrossVmReceipt {
            call_hash: finalized_hash(0x11),
            source_state_root: finalized_hash(0x22),
            target_state_root: finalized_hash(0x33),
            status: CrossVmStatus::Success,
            gas_used: 12_345,
            logs: vec![b"log-0".to_vec(), b"log-1".to_vec()],
        };
        let encoded = receipt.encode();
        let decoded = CrossVmReceipt::decode(&mut &encoded[..]).expect("decode");
        assert_eq!(receipt, decoded);
    }

    #[test]
    fn all_status_variants_roundtrip() {
        for status in [
            CrossVmStatus::Success,
            CrossVmStatus::Reverted,
            CrossVmStatus::OutOfGas,
            CrossVmStatus::DeadlineExpired,
            CrossVmStatus::ReplayRejected,
            CrossVmStatus::InternalError,
        ] {
            let encoded = status.encode();
            let decoded = CrossVmStatus::decode(&mut &encoded[..]).expect("decode");
            assert_eq!(status, decoded);
        }
    }
}
