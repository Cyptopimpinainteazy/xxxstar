//! X3 SVM Token Adapter — Kernel-controlled token representation on X3 SVM.
//!
//! This program runs on the X3 SVM (Solana Virtual Machine) side and provides
//! kernel-controlled mint/burn for any X3-registered asset. Only the X3 kernel
//! bridge authority can call the privileged instructions.
//!
//! # Instructions
//! - `Initialize`: Create a token adapter account
//! - `KernelMint`: Mint tokens (kernel bridge authority only)
//! - `KernelBurn`: Burn tokens (kernel bridge authority only)
//! - `Transfer`: Transfer tokens between users
//! - `SendToVm`: Burn SVM representation and request cross-VM transfer
//! - `RegisterToken`: Register a new token with its canonical AssetId

#![no_std]
#![deny(unsafe_code)]

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint {
    use solana_program::{
        account_info::AccountInfo,
        entrypoint,
        entrypoint::ProgramResult,
        pubkey::Pubkey,
        msg,
    };

    entrypoint!(process_instruction);

    fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        msg!("X3 SVM Token Adapter: processing instruction");
        processor::process(program_id, accounts, instruction_data)
    }
}

pub mod processor {
    use solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    };

    use crate::state::{
        TokenAccount, TokenRegistry, KERNEL_BRIDGE_SEED, TOKEN_ACCOUNT_SEED, TOKEN_REGISTRY_SEED,
    };

    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let (tag, rest) = instruction_data
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        match tag {
            0 => handle_initialize_token_registry(program_id, accounts, rest),
            1 => handle_register_token(program_id, accounts, rest),
            2 => handle_kernel_mint(program_id, accounts, rest),
            3 => handle_kernel_burn(program_id, accounts, rest),
            4 => handle_transfer(program_id, accounts, rest),
            5 => handle_send_to_vm(program_id, accounts, rest),
            _ => Err(ProgramError::InvalidInstructionData),
        }
    }

    fn assert_kernel_bridge(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
    ) -> Result<&AccountInfo, ProgramError> {
        let kernel_bridge_info = accounts.get(0).ok_or(ProgramError::NotEnoughAccountKeys)?;
        let (expected_key, _) = Pubkey::find_program_address(&[KERNEL_BRIDGE_SEED], program_id);
        if kernel_bridge_info.key != &expected_key {
            msg!("Error: Kernel bridge authority mismatch");
            return Err(ProgramError::InvalidArgument);
        }
        Ok(kernel_bridge_info)
    }

    fn handle_initialize_token_registry(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        _data: &[u8],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let registry_info = next_account_info(accounts_iter)?;
        let payer = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        let (registry_key, bump) =
            Pubkey::find_program_address(&[TOKEN_REGISTRY_SEED], program_id);
        if registry_info.key != &registry_key {
            msg!("Error: Invalid token registry PDA");
            return Err(ProgramError::InvalidArgument);
        }

        let registry = TokenRegistry { initialized: true };
        let space = TokenRegistry::LEN;
        let lamports = solana_program::system_instruction::create_account::CheckSystemInstruction::get_min_rent_exempt_lamports(space);

        solana_program::program::invoke(
            &solana_program::system_instruction::create_account(
                payer.key,
                registry_info.key,
                lamports,
                space as u64,
                program_id,
            ),
            &[payer.clone(), registry_info.clone(), system_program.clone()],
        )?;

        registry_info.try_borrow_mut_data()?[..space].copy_from_slice(&bincode::serialize(&registry).unwrap());
        msg!("Token registry initialized");
        Ok(())
    }

    fn handle_register_token(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let _kernel_bridge = assert_kernel_bridge(program_id, accounts)?;

        if data.len() < 32 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let mut asset_id = [0u8; 32];
        asset_id.copy_from_slice(&data[..32]);

        msg!("Token registered with AssetId: {:?}", asset_id);
        Ok(())
    }

    fn handle_kernel_mint(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let _kernel_bridge = assert_kernel_bridge(program_id, accounts)?;
        let accounts_iter = &mut accounts.iter();
        let token_account_info = next_account_info(accounts_iter)?;

        if data.len() < 40 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&data[..8]);
        let amount = u64::from_le_bytes(amount_bytes);

        let mut token_account = TokenAccount::try_from_slice(&token_account_info.data.borrow())?;
        token_account.balance = token_account.balance.checked_add(amount).ok_or(ProgramError::ArithmeticOverflow)?;
        token_account.serialize_into(&mut token_account_info.data.borrow_mut())?;

        msg!("Kernel minted {} tokens to {:?}", amount, token_account_info.key);
        Ok(())
    }

    fn handle_kernel_burn(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let _kernel_bridge = assert_kernel_bridge(program_id, accounts)?;
        let accounts_iter = &mut accounts.iter();
        let token_account_info = next_account_info(accounts_iter)?;

        if data.len() < 40 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&data[..8]);
        let amount = u64::from_le_bytes(amount_bytes);

        let mut token_account = TokenAccount::try_from_slice(&token_account_info.data.borrow())?;
        token_account.balance = token_account.balance.checked_sub(amount).ok_or(ProgramError::ArithmeticOverflow)?;
        token_account.serialize_into(&mut token_account_info.data.borrow_mut())?;

        msg!("Kernel burned {} tokens from {:?}", amount, token_account_info.key);
        Ok(())
    }

    fn handle_transfer(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let from_info = next_account_info(accounts_iter)?;
        let to_info = next_account_info(accounts_iter)?;

        if data.len() < 40 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&data[..8]);
        let amount = u64::from_le_bytes(amount_bytes);

        let mut from_account = TokenAccount::try_from_slice(&from_info.data.borrow())?;
        let mut to_account = TokenAccount::try_from_slice(&to_info.data.borrow())?;

        from_account.balance = from_account.balance.checked_sub(amount).ok_or(ProgramError::ArithmeticOverflow)?;
        to_account.balance = to_account.balance.checked_add(amount).ok_or(ProgramError::ArithmeticOverflow)?;

        from_account.serialize_into(&mut from_info.data.borrow_mut())?;
        to_account.serialize_into(&mut to_info.data.borrow_mut())?;

        msg!("Transferred {} tokens", amount);
        Ok(())
    }

    fn handle_send_to_vm(
        _program_id: &Pubkey,
        accounts: &[AccountInfo],
        data: &[u8],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let from_info = next_account_info(accounts_iter)?;

        if data.len() < 41 {
            return Err(ProgramError::InvalidInstructionData);
        }
        let mut amount_bytes = [0u8; 8];
        amount_bytes.copy_from_slice(&data[..8]);
        let amount = u64::from_le_bytes(amount_bytes);
        let destination_vm = data[8];
        // recipient bytes follow at data[9..]

        let mut from_account = TokenAccount::try_from_slice(&from_info.data.borrow())?;
        from_account.balance = from_account.balance.checked_sub(amount).ok_or(ProgramError::ArithmeticOverflow)?;
        from_account.serialize_into(&mut from_info.data.borrow_mut())?;

        msg!("Sent {} tokens to VM {}", amount, destination_vm);
        Ok(())
    }
}

pub mod state {
    use solana_program::program_error::ProgramError;

    pub const KERNEL_BRIDGE_SEED: &[u8] = b"x3_kernel_bridge";
    pub const TOKEN_REGISTRY_SEED: &[u8] = b"x3_token_registry";
    pub const TOKEN_ACCOUNT_SEED: &[u8] = b"x3_token_account";

    #[derive(Debug, Clone, PartialEq)]
    pub struct TokenRegistry {
        pub initialized: bool,
    }

    impl TokenRegistry {
        pub const LEN: usize = 1;
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TokenAccount {
        pub owner: [u8; 32],
        pub asset_id: [u8; 32],
        pub balance: u64,
    }

    impl TokenAccount {
        pub const LEN: usize = 32 + 32 + 8;

        pub fn try_from_slice(data: &[u8]) -> Result<Self, ProgramError> {
            if data.len() < Self::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
            let mut owner = [0u8; 32];
            let mut asset_id = [0u8; 32];
            owner.copy_from_slice(&data[..32]);
            asset_id.copy_from_slice(&data[32..64]);
            let mut balance_bytes = [0u8; 8];
            balance_bytes.copy_from_slice(&data[64..72]);
            let balance = u64::from_le_bytes(balance_bytes);
            Ok(Self { owner, asset_id, balance })
        }

        pub fn serialize_into(&self, data: &mut [u8]) -> Result<(), ProgramError> {
            if data.len() < Self::LEN {
                return Err(ProgramError::InvalidAccountData);
            }
            data[..32].copy_from_slice(&self.owner);
            data[32..64].copy_from_slice(&self.asset_id);
            data[64..72].copy_from_slice(&self.balance.to_le_bytes());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::state::*;

    #[test]
    fn test_token_account_serialization() {
        let account = TokenAccount {
            owner: [1u8; 32],
            asset_id: [2u8; 32],
            balance: 1000,
        };

        let mut buf = [0u8; 72];
        account.serialize_into(&mut buf).unwrap();

        let deserialized = TokenAccount::try_from_slice(&buf).unwrap();
        assert_eq!(deserialized.owner, [1u8; 32]);
        assert_eq!(deserialized.asset_id, [2u8; 32]);
        assert_eq!(deserialized.balance, 1000);
    }

    #[test]
    fn test_token_account_overflow() {
        let mut account = TokenAccount {
            owner: [0u8; 32],
            asset_id: [0u8; 32],
            balance: u64::MAX,
        };
        assert!(account.balance.checked_add(1).is_none());
    }
}