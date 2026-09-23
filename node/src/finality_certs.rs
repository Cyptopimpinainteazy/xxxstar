//! The finality certificates **this node** observed, and the rule for using them.
//!
//! `FinalityCertAnchors` on chain is written by `record_flash_finality_anchor`, which is an
//! unsigned call that stores the first non-zero certificate for a height — any peer can write it
//! first, and it cannot be replaced afterwards. The atomic gateway service used to take the
//! certificate for its signed `finalize_atomic_bundle` straight out of that map, so a peer who
//! planted a certificate could make this node **sign a finalization committing to a certificate no
//! voter ever produced** (TICKET-107).
//!
//! The fix is here rather than on chain: the node keeps the certificate *it* observed for each
//! finalized block (the flash-finality voter's certificate, or the GRANDPA-derived hash it anchors
//! itself), and finalizes with that. The chain's anchor is then a cross-check: if it disagrees with
//! what this node observed, the node refuses to finalize rather than signing the planted value,
//! which turns forgery into a bounded liveness failure that names the block and both hashes in the
//! log.

use sp_core::H256;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// Certificates observed for finalized blocks, shared by the finality tasks and the atomic
/// gateway service.
///
/// Cloning shares the map; the tasks run on different threads inside one node.
#[derive(Clone, Default)]
pub struct ObservedFinalityCerts {
    inner: Arc<Mutex<BTreeMap<u64, H256>>>,
}

/// How many blocks of observations to keep. A finalization names a recent block; keeping the whole
/// chain of certificates would grow without bound for no benefit.
const KEEP_BLOCKS: u64 = 256;

impl ObservedFinalityCerts {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the certificate this node observed for `block`.
    ///
    /// First observation wins for a height, matching the chain's own rule, and anything older than
    /// [`KEEP_BLOCKS`] behind the newest observation is dropped.
    pub fn record(&self, block: u64, cert: H256) {
        let Ok(mut map) = self.inner.lock() else {
            // A poisoned lock means another task panicked while holding it. Dropping the
            // observation is the safe direction: the service then waits instead of finalizing.
            log::error!("observed finality certificates: lock poisoned; dropping observation");
            return;
        };
        map.entry(block).or_insert(cert);
        let oldest = block.saturating_sub(KEEP_BLOCKS);
        map.retain(|height, _| *height >= oldest);
    }

    /// The certificate this node observed for `block`, if it has one.
    pub fn get(&self, block: u64) -> Option<H256> {
        self.inner.lock().ok()?.get(&block).copied()
    }
}

/// What to do with a block's certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizationCertificate {
    /// No certificate observed yet, or this node's certificate is not anchored on chain yet.
    Wait,
    /// The node's own certificate, anchored on chain. Safe to finalize with.
    Finalize(H256),
    /// The chain's anchor disagrees with the certificate this node observed: somebody else planted
    /// it, first-write-wins makes it permanent, and this node must not sign it.
    Poisoned {
        /// What this node observed for the block.
        observed: H256,
        /// What the chain says.
        anchored: H256,
    },
}

/// Decide whether `block` can be finalized, from the certificate this node observed and the one the
/// chain anchored.
///
/// The node finalizes only with a certificate it produced itself *and* that the chain agrees with.
pub fn decide_finalization_cert(
    observed: Option<H256>,
    anchored: Option<H256>,
) -> FinalizationCertificate {
    match (observed, anchored) {
        (Some(observed), Some(anchored)) if observed == anchored => {
            FinalizationCertificate::Finalize(observed)
        }
        (Some(observed), Some(anchored)) => FinalizationCertificate::Poisoned { observed, anchored },
        // Either we have not seen a certificate ourselves, or ours has not been anchored yet.
        // Both are "wait": never finalize on a value this node did not observe.
        _ => FinalizationCertificate::Wait,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cert(byte: u8) -> H256 {
        H256::repeat_byte(byte)
    }

    #[test]
    fn a_node_certificate_the_chain_agrees_with_is_used() {
        assert_eq!(
            decide_finalization_cert(Some(cert(0xAA)), Some(cert(0xAA))),
            FinalizationCertificate::Finalize(cert(0xAA))
        );
    }

    #[test]
    fn a_planted_anchor_is_refused_rather_than_signed() {
        // TICKET-107, stated as a test: the chain's anchor is the attacker's, ours is not, and the
        // decision is "do not sign" — not "use whichever the chain has".
        assert_eq!(
            decide_finalization_cert(Some(cert(0xAA)), Some(cert(0xBB))),
            FinalizationCertificate::Poisoned {
                observed: cert(0xAA),
                anchored: cert(0xBB)
            }
        );
    }

    #[test]
    fn an_unanchored_certificate_waits() {
        assert_eq!(
            decide_finalization_cert(Some(cert(0xAA)), None),
            FinalizationCertificate::Wait
        );
    }

    #[test]
    fn an_anchor_without_an_observation_waits() {
        // The mistaken version of the fix is "finalize with whatever the chain anchored". This is
        // that case: we have nothing of our own for the block, so we wait for our own voter.
        assert_eq!(
            decide_finalization_cert(None, Some(cert(0xBB))),
            FinalizationCertificate::Wait
        );
        assert_eq!(decide_finalization_cert(None, None), FinalizationCertificate::Wait);
    }

    #[test]
    fn observations_keep_the_first_value_and_stay_bounded() {
        let certs = ObservedFinalityCerts::new();
        certs.record(10, cert(0x01));
        certs.record(10, cert(0x02));
        assert_eq!(certs.get(10), Some(cert(0x01)), "first observation wins, as on chain");

        certs.record(10 + KEEP_BLOCKS + 1, cert(0x03));
        assert_eq!(certs.get(10), None, "old heights are dropped");
        assert_eq!(certs.get(10 + KEEP_BLOCKS + 1), Some(cert(0x03)));
    }
}
