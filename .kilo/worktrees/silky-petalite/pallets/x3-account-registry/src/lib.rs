//! # X3 Account Registry Pallet
//!
//! Universal account registry for the X3 chain.
//!
//! This pallet tracks a canonical Atlas ID, optional account kind, and a
//! cross-VM nonce for replay prevention across EVM / SVM / X3-VM flows.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::Get};
    use frame_system::pallet_prelude::*;
    use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
    use scale_info::TypeInfo;
    #[cfg(feature = "std")]
    use serde::{Deserialize, Serialize};
    use sp_std::vec::Vec;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        type AtlasId: Parameter + Member + Default + Copy + MaxEncodedLen;

        #[pallet::constant]
        type MaxNameLength: Get<u32>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Classification of an account.
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
    #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
    pub enum AccountKind {
        Eoa,
        EvmContract,
        SvmProgram,
        X3AppZone,
        Validator,
        System,
    }

    #[pallet::type_value]
    pub fn DefaultForAccountCount() -> u64 {
        0
    }

    #[pallet::storage]
    #[pallet::getter(fn account_registry)]
    pub type AccountRegistry<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, T::AtlasId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn atlas_registry)]
    pub type AtlasRegistry<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AtlasId, T::AccountId, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn account_kind)]
    pub type AccountKinds<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, AccountKind, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn cross_vm_nonce)]
    pub type CrossVmNonces<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, u64, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn account_count)]
    pub type AccountCount<T> = StorageValue<_, u64, ValueQuery, DefaultForAccountCount>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        AccountRegistered {
            account: T::AccountId,
            atlas_id: T::AtlasId,
        },
        AccountDeregistered {
            account: T::AccountId,
            atlas_id: T::AtlasId,
        },
        NonceAnchored {
            account: T::AccountId,
            nonce: u64,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        AlreadyRegistered,
        NotRegistered,
        AtlasIdInUse,
        NameTooLong,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn register_account(
            origin: OriginFor<T>,
            atlas_id: T::AtlasId,
            kind: AccountKind,
            display_name: Vec<u8>,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                !AccountRegistry::<T>::contains_key(&who),
                Error::<T>::AlreadyRegistered
            );
            ensure!(
                !AtlasRegistry::<T>::contains_key(&atlas_id),
                Error::<T>::AtlasIdInUse
            );

            ensure!(
                display_name.len() <= T::MaxNameLength::get() as usize,
                Error::<T>::NameTooLong
            );

            AccountRegistry::<T>::insert(&who, atlas_id);
            AtlasRegistry::<T>::insert(&atlas_id, &who);
            AccountKinds::<T>::insert(&who, kind);
            AccountCount::<T>::mutate(|count| *count = count.saturating_add(1));

            Self::deposit_event(Event::AccountRegistered {
                account: who,
                atlas_id,
            });
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10_000)]
        pub fn deregister_account(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let atlas_id = AccountRegistry::<T>::take(&who).ok_or(Error::<T>::NotRegistered)?;

            AtlasRegistry::<T>::remove(&atlas_id);
            AccountKinds::<T>::remove(&who);
            AccountCount::<T>::mutate(|count| *count = count.saturating_sub(1));

            Self::deposit_event(Event::AccountDeregistered {
                account: who,
                atlas_id,
            });
            Ok(())
        }

        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn anchor_nonce(origin: OriginFor<T>) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(
                AccountRegistry::<T>::contains_key(&who),
                Error::<T>::NotRegistered
            );

            let nonce = CrossVmNonces::<T>::get(&who);
            Self::deposit_event(Event::NonceAnchored {
                account: who,
                nonce,
            });
            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        pub fn get_atlas_id(account: &T::AccountId) -> Option<T::AtlasId> {
            AccountRegistry::<T>::get(account)
        }

        pub fn get_account(atlas_id: T::AtlasId) -> Option<T::AccountId> {
            AtlasRegistry::<T>::get(atlas_id)
        }

        pub fn get_next_cross_vm_nonce(account: &T::AccountId) -> u64 {
            CrossVmNonces::<T>::get(account)
        }

        pub fn increment_cross_vm_nonce(account: &T::AccountId) {
            CrossVmNonces::<T>::mutate(account, |nonce| *nonce = nonce.saturating_add(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::pallet::*;
    use crate as pallet_x3_account_registry;
    use frame_support::{assert_noop, assert_ok, construct_runtime, parameter_types, traits::ConstU32};
    use frame_system as system;
    use sp_core::H256;
    use sp_io::TestExternalities;
    use sp_runtime::{
        traits::{BlakeTwo256, IdentityLookup},
        BuildStorage,
    };

    // ── Mock types ────────────────────────────────────────────────────────────

    type AccountId = u64;
    type AtlasId = u32;
    type Block = system::mocking::MockBlock<Test>;

    pub const ALICE: AccountId = 1;
    pub const BOB: AccountId = 2;
    pub const CHARLIE: AccountId = 3;

    pub const ATLAS_ALICE: AtlasId = 100;
    pub const ATLAS_BOB: AtlasId = 200;
    pub const ATLAS_CHARLIE: AtlasId = 300;

    construct_runtime!(
        pub enum Test {
            System: frame_system,
            AccountRegistry: pallet_x3_account_registry,
        }
    );

    parameter_types! {
        pub const BlockHashCount: u64 = 250;
        pub const MaxNameLength: u32 = 32;
    }

    impl system::Config for Test {
        type BaseCallFilter = frame_support::traits::Everything;
        type BlockWeights = ();
        type BlockLength = ();
        type DbWeight = ();
        type RuntimeOrigin = RuntimeOrigin;
        type RuntimeCall = RuntimeCall;
        type Nonce = u64;
        type Block = Block;
        type Hash = H256;
        type Hashing = BlakeTwo256;
        type AccountId = AccountId;
        type Lookup = IdentityLookup<AccountId>;
        type RuntimeEvent = RuntimeEvent;
        type BlockHashCount = BlockHashCount;
        type Version = ();
        type PalletInfo = PalletInfo;
        type AccountData = ();
        type OnNewAccount = ();
        type OnKilledAccount = ();
        type SystemWeightInfo = ();
        type ExtensionsWeightInfo = ();
        type SS58Prefix = ();
        type OnSetCode = ();
        type MaxConsumers = ConstU32<16>;
        type RuntimeTask = ();
        type SingleBlockMigrations = ();
        type MultiBlockMigrator = ();
        type PreInherents = ();
        type PostInherents = ();
        type PostTransactions = ();
    }

    impl Config for Test {
        type RuntimeEvent = RuntimeEvent;
        type AtlasId = AtlasId;
        type MaxNameLength = MaxNameLength;
    }

    fn new_test_ext() -> TestExternalities {
        system::GenesisConfig::<Test>::default()
            .build_storage()
            .unwrap()
            .into()
    }

    // ── register_account tests ─────────────────────────────────────────────

    #[test]
    fn register_account_success() {
        new_test_ext().execute_with(|| {
            let name = b"alice".to_vec();
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                name,
            ));

            // Both maps populated.
            assert_eq!(AccountRegistry::account_registry(ALICE), Some(ATLAS_ALICE));
            assert_eq!(AccountRegistry::atlas_registry(ATLAS_ALICE), Some(ALICE));
            assert_eq!(AccountRegistry::account_kind(ALICE), Some(AccountKind::Eoa));
            assert_eq!(AccountRegistry::account_count(), 1);
        });
    }

    #[test]
    fn register_account_increments_count() {
        new_test_ext().execute_with(|| {
            assert_eq!(AccountRegistry::account_count(), 0);
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            assert_eq!(AccountRegistry::account_count(), 1);
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(BOB),
                ATLAS_BOB,
                AccountKind::Validator,
                b"bob".to_vec(),
            ));
            assert_eq!(AccountRegistry::account_count(), 2);
        });
    }

    #[test]
    fn register_account_emits_event() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            System::assert_last_event(
                Event::AccountRegistered {
                    account: ALICE,
                    atlas_id: ATLAS_ALICE,
                }
                .into(),
            );
        });
    }

    #[test]
    fn register_account_fails_if_already_registered() {
        new_test_ext().execute_with(|| {
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            // Same account, different atlas id — must fail.
            assert_noop!(
                AccountRegistry::register_account(
                    RuntimeOrigin::signed(ALICE),
                    ATLAS_BOB,
                    AccountKind::Eoa,
                    b"alice2".to_vec(),
                ),
                Error::<Test>::AlreadyRegistered
            );
        });
    }

    #[test]
    fn register_account_fails_if_atlas_id_in_use() {
        new_test_ext().execute_with(|| {
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            // Different account, same atlas id — must fail.
            assert_noop!(
                AccountRegistry::register_account(
                    RuntimeOrigin::signed(BOB),
                    ATLAS_ALICE,
                    AccountKind::Eoa,
                    b"bob".to_vec(),
                ),
                Error::<Test>::AtlasIdInUse
            );
        });
    }

    #[test]
    fn register_account_fails_if_name_too_long() {
        new_test_ext().execute_with(|| {
            // MaxNameLength = 32; send 33 bytes.
            let long_name = vec![b'x'; 33];
            assert_noop!(
                AccountRegistry::register_account(
                    RuntimeOrigin::signed(ALICE),
                    ATLAS_ALICE,
                    AccountKind::Eoa,
                    long_name,
                ),
                Error::<Test>::NameTooLong
            );
            // Storage must remain clean.
            assert!(AccountRegistry::account_registry(ALICE).is_none());
            assert_eq!(AccountRegistry::account_count(), 0);
        });
    }

    #[test]
    fn register_account_allows_max_length_name() {
        new_test_ext().execute_with(|| {
            let max_name = vec![b'x'; 32];
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                max_name,
            ));
        });
    }

    #[test]
    fn register_all_account_kinds() {
        new_test_ext().execute_with(|| {
            let kinds = [
                (ALICE, ATLAS_ALICE, AccountKind::Eoa),
                (BOB, ATLAS_BOB, AccountKind::EvmContract),
                (CHARLIE, ATLAS_CHARLIE, AccountKind::SvmProgram),
            ];
            for (acc, atlas, kind) in kinds {
                assert_ok!(AccountRegistry::register_account(
                    RuntimeOrigin::signed(acc),
                    atlas,
                    kind,
                    b"name".to_vec(),
                ));
                assert_eq!(AccountRegistry::account_kind(acc), Some(kind));
            }
        });
    }

    // ── deregister_account tests ───────────────────────────────────────────

    #[test]
    fn deregister_account_cleans_all_storage() {
        new_test_ext().execute_with(|| {
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            assert_eq!(AccountRegistry::account_count(), 1);

            assert_ok!(AccountRegistry::deregister_account(
                RuntimeOrigin::signed(ALICE)
            ));

            assert!(AccountRegistry::account_registry(ALICE).is_none());
            assert!(AccountRegistry::atlas_registry(ATLAS_ALICE).is_none());
            assert!(AccountRegistry::account_kind(ALICE).is_none());
            assert_eq!(AccountRegistry::account_count(), 0);
        });
    }

    #[test]
    fn deregister_account_emits_event() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            assert_ok!(AccountRegistry::deregister_account(
                RuntimeOrigin::signed(ALICE)
            ));
            System::assert_last_event(
                Event::AccountDeregistered {
                    account: ALICE,
                    atlas_id: ATLAS_ALICE,
                }
                .into(),
            );
        });
    }

    #[test]
    fn deregister_account_fails_if_not_registered() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                AccountRegistry::deregister_account(RuntimeOrigin::signed(ALICE)),
                Error::<Test>::NotRegistered
            );
        });
    }

    #[test]
    fn deregister_frees_atlas_id_for_reuse() {
        new_test_ext().execute_with(|| {
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            assert_ok!(AccountRegistry::deregister_account(
                RuntimeOrigin::signed(ALICE)
            ));
            // BOB should now be able to claim ATLAS_ALICE.
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(BOB),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"bob".to_vec(),
            ));
            assert_eq!(AccountRegistry::atlas_registry(ATLAS_ALICE), Some(BOB));
        });
    }

    #[test]
    fn deregister_does_not_affect_other_accounts() {
        new_test_ext().execute_with(|| {
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(BOB),
                ATLAS_BOB,
                AccountKind::Eoa,
                b"bob".to_vec(),
            ));
            assert_ok!(AccountRegistry::deregister_account(
                RuntimeOrigin::signed(ALICE)
            ));
            // BOB must be untouched.
            assert_eq!(AccountRegistry::account_registry(BOB), Some(ATLAS_BOB));
            assert_eq!(AccountRegistry::account_count(), 1);
        });
    }

    // ── anchor_nonce tests ─────────────────────────────────────────────────

    #[test]
    fn anchor_nonce_emits_event_with_current_nonce() {
        new_test_ext().execute_with(|| {
            System::set_block_number(1);
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            // Nonce starts at 0.
            assert_ok!(AccountRegistry::anchor_nonce(RuntimeOrigin::signed(ALICE)));
            System::assert_last_event(
                Event::NonceAnchored {
                    account: ALICE,
                    nonce: 0,
                }
                .into(),
            );
        });
    }

    #[test]
    fn anchor_nonce_fails_if_not_registered() {
        new_test_ext().execute_with(|| {
            assert_noop!(
                AccountRegistry::anchor_nonce(RuntimeOrigin::signed(ALICE)),
                Error::<Test>::NotRegistered
            );
        });
    }

    // ── helper / public API tests ──────────────────────────────────────────

    #[test]
    fn get_atlas_id_returns_none_for_unregistered() {
        new_test_ext().execute_with(|| {
            assert!(AccountRegistry::get_atlas_id(&ALICE).is_none());
        });
    }

    #[test]
    fn get_atlas_id_returns_value_after_registration() {
        new_test_ext().execute_with(|| {
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            assert_eq!(AccountRegistry::get_atlas_id(&ALICE), Some(ATLAS_ALICE));
        });
    }

    #[test]
    fn get_account_returns_account_for_known_atlas_id() {
        new_test_ext().execute_with(|| {
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            assert_eq!(AccountRegistry::get_account(ATLAS_ALICE), Some(ALICE));
        });
    }

    #[test]
    fn cross_vm_nonce_starts_at_zero() {
        new_test_ext().execute_with(|| {
            assert_eq!(AccountRegistry::get_next_cross_vm_nonce(&ALICE), 0);
        });
    }

    #[test]
    fn increment_cross_vm_nonce_increments_correctly() {
        new_test_ext().execute_with(|| {
            AccountRegistry::increment_cross_vm_nonce(&ALICE);
            assert_eq!(AccountRegistry::get_next_cross_vm_nonce(&ALICE), 1);
            AccountRegistry::increment_cross_vm_nonce(&ALICE);
            assert_eq!(AccountRegistry::get_next_cross_vm_nonce(&ALICE), 2);
        });
    }

    #[test]
    fn increment_nonce_does_not_overflow() {
        new_test_ext().execute_with(|| {
            CrossVmNonces::<Test>::insert(ALICE, u64::MAX);
            // saturating_add must not wrap.
            AccountRegistry::increment_cross_vm_nonce(&ALICE);
            assert_eq!(AccountRegistry::get_next_cross_vm_nonce(&ALICE), u64::MAX);
        });
    }

    #[test]
    fn nonces_are_per_account() {
        new_test_ext().execute_with(|| {
            AccountRegistry::increment_cross_vm_nonce(&ALICE);
            AccountRegistry::increment_cross_vm_nonce(&ALICE);
            assert_eq!(AccountRegistry::get_next_cross_vm_nonce(&ALICE), 2);
            assert_eq!(AccountRegistry::get_next_cross_vm_nonce(&BOB), 0);
        });
    }

    // ── replay protection invariant ────────────────────────────────────────

    #[test]
    fn atlas_id_uniqueness_is_enforced_across_registrations() {
        new_test_ext().execute_with(|| {
            // Register ALICE with ATLAS_ALICE.
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_ALICE,
                AccountKind::Eoa,
                b"alice".to_vec(),
            ));
            // Deregister ALICE.
            assert_ok!(AccountRegistry::deregister_account(
                RuntimeOrigin::signed(ALICE)
            ));
            // Re-register ALICE with ATLAS_BOB — different atlas id.
            assert_ok!(AccountRegistry::register_account(
                RuntimeOrigin::signed(ALICE),
                ATLAS_BOB,
                AccountKind::Eoa,
                b"alice-v2".to_vec(),
            ));
            assert_eq!(AccountRegistry::get_atlas_id(&ALICE), Some(ATLAS_BOB));
            // ATLAS_ALICE must be free.
            assert!(AccountRegistry::get_account(ATLAS_ALICE).is_none());
        });
    }
}
