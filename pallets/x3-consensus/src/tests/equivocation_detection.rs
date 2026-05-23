//! Tests for equivocation detection and slashing

use crate::mock::*;
use crate::{
    pallet::{Event, ValidatorInfo, ValidatorStake},
    SlashReason,
};
use frame_support::assert_ok;
use sp_runtime::Perbill;
use sp_staking::offence::{Kind, Offence, OffenceDetails, OnOffenceHandler, ReportOffence};
use sp_staking::SessionIndex;

struct MockEquivocationOffence {
    offender: u64,
    time_slot: u32,
    session_index: SessionIndex,
    validator_set_count: u32,
}

impl Offence<(u64, ())> for MockEquivocationOffence {
    const ID: Kind = *b"X3Equivocation!!";
    type TimeSlot = u32;

    fn offenders(&self) -> Vec<(u64, ())> {
        vec![(self.offender, ())]
    }

    fn session_index(&self) -> SessionIndex {
        self.session_index
    }

    fn validator_set_count(&self) -> u32 {
        self.validator_set_count
    }

    fn time_slot(&self) -> Self::TimeSlot {
        self.time_slot
    }

    fn slash_fraction(&self, _offenders_count: u32) -> Perbill {
        Perbill::from_percent(10)
    }
}

#[test]
fn test_double_sign_detection_is_modeled_by_slash_call() {
    new_test_ext().execute_with(|| {
        let offender: u64 = 1;
        ValidatorStake::<Test>::insert(
            offender,
            ValidatorInfo {
                stake: 10_000_000,
                is_active: true,
            },
        );

        System::set_block_number(1);

        assert_ok!(Consensus::report_misbehavior(
            RuntimeOrigin::signed(2),
            offender,
            SlashReason::DoubleSign,
        ));

        let info = ValidatorStake::<Test>::get(offender).expect("validator must exist");
        assert_eq!(info.stake, 9_000_000);
        assert!(info.is_active);
        System::assert_has_event(
            Event::SlashApplied {
                validator: offender,
                slash_amount: 1_000_000,
                new_stake: 9_000_000,
            }
            .into(),
        );
    });
}

#[test]
fn test_equivocation_slashing_disables_at_floor() {
    new_test_ext().execute_with(|| {
        let offender: u64 = 3;
        ValidatorStake::<Test>::insert(
            offender,
            ValidatorInfo {
                stake: 1_100_000,
                is_active: true,
            },
        );

        assert_ok!(Consensus::report_misbehavior(
            RuntimeOrigin::signed(4),
            offender,
            SlashReason::Equivocation,
        ));

        let info = ValidatorStake::<Test>::get(offender).expect("validator must exist");
        assert_eq!(info.stake, 1_000_000);
        assert!(
            !info.is_active,
            "equivocation slash landing on floor must deactivate validator"
        );
    });
}

#[test]
fn test_offence_reporting_extrinsic_accepts_known_validator() {
    new_test_ext().execute_with(|| {
        let offender: u64 = 5;
        ValidatorStake::<Test>::insert(
            offender,
            ValidatorInfo {
                stake: 2_000_000,
                is_active: true,
            },
        );

        assert_ok!(Consensus::report_misbehavior(
            RuntimeOrigin::signed(6),
            offender,
            SlashReason::InvalidFinality,
        ));

        let info = ValidatorStake::<Test>::get(offender).unwrap();
        assert_eq!(info.stake, 1_800_000);
    });
}

#[test]
fn test_offences_report_triggers_consensus_slash() {
    new_test_ext().execute_with(|| {
        let offender: u64 = 7;
        ValidatorStake::<Test>::insert(
            offender,
            ValidatorInfo {
                stake: 10_000_000,
                is_active: true,
            },
        );

        let offence = MockEquivocationOffence {
            offender,
            time_slot: 42,
            session_index: 0,
            validator_set_count: 1,
        };

        assert_ok!(Offences::report_offence(vec![1], offence));

        let info = ValidatorStake::<Test>::get(offender).expect("validator must exist");
        assert_eq!(info.stake, 9_000_000);
        assert!(info.is_active);
        System::assert_has_event(
            Event::SlashApplied {
                validator: offender,
                slash_amount: 1_000_000,
                new_stake: 9_000_000,
            }
            .into(),
        );
    });
}

#[test]
fn test_on_offence_handler_slashes_reported_offenders() {
    new_test_ext().execute_with(|| {
        let offender: u64 = 7;
        ValidatorStake::<Test>::insert(
            offender,
            ValidatorInfo {
                stake: 10_000_000,
                is_active: true,
            },
        );

        let offence = OffenceDetails {
            offender: (offender, ()),
            reporters: vec![8],
        };

        Consensus::on_offence(&[offence], &[Perbill::from_percent(10)], 0);

        let info = ValidatorStake::<Test>::get(offender).expect("validator must exist");
        assert_eq!(info.stake, 9_000_000);
        assert!(info.is_active);
    });
}
