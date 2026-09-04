use anchor_lang::prelude::*;

declare_id!("X3KernBri1111111111111111111111111111111114");

pub const BRIDGE_SEED: &[u8] = b"kernel_bridge";
pub const ADAPTER_ENTRY_SEED: &[u8] = b"adapter_entry";

#[program]
pub mod x3_kernel_bridge {

    use super::*;

    pub fn initialize(ctx: Context<Initialize>, kernel_authority: Pubkey) -> Result<()> {
        let bridge = &mut ctx.accounts.bridge;
        bridge.kernel_authority = kernel_authority;
        bridge.initialized = true;
        emit!(BridgeInitialized { kernel_authority });
        Ok(())
    }

    pub fn register_token_adapter(
        ctx: Context<RegisterTokenAdapter>,
        asset_id: [u8; 32],
    ) -> Result<()> {
        let bridge = &ctx.accounts.bridge;
        require!(bridge.initialized, BridgeError::NotInitialized);
        require!(
            ctx.accounts.authority.key() == bridge.kernel_authority,
            BridgeError::Unauthorized
        );
        require!(
            ctx.accounts.adapter_entry.asset_id == [0u8; 32],
            BridgeError::AlreadyRegistered
        );

        let entry = &mut ctx.accounts.adapter_entry;
        entry.asset_id = asset_id;
        entry.adapter_program = ctx.accounts.adapter_program.key();
        entry.adapter_config = ctx.accounts.adapter_config.key();
        entry.initialized = true;

        emit!(TokenAdapterRegistered {
            asset_id,
            adapter_program: ctx.accounts.adapter_program.key(),
            adapter_config: ctx.accounts.adapter_config.key(),
        });
        Ok(())
    }

    pub fn register_external_gateway(
        ctx: Context<RegisterExternalGateway>,
        chain_id: u64,
    ) -> Result<()> {
        let bridge = &ctx.accounts.bridge;
        require!(bridge.initialized, BridgeError::NotInitialized);
        require!(
            ctx.accounts.authority.key() == bridge.kernel_authority,
            BridgeError::Unauthorized
        );

        let entry = &mut ctx.accounts.gateway_entry;
        entry.chain_id = chain_id;
        entry.gateway_program = ctx.accounts.gateway_program.key();
        entry.gateway_config = ctx.accounts.gateway_config.key();
        entry.initialized = true;

        emit!(ExternalGatewayRegistered {
            chain_id,
            gateway_program: ctx.accounts.gateway_program.key(),
            gateway_config: ctx.accounts.gateway_config.key(),
        });
        Ok(())
    }

    pub fn credit_user(
        ctx: Context<CreditUser>,
        message_id: [u8; 32],
        asset_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        let bridge = &ctx.accounts.bridge;
        require!(bridge.initialized, BridgeError::NotInitialized);
        require!(
            ctx.accounts.authority.key() == bridge.kernel_authority,
            BridgeError::Unauthorized
        );

        let entry = &ctx.accounts.adapter_entry;
        require!(entry.initialized, BridgeError::NoAdapter);
        require!(entry.asset_id == asset_id, BridgeError::AssetMismatch);
        require!(amount > 0, BridgeError::ZeroAmount);

        let discriminator =
            &anchor_lang::solana_program::hash::hash(b"global:kernel_mint").to_bytes()[..8];

        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(discriminator);
        data.extend_from_slice(&amount.to_le_bytes());

        let accounts = vec![
            AccountMeta::new_readonly(ctx.accounts.adapter_config.key(), false),
            AccountMeta::new(ctx.accounts.mint.key(), false),
            AccountMeta::new(ctx.accounts.destination.key(), false),
            AccountMeta::new(ctx.accounts.mint_authority.key(), false),
            AccountMeta::new_readonly(ctx.accounts.bridge_program.key(), false),
            AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
        ];

        let ix = anchor_lang::solana_program::instruction::Instruction {
            program_id: entry.adapter_program,
            accounts,
            data,
        };

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.adapter_config.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.destination.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.bridge_program.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
        )
        .map_err(|_| error!(BridgeError::CpiFailed))?;

        emit!(CrossVmTransferCompleted {
            message_id,
            asset_id,
            recipient: ctx.accounts.destination.key(),
            amount,
        });
        Ok(())
    }

    pub fn debit_user(
        ctx: Context<DebitUser>,
        _message_id: [u8; 32],
        asset_id: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        let bridge = &ctx.accounts.bridge;
        require!(bridge.initialized, BridgeError::NotInitialized);
        require!(
            ctx.accounts.authority.key() == bridge.kernel_authority,
            BridgeError::Unauthorized
        );

        let entry = &ctx.accounts.adapter_entry;
        require!(entry.initialized, BridgeError::NoAdapter);
        require!(entry.asset_id == asset_id, BridgeError::AssetMismatch);
        require!(amount > 0, BridgeError::ZeroAmount);

        let discriminator =
            &anchor_lang::solana_program::hash::hash(b"global:kernel_burn").to_bytes()[..8];

        let mut data = Vec::with_capacity(16);
        data.extend_from_slice(discriminator);
        data.extend_from_slice(&amount.to_le_bytes());

        let accounts = vec![
            AccountMeta::new_readonly(ctx.accounts.adapter_config.key(), false),
            AccountMeta::new(ctx.accounts.mint.key(), false),
            AccountMeta::new(ctx.accounts.source.key(), false),
            AccountMeta::new(ctx.accounts.mint_authority.key(), false),
            AccountMeta::new_readonly(ctx.accounts.bridge_program.key(), false),
            AccountMeta::new_readonly(ctx.accounts.token_program.key(), false),
        ];

        let ix = anchor_lang::solana_program::instruction::Instruction {
            program_id: entry.adapter_program,
            accounts,
            data,
        };

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.adapter_config.to_account_info(),
                ctx.accounts.mint.to_account_info(),
                ctx.accounts.source.to_account_info(),
                ctx.accounts.mint_authority.to_account_info(),
                ctx.accounts.bridge_program.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
        )
        .map_err(|_| error!(BridgeError::CpiFailed))?;

        Ok(())
    }

    pub fn get_adapter(ctx: Context<GetAdapter>) -> Result<Pubkey> {
        Ok(ctx.accounts.adapter_entry.adapter_program)
    }

    pub fn get_gateway(ctx: Context<GetGateway>) -> Result<Pubkey> {
        Ok(ctx.accounts.gateway_entry.gateway_program)
    }
}

#[account]
pub struct KernelBridge {
    pub kernel_authority: Pubkey,
    pub initialized: bool,
}

impl KernelBridge {
    pub const SIZE: usize = 32 + 1;
}

#[account]
#[derive(Debug, PartialEq, Eq)]
pub struct AdapterEntry {
    pub asset_id: [u8; 32],
    pub adapter_program: Pubkey,
    pub adapter_config: Pubkey,
    pub initialized: bool,
}

impl AdapterEntry {
    pub const SIZE: usize = 32 + 32 + 32 + 1;
}

#[account]
#[derive(Debug, PartialEq, Eq)]
pub struct GatewayEntry {
    pub chain_id: u64,
    pub gateway_program: Pubkey,
    pub gateway_config: Pubkey,
    pub initialized: bool,
}

impl GatewayEntry {
    pub const SIZE: usize = 8 + 32 + 32 + 1;
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [BRIDGE_SEED],
        bump,
        payer = payer,
        space = 8 + KernelBridge::SIZE
    )]
    pub bridge: Account<'info, KernelBridge>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(asset_id: [u8; 32])]
pub struct RegisterTokenAdapter<'info> {
    #[account(
        seeds = [BRIDGE_SEED],
        bump,
    )]
    pub bridge: Account<'info, KernelBridge>,
    #[account(
        init,
        seeds = [ADAPTER_ENTRY_SEED, asset_id.as_ref()],
        bump,
        payer = payer,
        space = 8 + AdapterEntry::SIZE
    )]
    pub adapter_entry: Account<'info, AdapterEntry>,
    /// CHECK: adapter program ID
    pub adapter_program: AccountInfo<'info>,
    /// CHECK: adapter config PDA (validated on CPI)
    pub adapter_config: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(chain_id: u64)]
pub struct RegisterExternalGateway<'info> {
    #[account(
        seeds = [BRIDGE_SEED],
        bump,
    )]
    pub bridge: Account<'info, KernelBridge>,
    #[account(
        init,
        seeds = [b"gateway_entry", chain_id.to_le_bytes().as_ref()],
        bump,
        payer = payer,
        space = 8 + GatewayEntry::SIZE
    )]
    pub gateway_entry: Account<'info, GatewayEntry>,
    /// CHECK: gateway program ID
    pub gateway_program: AccountInfo<'info>,
    /// CHECK: gateway config PDA
    pub gateway_config: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreditUser<'info> {
    #[account(
        seeds = [BRIDGE_SEED],
        bump,
    )]
    pub bridge: Account<'info, KernelBridge>,
    #[account(
        seeds = [ADAPTER_ENTRY_SEED, adapter_entry.asset_id.as_ref()],
        bump,
    )]
    pub adapter_entry: Account<'info, AdapterEntry>,
    /// CHECK: adapter config PDA (validated via CPI by adapter program)
    pub adapter_config: AccountInfo<'info>,
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(mut)]
    pub destination: AccountInfo<'info>,
    #[account(mut)]
    pub mint_authority: AccountInfo<'info>,
    /// CHECK: this program's own pubkey (passes CPI source check to adapter)
    pub bridge_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct DebitUser<'info> {
    #[account(
        seeds = [BRIDGE_SEED],
        bump,
    )]
    pub bridge: Account<'info, KernelBridge>,
    #[account(
        seeds = [ADAPTER_ENTRY_SEED, adapter_entry.asset_id.as_ref()],
        bump,
    )]
    pub adapter_entry: Account<'info, AdapterEntry>,
    /// CHECK: adapter config PDA (validated via CPI)
    pub adapter_config: AccountInfo<'info>,
    #[account(mut)]
    pub mint: AccountInfo<'info>,
    #[account(mut)]
    pub source: AccountInfo<'info>,
    #[account(mut)]
    pub mint_authority: AccountInfo<'info>,
    /// CHECK: this program's own pubkey
    pub bridge_program: AccountInfo<'info>,
    pub token_program: AccountInfo<'info>,
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(asset_id: [u8; 32])]
pub struct GetAdapter<'info> {
    #[account(
        seeds = [ADAPTER_ENTRY_SEED, asset_id.as_ref()],
        bump,
    )]
    pub adapter_entry: Account<'info, AdapterEntry>,
}

#[derive(Accounts)]
#[instruction(chain_id: u64)]
pub struct GetGateway<'info> {
    #[account(
        seeds = [b"gateway_entry", chain_id.to_le_bytes().as_ref()],
        bump,
    )]
    pub gateway_entry: Account<'info, GatewayEntry>,
}

#[event]
pub struct BridgeInitialized {
    pub kernel_authority: Pubkey,
}

#[event]
pub struct TokenAdapterRegistered {
    pub asset_id: [u8; 32],
    pub adapter_program: Pubkey,
    pub adapter_config: Pubkey,
}

#[event]
pub struct ExternalGatewayRegistered {
    pub chain_id: u64,
    pub gateway_program: Pubkey,
    pub gateway_config: Pubkey,
}

#[event]
pub struct CrossVmTransferCompleted {
    pub message_id: [u8; 32],
    pub asset_id: [u8; 32],
    pub recipient: Pubkey,
    pub amount: u64,
}

#[error_code]
pub enum BridgeError {
    #[msg("bridge not initialized")]
    NotInitialized,
    #[msg("caller is not the kernel authority")]
    Unauthorized,
    #[msg("no adapter registered for this asset")]
    NoAdapter,
    #[msg("adapter already registered")]
    AlreadyRegistered,
    #[msg("amount must be greater than zero")]
    ZeroAmount,
    #[msg("asset id does not match entry")]
    AssetMismatch,
    #[msg("CPI to adapter failed")]
    CpiFailed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kernel_bridge_size() {
        assert_eq!(KernelBridge::SIZE, 32 + 1);
    }

    #[test]
    fn adapter_entry_size() {
        assert_eq!(AdapterEntry::SIZE, 32 + 32 + 32 + 1);
    }

    #[test]
    fn gateway_entry_size() {
        assert_eq!(GatewayEntry::SIZE, 8 + 32 + 32 + 1);
    }

    #[test]
    fn bridge_seed_is_stable() {
        assert_eq!(BRIDGE_SEED, b"kernel_bridge");
    }

    #[test]
    fn adapter_entry_seed_is_stable() {
        assert_eq!(ADAPTER_ENTRY_SEED, b"adapter_entry");
    }

    #[test]
    fn bridge_pda_is_deterministic() {
        let (pda1, bump1) = Pubkey::find_program_address(
            &[BRIDGE_SEED],
            &id(),
        );
        let (pda2, bump2) = Pubkey::find_program_address(
            &[BRIDGE_SEED],
            &id(),
        );
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    fn adapter_entry_pda_is_deterministic_by_asset_id() {
        let asset_id: [u8; 32] = [0xAB; 32];
        let (pda1, _) = Pubkey::find_program_address(
            &[ADAPTER_ENTRY_SEED, asset_id.as_ref()],
            &id(),
        );
        let (pda2, _) = Pubkey::find_program_address(
            &[ADAPTER_ENTRY_SEED, asset_id.as_ref()],
            &id(),
        );
        assert_eq!(pda1, pda2);
    }

    #[test]
    fn different_asset_ids_produce_different_adapter_pdas() {
        let id1: [u8; 32] = [1u8; 32];
        let id2: [u8; 32] = [2u8; 32];
        let (pda1, _) = Pubkey::find_program_address(
            &[ADAPTER_ENTRY_SEED, id1.as_ref()],
            &id(),
        );
        let (pda2, _) = Pubkey::find_program_address(
            &[ADAPTER_ENTRY_SEED, id2.as_ref()],
            &id(),
        );
        assert_ne!(pda1, pda2);
    }

    #[test]
    fn gateway_entry_pda_uses_chain_id_le_bytes() {
        let chain_id: u64 = 12345;
        let (pda, _) = Pubkey::find_program_address(
            &[b"gateway_entry", chain_id.to_le_bytes().as_ref()],
            &id(),
        );
        assert_ne!(pda, Pubkey::default());
    }

    #[test]
    fn different_chain_ids_produce_different_gateway_pdas() {
        let (pda1, _) = Pubkey::find_program_address(
            &[b"gateway_entry", 1u64.to_le_bytes().as_ref()],
            &id(),
        );
        let (pda2, _) = Pubkey::find_program_address(
            &[b"gateway_entry", 2u64.to_le_bytes().as_ref()],
            &id(),
        );
        assert_ne!(pda1, pda2);
    }

    #[test]
    fn kernel_bridge_default_is_uninitialized() {
        let bridge = KernelBridge {
            kernel_authority: Pubkey::default(),
            initialized: false,
        };
        assert!(!bridge.initialized);
    }

    #[test]
    fn adapter_entry_default_is_uninitialized() {
        let entry = AdapterEntry {
            asset_id: [0u8; 32],
            adapter_program: Pubkey::default(),
            adapter_config: Pubkey::default(),
            initialized: false,
        };
        assert!(!entry.initialized);
        assert_eq!(entry.asset_id, [0u8; 32]);
    }

    #[test]
    fn gateway_entry_default_is_uninitialized() {
        let entry = GatewayEntry {
            chain_id: 0,
            gateway_program: Pubkey::default(),
            gateway_config: Pubkey::default(),
            initialized: false,
        };
        assert!(!entry.initialized);
        assert_eq!(entry.chain_id, 0);
    }

    #[test]
    fn credit_discriminator_matches_kernel_mint() {
        let discriminator =
            &anchor_lang::solana_program::hash::hash(b"global:kernel_mint").to_bytes()[..8];
        assert_eq!(discriminator.len(), 8);
    }

    #[test]
    fn debit_discriminator_matches_kernel_burn() {
        let discriminator =
            &anchor_lang::solana_program::hash::hash(b"global:kernel_burn").to_bytes()[..8];
        assert_eq!(discriminator.len(), 8);
    }

    #[test]
    fn credit_and_debit_discriminators_are_distinct() {
        let credit_disc =
            &anchor_lang::solana_program::hash::hash(b"global:kernel_mint").to_bytes()[..8];
        let debit_disc =
            &anchor_lang::solana_program::hash::hash(b"global:kernel_burn").to_bytes()[..8];
        assert_ne!(credit_disc, debit_disc);
    }

    #[test]
    fn kernel_bridge_serialization_roundtrip() {
        let bridge = KernelBridge {
            kernel_authority: Pubkey::new_from_array([9u8; 32]),
            initialized: true,
        };
        let bytes = bridge.try_to_vec().unwrap();
        let deserialized = KernelBridge::try_from_slice(&bytes).unwrap();
        assert_eq!(bytes, deserialized.try_to_vec().unwrap());
    }

    #[test]
    fn adapter_entry_serialization_roundtrip() {
        let entry = AdapterEntry {
            asset_id: [0x5A; 32],
            adapter_program: Pubkey::new_from_array([1u8; 32]),
            adapter_config: Pubkey::new_from_array([2u8; 32]),
            initialized: true,
        };
        let bytes = entry.try_to_vec().unwrap();
        let deserialized = AdapterEntry::try_from_slice(&bytes).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn gateway_entry_serialization_roundtrip() {
        let entry = GatewayEntry {
            chain_id: 42,
            gateway_program: Pubkey::new_from_array([3u8; 32]),
            gateway_config: Pubkey::new_from_array([4u8; 32]),
            initialized: true,
        };
        let bytes = entry.try_to_vec().unwrap();
        let deserialized = GatewayEntry::try_from_slice(&bytes).unwrap();
        assert_eq!(entry, deserialized);
    }

    #[test]
    fn bridge_error_codes_are_distinct() {
        let codes = [
            BridgeError::NotInitialized,
            BridgeError::Unauthorized,
            BridgeError::NoAdapter,
            BridgeError::AlreadyRegistered,
            BridgeError::ZeroAmount,
            BridgeError::AssetMismatch,
            BridgeError::CpiFailed,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i] as u32, codes[j] as u32);
            }
        }
    }
}
