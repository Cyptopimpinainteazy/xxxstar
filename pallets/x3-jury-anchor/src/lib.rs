#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_support::traits::Time;
    use frame_system::pallet_prelude::*;
    use sp_core::H256;
    use sp_std::vec::Vec;

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    // Configuration trait
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_timestamp::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        #[pallet::constant]
        type MaxSessionIdLength: Get<u32>;
    }

    // Types
    #[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
    pub struct JuryDecisionRecord<BlockNumber, Moment, AccountId> {
        pub decision_hash: H256,
        pub block_number: BlockNumber,
        pub timestamp: Moment,
        pub jury_authority: AccountId,
        pub metadata: JuryMetadata,
    }

    #[derive(Encode, Decode, DecodeWithMemTracking, Clone, PartialEq, Eq, Debug, TypeInfo)]
    pub struct JuryMetadata {
        pub member_count: u32,
        pub quorum_threshold: u32, // 0-100, e.g., 66
        pub result: bool,
        pub session_duration_secs: u32,
    }

    // Storage
    #[pallet::storage]
    #[pallet::getter(fn jury_decisions)]
    pub type JuryDecisions<T> = StorageMap<
        _,
        Blake2_128Concat,
        Vec<u8>,
        JuryDecisionRecord<
            BlockNumberFor<T>,
            <T as pallet_timestamp::Config>::Moment,
            <T as frame_system::Config>::AccountId,
        >,
    >;

    #[pallet::storage]
    #[pallet::getter(fn jury_authority)]
    pub type JuryAuthority<T: Config> =
        StorageValue<_, <T as frame_system::Config>::AccountId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn decision_count)]
    pub type DecisionCount<T> = StorageValue<_, u32, ValueQuery>;

    // Events
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        JuryDecisionAnchored {
            session_id: Vec<u8>,
            decision_hash: H256,
            block_number: BlockNumberFor<T>,
        },
        AuthorityChanged {
            new_authority: <T as frame_system::Config>::AccountId,
        },
        VerificationSucceeded {
            session_id: Vec<u8>,
        },
    }

    // Errors
    #[pallet::error]
    pub enum Error<T> {
        Unauthorized,
        InvalidSessionId,
        SessionIdTooLong,
        DecisionAlreadyExists,
        DecisionNotFound,
    }

    // Extrinsics
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(T::DbWeight::get().reads_writes(1, 2))]
        pub fn anchor_decision(
            origin: OriginFor<T>,
            session_id: Vec<u8>,
            decision_hash: H256,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;

            // Check authorization
            let authority = Self::jury_authority().ok_or(Error::<T>::Unauthorized)?;
            ensure!(caller == authority, Error::<T>::Unauthorized);

            // Validate session_id
            ensure!(!session_id.is_empty(), Error::<T>::InvalidSessionId);
            ensure!(
                session_id.len() <= T::MaxSessionIdLength::get() as usize,
                Error::<T>::SessionIdTooLong
            );

            // Check not already anchored
            ensure!(
                !JuryDecisions::<T>::contains_key(&session_id),
                Error::<T>::DecisionAlreadyExists
            );

            // Create record
            let record = JuryDecisionRecord {
                decision_hash,
                block_number: frame_system::Pallet::<T>::block_number(),
                timestamp: pallet_timestamp::Pallet::<T>::now(),
                jury_authority: caller,
                metadata: JuryMetadata {
                    member_count: 5,
                    quorum_threshold: 66,
                    result: true,
                    session_duration_secs: 900,
                },
            };

            // Store and emit
            JuryDecisions::<T>::insert(&session_id, record.clone());
            DecisionCount::<T>::mutate(|count| *count = count.saturating_add(1));

            Self::deposit_event(Event::JuryDecisionAnchored {
                session_id,
                decision_hash,
                block_number: record.block_number,
            });

            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(T::DbWeight::get().writes(1))]
        pub fn set_jury_authority(
            origin: OriginFor<T>,
            new_authority: <T as frame_system::Config>::AccountId,
        ) -> DispatchResult {
            ensure_root(origin)?;
            JuryAuthority::<T>::put(&new_authority);

            Self::deposit_event(Event::AuthorityChanged { new_authority });

            Ok(())
        }
    }

    // RPC helpers
    impl<T: Config> Pallet<T> {
        pub fn get_jury_decision(
            session_id: Vec<u8>,
        ) -> Option<JuryDecisionRecord<BlockNumberFor<T>, T::Moment, T::AccountId>> {
            JuryDecisions::<T>::get(&session_id)
        }

        pub fn verify_decision(session_id: Vec<u8>, expected_hash: H256) -> bool {
            if let Some(record) = Self::get_jury_decision(session_id) {
                record.decision_hash == expected_hash
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use frame_support::assert_ok;
    use frame_support::traits::ConstU32;
    use frame_system as system;
    use sp_core::H256;
    use sp_runtime::BuildStorage;

    type Block = frame_system::mocking::MockBlock<Test>;

    frame_support::construct_runtime!(
        pub enum Test {
            System: frame_system,
            Timestamp: pallet_timestamp,
            JuryAnchor: pallet,
        }
    );

    impl frame_system::Config for Test {
        type BaseCallFilter = frame_support::traits::Everything;
        type BlockWeights = ();
        type BlockLength = ();
        type DbWeight = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Nonce = u64;
        type Hash = H256;
        type Hashing = sp_runtime::traits::BlakeTwo256;
        type AccountId = u64;
        type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
        type Block = Block;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = frame_support::traits::ConstU64<250>;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = ConstU32<16>;
    }

    impl pallet_timestamp::Config for Test {
        type Moment = u64;
        type OnTimestampSet = ();
        type MinimumPeriod = ();
        type WeightInfo = ();
    }

    impl Config for Test {
        type RuntimeEvent = RuntimeEvent;
        type MaxSessionIdLength = ConstU32<256>;
    }

    fn new_test_ext() -> sp_io::TestExternalities {
        let t = frame_system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap();
        t.into()
    }

    #[test]
    fn test_anchor_decision_success() {
        new_test_ext().execute_with(|| {
            let session_id = b"test-session".to_vec();
            let hash = H256::repeat_byte(1);

            JuryAnchor::set_jury_authority(RuntimeOrigin::root(), 1).unwrap();

            assert_ok!(JuryAnchor::anchor_decision(
                RuntimeOrigin::signed(1),
                session_id.clone(),
                hash,
            ));

            let record = JuryAnchor::get_jury_decision(session_id).unwrap();
            assert_eq!(record.decision_hash, hash);
        });
    }

    #[test]
    fn test_unauthorized_anchor() {
        new_test_ext().execute_with(|| {
            JuryAnchor::set_jury_authority(RuntimeOrigin::root(), 1).unwrap();

            assert!(JuryAnchor::anchor_decision(
                RuntimeOrigin::signed(2),
                b"test".to_vec(),
                H256::repeat_byte(1),
            )
            .is_err());
        });
    }

    #[test]
    fn test_invalid_session_id() {
        new_test_ext().execute_with(|| {
            JuryAnchor::set_jury_authority(RuntimeOrigin::root(), 1).unwrap();

            assert!(JuryAnchor::anchor_decision(
                RuntimeOrigin::signed(1),
                vec![],
                H256::repeat_byte(1),
            )
            .is_err());
        });
    }

    #[test]
    fn test_verify_decision_match() {
        new_test_ext().execute_with(|| {
            let session_id = b"verify-test".to_vec();
            let hash = H256::repeat_byte(2);

            JuryAnchor::set_jury_authority(RuntimeOrigin::root(), 1).unwrap();
            JuryAnchor::anchor_decision(RuntimeOrigin::signed(1), session_id.clone(), hash)
                .unwrap();

            assert!(JuryAnchor::verify_decision(session_id.clone(), hash));
        });
    }

    #[test]
    fn test_verify_decision_mismatch() {
        new_test_ext().execute_with(|| {
            let session_id = b"verify-test-2".to_vec();
            let hash1 = H256::repeat_byte(3);
            let hash2 = H256::repeat_byte(4);

            JuryAnchor::set_jury_authority(RuntimeOrigin::root(), 1).unwrap();
            JuryAnchor::anchor_decision(RuntimeOrigin::signed(1), session_id.clone(), hash1)
                .unwrap();

            assert!(!JuryAnchor::verify_decision(session_id, hash2));
        });
    }

    #[test]
    fn test_duplicate_anchor_rejected() {
        new_test_ext().execute_with(|| {
            let session_id = b"duplicate-test".to_vec();
            let hash = H256::repeat_byte(5);

            JuryAnchor::set_jury_authority(RuntimeOrigin::root(), 1).unwrap();
            JuryAnchor::anchor_decision(RuntimeOrigin::signed(1), session_id.clone(), hash)
                .unwrap();

            assert!(
                JuryAnchor::anchor_decision(RuntimeOrigin::signed(1), session_id, hash,).is_err()
            );
        });
    }

    #[test]
    fn test_multiple_decisions() {
        new_test_ext().execute_with(|| {
            JuryAnchor::set_jury_authority(RuntimeOrigin::root(), 1).unwrap();

            for i in 0..5 {
                let session_id = format!("session-{}", i).into_bytes();
                let hash = H256::repeat_byte(i as u8);

                assert_ok!(JuryAnchor::anchor_decision(
                    RuntimeOrigin::signed(1),
                    session_id,
                    hash,
                ));
            }

            assert_eq!(JuryAnchor::decision_count(), 5);
        });
    }

    #[test]
    fn test_authority_change() {
        new_test_ext().execute_with(|| {
            JuryAnchor::set_jury_authority(RuntimeOrigin::root(), 1).unwrap();
            JuryAnchor::set_jury_authority(RuntimeOrigin::root(), 2).unwrap();

            assert_eq!(JuryAnchor::jury_authority(), Some(2));

            assert!(JuryAnchor::anchor_decision(
                RuntimeOrigin::signed(1),
                b"test".to_vec(),
                H256::repeat_byte(1),
            )
            .is_err());

            assert_ok!(JuryAnchor::anchor_decision(
                RuntimeOrigin::signed(2),
                b"test".to_vec(),
                H256::repeat_byte(1),
            ));
        });
    }
}
