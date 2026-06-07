#![allow(
    dead_code,
    unused_imports,
    unused_variables,
    unused_mut,
    non_snake_case,
    unexpected_cfgs,
    unused_parens,
    non_camel_case_types,
    clippy::all
)]
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Instruction format: empty -> increment by 1
pub fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;

    if !counter_account.is_writable {
        msg!("Counter account is not writable");
        return Err(ProgramError::InvalidArgument);
    }

    // Account data holds a little-endian u64 counter (8 bytes)
    let data = &mut *counter_account.try_borrow_mut_data()?;
    if data.len() < 8 {
        msg!("Counter account data too small");
        return Err(ProgramError::InvalidAccountData);
    }

    let mut v = [0u8; 8];
    v.copy_from_slice(&data[0..8]);
    let mut counter = u64::from_le_bytes(v);
    counter = counter
        .checked_add(1)
        .ok_or(ProgramError::InvalidInstructionData)?;
    data[0..8].copy_from_slice(&counter.to_le_bytes());

    msg!("Counter incremented to: {}", counter);
    Ok(())
}

entrypoint!(process_instruction);
