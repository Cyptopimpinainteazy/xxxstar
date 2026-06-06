//! Canonical hashing primitives for the packet standard.
//!
//! All hashes are blake2_256 with a domain-separation tag prefixed in front
//! of the SCALE-encoded payload.  Changing a tag is a consensus-breaking
//! change.

use alloc::vec::Vec;
use blake2::{digest::consts::U32, Blake2b, Digest};
use parity_scale_codec::Encode;
use sp_core::H256;

use crate::packet::{Packet, PacketReceipt};

/// Domain tag for `PacketCommitment` hashes.  Do not modify without a
/// consensus migration.
pub const COMMITMENT_DOMAIN: &[u8] = b"x3.packet.commitment.v1";

/// Domain tag for `PacketReceipt` hashes.  Do not modify without a
/// consensus migration.
pub const RECEIPT_DOMAIN: &[u8] = b"x3.packet.receipt.v1";

/// Hash arbitrary bytes with a domain tag.  `blake2_256(domain || payload)`.
pub fn hash_with_domain(domain: &[u8], payload: &[u8]) -> H256 {
    let mut hasher = Blake2b::<U32>::new();
    hasher.update(domain);
    hasher.update(payload);
    let out = hasher.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(&out);
    H256(buf)
}

/// Canonical commitment hash for a packet.
pub fn commit_packet(packet: &Packet) -> H256 {
    let encoded: Vec<u8> = packet.encode();
    hash_with_domain(COMMITMENT_DOMAIN, &encoded)
}

/// Canonical commitment hash for a receipt.
pub fn commit_receipt(receipt: &PacketReceipt) -> H256 {
    let encoded: Vec<u8> = receipt.encode();
    hash_with_domain(RECEIPT_DOMAIN, &encoded)
}
