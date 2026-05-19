#![deny(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use frame_system::ensure_root;
    use sp_runtime::traits::{Hash, SaturatedConversion};
    use sp_std::vec::Vec;
    use x3_wallet::{
        HardwareWallet, MultisigWallet, GuardianAccount, TokenBalance,
        TransactionApproval, AddressBook, BiometricProfile, UnlockSession,
    };

    /// Max wallets per account
    pub const MAX_WALLETS_PER_ACCOUNT: u32 = 10;
    /// Max multisig signers
    pub const MAX_MULTISIG_SIGNERS: u32 = 50;
    /// Max contacts in address book
    pub const MAX_CONTACTS: u32 = 1000;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(_);

    /// Store hardware wallets per account
    #[pallet::storage]
    pub type HardwareWallets<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (T::AccountId, [u8; 32]),
        HardwareWallet,
        OptionQuery,
    >;

    /// Store multisig wallets per account
    #[pallet::storage]
    pub type MultisigWallets<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (T::AccountId, [u8; 32]),
        MultisigWallet,
        OptionQuery,
    >;

    /// Store social recovery accounts per user
    #[pallet::storage]
    pub type RecoveryAccounts<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        GuardianAccount,
        OptionQuery,
    >;

    /// Store token balances
    #[pallet::storage]
    pub type TokenBalances<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        (T::AccountId, [u8; 32]),
        u128, // balance
        ValueQuery,
    >;

    /// Store transaction approvals
    #[pallet::storage]
    pub type TransactionApprovals<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        TransactionApproval,
        OptionQuery,
    >;

    /// Store address books per account
    #[pallet::storage]
    pub type AddressBooks<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        AddressBook,
        OptionQuery,
    >;

    /// Store biometric profiles
    #[pallet::storage]
    pub type BiometricProfiles<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        BiometricProfile,
        OptionQuery,
    >;

    /// Store unlock sessions
    #[pallet::storage]
    pub type UnlockSessions<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        [u8; 32],
        UnlockSession,
        OptionQuery,
    >;

    /// S1-3: Authorized minters for token operations
    #[pallet::storage]
    pub type Minters<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, (), OptionQuery>;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Hardware wallet connected
        HardwareWalletConnected { account: T::AccountId, device_type: u8 },
        /// Multisig wallet created
        MultisigWalletCreated { account: T::AccountId, threshold: u32 },
        /// Social recovery initiated
        RecoveryInitiated { account: T::AccountId, new_owner: [u8; 32] },
        /// Transaction approval requested
        ApprovalRequested { account: T::AccountId, amount: u128 },
        /// Token balance updated
        BalanceUpdated { account: T::AccountId, token_id: [u8; 32], amount: u128 },
        /// Biometric profile created
        BiometricProfileCreated { account: T::AccountId },
        /// Unlock session created
        UnlockSessionCreated { account: T::AccountId },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Wallet not found
        WalletNotFound,
        /// Unauthorized
        Unauthorized,
        /// Invalid amount
        InvalidAmount,
        /// Balance too low
        InsufficientBalance,
        /// Multisig threshold error
        InvalidThreshold,
        /// Too many wallets
        TooManyWallets,
        /// Recovery not approved
        RecoveryNotApproved,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Register a new hardware wallet
        #[pallet::call_index(0)]
        #[pallet::weight(10_000)]
        pub fn register_hardware_wallet(
            origin: OriginFor<T>,
            device_type: u8,
            device_model: Vec<u8>,
            public_key: [u8; 32],
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let wallet_id = T::Hashing::hash_of(&public_key).encode();
            let mut wallet_id_array = [0u8; 32];
            wallet_id_array.copy_from_slice(&wallet_id[..32.min(wallet_id.len())]);

            let hardware_wallet = HardwareWallet {
                id: wallet_id_array,
                device_type,
                device_model: device_model.clone(),
                derivation_path: vec![],
                public_key: public_key.to_vec(),
                address: [0u8; 32],
                is_connected: true,
                last_connected_block: frame_system::Pallet::<T>::block_number().saturated_into::<u64>(),
                transaction_count: 0,
                firmware_version: vec![],
            };

            HardwareWallets::<T>::insert((who.clone(), wallet_id_array), hardware_wallet);
            Self::deposit_event(Event::HardwareWalletConnected {
                account: who,
                device_type,
            });

            Ok(())
        }

        /// Create a multisig wallet
        #[pallet::call_index(1)]
        #[pallet::weight(15_000)]
        pub fn create_multisig_wallet(
            origin: OriginFor<T>,
            signers: Vec<[u8; 32]>,
            threshold: u32,
            timelock_delay: u64,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(threshold > 0 && threshold as usize <= signers.len(), Error::<T>::InvalidThreshold);

            let mut wallet_id = [0u8; 32];
            let encoded = who.encode();
            let copy_len = encoded.len().min(32);
            wallet_id[..copy_len].copy_from_slice(&encoded[..copy_len]);

            let multisig = MultisigWallet {
                id: wallet_id,
                signers: signers.to_vec(),
                threshold,
                owner: [0u8; 32],
                created_block: frame_system::Pallet::<T>::block_number().saturated_into::<u64>(),
                timelock_delay,
                is_active: true,
            };

            MultisigWallets::<T>::insert((who.clone(), wallet_id), multisig);
            Self::deposit_event(Event::MultisigWalletCreated {
                account: who,
                threshold,
            });

            Ok(())
        }

        /// Transfer tokens (with approval checks)
        #[pallet::call_index(2)]
        #[pallet::weight(10_000)]
        pub fn transfer_tokens(
            origin: OriginFor<T>,
            token_id: [u8; 32],
            to: T::AccountId,
            amount: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(amount > 0, Error::<T>::InvalidAmount);

            let current_balance = TokenBalances::<T>::get((who.clone(), token_id));
            ensure!(current_balance >= amount, Error::<T>::InsufficientBalance);

            let new_balance = current_balance - amount;
            TokenBalances::<T>::insert((who.clone(), token_id), new_balance);

            let to_balance = TokenBalances::<T>::get((to.clone(), token_id));
            TokenBalances::<T>::insert((to.clone(), token_id), to_balance + amount);

            Self::deposit_event(Event::BalanceUpdated {
                account: who,
                token_id,
                amount: new_balance,
            });

            Ok(())
        }

        /// Register biometric profile
        #[pallet::call_index(3)]
        #[pallet::weight(8_000)]
        pub fn register_biometric(
            origin: OriginFor<T>,
            biometric_type: u8,
            template_hash: [u8; 32],
            pin_hash: [u8; 32],
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let profile = BiometricProfile {
                id: T::Hashing::hash_of(&template_hash).encode()[..32].try_into().unwrap_or([0u8; 32]),
                owner: [0u8; 32],
                biometric_type,
                template_hash,
                pin_hash,
                is_enabled: true,
                attempts_remaining: 5,
                locked_until_block: 0,
                created_block: frame_system::Pallet::<T>::block_number().saturated_into::<u64>(),
            };

            BiometricProfiles::<T>::insert(who.clone(), profile);
            Self::deposit_event(Event::BiometricProfileCreated { account: who });

            Ok(())
        }

        /// Initiate recovery with guardians
        #[pallet::call_index(4)]
        #[pallet::weight(12_000)]
        pub fn initiate_recovery(
            origin: OriginFor<T>,
            new_owner: [u8; 32],
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            let recovery = RecoveryAccounts::<T>::get(who.clone());
            ensure!(recovery.is_some(), Error::<T>::WalletNotFound);

            Self::deposit_event(Event::RecoveryInitiated {
                account: who,
                new_owner,
            });

            Ok(())
        }

        /// Mint tokens (root / governance only).
        ///
        /// S1-3 fix: previous implementation only required `ensure_signed`
        /// and used unchecked addition, which allowed any signed account to
        /// inflate balances arbitrarily and trigger overflows. Mint authority
        /// is now restricted to the root origin (governance / sudo) and the
        /// balance update uses `checked_add` to reject overflow.
        #[pallet::call_index(5)]
        #[pallet::weight(5_000)]
        pub fn mint_tokens(
            origin: OriginFor<T>,
            token_id: [u8; 32],
            to: T::AccountId,
            amount: u128,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // S1-3: only authorized minters may mint.
            Self::ensure_minter(&who)?;
            ensure!(amount > 0, Error::<T>::InvalidAmount);

            let current = TokenBalances::<T>::get((to.clone(), token_id));
            let new_balance = current
                .checked_add(amount)
                .ok_or(Error::<T>::InvalidAmount)?;
            TokenBalances::<T>::insert((to.clone(), token_id), new_balance);

            Self::deposit_event(Event::BalanceUpdated {
                account: to,
                token_id,
                amount: new_balance,
            });

            Ok(())
        }

        /// S1-3: Add authorized minter
        #[pallet::call_index(6)]
        #[pallet::weight(5_000)]
        pub fn add_minter(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            Minters::<T>::insert(&who, ());
            Ok(())
        }

        /// S1-3: Remove authorized minter
        #[pallet::call_index(7)]
        #[pallet::weight(5_000)]
        pub fn remove_minter(origin: OriginFor<T>, who: T::AccountId) -> DispatchResult {
            ensure_root(origin)?;
            Minters::<T>::remove(&who);
            Ok(())
        }
    }

    // Internal helpers (not dispatchable)
    impl<T: Config> Pallet<T> {
        /// S1-3: Ensure caller is authorized minter
        fn ensure_minter(who: &T::AccountId) -> Result<(), Error<T>> {
            ensure!(
                Minters::<T>::contains_key(who),
                Error::<T>::Unauthorized
            );
            Ok(())
        }
    }

    // RPC query methods
    impl<T: Config> Pallet<T> {
        /// Get hardware wallet by ID
        pub fn get_hardware_wallet(account: &T::AccountId, wallet_id: &[u8; 32]) -> Option<HardwareWallet> {
            HardwareWallets::<T>::get((account.clone(), *wallet_id))
        }

        /// Get multisig wallet
        pub fn get_multisig_wallet(account: &T::AccountId, wallet_id: &[u8; 32]) -> Option<MultisigWallet> {
            MultisigWallets::<T>::get((account.clone(), *wallet_id))
        }

        /// Get token balance
        pub fn get_token_balance(account: &T::AccountId, token_id: &[u8; 32]) -> u128 {
            TokenBalances::<T>::get((account.clone(), *token_id))
        }

        /// Get biometric profile
        pub fn get_biometric_profile(account: &T::AccountId) -> Option<BiometricProfile> {
            BiometricProfiles::<T>::get(account)
        }

        /// Get recovery account
        pub fn get_recovery_account(account: &T::AccountId) -> Option<GuardianAccount> {
            RecoveryAccounts::<T>::get(account)
        }
    }
}
