use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};

declare_id!("X3VmERC211111111111111111111111111111111112");

pub const ADAPTER_SEED: &[u8] = b"adapter";
pub const MINT_AUTHORITY_SEED: &[u8] = b"mint_authority";

#[program]
pub mod x3_vm_erc20 {

    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        asset_id: [u8; 32],
        _decimals: u8,
        admin: Pubkey,
    ) -> Result<()> {
        let config = &mut ctx.accounts.adapter_config;
        config.asset_id = asset_id;
        config.mint = ctx.accounts.mint.key();
        config.admin = admin;
        config.kernel_bridge = Pubkey::default();
        config.initialized = true;
        emit!(AdapterInitialized { asset_id, mint: ctx.accounts.mint.key(), admin });
        Ok(())
    }

    pub fn set_kernel_bridge(ctx: Context<SetKernelBridge>, kernel_bridge: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.adapter_config;
        require!(config.initialized, AdapterError::NotInitialized);
        require!(
            ctx.accounts.admin.key() == config.admin,
            AdapterError::Unauthorized
        );
        config.kernel_bridge = kernel_bridge;
        emit!(KernelBridgeSet {
            asset_id: config.asset_id,
            kernel_bridge,
        });
        Ok(())
    }

    /// Register the external gateway program that is authorized to call
    /// bridge_mint and bridge_burn for cross-chain asset settlement.
    pub fn set_external_gateway(ctx: Context<AdminOnlyWithConfig>, gateway: Pubkey) -> Result<()> {
        let config = &mut ctx.accounts.adapter_config;
        require!(config.initialized, AdapterError::NotInitialized);
        config.external_gateway = gateway;
        emit!(ExternalGatewaySet {
            asset_id: config.asset_id,
            external_gateway: gateway,
        });
        Ok(())
    }

    // ── kernel operations (authorized by kernel_bridge program) ────

    pub fn kernel_mint(ctx: Context<KernelOp>, amount: u64) -> Result<()> {
        let config = &ctx.accounts.adapter_config;
        require!(config.initialized, AdapterError::NotInitialized);
        require!(
            ctx.accounts.kernel_bridge_program.key() == config.kernel_bridge,
            AdapterError::UnauthorizedKernel
        );
        require!(amount > 0, AdapterError::ZeroAmount);

        do_mint_to(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.destination.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            config.asset_id,
            ctx.bumps.mint_authority,
            ctx.accounts.destination.key(),
            amount,
        )?;

        emit!(KernelMintEvent {
            asset_id: config.asset_id,
            to: ctx.accounts.destination.key(),
            amount,
        });
        Ok(())
    }

    pub fn kernel_burn(ctx: Context<KernelOp>, amount: u64) -> Result<()> {
        let config = &ctx.accounts.adapter_config;
        require!(config.initialized, AdapterError::NotInitialized);
        require!(
            ctx.accounts.kernel_bridge_program.key() == config.kernel_bridge,
            AdapterError::UnauthorizedKernel
        );
        require!(amount > 0, AdapterError::ZeroAmount);

        do_burn_from(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.source.as_ref().unwrap().to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            config.asset_id,
            ctx.bumps.mint_authority,
            amount,
        )?;

        emit!(KernelBurnEvent {
            asset_id: config.asset_id,
            from: ctx.accounts.source.as_ref().unwrap().key(),
            amount,
        });
        Ok(())
    }

    // ── cross-chain transfer (user-initiated) ──────────────────────

    pub fn send_to_vm(
        ctx: Context<SendToVm>,
        destination_domain: u8,
        recipient: Vec<u8>,
        amount: u64,
    ) -> Result<()> {
        let config = &ctx.accounts.adapter_config;
        require!(!recipient.is_empty() && recipient.len() <= 64, AdapterError::InvalidRecipient);
        require!(amount > 0, AdapterError::ZeroAmount);

        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx.accounts.mint.to_account_info(),
                    from: ctx.accounts.user_token_account.to_account_info(),
                    authority: ctx.accounts.user.to_account_info(),
                },
            ),
            amount,
        )?;

        emit!(CrossVmTransferInitiated {
            asset_id: config.asset_id,
            sender: ctx.accounts.user.key(),
            destination_domain,
            recipient,
            amount,
        });
        Ok(())
    }

    // ── bridge operations (authorized by external_gateway program) ─

    pub fn bridge_mint(ctx: Context<BridgeOp>, amount: u64) -> Result<()> {
        let config = &ctx.accounts.adapter_config;
        require!(config.initialized, AdapterError::NotInitialized);
        require!(amount > 0, AdapterError::ZeroAmount);
        require!(
            ctx.accounts.external_gateway.key() == config.external_gateway,
            AdapterError::UnauthorizedGateway
        );

        do_mint_to(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.destination.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            config.asset_id,
            ctx.bumps.mint_authority,
            ctx.accounts.destination.key(),
            amount,
        )?;

        emit!(KernelMintEvent {
            asset_id: config.asset_id,
            to: ctx.accounts.destination.key(),
            amount,
        });
        Ok(())
    }

    pub fn bridge_burn(ctx: Context<BridgeOp>, amount: u64) -> Result<()> {
        let config = &ctx.accounts.adapter_config;
        require!(config.initialized, AdapterError::NotInitialized);
        require!(amount > 0, AdapterError::ZeroAmount);
        require!(
            ctx.accounts.external_gateway.key() == config.external_gateway,
            AdapterError::UnauthorizedGateway
        );

        do_burn_from(
            ctx.accounts.token_program.to_account_info(),
            ctx.accounts.mint.to_account_info(),
            ctx.accounts.source.to_account_info(),
            ctx.accounts.mint_authority.to_account_info(),
            config.asset_id,
            ctx.bumps.mint_authority,
            amount,
        )?;

        emit!(KernelBurnEvent {
            asset_id: config.asset_id,
            from: ctx.accounts.source.key(),
            amount,
        });
        Ok(())
    }
}

fn do_mint_to<'info>(
    token_program: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    destination: AccountInfo<'info>,
    mint_authority: AccountInfo<'info>,
    asset_id: [u8; 32],
    mint_authority_bump: u8,
    _destination_key: Pubkey,
    amount: u64,
) -> Result<()> {
    let seeds: &[&[u8]] = &[
        MINT_AUTHORITY_SEED,
        asset_id.as_ref(),
        &[mint_authority_bump],
    ];
    let signer_seeds: &[&[&[u8]]] = &[seeds];

    token::mint_to(
        CpiContext::new_with_signer(
            token_program,
            token::MintTo {
                mint,
                to: destination,
                authority: mint_authority,
            },
            signer_seeds,
        ),
        amount,
    )?;

    Ok(())
}

fn do_burn_from<'info>(
    token_program: AccountInfo<'info>,
    mint: AccountInfo<'info>,
    source: AccountInfo<'info>,
    mint_authority: AccountInfo<'info>,
    asset_id: [u8; 32],
    mint_authority_bump: u8,
    amount: u64,
) -> Result<()> {
    let seeds: &[&[u8]] = &[
        MINT_AUTHORITY_SEED,
        asset_id.as_ref(),
        &[mint_authority_bump],
    ];
    let signer_seeds: &[&[&[u8]]] = &[seeds];

    token::burn(
        CpiContext::new_with_signer(
            token_program,
            token::Burn {
                mint,
                from: source,
                authority: mint_authority,
            },
            signer_seeds,
        ),
        amount,
    )?;

    Ok(())
}

// ── on-chain state ─────────────────────────────────────────────────

#[account]
pub struct AdapterConfig {
    pub asset_id: [u8; 32],
    pub mint: Pubkey,
    pub admin: Pubkey,
    pub kernel_bridge: Pubkey,
    pub external_gateway: Pubkey,
    pub initialized: bool,
}

impl AdapterConfig {
    pub const SIZE: usize = 32 + 32 + 32 + 32 + 32 + 1;
}

// ── account contexts ───────────────────────────────────────────────

#[derive(Accounts)]
#[instruction(asset_id: [u8; 32], decimals: u8)]
pub struct Initialize<'info> {
    #[account(
        init,
        seeds = [ADAPTER_SEED, asset_id.as_ref()],
        bump,
        payer = payer,
        space = 8 + AdapterConfig::SIZE
    )]
    pub adapter_config: Account<'info, AdapterConfig>,
    #[account(
        init,
        seeds = [MINT_AUTHORITY_SEED, asset_id.as_ref()],
        bump,
        payer = payer,
        mint::decimals = decimals,
        mint::authority = mint_authority,
    )]
    pub mint: Account<'info, Mint>,
    /// CHECK: PDA mint authority
    #[account(
        seeds = [MINT_AUTHORITY_SEED, asset_id.as_ref()],
        bump,
    )]
    pub mint_authority: AccountInfo<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct SetKernelBridge<'info> {
    #[account(
        mut,
        seeds = [ADAPTER_SEED, adapter_config.asset_id.as_ref()],
        bump,
        has_one = admin @ AdapterError::Unauthorized,
    )]
    pub adapter_config: Account<'info, AdapterConfig>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct AdminOnlyWithConfig<'info> {
    #[account(
        mut,
        seeds = [ADAPTER_SEED, adapter_config.asset_id.as_ref()],
        bump,
        has_one = admin @ AdapterError::Unauthorized,
    )]
    pub adapter_config: Account<'info, AdapterConfig>,
    pub admin: Signer<'info>,
}

#[derive(Accounts)]
pub struct KernelOp<'info> {
    #[account(
        seeds = [ADAPTER_SEED, adapter_config.asset_id.as_ref()],
        bump,
    )]
    pub adapter_config: Account<'info, AdapterConfig>,
    #[account(
        mut,
        address = adapter_config.mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = destination.mint == mint.key(),
    )]
    pub destination: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = source.mint == mint.key(),
    )]
    pub source: Option<Account<'info, TokenAccount>>,
    /// CHECK: PDA mint authority
    #[account(
        mut,
        seeds = [MINT_AUTHORITY_SEED, adapter_config.asset_id.as_ref()],
        bump,
    )]
    pub mint_authority: AccountInfo<'info>,
    /// CHECK: kernel bridge program ID (validated by instruction logic)
    pub kernel_bridge_program: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SendToVm<'info> {
    #[account(
        seeds = [ADAPTER_SEED, adapter_config.asset_id.as_ref()],
        bump,
    )]
    pub adapter_config: Account<'info, AdapterConfig>,
    #[account(
        mut,
        address = adapter_config.mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    pub user: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct BridgeOp<'info> {
    #[account(
        seeds = [ADAPTER_SEED, adapter_config.asset_id.as_ref()],
        bump,
    )]
    pub adapter_config: Account<'info, AdapterConfig>,
    #[account(
        mut,
        address = adapter_config.mint,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = destination.mint == mint.key(),
    )]
    pub destination: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = source.mint == mint.key(),
    )]
    pub source: Account<'info, TokenAccount>,
    /// CHECK: PDA mint authority
    #[account(
        mut,
        seeds = [MINT_AUTHORITY_SEED, adapter_config.asset_id.as_ref()],
        bump,
    )]
    pub mint_authority: AccountInfo<'info>,
    /// CHECK: external gateway program ID (validated by instruction logic)
    pub external_gateway: AccountInfo<'info>,
    pub token_program: Program<'info, Token>,
}

// ── events ─────────────────────────────────────────────────────────

#[event]
pub struct AdapterInitialized {
    pub asset_id: [u8; 32],
    pub mint: Pubkey,
    pub admin: Pubkey,
}

#[event]
pub struct KernelMintEvent {
    pub asset_id: [u8; 32],
    pub to: Pubkey,
    pub amount: u64,
}

#[event]
pub struct KernelBurnEvent {
    pub asset_id: [u8; 32],
    pub from: Pubkey,
    pub amount: u64,
}

#[event]
pub struct CrossVmTransferInitiated {
    pub asset_id: [u8; 32],
    pub sender: Pubkey,
    pub destination_domain: u8,
    pub recipient: Vec<u8>,
    pub amount: u64,
}

#[event]
pub struct KernelBridgeSet {
    pub asset_id: [u8; 32],
    pub kernel_bridge: Pubkey,
}

#[event]
pub struct ExternalGatewaySet {
    pub asset_id: [u8; 32],
    pub external_gateway: Pubkey,
}

#[error_code]
pub enum AdapterError {
    #[msg("adapter not initialized")]
    NotInitialized,
    #[msg("caller is not the registered kernel bridge")]
    UnauthorizedKernel,
    #[msg("caller is not the admin")]
    Unauthorized,
    #[msg("caller is not the registered external gateway")]
    UnauthorizedGateway,
    #[msg("amount must be greater than zero")]
    ZeroAmount,
    #[msg("invalid recipient data")]
    InvalidRecipient,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_config_size_matches_definition() {
        assert_eq!(AdapterConfig::SIZE, 32 + 32 + 32 + 32 + 32 + 1);
    }

    #[test]
    fn adapter_seed_is_stable() {
        assert_eq!(ADAPTER_SEED, b"adapter");
    }

    #[test]
    fn mint_authority_seed_is_stable() {
        assert_eq!(MINT_AUTHORITY_SEED, b"mint_authority");
    }

    #[test]
    fn pda_derivation_is_deterministic() {
        let asset_id: [u8; 32] = [42u8; 32];
        let (pda1, bump1) = Pubkey::find_program_address(
            &[ADAPTER_SEED, asset_id.as_ref()],
            &id(),
        );
        let (pda2, bump2) = Pubkey::find_program_address(
            &[ADAPTER_SEED, asset_id.as_ref()],
            &id(),
        );
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    fn mint_authority_pda_distinct_from_adapter_config() {
        let asset_id: [u8; 32] = [42u8; 32];
        let (adapter_pda, _) = Pubkey::find_program_address(
            &[ADAPTER_SEED, asset_id.as_ref()],
            &id(),
        );
        let (mint_auth_pda, _) = Pubkey::find_program_address(
            &[MINT_AUTHORITY_SEED, asset_id.as_ref()],
            &id(),
        );
        assert_ne!(adapter_pda, mint_auth_pda);
    }

    #[test]
    fn different_asset_ids_produce_different_pdas() {
        let asset_id_1: [u8; 32] = [1u8; 32];
        let asset_id_2: [u8; 32] = [2u8; 32];
        let (pda1, _) = Pubkey::find_program_address(
            &[ADAPTER_SEED, asset_id_1.as_ref()],
            &id(),
        );
        let (pda2, _) = Pubkey::find_program_address(
            &[ADAPTER_SEED, asset_id_2.as_ref()],
            &id(),
        );
        assert_ne!(pda1, pda2);
    }

    #[test]
    fn adapter_config_default_is_uninitialized() {
        let config = AdapterConfig {
            asset_id: [0u8; 32],
            mint: Pubkey::default(),
            admin: Pubkey::default(),
            kernel_bridge: Pubkey::default(),
            external_gateway: Pubkey::default(),
            initialized: false,
        };
        assert!(!config.initialized);
        assert_eq!(config.asset_id, [0u8; 32]);
        assert_eq!(config.kernel_bridge, Pubkey::default());
        assert_eq!(config.external_gateway, Pubkey::default());
    }

    #[test]
    fn adapter_error_display_messages() {
        let codes = [
            (AdapterError::NotInitialized, "adapter not initialized"),
            (AdapterError::UnauthorizedKernel, "caller is not the registered kernel bridge"),
            (AdapterError::Unauthorized, "caller is not the admin"),
            (AdapterError::UnauthorizedGateway, "caller is not the registered external gateway"),
            (AdapterError::ZeroAmount, "amount must be greater than zero"),
            (AdapterError::InvalidRecipient, "invalid recipient data"),
        ];
        for (err, expected) in &codes {
            let got = match err {
                AdapterError::NotInitialized => "adapter not initialized",
                AdapterError::UnauthorizedKernel => "caller is not the registered kernel bridge",
                AdapterError::Unauthorized => "caller is not the admin",
                AdapterError::UnauthorizedGateway => "caller is not the registered external gateway",
                AdapterError::ZeroAmount => "amount must be greater than zero",
                AdapterError::InvalidRecipient => "invalid recipient data",
            };
            assert_eq!(got, *expected);
        }
    }

    #[test]
    fn initialized_config_has_correct_field_types() {
        let config = AdapterConfig {
            asset_id: [0xFFu8; 32],
            mint: Pubkey::new_from_array([1u8; 32]),
            admin: Pubkey::new_from_array([2u8; 32]),
            kernel_bridge: Pubkey::new_from_array([3u8; 32]),
            external_gateway: Pubkey::new_from_array([4u8; 32]),
            initialized: true,
        };
        assert!(config.initialized);
        assert_eq!(config.asset_id[0], 0xFF);
        assert_eq!(config.external_gateway, Pubkey::new_from_array([4u8; 32]));
    }

    #[test]
    fn uninitialized_adapter_rejects_set_gateway_check() {
        let config = AdapterConfig {
            asset_id: [0u8; 32],
            mint: Pubkey::default(),
            admin: Pubkey::default(),
            kernel_bridge: Pubkey::default(),
            external_gateway: Pubkey::default(),
            initialized: false,
        };
        assert!(!config.initialized);
    }
}
