//! X3 Atomic Swap HTLC — Solana BPF Program
//!
//! Implements Hashed Timelock Contracts (HTLC) for cross-chain atomic swaps
//! on the Solana blockchain. Lock accounts use PDA derivation with seeds
//! `["htlc", swap_id]`.
//!
//! # Instructions
//!
//! | Tag | Instruction    | Description                                     |
//! |-----|----------------|-------------------------------------------------|
//! | 0   | `CreateHtlc`   | Create a new HTLC lock account                  |
//! | 1   | `ClaimHtlc`    | Claim locked funds by revealing the preimage    |
//! | 2   | `RefundHtlc`   | Refund locked funds after timeout expires       |
//!
//! # Account Data Layout (HtlcAccount, 211 bytes)
//!
//! | Offset | Size | Field            | Description                               |
//! |--------|------|------------------|-------------------------------------------|
//! | 0      | 32   | initializer      | Pubkey who locked funds                   |
//! | 32     | 32   | claimant         | Pubkey authorized to claim                |
//! | 64     | 32   | refund_authority | Pubkey authorized to refund after timeout  |
//! | 96     | 32   | hashlock         | SHA-256 hash of the preimage              |
//! | 128    | 32   | token_mint       | Token mint (all-zeros for SOL)            |
//! | 160    | 8    | amount           | Locked amount (u64 LE)                    |
//! | 168    | 8    | timeout          | Slot number after which refund is allowed  |
//! | 176    | 1    | bump_seed        | PDA bump seed                             |
//! | 177    | 32   | swap_id          | Unique swap identifier                    |
//! | 209    | 1    | claimed          | 1 if claimed, 0 otherwise                 |
//! | 210    | 1    | refunded         | 1 if refunded, 0 otherwise                |
//!
//! # CPI Integration
//!
//! Other Solana programs can invoke this HTLC program using the CPI helper
//! functions in [`cpi`] module. See the [`cpi`] module documentation for
//! details on constructing CPI instructions.

#![no_std]
#![deny(unsafe_code)]

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint {
    use solana_program::{
        account_info::AccountInfo,
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    };

    entrypoint!(process_instruction);

    /// Program entrypoint.
    ///
    /// Delegates to [`processor::process`] for instruction dispatch.
    fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        msg!("X3 Atomic Swap HTLC: processing instruction");
        processor::process(program_id, accounts, instruction_data)
    }
}

/// Program-specific error codes for the HTLC program.
///
/// These map to [`solana_program::program_error::ProgramError::Custom`]
/// with the discriminant as the custom error code.
pub mod error {
    use solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    };

    /// Error codes for the X3 Atomic Swap HTLC program.
    ///
    /// Each variant is logged via a human-readable prefix when the error
    /// occurs through the [`PrintProgramError`] implementation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum HtlcError {
        /// The preimage provided does not match the stored hashlock.
        WrongPreimage = 0,
        /// The computed SHA-256 hash does not match the hashlock field.
        HashlockMismatch = 1,
        /// The caller is not the authorized claimant for this HTLC.
        UnauthorizedClaimant = 2,
        /// The caller is not the authorized refund authority for this HTLC.
        UnauthorizedRefund = 3,
        /// The HTLC timelock has expired; use refund instead of claim.
        HtlcExpired = 4,
        /// The HTLC timelock has not yet expired; cannot refund yet.
        HtlcNotExpired = 5,
        /// The HTLC has already been claimed.
        HtlcAlreadyClaimed = 6,
        /// The HTLC has already been refunded.
        HtlcAlreadyRefunded = 7,
        /// The amount provided is zero or exceeds allowed limits.
        InvalidAmount = 8,
        /// The timelock value is in the past (less than current slot).
        TimelockInPast = 9,
        /// The timelock value is unreasonably far in the future.
        TimelockTooFar = 10,
        /// Arithmetic overflow or underflow occurred.
        Overflow = 11,
    }

    impl HtlcError {
        /// Human-readable description of each error variant.
        pub fn msg(&self) -> &'static str {
            match self {
                Self::WrongPreimage => "WrongPreimage: preimage does not match stored hashlock",
                Self::HashlockMismatch => "HashlockMismatch: computed hash does not equal stored hashlock",
                Self::UnauthorizedClaimant => "UnauthorizedClaimant: caller is not the authorized claimant",
                Self::UnauthorizedRefund => "UnauthorizedRefund: caller is not the refund authority",
                Self::HtlcExpired => "HtlcExpired: timelock has expired, cannot claim",
                Self::HtlcNotExpired => "HtlcNotExpired: timelock has not expired, cannot refund",
                Self::HtlcAlreadyClaimed => "HtlcAlreadyClaimed: HTLC has already been claimed",
                Self::HtlcAlreadyRefunded => "HtlcAlreadyRefunded: HTLC has already been refunded",
                Self::InvalidAmount => "InvalidAmount: amount must be greater than zero",
                Self::TimelockInPast => "TimelockInPast: timelock must be greater than current slot",
                Self::TimelockTooFar => "TimelockTooFar: timelock exceeds maximum allowed offset",
                Self::Overflow => "Overflow: arithmetic overflow or underflow",
            }
        }
    }

    impl From<HtlcError> for ProgramError {
        fn from(e: HtlcError) -> Self {
            ProgramError::Custom(e as u32)
        }
    }

    impl<T> DecodeError<T> for HtlcError {
        fn type_of() -> &'static str {
            "HtlcError"
        }
    }

    impl PrintProgramError for HtlcError {
        fn print<E>(&self) {
            msg!("X3 HTLC Error: {}", self.msg());
        }
    }
}

/// Instruction processor and handler functions.
pub mod processor {
    use solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        hash::hashv,
        msg,
        program::invoke,
        program_error::ProgramError,
        pubkey::Pubkey,
        system_instruction,
        sysvar::{clock::Clock, rent::Rent, Sysvar},
    };

    use crate::error::HtlcError;
    use crate::state::{
        HtlcAccount, HtlcInstruction, HTLC_ACCOUNT_SEED, HTLC_ACCOUNT_SIZE,
    };

    /// Main instruction dispatcher.
    ///
    /// Reads the first byte of `instruction_data` as an instruction tag:
    /// - `0` → [`handle_create_htlc`]
    /// - `1` → [`handle_claim_htlc`]
    /// - `2` → [`handle_refund_htlc`]
    /// - Any other tag → [`ProgramError::InvalidInstructionData`]
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let (tag, rest) = instruction_data
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        match tag {
            0 => handle_create_htlc(program_id, accounts, rest),
            1 => handle_claim_htlc(program_id, accounts, rest),
            2 => handle_refund_htlc(program_id, accounts, rest),
            _ => {
                msg!("Error: Unknown instruction tag {}", tag);
                Err(ProgramError::InvalidInstructionData)
            }
        }
    }

    /// Instruction 0: `CreateHtlc`
    ///
    /// Creates a new PDA lock account holding HTLC state. The PDA is derived
    /// from seeds `["htlc", swap_id]` and the program ID.
    ///
    /// ### Accounts (in order)
    ///
    /// | Index | Writable | Signer | Account          | Description                         |
    /// |-------|----------|--------|------------------|-------------------------------------|
    /// | 0     | ✅       | ❌     | `htlc_account`   | PDA to create (seeds: ["htlc", swap_id]) |
    /// | 1     | ✅       | ✅     | `payer`          | Account funding rent                 |
    /// | 2     | ❌       | ✅     | `initializer`    | Party locking funds                  |
    /// | 3     | ❌       | ❌     | `system_program` | System program                      |
    ///
    /// ### Data (after 1-byte tag)
    ///
    /// | Offset | Size | Field            | Description                         |
    /// |--------|------|------------------|-------------------------------------|
    /// | 0      | 32   | swap_id          | Unique swap identifier              |
    /// | 32     | 32   | claimant         | Pubkey authorized to claim          |
    /// | 64     | 32   | refund_authority | Pubkey authorized to refund         |
    /// | 96     | 32   | hashlock         | SHA-256 hash of the preimage        |
    /// | 128    | 32   | token_mint       | Token mint (Pubkey::default() for SOL) |
    /// | 160    | 8    | amount           | Locked amount (u64 LE)              |
    /// | 168    | 8    | timeout          | Slot after which refund is allowed   |
    fn handle_create_htlc(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let htlc_info = next_account_info(accounts_iter)?;
        let payer = next_account_info(accounts_iter)?;
        let initializer = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        if !initializer.is_signer {
            msg!("Error: initializer must sign");
            return Err(ProgramError::MissingRequiredSignature);
        }
        if !payer.is_signer {
            msg!("Error: payer must sign");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Deserialize instruction data
        let create = HtlcInstruction::unpack_create(data)?;

        // Validate timelock is in the future
        let clock = Clock::get()?;
        if create.timeout <= clock.slot {
            msg!("Error: timelock must be greater than current slot");
            return Err(HtlcError::TimelockInPast.into());
        }

        // Validate amount
        if create.amount == 0 {
            msg!("Error: amount must be > 0");
            return Err(HtlcError::InvalidAmount.into());
        }

        // Derive PDA
        let (expected_pda, bump) =
            Pubkey::find_program_address(&[HTLC_ACCOUNT_SEED, &create.swap_id], program_id);
        if htlc_info.key != &expected_pda {
            msg!("Error: HTLC account is not the expected PDA");
            return Err(ProgramError::InvalidArgument);
        }

        // Create the account
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(HTLC_ACCOUNT_SIZE);

        invoke(
            &system_instruction::create_account(
                payer.key,
                htlc_info.key,
                lamports,
                HTLC_ACCOUNT_SIZE as u64,
                program_id,
            ),
            &[
                payer.clone(),
                htlc_info.clone(),
                system_program.clone(),
            ],
        )?;

        // Initialize account data
        let account = HtlcAccount {
            initializer: *initializer.key,
            claimant: create.claimant,
            refund_authority: create.refund_authority,
            hashlock: create.hashlock,
            token_mint: create.token_mint,
            amount: create.amount,
            timeout: create.timeout,
            bump_seed: bump,
            swap_id: create.swap_id,
            claimed: false,
            refunded: false,
        };

        let mut dst = htlc_info.try_borrow_mut_data()?;
        account.serialize_into(&mut dst);

        // Emit event via log data
        let event_parts = [
            &[0u8; 1], // event tag: Locked
            create.swap_id.as_ref(),
            htlc_info.key.as_ref(),
            initializer.key.as_ref(),
            create.claimant.as_ref(),
            create.refund_authority.as_ref(),
            &create.amount.to_le_bytes(),
            create.hashlock.as_ref(),
            create.token_mint.as_ref(),
        ];
        solana_program::log::sol_log_data(&event_parts);

        msg!(
            "HTLC created: swap_id={:?}, amount={}, timeout={}",
            create.swap_id,
            create.amount,
            create.timeout,
        );
        Ok(())
    }

    /// Instruction 1: `ClaimHtlc`
    ///
    /// Verifies the SHA-256 hash of the provided preimage matches the stored
    /// hashlock, then marks the HTLC as claimed. Only the authorized claimant
    /// may call this instruction, and only before the timelock expires.
    ///
    /// ### Accounts (in order)
    ///
    /// | Index | Writable | Signer | Account          | Description                         |
    /// |-------|----------|--------|------------------|-------------------------------------|
    /// | 0     | ✅       | ❌     | `htlc_account`   | PDA lock account                    |
    /// | 1     | ❌       | ✅     | `claimant`       | Authorized claimant                 |
    ///
    /// ### Data (after 1-byte tag)
    ///
    /// | Offset | Size     | Field      | Description                         |
    /// |--------|----------|------------|-------------------------------------|
    /// | 0      | 1        | len        | Length of preimage (1-255)          |
    /// | 1      | len      | preimage   | The preimage bytes                  |
    fn handle_claim_htlc(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let htlc_info = next_account_info(accounts_iter)?;
        let claimant = next_account_info(accounts_iter)?;

        if !claimant.is_signer {
            msg!("Error: claimant must sign");
            return Err(ProgramError::MissingRequiredSignature);
        }

        let account_data = htlc_info.try_borrow_data()?;
        let htlc = HtlcAccount::deserialize_from(&account_data)?;

        // Verify PDA matches
        let (expected_pda, _) =
            Pubkey::find_program_address(&[HTLC_ACCOUNT_SEED, &htlc.swap_id], program_id);
        if htlc_info.key != &expected_pda {
            msg!("Error: HTLC account PDA mismatch");
            return Err(ProgramError::InvalidArgument);
        }

        // Check not already claimed/refunded
        if htlc.claimed {
            msg!("Error: HTLC already claimed");
            return Err(HtlcError::HtlcAlreadyClaimed.into());
        }
        if htlc.refunded {
            msg!("Error: HTLC already refunded");
            return Err(HtlcError::HtlcAlreadyRefunded.into());
        }

        // Verify claimant
        if claimant.key != &htlc.claimant {
            msg!("Error: caller is not the authorized claimant");
            return Err(HtlcError::UnauthorizedClaimant.into());
        }

        // Check timeout — must not have expired for claim
        let clock = Clock::get()?;
        let current_slot = clock.slot;
        if htlc.timeout > 0 && current_slot > htlc.timeout {
            msg!("Error: timeout has expired, use refund instead");
            return Err(HtlcError::HtlcExpired.into());
        }

        // Verify preimage
        if data.is_empty() {
            msg!("Error: preimage is empty");
            return Err(ProgramError::InvalidInstructionData);
        }
        let preimage_len = data[0] as usize;
        if preimage_len == 0 || preimage_len > 255 || 1 + preimage_len > data.len() {
            msg!("Error: invalid preimage length");
            return Err(ProgramError::InvalidInstructionData);
        }
        let preimage = &data[1..1 + preimage_len];

        let computed_hash = hashv(&[preimage]).to_bytes();
        if computed_hash != htlc.hashlock {
            msg!("Error: hashlock mismatch: preimage does not match hashlock");
            return Err(HtlcError::WrongPreimage.into());
        }

        // Mark as claimed
        drop(account_data);
        let mut data_mut = htlc_info.try_borrow_mut_data()?;
        data_mut[HtlcAccount::CLAIMED_OFFSET] = 1;

        // Emit event
        let event_parts = [
            &[1u8; 1], // event tag: Claimed
            htlc.swap_id.as_ref(),
            htlc_info.key.as_ref(),
            claimant.key.as_ref(),
        ];
        solana_program::log::sol_log_data(&event_parts);

        msg!("HTLC claimed: swap_id={:?}", htlc.swap_id);
        Ok(())
    }

    /// Instruction 2: `RefundHtlc`
    ///
    /// Marks the HTLC as refunded after the timelock has expired. Only the
    /// authorized refund authority may call this instruction.
    ///
    /// ### Accounts (in order)
    ///
    /// | Index | Writable | Signer | Account             | Description                         |
    /// |-------|----------|--------|---------------------|-------------------------------------|
    /// | 0     | ✅       | ❌     | `htlc_account`      | PDA lock account                    |
    /// | 1     | ❌       | ✅     | `refund_authority`  | Authorized refund authority         |
    ///
    /// ### Data
    ///
    /// No additional data beyond the 1-byte tag is required.
    fn handle_refund_htlc(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        _data: &[u8],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let htlc_info = next_account_info(accounts_iter)?;
        let refund_authority = next_account_info(accounts_iter)?;

        if !refund_authority.is_signer {
            msg!("Error: refund authority must sign");
            return Err(ProgramError::MissingRequiredSignature);
        }

        let account_data = htlc_info.try_borrow_data()?;
        let htlc = HtlcAccount::deserialize_from(&account_data)?;

        // Verify PDA
        let (expected_pda, _) =
            Pubkey::find_program_address(&[HTLC_ACCOUNT_SEED, &htlc.swap_id], program_id);
        if htlc_info.key != &expected_pda {
            msg!("Error: HTLC account PDA mismatch");
            return Err(ProgramError::InvalidArgument);
        }

        // Check not already claimed/refunded
        if htlc.claimed {
            msg!("Error: HTLC already claimed");
            return Err(HtlcError::HtlcAlreadyClaimed.into());
        }
        if htlc.refunded {
            msg!("Error: HTLC already refunded");
            return Err(HtlcError::HtlcAlreadyRefunded.into());
        }

        // Verify refund authority
        if refund_authority.key != &htlc.refund_authority {
            msg!("Error: caller is not the refund authority");
            return Err(HtlcError::UnauthorizedRefund.into());
        }

        // Check timeout expired
        let clock = Clock::get()?;
        let current_slot = clock.slot;
        if htlc.timeout > 0 && current_slot <= htlc.timeout {
            msg!("Error: timeout has not yet expired");
            return Err(HtlcError::HtlcNotExpired.into());
        }

        // Mark as refunded
        drop(account_data);
        let mut data_mut = htlc_info.try_borrow_mut_data()?;
        data_mut[HtlcAccount::REFUNDED_OFFSET] = 1;

        // Emit event
        let event_parts = [
            &[2u8; 1], // event tag: Refunded
            htlc.swap_id.as_ref(),
            htlc_info.key.as_ref(),
            refund_authority.key.as_ref(),
        ];
        solana_program::log::sol_log_data(&event_parts);

        msg!("HTLC refunded: swap_id={:?}", htlc.swap_id);
        Ok(())
    }
}

/// HTLC account state and instruction data types.
pub mod state {
    use solana_program::{
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    };

    /// Seed prefix for HTLC PDA derivation.
    ///
    /// PDAs are derived as:
    /// ```ignore
    /// Pubkey::find_program_address(&[b"htlc", &swap_id], program_id)
    /// ```
    pub const HTLC_ACCOUNT_SEED: &[u8] = b"htlc";

    /// Total byte size of a serialized [`HtlcAccount`].
    ///
    /// Layout: initializer(32) + claimant(32) + refund_authority(32) + hashlock(32)
    ///         + token_mint(32) + amount(8) + timeout(8) + bump_seed(1) + swap_id(32)
    ///         + claimed(1) + refunded(1) = 211
    pub const HTLC_ACCOUNT_SIZE: usize = 211;

    /// Parsed instruction data for `CreateHtlc`.
    ///
    /// This is the deserialized form of the instruction payload after the
    /// 1-byte instruction tag has been consumed.
    #[derive(Debug, Clone, PartialEq)]
    pub struct CreateHtlcParams {
        /// Unique swap identifier (32 bytes).
        pub swap_id: [u8; 32],
        /// Pubkey authorized to claim the locked funds.
        pub claimant: Pubkey,
        /// Pubkey authorized to refund the locked funds after timeout.
        pub refund_authority: Pubkey,
        /// SHA-256 hash of the preimage (32 bytes).
        pub hashlock: [u8; 32],
        /// Token mint address. Use `Pubkey::default()` for native SOL.
        pub token_mint: Pubkey,
        /// Amount to lock (in smallest unit, e.g., lamports).
        pub amount: u64,
        /// Slot number after which the refund authority may reclaim funds.
        pub timeout: u64,
    }

    /// On-chain HTLC account state.
    ///
    /// Stored in a PDA account at seeds `["htlc", swap_id]`. The account is
    /// created by [`processor::handle_create_htlc`] and serialized/deserialized
    /// using the fixed-size layout described at the module level.
    #[derive(Debug, Clone, PartialEq)]
    pub struct HtlcAccount {
        /// Pubkey who locked the funds.
        pub initializer: Pubkey,
        /// Pubkey authorized to claim the funds.
        pub claimant: Pubkey,
        /// Pubkey authorized to refund the funds after timeout.
        pub refund_authority: Pubkey,
        /// SHA-256 hash of the preimage.
        pub hashlock: [u8; 32],
        /// Token mint address. All-zeros (Pubkey::default()) for native SOL.
        pub token_mint: Pubkey,
        /// Amount locked (in smallest unit).
        pub amount: u64,
        /// Slot number after which refund is allowed.
        pub timeout: u64,
        /// PDA bump seed used in derivation.
        pub bump_seed: u8,
        /// Unique swap identifier.
        pub swap_id: [u8; 32],
        /// Whether the HTLC has been claimed.
        pub claimed: bool,
        /// Whether the HTLC has been refunded.
        pub refunded: bool,
    }

    impl HtlcAccount {
        // Byte offsets for field access (used for in-place mutation).
        const INITIALIZER_OFF: usize = 0;
        const CLAIMANT_OFF: usize = 32;
        const REFUND_AUTH_OFF: usize = 64;
        const HASHLOCK_OFF: usize = 96;
        const TOKEN_MINT_OFF: usize = 128;
        const AMOUNT_OFF: usize = 160;
        const TIMEOUT_OFF: usize = 168;
        const BUMP_OFF: usize = 176;
        const SWAP_ID_OFF: usize = 177;
        /// Byte offset of the `claimed` boolean flag.
        pub const CLAIMED_OFFSET: usize = 209;
        /// Byte offset of the `refunded` boolean flag.
        pub const REFUNDED_OFFSET: usize = 210;

        /// Deserialize an [`HtlcAccount`] from raw account data bytes.
        ///
        /// # Errors
        ///
        /// Returns [`ProgramError::InvalidAccountData`] if the buffer is
        /// smaller than [`HTLC_ACCOUNT_SIZE`].
        pub fn deserialize_from(data: &[u8]) -> Result<Self, ProgramError> {
            if data.len() < HTLC_ACCOUNT_SIZE {
                msg!("Error: account data too small for HtlcAccount");
                return Err(ProgramError::InvalidAccountData);
            }

            let initializer = Pubkey::new_from_array(read_array32(data, Self::INITIALIZER_OFF));
            let claimant = Pubkey::new_from_array(read_array32(data, Self::CLAIMANT_OFF));
            let refund_authority =
                Pubkey::new_from_array(read_array32(data, Self::REFUND_AUTH_OFF));
            let hashlock = read_array32(data, Self::HASHLOCK_OFF);
            let token_mint = Pubkey::new_from_array(read_array32(data, Self::TOKEN_MINT_OFF));
            let amount = u64::from_le_bytes(read_array8(data, Self::AMOUNT_OFF));
            let timeout = u64::from_le_bytes(read_array8(data, Self::TIMEOUT_OFF));
            let bump_seed = data[Self::BUMP_OFF];
            let swap_id = read_array32(data, Self::SWAP_ID_OFF);
            let claimed = data[Self::CLAIMED_OFFSET] != 0;
            let refunded = data[Self::REFUNDED_OFFSET] != 0;

            Ok(Self {
                initializer,
                claimant,
                refund_authority,
                hashlock,
                token_mint,
                amount,
                timeout,
                bump_seed,
                swap_id,
                claimed,
                refunded,
            })
        }

        /// Serialize this [`HtlcAccount`] into a pre-allocated byte buffer.
        ///
        /// The buffer must be at least [`HTLC_ACCOUNT_SIZE`] bytes long.
        /// Panics if the buffer is too small.
        pub fn serialize_into(&self, dst: &mut [u8]) {
            dst[Self::INITIALIZER_OFF..Self::CLAIMANT_OFF].copy_from_slice(self.initializer.as_ref());
            dst[Self::CLAIMANT_OFF..Self::REFUND_AUTH_OFF].copy_from_slice(self.claimant.as_ref());
            dst[Self::REFUND_AUTH_OFF..Self::HASHLOCK_OFF]
                .copy_from_slice(self.refund_authority.as_ref());
            dst[Self::HASHLOCK_OFF..Self::TOKEN_MINT_OFF].copy_from_slice(&self.hashlock);
            dst[Self::TOKEN_MINT_OFF..Self::AMOUNT_OFF].copy_from_slice(self.token_mint.as_ref());
            dst[Self::AMOUNT_OFF..Self::TIMEOUT_OFF].copy_from_slice(&self.amount.to_le_bytes());
            dst[Self::TIMEOUT_OFF..Self::BUMP_OFF].copy_from_slice(&self.timeout.to_le_bytes());
            dst[Self::BUMP_OFF] = self.bump_seed;
            dst[Self::SWAP_ID_OFF..Self::CLAIMED_OFFSET].copy_from_slice(&self.swap_id);
            dst[Self::CLAIMED_OFFSET] = self.claimed as u8;
            dst[Self::REFUNDED_OFFSET] = self.refunded as u8;
        }

        /// Convert this account to a fixed-size byte array.
        ///
        /// Useful for testing and off-chain introspection.
        pub fn to_bytes(&self) -> [u8; HTLC_ACCOUNT_SIZE] {
            let mut buf = [0u8; HTLC_ACCOUNT_SIZE];
            self.serialize_into(&mut buf);
            buf
        }
    }

    /// Instruction data parsing for the HTLC program.
    ///
    /// This is not instantiated; it serves as a namespace for static methods.
    pub enum HtlcInstruction {}

    impl HtlcInstruction {
        /// Parse a `CreateHtlc` instruction payload (after the 1-byte tag).
        ///
        /// Data layout (total 176 bytes):
        ///
        /// | Offset | Size | Field            |
        /// |--------|------|------------------|
        /// | 0      | 32   | swap_id          |
        /// | 32     | 32   | claimant         |
        /// | 64     | 32   | refund_authority |
        /// | 96     | 32   | hashlock         |
        /// | 128    | 32   | token_mint       |
        /// | 160    | 8    | amount (u64 LE)  |
        /// | 168    | 8    | timeout (u64 LE) |
        ///
        /// # Errors
        ///
        /// Returns [`ProgramError::InvalidInstructionData`] if the buffer is
        /// shorter than 176 bytes.
        pub fn unpack_create(data: &[u8]) -> Result<CreateHtlcParams, ProgramError> {
            if data.len() < 176 {
                msg!("Error: CreateHtlc data too short: {} bytes", data.len());
                return Err(ProgramError::InvalidInstructionData);
            }

            let swap_id = read_array32(data, 0);
            let claimant = Pubkey::new_from_array(read_array32(data, 32));
            let refund_authority = Pubkey::new_from_array(read_array32(data, 64));
            let hashlock = read_array32(data, 96);
            let token_mint = Pubkey::new_from_array(read_array32(data, 128));
            let amount = u64::from_le_bytes(read_array8(data, 160));
            let timeout = u64::from_le_bytes(read_array8(data, 168));

            Ok(CreateHtlcParams {
                swap_id,
                claimant,
                refund_authority,
                hashlock,
                token_mint,
                amount,
                timeout,
            })
        }
    }

    // ── helpers ──────────────────────────────────────────────────────────

    /// Read a 32-byte array from `data` at the given `offset`.
    fn read_array32(data: &[u8], offset: usize) -> [u8; 32] {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&data[offset..offset + 32]);
        arr
    }

    /// Read an 8-byte array from `data` at the given `offset`.
    fn read_array8(data: &[u8], offset: usize) -> [u8; 8] {
        let mut arr = [0u8; 8];
        arr.copy_from_slice(&data[offset..offset + 8]);
        arr
    }
}

/// CPI helper functions for invoking the HTLC program from other Solana programs.
pub mod cpi;

#[cfg(test)]
mod tests {
    use super::state::*;
    use solana_program::hash::hashv;
    use solana_program::pubkey::Pubkey;

    #[test]
    fn test_htlc_account_roundtrip() {
        let account = HtlcAccount {
            initializer: Pubkey::new_from_array([1u8; 32]),
            claimant: Pubkey::new_from_array([2u8; 32]),
            refund_authority: Pubkey::new_from_array([3u8; 32]),
            hashlock: [4u8; 32],
            token_mint: Pubkey::new_from_array([5u8; 32]),
            amount: 1_000_000,
            timeout: 123456789,
            bump_seed: 255,
            swap_id: [6u8; 32],
            claimed: false,
            refunded: false,
        };

        let bytes = account.to_bytes();
        assert_eq!(bytes.len(), HTLC_ACCOUNT_SIZE);

        let deserialized = HtlcAccount::deserialize_from(&bytes).unwrap();
        assert_eq!(deserialized, account);
    }

    #[test]
    fn test_htlc_account_with_claimed_flag() {
        let mut account = HtlcAccount {
            initializer: Pubkey::new_from_array([1u8; 32]),
            claimant: Pubkey::new_from_array([2u8; 32]),
            refund_authority: Pubkey::new_from_array([3u8; 32]),
            hashlock: [4u8; 32],
            token_mint: Pubkey::new_from_array([5u8; 32]),
            amount: 500,
            timeout: 100,
            bump_seed: 100,
            swap_id: [6u8; 32],
            claimed: true,
            refunded: false,
        };

        let bytes = account.to_bytes();
        assert_eq!(bytes[HtlcAccount::CLAIMED_OFFSET], 1);
        assert_eq!(bytes[HtlcAccount::REFUNDED_OFFSET], 0);

        let deserialized = HtlcAccount::deserialize_from(&bytes).unwrap();
        assert!(deserialized.claimed);
        assert!(!deserialized.refunded);
    }

    #[test]
    fn test_htlc_account_with_refunded_flag() {
        let account = HtlcAccount {
            initializer: Pubkey::new_from_array([1u8; 32]),
            claimant: Pubkey::new_from_array([2u8; 32]),
            refund_authority: Pubkey::new_from_array([3u8; 32]),
            hashlock: [4u8; 32],
            token_mint: Pubkey::new_from_array([5u8; 32]),
            amount: 500,
            timeout: 100,
            bump_seed: 100,
            swap_id: [6u8; 32],
            claimed: false,
            refunded: true,
        };

        let bytes = account.to_bytes();
        assert_eq!(bytes[HtlcAccount::CLAIMED_OFFSET], 0);
        assert_eq!(bytes[HtlcAccount::REFUNDED_OFFSET], 1);

        let deserialized = HtlcAccount::deserialize_from(&bytes).unwrap();
        assert!(!deserialized.claimed);
        assert!(deserialized.refunded);
    }

    #[test]
    fn test_create_htlc_params_unpack() {
        let mut data = [0u8; 176];
        // swap_id
        data[0..32].copy_from_slice(&[1u8; 32]);
        // claimant
        let claimant = Pubkey::new_from_array([2u8; 32]);
        data[32..64].copy_from_slice(claimant.as_ref());
        // refund_authority
        let refund = Pubkey::new_from_array([3u8; 32]);
        data[64..96].copy_from_slice(refund.as_ref());
        // hashlock
        data[96..128].copy_from_slice(&[4u8; 32]);
        // token_mint
        let token_mint = Pubkey::new_from_array([5u8; 32]);
        data[128..160].copy_from_slice(token_mint.as_ref());
        // amount
        data[160..168].copy_from_slice(&1_000_000u64.to_le_bytes());
        // timeout
        data[168..176].copy_from_slice(&100u64.to_le_bytes());

        let params = HtlcInstruction::unpack_create(&data).unwrap();
        assert_eq!(params.swap_id, [1u8; 32]);
        assert_eq!(params.claimant, claimant);
        assert_eq!(params.refund_authority, refund);
        assert_eq!(params.hashlock, [4u8; 32]);
        assert_eq!(params.token_mint, token_mint);
        assert_eq!(params.amount, 1_000_000);
        assert_eq!(params.timeout, 100);
    }

    #[test]
    fn test_hash_verification() {
        let preimage = b"secret123";
        let hash = hashv(&[preimage]).to_bytes();

        let computed = hashv(&[preimage]).to_bytes();
        assert_eq!(computed, hash);

        // Verify wrong preimage fails
        let wrong_preimage = b"wrong";
        let wrong_hash = hashv(&[wrong_preimage]).to_bytes();
        assert_ne!(wrong_hash, hash);
    }

    #[test]
    fn test_create_htlc_params_rejects_short_data() {
        let short_data = [0u8; 10];
        let result = HtlcInstruction::unpack_create(&short_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_rejects_short_buffer() {
        let short = [0u8; 10];
        let result = HtlcAccount::deserialize_from(&short);
        assert!(result.is_err());
    }

    #[test]
    fn test_default_account_all_zeros() {
        let bytes = [0u8; HTLC_ACCOUNT_SIZE];
        let account = HtlcAccount::deserialize_from(&bytes).unwrap();
        assert!(account.initializer == Pubkey::new_from_array([0u8; 32]));
        assert_eq!(account.amount, 0);
        assert!(!account.claimed);
        assert!(!account.refunded);
    }
}
