//! Test-only helpers for the x3-kernel pallet.
//!
//! A comit payload **is the artifact its adapter executes** (X3-LANG-004): EVM bytecode for
//! `T::EvmAdapter`, an eBPF program for `T::SvmAdapter`, X3BC bytecode for `T::X3Adapter`. The
//! kernel asks the adapter that will do the work whether the payload is valid; it does not decode
//! a payload as a SCALE-encoded `Packet`, because a packet is a semantic operation naming a target
//! contract or program, and the adapter interface here is `(payload, limit) -> receipt` with
//! nowhere to put that target. Every adapter therefore refuses a packet by name.
//!
//! The wrappers below take a test's "intent bytes" and turn them into a payload of the right
//! *kind* for that slot, so a fixture cannot be a shape the live chain refuses. Empty intent bytes
//! stay empty — the kernel already treats an empty payload as "no side effect for this VM".
//!
//! `compute_prepare_root` is the pallet's canonical algorithm
//! (blake2_256 over the concatenation of every input). Tests must
//! feed the **wrapped** bytes to `compute_prepare_root` so the
//! prepare_root matches what the kernel computes internally.

use parity_scale_codec::Encode;
use sp_std::vec::Vec;
use x3_packet_schema::{Packet, X3VmPacket};

/// `MOV64_IMM` — write a 32-bit immediate into a 64-bit register; opcode class 0x07 (`BPF_ALU64`).
const SVM_MOV64_IMM: u8 = 0xb7;
/// Register nibble for `r1`. The intent goes into `r1`, never `r0`: the interpreter follows
/// Solana's convention that a program exits with `r0` as its status and `r0 != 0` is a failure.
const SVM_R1: u8 = 0x01;
/// `EXIT` — return `r0`; opcode class 0x05 (`BPF_JMP`).
const SVM_EXIT: [u8; 8] = [0x95, 0, 0, 0, 0, 0, 0, 0];

/// The EVM payload for a test: **bytecode**, the artifact `T::EvmAdapter` executes.
///
/// `PUSH1 0; PUSH1 0; RETURN` halts successfully with empty return data, and the test's intent
/// follows it: unreachable, but it keeps the fixture deterministic, content-addressable, and its
/// length tracking the intent. The first byte is always `0x60` and never a `Packet` discriminant,
/// and `mini_evm::validate_evm` (what `WasmEvmAdapter::validate` runs) accepts it. Before
/// X3-LANG-004 this returned `Packet::Evm(EvmPacket::Call)`: a payload the kernel accepted and
/// every adapter refuses, which is why the pallet's own suite could not see the disagreement.
/// Empty input stays empty so the kernel's empty-payload fast path still fires.
pub fn wrap_evm_payload(intent: &[u8]) -> Vec<u8> {
    if intent.is_empty() {
        return Vec::new();
    }
    let mut code = Vec::with_capacity(intent.len() + 5);
    code.extend_from_slice(&[0x60, 0x00, 0x60, 0x00, 0xf3]);
    code.extend_from_slice(intent);
    code
}

/// The SVM payload for a test: an **eBPF program**, the artifact `T::SvmAdapter` executes.
///
/// The intent is packed four bytes at a time into `MOV64_IMM r1, imm` instructions, then `r0` is
/// zeroed and the program ends in `EXIT` — so it is a whole number of 8-byte instructions carrying
/// only opcode classes `x3_svm_integration::interp_validate_program` accepts (the same validator the
/// wasm adapter's `validate` runs), and it *succeeds* when run. Empty input stays empty.
pub fn wrap_svm_payload(intent: &[u8]) -> Vec<u8> {
    if intent.is_empty() {
        return Vec::new();
    }
    let mut program = Vec::with_capacity(intent.len() + 16);
    for chunk in intent.chunks(4) {
        let mut insn = [0u8; 8];
        insn[0] = SVM_MOV64_IMM;
        insn[1] = SVM_R1;
        insn[4..4 + chunk.len()].copy_from_slice(chunk);
        program.extend_from_slice(&insn);
    }
    // `r0 = 0`: the exit status. A fixture that left the intent in `r0` would be a *failed* program.
    program.extend_from_slice(&[SVM_MOV64_IMM, 0, 0, 0, 0, 0, 0, 0]);
    program.extend_from_slice(&SVM_EXIT);
    program
}

/// Wrap a test's X3VM "intent bytes" as a valid SCALE-encoded
/// `Packet::X3Vm(X3VmPacket::Transfer)`. Empty input stays empty.
///
/// The recipient field is padded with zero bytes so the encoded form
/// is at least 30 bytes — the kernel's `deserialize_packet` rejects
/// anything shorter as `PayloadTooSmall`.
pub fn wrap_x3_payload(intent: &[u8]) -> Vec<u8> {
    if intent.is_empty() {
        return Vec::new();
    }
    // Build a recipient long enough that the SCALE-encoded packet
    // is >= 30 bytes. The outer Packet enum discriminant (1) +
    // X3VmPacket::Transfer fields (1+1+4+16) + compact-int length
    // prefix (1) = 24 bytes; we want the recipient alone to be
    // at least 7 bytes, padded with zeros after the intent.
    let mut recipient = intent.to_vec();
    while recipient.len() < 8 {
        recipient.push(0);
    }
    Packet::X3Vm(X3VmPacket::Transfer {
        from_domain: 0,
        to_domain: 1,
        asset_id: 0,
        amount: 0,
        recipient,
    })
    .encode()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet_adapters::deserialize_packet;

    /// The EVM fixture is bytecode the chain's own validator accepts, and it is not a packet.
    ///
    /// Both halves matter. `mini_evm::validate_evm` is what `WasmEvmAdapter::validate` runs, so a
    /// payload it rejected could not reach execution on a live chain; and `payload_is_packet`
    /// being false is the X3-LANG-004 rule — a payload is the artifact the adapter executes, and
    /// the fixture must not be a shape every adapter refuses.
    #[test]
    fn wrapped_evm_payload_is_bytecode_not_a_packet() {
        let bytes = wrap_evm_payload(&[0xaa, 0xbb, 0xcc]);
        assert_eq!(
            bytes[..5],
            [0x60, 0x00, 0x60, 0x00, 0xf3],
            "the fixture has to be EVM bytecode"
        );
        assert!(
            !crate::packet_adapters::payload_is_packet(&bytes),
            "an EVM payload is the artifact the adapter executes, never a SCALE-encoded Packet"
        );
        x3_evm_integration::mini_evm::validate_evm(&bytes)
            .expect("the chain's own EVM validator has to accept the fixture");
    }

    /// The SVM fixture is an eBPF program the chain's own validator accepts, and it is not a packet.
    #[test]
    fn wrapped_svm_payload_is_a_program_not_a_packet() {
        let bytes = wrap_svm_payload(&[0xaa, 0xbb, 0xcc]);
        assert_eq!(bytes[0], SVM_MOV64_IMM, "the fixture has to be eBPF");
        assert_eq!(
            &bytes[bytes.len() - 8..],
            &SVM_EXIT,
            "an eBPF program ends in EXIT"
        );
        assert!(
            !crate::packet_adapters::payload_is_packet(&bytes),
            "an SVM payload is the artifact the adapter executes, never a SCALE-encoded Packet"
        );
        x3_svm_integration::interp_validate_program(&bytes)
            .expect("the chain's own SVM validator has to accept the fixture");
    }

    /// Empty intent stays empty: the kernel's empty-payload fast path depends on it.
    #[test]
    fn empty_intent_stays_empty() {
        assert!(wrap_evm_payload(&[]).is_empty());
        assert!(wrap_svm_payload(&[]).is_empty());
        assert!(wrap_x3_payload(&[]).is_empty());
    }

    #[test]
    fn wrapped_x3_packet_round_trips() {
        let bytes = wrap_x3_payload(&[0x58, 0x33, 0x00, 0x01]);
        let packet = deserialize_packet(&bytes).expect("x3 wrap must deserialize");
        assert_eq!(crate::packet_adapters::get_domain_mask(&packet), 0b0100);
    }
}
