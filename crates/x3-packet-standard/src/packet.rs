//! Packet structures.
//!
//! Field layout intentionally mirrors IBC ICS-04 so we can reuse the
//! mental model: every packet carries source/destination chain + port,
//! a monotonic per-stream sequence, optional height/time bounds for
//! timeouts, and an opaque payload interpreted by the destination port.

use alloc::vec::Vec;
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::H256;

/// Internal chain identifier (e.g. `b"x3-native"`, `b"x3-evm"`, `b"x3-svm"`).
/// Bounded to 32 bytes; longer ids are rejected at construction time.
pub type ChainId = [u8; 32];

/// Internal port identifier (e.g. `b"transfer"`, `b"swap"`).  Bounded.
pub type PortId = [u8; 32];

/// Monotonic per-(src_chain,src_port,dst_chain,dst_port) sequence.
pub type Sequence = u64;

/// Errors that can be raised by the lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub enum PacketError {
    /// Sequence number reused on this stream.
    SequenceReplay,
    /// Packet exceeded its height-based timeout.
    TimedOutHeight,
    /// Packet exceeded its timestamp-based timeout.
    TimedOutTimestamp,
    /// Commitment provided by sender does not match recomputed commitment.
    CommitmentMismatch,
    /// Packet payload exceeds the configured maximum.
    PayloadTooLarge,
    /// Destination port did not register an acknowledgement.
    AckMissing,
}

/// On-wire packet.  This is what the source chain commits and the destination
/// chain consumes.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct Packet {
    pub src_chain: ChainId,
    pub src_port: PortId,
    pub dst_chain: ChainId,
    pub dst_port: PortId,
    pub sequence: Sequence,
    /// Block-height timeout on the destination; `0` means disabled.
    pub timeout_height: u64,
    /// Unix-millis timestamp timeout on the destination; `0` means disabled.
    pub timeout_timestamp: u64,
    /// Opaque application payload.  The destination port decodes it.
    pub data: Vec<u8>,
}

impl Packet {
    /// Maximum payload size we will allow through the lifecycle.  The router
    /// pallet may impose stricter caps; this is a hard ceiling.
    pub const MAX_PAYLOAD: usize = 64 * 1024;

    /// Construct a packet, validating bounds.  Returns
    /// [`PacketError::PayloadTooLarge`] if the payload exceeds [`MAX_PAYLOAD`].
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        src_chain: ChainId,
        src_port: PortId,
        dst_chain: ChainId,
        dst_port: PortId,
        sequence: Sequence,
        timeout_height: u64,
        timeout_timestamp: u64,
        data: Vec<u8>,
    ) -> Result<Self, PacketError> {
        if data.len() > Self::MAX_PAYLOAD {
            return Err(PacketError::PayloadTooLarge);
        }
        Ok(Self {
            src_chain,
            src_port,
            dst_chain,
            dst_port,
            sequence,
            timeout_height,
            timeout_timestamp,
            data,
        })
    }

    /// Stream key used by the replay guard and sequence tracker.
    pub fn stream_key(&self) -> StreamKey {
        StreamKey {
            src_chain: self.src_chain,
            src_port: self.src_port,
            dst_chain: self.dst_chain,
            dst_port: self.dst_port,
        }
    }
}

/// Quad-tuple identifying a unidirectional packet stream.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct StreamKey {
    pub src_chain: ChainId,
    pub src_port: PortId,
    pub dst_chain: ChainId,
    pub dst_port: PortId,
}

/// A packet commitment is just the hash; we wrap it for type safety.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Encode,
    Decode,
    DecodeWithMemTracking,
    TypeInfo,
    MaxEncodedLen,
)]
pub struct PacketCommitment(pub H256);

impl PacketCommitment {
    pub fn of(packet: &Packet) -> Self {
        Self(crate::proof::commit_packet(packet))
    }
}

/// Receipt stored by the destination chain to prevent the same packet from
/// being processed twice.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct PacketReceipt {
    pub stream: StreamKey,
    pub sequence: Sequence,
    /// Hash of the packet that was processed.  Required so a relayer cannot
    /// claim a receipt for a different packet on the same sequence number.
    pub packet_hash: H256,
}

/// Application-level acknowledgement returned by the destination port.
///
/// `success = true` means the destination committed the side-effects; the
/// source can release escrow.  `success = false` means the destination
/// refused; the source must refund.
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, DecodeWithMemTracking, TypeInfo)]
pub struct Acknowledgement {
    pub stream: StreamKey,
    pub sequence: Sequence,
    pub success: bool,
    /// Optional opaque result payload (e.g. minted token id, swap output).
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(s: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        let n = s.len().min(32);
        out[..n].copy_from_slice(&s[..n]);
        out
    }

    #[test]
    fn payload_size_enforced() {
        let big = vec![0u8; Packet::MAX_PAYLOAD + 1];
        let r = Packet::try_new(
            id(b"x3-native"),
            id(b"transfer"),
            id(b"x3-evm"),
            id(b"transfer"),
            1,
            0,
            0,
            big,
        );
        assert_eq!(r.unwrap_err(), PacketError::PayloadTooLarge);
    }

    #[test]
    fn commitment_is_deterministic_and_input_sensitive() {
        let p1_result = Packet::try_new(
            id(b"x3-native"),
            id(b"transfer"),
            id(b"x3-evm"),
            id(b"transfer"),
            1,
            0,
            0,
            b"hello".to_vec(),
        );
        assert!(p1_result.is_ok());
        let mut p1 = Packet {
            src_chain: [0u8; 32],
            src_port: [0u8; 32],
            dst_chain: [0u8; 32],
            dst_port: [0u8; 32],
            sequence: 0,
            timeout_height: 0,
            timeout_timestamp: 0,
            data: Vec::new(),
        };
        if let Ok(ok_packet) = p1_result {
            p1 = ok_packet;
        }
        let p2 = p1.clone();
        let mut p3 = p1.clone();
        p3.data = b"world".to_vec();

        assert_eq!(PacketCommitment::of(&p1), PacketCommitment::of(&p2));
        assert_ne!(PacketCommitment::of(&p1), PacketCommitment::of(&p3));
    }

    #[test]
    fn scale_roundtrip() {
        let packet_result = Packet::try_new(
            id(b"x3-native"),
            id(b"transfer"),
            id(b"x3-evm"),
            id(b"transfer"),
            7,
            1234,
            5678,
            b"payload".to_vec(),
        );
        assert!(packet_result.is_ok());
        let mut p = Packet {
            src_chain: [0u8; 32],
            src_port: [0u8; 32],
            dst_chain: [0u8; 32],
            dst_port: [0u8; 32],
            sequence: 0,
            timeout_height: 0,
            timeout_timestamp: 0,
            data: Vec::new(),
        };
        if let Ok(ok_packet) = packet_result {
            p = ok_packet;
        }
        let bytes = p.encode();
        let decode_result = Packet::decode(&mut &bytes[..]);
        assert!(decode_result.is_ok());
        let mut back = p.clone();
        if let Ok(decoded) = decode_result {
            back = decoded;
        }
        assert_eq!(p, back);
    }
}
