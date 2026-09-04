//! CPI helper functions for invoking the X3 Atomic Swap HTLC program.
//!
//! Use these functions to build CPI instructions that call the HTLC program
//! from other Solana programs. Each function returns a
//! [`solana_program::instruction::Instruction`] that can be passed to
//! [`solana_program::program::invoke`] or `invoke_signed`.
//!
//! # Example
//!
//! ```ignore
//! use x3_atomic_swap::cpi;
//!
//! // Derive the HTLC PDA
//! let (htlc_pda, bump) = cpi::derive_htlc_pda(&htlc_program_id, &swap_id);
//!
//! // Build a CreateHtlc instruction
//! let ix = cpi::create_htlc(
//!     &htlc_program_id,
//!     &htlc_pda,
//!     &payer.key,
//!     &initializer.key,
//!     &system_program::ID,
//!     &swap_id,
//!     &claimant.key,
//!     &refund_authority.key,
//!     &hashlock,
//!     &token_mint,
//!     amount,
//!     timeout,
//! );
//!
//! // Invoke via CPI
//! invoke(&ix, &[htlc_account, payer, initializer, system_program])?;
//! ```

use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

use crate::state::HTLC_ACCOUNT_SEED;

/// Build a CPI instruction to create an HTLC.
///
/// This is a thin wrapper around [`Instruction`] construction. The caller
/// must ensure the PDA account is pre-funded or created via
/// `system_instruction::create_account` in the same CPI batch.
///
/// ### Parameters
///
/// - `program_id` — The HTLC program ID.
/// - `htlc_account` — The PDA address of the HTLC (seeds: ["htlc", swap_id]).
/// - `payer` — Account funding the rent for the HTLC account.
/// - `initializer` — The party locking funds (must be a signer).
/// - `system_program` — System program ID.
/// - `swap_id` — Unique 32-byte swap identifier.
/// - `claimant` — Pubkey authorized to claim.
/// - `refund_authority` — Pubkey authorized to refund.
/// - `hashlock` — 32-byte SHA-256 hash of the preimage.
/// - `token_mint` — Token mint (Pubkey::default() for SOL).
/// - `amount` — Amount to lock in smallest unit.
/// - `timeout` — Slot number after which refund is allowed.
///
/// ### Returns
///
/// An [`Instruction`] ready for `invoke` or `invoke_signed`.
pub fn create_htlc(
    program_id: &Pubkey,
    htlc_account: &Pubkey,
    payer: &Pubkey,
    initializer: &Pubkey,
    system_program: &Pubkey,
    swap_id: &[u8; 32],
    claimant: &Pubkey,
    refund_authority: &Pubkey,
    hashlock: &[u8; 32],
    token_mint: &Pubkey,
    amount: u64,
    timeout: u64,
) -> Instruction {
    let mut data = Vec::with_capacity(1 + 176);
    data.push(0u8); // instruction tag: CreateHtlc
    data.extend_from_slice(swap_id);
    data.extend_from_slice(claimant.as_ref());
    data.extend_from_slice(refund_authority.as_ref());
    data.extend_from_slice(hashlock);
    data.extend_from_slice(token_mint.as_ref());
    data.extend_from_slice(&amount.to_le_bytes());
    data.extend_from_slice(&timeout.to_le_bytes());

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*htlc_account, false),
            AccountMeta::new(*payer, true),
            AccountMeta::new_readonly(*initializer, true),
            AccountMeta::new_readonly(*system_program, false),
        ],
        data,
    }
}

/// Build a CPI instruction to claim an HTLC by revealing the preimage.
///
/// ### Parameters
///
/// - `program_id` — The HTLC program ID.
/// - `htlc_account` — The PDA address of the HTLC.
/// - `claimant` — The authorized claimant (must be a signer).
/// - `preimage` — The preimage bytes (1–255 bytes).
///
/// ### Returns
///
/// An [`Instruction`] ready for `invoke` or `invoke_signed`.
pub fn claim_htlc(
    program_id: &Pubkey,
    htlc_account: &Pubkey,
    claimant: &Pubkey,
    preimage: &[u8],
) -> Instruction {
    let preimage_len = preimage.len().min(255) as u8;
    let mut data = Vec::with_capacity(2 + preimage_len as usize);
    data.push(1u8); // instruction tag: ClaimHtlc
    data.push(preimage_len);
    data.extend_from_slice(&preimage[..preimage_len as usize]);

    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*htlc_account, false),
            AccountMeta::new_readonly(*claimant, true),
        ],
        data,
    }
}

/// Build a CPI instruction to refund an expired HTLC.
///
/// ### Parameters
///
/// - `program_id` — The HTLC program ID.
/// - `htlc_account` — The PDA address of the HTLC.
/// - `refund_authority` — The refund authority (must be a signer).
///
/// ### Returns
///
/// An [`Instruction`] ready for `invoke` or `invoke_signed`.
pub fn refund_htlc(
    program_id: &Pubkey,
    htlc_account: &Pubkey,
    refund_authority: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: vec![
            AccountMeta::new(*htlc_account, false),
            AccountMeta::new_readonly(*refund_authority, true),
        ],
        data: vec![2u8], // instruction tag: RefundHtlc
    }
}

/// Derive the HTLC PDA address for a given swap ID.
///
/// This is equivalent to:
/// ```ignore
/// Pubkey::find_program_address(&[b"htlc", &swap_id], program_id)
/// ```
///
/// Returns `(pda_address, bump_seed)`.
pub fn derive_htlc_pda(
    program_id: &Pubkey,
    swap_id: &[u8; 32],
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[HTLC_ACCOUNT_SEED, swap_id], program_id)
}
