//! Timeout / refund evaluation.
//!
//! A timeout is a deterministic function of the packet's declared
//! `(timeout_height, timeout_timestamp)` and the destination's *observed*
//! `(now_height, now_timestamp)`.  The decision is intentionally separated
//! from any storage-mutating action so it can be unit-tested in isolation.

use crate::packet::{Packet, PacketError};

/// Effective timeout policy attached to a packet.  Values of `0` disable the
/// corresponding bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeoutPolicy {
    pub height: u64,
    pub timestamp: u64,
}

impl TimeoutPolicy {
    pub fn from_packet(p: &Packet) -> Self {
        Self {
            height: p.timeout_height,
            timestamp: p.timeout_timestamp,
        }
    }

    /// Returns `true` if at least one bound is enabled.
    pub fn is_active(&self) -> bool {
        self.height != 0 || self.timestamp != 0
    }
}

/// Outcome of evaluating a packet against the current chain state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeoutOutcome {
    /// Packet is still within its window — destination may execute.
    Live,
    /// Block-height bound exceeded — source must refund, destination must reject.
    ExpiredHeight,
    /// Timestamp bound exceeded — source must refund, destination must reject.
    ExpiredTimestamp,
}

impl TimeoutOutcome {
    pub fn is_expired(self) -> bool {
        !matches!(self, TimeoutOutcome::Live)
    }

    /// Map an expiration to the canonical [`PacketError`].
    pub fn as_error(self) -> Option<PacketError> {
        match self {
            TimeoutOutcome::Live => None,
            TimeoutOutcome::ExpiredHeight => Some(PacketError::TimedOutHeight),
            TimeoutOutcome::ExpiredTimestamp => Some(PacketError::TimedOutTimestamp),
        }
    }
}

/// Evaluate a packet against `(now_height, now_timestamp)`.  Height is checked
/// first so the most operator-friendly bound dominates.
pub fn evaluate(packet: &Packet, now_height: u64, now_timestamp: u64) -> TimeoutOutcome {
    if packet.timeout_height != 0 && now_height >= packet.timeout_height {
        return TimeoutOutcome::ExpiredHeight;
    }
    if packet.timeout_timestamp != 0 && now_timestamp >= packet.timeout_timestamp {
        return TimeoutOutcome::ExpiredTimestamp;
    }
    TimeoutOutcome::Live
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

    fn pkt(h: u64, t: u64) -> Packet {
        let packet_result = Packet::try_new(
            id(b"x3-native"),
            id(b"transfer"),
            id(b"x3-evm"),
            id(b"transfer"),
            1,
            h,
            t,
            vec![],
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
    fn no_bounds_means_always_live() {
        let p = pkt(0, 0);
        assert_eq!(evaluate(&p, 1_000_000, 9_999_999_999), TimeoutOutcome::Live);
    }

    #[test]
    fn height_expiration_dominates() {
        let p = pkt(100, 5_000);
        // height already past, timestamp still in window
        assert_eq!(evaluate(&p, 100, 4_000), TimeoutOutcome::ExpiredHeight);
    }

    #[test]
    fn timestamp_expiration() {
        let p = pkt(0, 5_000);
        assert_eq!(evaluate(&p, 0, 5_000), TimeoutOutcome::ExpiredTimestamp);
        assert_eq!(evaluate(&p, 0, 4_999), TimeoutOutcome::Live);
    }

    #[test]
    fn outcome_to_error_is_lossless() {
        assert_eq!(TimeoutOutcome::Live.as_error(), None);
        assert_eq!(
            TimeoutOutcome::ExpiredHeight.as_error(),
            Some(PacketError::TimedOutHeight)
        );
        assert_eq!(
            TimeoutOutcome::ExpiredTimestamp.as_error(),
            Some(PacketError::TimedOutTimestamp)
        );
    }
}
