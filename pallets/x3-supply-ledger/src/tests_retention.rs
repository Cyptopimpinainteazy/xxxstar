// SPDX-License-Identifier: Apache-2.0
//
// tests_retention.rs — the historical supply-proof window stays bounded.
//
// `on_finalize` is the only writer of `HistoricalProofs`, and the pruning guard is the only thing
// keeping that storage bounded. Nothing exercised the guard, so a mutation of
// `current_block > HISTORICAL_PROOF_RETENTION_BLOCKS` to `==` survived the whole suite: with that
// change the oldest proofs are never evicted after the boundary block. These tests pin the
// boundary from below and above, and pin the bound itself against a key that no live
// finalization produces — a restored or migrated state can carry one, and it must not leave
// the window one entry over the documented bound.

use crate::mock::{asset, new_test_ext, register_asset, Test};
use crate::{CurrentSupplyProof, HistoricalProofs, Pallet, HISTORICAL_PROOF_RETENTION_BLOCKS};
use frame_support::traits::Hooks;

/// Finalize a block the way the runtime would.
fn finalize(block: u32) {
    <Pallet<Test> as Hooks<u64>>::on_finalize(u64::from(block));
}

/// The window keeps at most `HISTORICAL_PROOF_RETENTION_BLOCKS` blocks of history: nothing is
/// pruned while the oldest proof is still inside it, and every later block evicts exactly the one
/// that has just fallen out.
#[test]
fn the_proof_window_is_pruned_to_the_retention_boundary() {
    new_test_ext().execute_with(|| {
        register_asset(asset(1), 1_000_000, 1_000_000);

        let boundary = HISTORICAL_PROOF_RETENTION_BLOCKS;

        // Up to and including the boundary, the oldest proof is still inside the window.
        for block in 1..=boundary {
            finalize(block);
        }
        assert!(
            HistoricalProofs::<Test>::contains_key(1),
            "block 1 must survive while the window still covers it"
        );
        assert!(HistoricalProofs::<Test>::contains_key(boundary));

        // The next block is the first one that evicts the proof `RETENTION` blocks behind it.
        finalize(boundary + 1);
        assert!(
            !HistoricalProofs::<Test>::contains_key(1),
            "block 1 must fall out of the window when block RETENTION + 1 finalizes"
        );
        assert!(HistoricalProofs::<Test>::contains_key(2));

        finalize(boundary + 2);
        assert!(!HistoricalProofs::<Test>::contains_key(2));
        assert!(HistoricalProofs::<Test>::contains_key(3));
    });
}

/// The window bound must hold even when storage already contains a proof below the first
/// block a live chain finalizes (a restored snapshot or a migrated state can carry one).
/// Block 0's proof has fallen out once block `RETENTION` finalizes, so it must be evicted
/// there and the window must hold exactly `RETENTION` entries — a `current_block > RETENTION`
/// guard skips that eviction and leaves one entry over the bound.
#[test]
fn the_window_bound_holds_even_for_keys_below_the_first_finalized_block() {
    new_test_ext().execute_with(|| {
        register_asset(asset(1), 1_000_000, 1_000_000);

        let boundary = HISTORICAL_PROOF_RETENTION_BLOCKS;

        finalize(0);
        assert!(
            HistoricalProofs::<Test>::contains_key(0),
            "block 0 wrote a proof"
        );

        for block in 1..boundary {
            finalize(block);
        }
        assert!(
            HistoricalProofs::<Test>::contains_key(0),
            "nothing falls out until the window is full"
        );
        assert_eq!(
            HistoricalProofs::<Test>::iter().count(),
            boundary as usize,
            "the window holds exactly RETENTION entries before the boundary block"
        );

        finalize(boundary);
        assert!(
            !HistoricalProofs::<Test>::contains_key(0),
            "block 0 falls out when block RETENTION finalizes"
        );
        assert_eq!(
            HistoricalProofs::<Test>::iter().count(),
            boundary as usize,
            "the eviction keeps the window at exactly RETENTION entries"
        );
        assert!(HistoricalProofs::<Test>::contains_key(1));
        assert!(HistoricalProofs::<Test>::contains_key(boundary));
    });
}

/// The proof is stamped with the block that built it. `current_timestamp` reads the block
/// number, but nothing asserted the stored `timestamp`, so replacing its body with 0 or 1
/// survived the suite; two different blocks must stamp two different values.
#[test]
fn the_supply_proof_is_stamped_with_the_block_number() {
    new_test_ext().execute_with(|| {
        register_asset(asset(1), 1_000_000, 1_000_000);

        frame_system::Pallet::<Test>::set_block_number(7);
        finalize(7);
        let proof = CurrentSupplyProof::<Test>::get().expect("block 7 wrote a proof");
        assert_eq!(proof.block_number, 7, "the proof names its own block");
        assert_eq!(
            proof.timestamp, 7,
            "the timestamp is the finalized block number"
        );

        frame_system::Pallet::<Test>::set_block_number(9);
        finalize(9);
        let proof = CurrentSupplyProof::<Test>::get().expect("block 9 wrote a proof");
        assert_eq!(
            proof.timestamp, 9,
            "a different block stamps a different value"
        );
    });
}
