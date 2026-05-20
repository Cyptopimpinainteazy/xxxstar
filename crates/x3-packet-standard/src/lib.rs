//! X3 packet standard — IBC-style packet lifecycle for internal cross-VM transfers.
//!
//! This crate is the *lifecycle* layer.  It defines:
//!
//! * [`packet::Packet`] — the on-wire structure carried between VMs.
//! * [`packet::PacketCommitment`] — `commit(packet) = blake2_256(domain || encode(packet))`,
//!   the hash a sender stores when it dispatches and a receiver checks before
//!   acting.
//! * [`packet::PacketReceipt`] — proof that a destination has processed a
//!   packet, used to short-circuit replay attempts.
//! * [`packet::Acknowledgement`] — the destination's response, optionally
//!   carrying domain-specific result data.
//! * [`replay::ReplayGuard`] — a deterministic dedup map keyed by
//!   `(src_chain, src_port, sequence)`.  Receivers must consult this *before*
//!   executing packet effects.
//! * [`timeout::TimeoutPolicy`] — height/timestamp bounds + the small state
//!   machine that drives refund-on-timeout.
//! * [`proof::commit`] — the canonical, domain-separated hashing primitive.
//!
//! ## Determinism
//!
//! Every byte that goes into a hash is SCALE-encoded.  Domain separation tags
//! are ASCII constants that **must not change** without a consensus migration.
//!
//! ## Scope
//!
//! This crate is wire- and lifecycle-only.  Routing, settlement, and balance
//! mutation live in `pallet-x3-cross-vm-router` / `x3-ixl`.  Keeping this
//! crate small and `no_std` lets the runtime, indexer, and offchain relayer
//! all share one source of truth for what a packet *is*.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod packet;
pub mod proof;
pub mod replay;
pub mod timeout;

pub use packet::{
    Acknowledgement, ChainId, Packet, PacketCommitment, PacketError, PacketReceipt, PortId,
    Sequence,
};
pub use proof::{commit_packet, hash_with_domain, COMMITMENT_DOMAIN, RECEIPT_DOMAIN};
pub use replay::ReplayGuard;
pub use timeout::{TimeoutOutcome, TimeoutPolicy};

/// Lifecycle state machine for a packet on the source chain.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    parity_scale_codec::Encode,
    parity_scale_codec::Decode,
    scale_info::TypeInfo,
)]
pub enum PacketState {
    /// Sent and committed on source; awaiting receipt or timeout.
    Sent,
    /// Receipt received from destination — destination is processing.
    Received,
    /// Acknowledgement received — terminal success path.
    Acknowledged,
    /// Timed out and refunded — terminal refund path.
    TimedOut,
}
