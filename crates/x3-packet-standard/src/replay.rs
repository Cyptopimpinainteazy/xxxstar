//! Replay guard for the destination chain.
//!
//! The destination must consult [`ReplayGuard`] *before* it executes the
//! side-effects of a packet.  If `mark_received` returns `Err(SequenceReplay)`,
//! the packet has already been processed and must be ignored.
//!
//! The guard is intentionally a small in-memory map so the same logic can be
//! used in tests, in offchain relayer state, and (when wrapped in storage) in
//! the runtime pallet.

use alloc::collections::BTreeMap;
use sp_core::H256;

use crate::packet::{Packet, PacketError, Sequence, StreamKey};

/// Deterministic dedup map keyed by `(StreamKey, Sequence)`.
///
/// Stores the hash of the packet that was processed at each sequence so a
/// relayer cannot re-bind a different packet to a sequence already used.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ReplayGuard {
    seen: BTreeMap<(StreamKey, Sequence), H256>,
}

impl ReplayGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if a packet at this `(stream, sequence)` has already
    /// been processed.
    pub fn is_replay(&self, stream: &StreamKey, sequence: Sequence) -> bool {
        self.seen.contains_key(&(*stream, sequence))
    }

    /// Mark this packet as processed.  Idempotent: re-marking *the same*
    /// packet hash is a no-op; marking a *different* hash for the same
    /// `(stream, sequence)` is rejected as `SequenceReplay`.
    pub fn mark_received(&mut self, packet: &Packet) -> Result<(), PacketError> {
        let key = (packet.stream_key(), packet.sequence);
        let new_hash = crate::proof::commit_packet(packet);
        match self.seen.get(&key) {
            Some(existing) if *existing == new_hash => Ok(()), // idempotent retry
            Some(_) => Err(PacketError::SequenceReplay),
            None => {
                self.seen.insert(key, new_hash);
                Ok(())
            }
        }
    }

    /// Number of distinct (stream, sequence) entries.  Useful for tests and
    /// metrics; do not depend on this for consensus.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packet::Packet;

    fn id(s: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        let n = s.len().min(32);
        out[..n].copy_from_slice(&s[..n]);
        out
    }

    fn pkt(seq: u64, data: &[u8]) -> Packet {
        let packet_result = Packet::try_new(
            id(b"x3-native"),
            id(b"transfer"),
            id(b"x3-evm"),
            id(b"transfer"),
            seq,
            0,
            0,
            data.to_vec(),
        );
        assert!(packet_result.is_ok());
        let mut packet = Packet {
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
            packet = ok_packet;
        }
        packet
    }

    #[test]
    fn first_receipt_is_recorded() {
        let mut g = ReplayGuard::new();
        let p = pkt(1, b"a");
        assert!(!g.is_replay(&p.stream_key(), p.sequence));
        assert!(g.mark_received(&p).is_ok());
        assert!(g.is_replay(&p.stream_key(), p.sequence));
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn idempotent_retry_of_same_packet() {
        let mut g = ReplayGuard::new();
        let p = pkt(2, b"x");
        assert!(g.mark_received(&p).is_ok());
        // Same packet, same hash — must succeed without growing the map.
        assert!(g.mark_received(&p).is_ok());
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn different_packet_same_sequence_is_replay() {
        let mut g = ReplayGuard::new();
        assert!(g.mark_received(&pkt(3, b"first")).is_ok());
        let second = g.mark_received(&pkt(3, b"second"));
        assert!(second.is_err());
        let mut err = PacketError::AckMissing;
        if let Err(packet_err) = second {
            err = packet_err;
        }
        assert_eq!(err, PacketError::SequenceReplay);
        assert_eq!(g.len(), 1);
    }

    #[test]
    fn different_sequence_is_independent() {
        let mut g = ReplayGuard::new();
        assert!(g.mark_received(&pkt(1, b"a")).is_ok());
        assert!(g.mark_received(&pkt(2, b"b")).is_ok());
        assert!(g.mark_received(&pkt(3, b"c")).is_ok());
        assert_eq!(g.len(), 3);
    }
}
