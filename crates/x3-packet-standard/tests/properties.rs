//! Property tests for the X3 packet standard.
//!
//! These tests exercise the **invariants** of the packet lifecycle, not just
//! handcrafted golden cases:
//!
//! 1. SCALE encoding round-trips for any well-formed `Packet`.
//! 2. `PacketCommitment::of` is deterministic and input-sensitive: any
//!    one-byte change anywhere in the input flips the commitment.
//! 3. `ReplayGuard` is deterministic and idempotent: the same `(stream, seq,
//!    hash)` may be marked any number of times; a different hash at the same
//!    sequence is always rejected.
//! 4. `TimeoutPolicy::evaluate` is monotonic: once a packet has expired at
//!    `(h, t)`, it stays expired for all `(h', t')` with `h' >= h, t' >= t`.

use parity_scale_codec::{Decode, Encode};
use proptest::prelude::*;
use x3_packet_standard::packet::{Packet, PacketCommitment};
use x3_packet_standard::replay::ReplayGuard;
use x3_packet_standard::timeout::{evaluate, TimeoutOutcome};

fn arb_id32() -> impl Strategy<Value = [u8; 32]> {
    proptest::array::uniform32(any::<u8>())
}

prop_compose! {
    fn arb_packet()(
        src_chain in arb_id32(),
        src_port in arb_id32(),
        dst_chain in arb_id32(),
        dst_port in arb_id32(),
        sequence in any::<u64>(),
        timeout_height in any::<u64>(),
        timeout_timestamp in any::<u64>(),
        // bound payload to MAX_PAYLOAD; smaller corpus = faster proptest
        data in proptest::collection::vec(any::<u8>(), 0..1024usize),
    ) -> Packet {
        Packet::try_new(
            src_chain, src_port, dst_chain, dst_port,
            sequence, timeout_height, timeout_timestamp, data,
        )
        .expect("payload <= MAX_PAYLOAD")
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, .. ProptestConfig::default() })]

    /// Property: SCALE encode/decode is lossless for any valid packet.
    #[test]
    fn packet_scale_roundtrip(p in arb_packet()) {
        let bytes = p.encode();
        let decoded = Packet::decode(&mut &bytes[..]).expect("decode");
        prop_assert_eq!(decoded, p);
    }

    /// Property: commitment is deterministic — same input, same output.
    #[test]
    fn commitment_is_deterministic(p in arb_packet()) {
        let a = PacketCommitment::of(&p);
        let b = PacketCommitment::of(&p);
        prop_assert_eq!(a, b);
    }

    /// Property: any one-byte mutation of the payload changes the commitment.
    /// This is the cross-chain replay defence: a relayer cannot smuggle a
    /// modified payload past a commitment check.
    #[test]
    fn commitment_is_input_sensitive(
        p in arb_packet(),
        flip_idx in any::<u8>(),
    ) {
        // Force the packet to have at least 1 byte of payload, then flip one bit.
        let mut p2 = p.clone();
        if p2.data.is_empty() {
            p2.data.push(0);
        }
        let idx = (flip_idx as usize) % p2.data.len();
        p2.data[idx] ^= 0x01;
        prop_assert_ne!(PacketCommitment::of(&p), PacketCommitment::of(&p2));
    }

    /// Property: marking the **same** packet repeatedly is a no-op success.
    /// Receivers may retry without poisoning the guard.
    #[test]
    fn replay_guard_idempotent_on_same_hash(p in arb_packet()) {
        let mut g = ReplayGuard::default();
        prop_assert!(g.mark_received(&p).is_ok());
        prop_assert!(g.mark_received(&p).is_ok());
        prop_assert!(g.mark_received(&p).is_ok());
        prop_assert_eq!(g.len(), 1);
    }

    /// Property: at the same (stream, seq), a *different* packet (i.e. a
    /// different hash) is always a replay rejection. This is the core defence
    /// against a relayer trying to substitute payloads at a known sequence.
    #[test]
    fn replay_guard_rejects_different_hash(p in arb_packet(), extra in any::<u8>()) {
        let mut g = ReplayGuard::default();
        // Build a "twin" packet that shares (stream, sequence) but has
        // different payload bytes — that is enough to flip the commitment.
        let mut twin = p.clone();
        twin.data.push(extra);
        // Pre-condition: same stream & sequence.
        prop_assert_eq!(p.stream_key(), twin.stream_key());
        prop_assert_eq!(p.sequence, twin.sequence);

        g.mark_received(&p).unwrap();
        prop_assert!(g.mark_received(&twin).is_err());
    }
}

// ─── Timeout monotonicity (expressed without packet generation) ────────────
//
// We exercise `evaluate` directly so we can express the monotonicity property
// independent of the rest of the type machinery.

prop_compose! {
    fn arb_timeout_inputs()(
        height_cap in 0u64..1_000_000,
        ts_cap in 0u64..1_000_000_000,
        h0 in 0u64..1_000_000,
        t0 in 0u64..1_000_000_000,
        dh in 0u64..10_000,
        dt in 0u64..10_000,
    ) -> ((u64, u64), (u64, u64), (u64, u64)) {
        ((height_cap, ts_cap), (h0, t0), (h0.saturating_add(dh), t0.saturating_add(dt)))
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 512, .. ProptestConfig::default() })]

    /// Property: if `evaluate` returns expired at (h0, t0), it must also be
    /// expired at any (h0+dh, t0+dt). Time only moves forward.
    #[test]
    fn timeout_is_monotonic(((hc, tc), (h0, t0), (h1, t1)) in arb_timeout_inputs()) {
        let p = Packet::try_new(
            [0u8; 32], [0u8; 32], [0u8; 32], [0u8; 32],
            0, hc, tc, Vec::new(),
        ).unwrap();
        let o0 = evaluate(&p, h0, t0);
        let o1 = evaluate(&p, h1, t1);
        if matches!(o0, TimeoutOutcome::ExpiredHeight | TimeoutOutcome::ExpiredTimestamp) {
            prop_assert!(
                matches!(o1, TimeoutOutcome::ExpiredHeight | TimeoutOutcome::ExpiredTimestamp),
                "expired at ({h0},{t0}) but live at ({h1},{t1})"
            );
        }
    }
}
