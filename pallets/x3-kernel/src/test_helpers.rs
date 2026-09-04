//! Test-only helpers for the x3-kernel pallet.
//!
//! Phase 1.4 of the kernel's strict-packet validation requires every
//! non-empty `evm_payload` / `svm_payload` / `x3_payload` to be a valid
//! SCALE-encoded `Packet` whose domain mask matches the slot. Pre-1.4
//! the kernel tolerated short raw bytes (a 30-byte threshold let
//! undecodable legacy payloads through). Post-1.4 they are rejected
//! with `InvalidEvmPacket` / `InvalidSvmPacket` / `InvalidX3VmPacket`.
//!
//! Existing tests were written against the old contract. Rather than
//! weaken the production validation, the test suite goes through the
//! wrappers below, which take the test's "intent bytes" (whatever
//! raw bytes the test would otherwise have used) and encode them as
//! a syntactically valid `Packet` whose `args` / `data` / `recipient`
//! field is those intent bytes. Empty intent bytes stay empty — the
//! kernel already treats empty payloads as "no side effect for this VM".
//!
//! `compute_prepare_root` is the pallet's canonical algorithm
//! (blake2_256 over the concatenation of every input). Tests must
//! feed the **wrapped** bytes to `compute_prepare_root` so the
//! prepare_root matches what the kernel computes internally.

use parity_scale_codec::Encode;
use x3_packet_schema::{EvmPacket, Packet, SvmPacket, X3VmPacket};

/// Wrap a test's EVM "intent bytes" as a valid SCALE-encoded
/// `Packet::Evm(EvmPacket::Call)`. Empty input stays empty so the
/// kernel's empty-payload fast path still triggers.
pub fn wrap_evm_payload(intent: &[u8]) -> Vec<u8> {
    if intent.is_empty() {
        return Vec::new();
    }
    Packet::Evm(EvmPacket::Call {
        // Deterministic, content-addressable test fields.
        contract: blake2_contract_address(intent),
        function_selector: [0xaa, 0xbb, 0xcc, 0xdd],
        args: intent.to_vec(),
        value: x3_packet_schema::U256::from(0),
    })
    .encode()
}

/// Wrap a test's SVM "intent bytes" as a valid SCALE-encoded
/// `Packet::Svm(SvmPacket::Invoke)`. Empty input stays empty.
pub fn wrap_svm_payload(intent: &[u8]) -> Vec<u8> {
    if intent.is_empty() {
        return Vec::new();
    }
    Packet::Svm(SvmPacket::Invoke {
        program_id: blake2_program_id(intent),
        accounts: Vec::new(),
        data: intent.to_vec(),
    })
    .encode()
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

fn blake2_contract_address(intent: &[u8]) -> [u8; 20] {
    let h = sp_core::hashing::blake2_256(intent);
    let mut out = [0u8; 20];
    out.copy_from_slice(&h[..20]);
    out
}

fn blake2_program_id(intent: &[u8]) -> [u8; 32] {
    sp_core::hashing::blake2_256(intent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet_adapters::deserialize_packet;

    /// The kernel's strict-packet validation requires the wrapped
    /// output to round-trip through `deserialize_packet`. This test
    /// pins that contract so a future change to the wrapping shape
    /// cannot silently break the entire test suite.
    #[test]
    fn wrapped_evm_packet_round_trips() {
        let bytes = wrap_evm_payload(&[0xaa, 0xbb, 0xcc]);
        let packet = deserialize_packet(&bytes).expect("evm wrap must deserialize");
        assert_eq!(crate::packet_adapters::get_domain_mask(&packet), 0b0001);
    }

    #[test]
    fn wrapped_svm_packet_round_trips() {
        let bytes = wrap_svm_payload(&[0xaa, 0xbb, 0xcc]);
        let packet = deserialize_packet(&bytes).expect("svm wrap must deserialize");
        assert_eq!(crate::packet_adapters::get_domain_mask(&packet), 0b0010);
    }

    #[test]
    fn wrapped_x3_packet_round_trips() {
        let bytes = wrap_x3_payload(&[0x58, 0x33, 0x00, 0x01]);
        let packet = deserialize_packet(&bytes).expect("x3 wrap must deserialize");
        assert_eq!(crate::packet_adapters::get_domain_mask(&packet), 0b0100);
    }
}
