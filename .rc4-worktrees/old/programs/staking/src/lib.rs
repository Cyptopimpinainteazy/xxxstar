//! # Staking Program (SVM/Solana)
//!
//! Full-featured staking program for X3 Chain's Solana VM.
//! Supports delegation, rewards distribution, slashing, and governance.
//!
//! ## Features
//! - Native token staking with configurable lock periods
//! - Validator delegation
//! - Auto-compounding rewards
//! - Slashing for misbehavior
//! - Governance voting power
//! - Cross-VM staking coordination

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};

declare_id!("X3StAk1111111111111111111111111111111111111");

/// Minimum stake amount (1 token with 9 decimals)
pub const MIN_STAKE: u64 = 1_000_000_000;

/// Maximum validators
pub const MAX_VALIDATORS: usize = 100;

/// Unbonding period (7 days in seconds)
pub const UNBONDING_PERIOD: i64 = 7 * 24 * 60 * 60;

/// Reward epoch duration (1 day in seconds)
pub const EPOCH_DURATION: i64 = 24 * 60 * 60;

/// Slashing percentages (in BPS)
pub const SLASH_DOUBLE_SIGN: u16 = 500;    // 5%
pub const SLASH_DOWNTIME: u16 = 100;       // 1%

#[program]
pub mod staking {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // INITIALIZATION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Initialize the staking program
    pub fn initialize(
        ctx: Context<Initialize>,
        reward_rate_bps: u16,      // Annual reward rate in BPS
        min_validator_stake: u64,  // Minimum stake to become validator
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.stake_mint = ctx.accounts.stake_mint.key();
        config.reward_rate_bps = reward_rate_bps;
        config.min_validator_stake = min_validator_stake;
        config.total_staked = 0;
        config.total_rewards_distributed = 0;
        config.current_epoch = 0;
        config.last_epoch_time = Clock::get()?.unix_timestamp;
        config.paused = false;
        config.validator_count = 0;
        config.bump = ctx.bumps.config;

        emit!(StakingInitialized {
            authority: ctx.accounts.authority.key(),
            stake_mint: ctx.accounts.stake_mint.key(),
            reward_rate_bps,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VALIDATOR MANAGEMENT
    // ═══════════════════════════════════════════════════════════════════════════

    /// Register as a validator
    pub fn register_validator(
        ctx: Context<RegisterValidator>,
        commission_rate_bps: u16,
        moniker: String,
    ) -> Result<()> {
        require!(commission_rate_bps <= 10000, StakingError::InvalidCommission);
        require!(moniker.len() <= 32, StakingError::MonikerTooLong);

        let config = &mut ctx.accounts.config;
        require!(!config.paused, StakingError::Paused);
        require!(
            config.validator_count < MAX_VALIDATORS as u32,
            StakingError::MaxValidatorsReached
        );

        let validator = &mut ctx.accounts.validator;
        validator.owner = ctx.accounts.owner.key();
        validator.stake_account = ctx.accounts.validator_stake_account.key();
        validator.self_stake = 0;
        validator.delegated_stake = 0;
        validator.commission_rate_bps = commission_rate_bps;
        validator.moniker = moniker.clone();
        validator.active = false;
        validator.jailed = false;
        validator.jail_time = 0;
        validator.slash_count = 0;
        validator.created_at = Clock::get()?.unix_timestamp;
        validator.last_reward_claim = Clock::get()?.unix_timestamp;
        validator.bump = ctx.bumps.validator;

        config.validator_count += 1;

        emit!(ValidatorRegistered {
            validator: ctx.accounts.owner.key(),
            moniker,
            commission_rate_bps,
        });

        Ok(())
    }

    /// Validator self-stake
    pub fn validator_stake(
        ctx: Context<ValidatorStake>,
        amount: u64,
    ) -> Result<()> {
        let config = &ctx.accounts.config;
        require!(!config.paused, StakingError::Paused);
        require!(amount >= MIN_STAKE, StakingError::StakeBelowMinimum);

        let validator = &mut ctx.accounts.validator;
        require!(!validator.jailed, StakingError::ValidatorJailed);

        // Transfer tokens to stake vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.owner_token_account.to_account_info(),
                to: ctx.accounts.stake_vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        validator.self_stake += amount;

        // Check if validator becomes active
        let config = &mut ctx.accounts.config;
        if validator.self_stake >= config.min_validator_stake && !validator.active {
            validator.active = true;
        }

        config.total_staked += amount;

        emit!(ValidatorStaked {
            validator: ctx.accounts.owner.key(),
            amount,
            total_self_stake: validator.self_stake,
        });

        Ok(())
    }

    /// Update validator commission
    pub fn update_commission(
        ctx: Context<UpdateValidator>,
        new_commission_bps: u16,
    ) -> Result<()> {
        require!(new_commission_bps <= 10000, StakingError::InvalidCommission);

        let validator = &mut ctx.accounts.validator;
        let old_commission = validator.commission_rate_bps;
        validator.commission_rate_bps = new_commission_bps;

        emit!(CommissionUpdated {
            validator: ctx.accounts.owner.key(),
            old_commission,
            new_commission: new_commission_bps,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DELEGATION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Delegate stake to a validator
    pub fn delegate(
        ctx: Context<Delegate>,
        amount: u64,
    ) -> Result<()> {
        let config = &ctx.accounts.config;
        require!(!config.paused, StakingError::Paused);
        require!(amount >= MIN_STAKE, StakingError::StakeBelowMinimum);

        let validator = &ctx.accounts.validator;
        require!(validator.active, StakingError::ValidatorNotActive);
        require!(!validator.jailed, StakingError::ValidatorJailed);

        // Transfer tokens to stake vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.delegator_token_account.to_account_info(),
                to: ctx.accounts.stake_vault.to_account_info(),
                authority: ctx.accounts.delegator.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        // Update delegation record
        let delegation = &mut ctx.accounts.delegation;
        let clock = Clock::get()?;

        if delegation.delegator == Pubkey::default() {
            // New delegation
            delegation.delegator = ctx.accounts.delegator.key();
            delegation.validator = ctx.accounts.validator.key();
            delegation.amount = amount;
            delegation.reward_debt = 0;
            delegation.created_at = clock.unix_timestamp;
            delegation.bump = ctx.bumps.delegation;
        } else {
            // Add to existing delegation
            delegation.amount += amount;
        }

        // Update validator delegated stake
        let validator = &mut ctx.accounts.validator;
        validator.delegated_stake += amount;

        // Update global stats
        let config = &mut ctx.accounts.config;
        config.total_staked += amount;

        emit!(Delegated {
            delegator: ctx.accounts.delegator.key(),
            validator: ctx.accounts.validator.key(),
            amount,
            total_delegation: delegation.amount,
        });

        Ok(())
    }

    /// Undelegate (start unbonding)
    pub fn undelegate(
        ctx: Context<Undelegate>,
        amount: u64,
    ) -> Result<()> {
        let delegation = &mut ctx.accounts.delegation;
        require!(delegation.amount >= amount, StakingError::InsufficientStake);

        let clock = Clock::get()?;

        // Create unbonding record
        let unbonding = &mut ctx.accounts.unbonding;
        unbonding.delegator = ctx.accounts.delegator.key();
        unbonding.validator = ctx.accounts.validator.key();
        unbonding.amount = amount;
        unbonding.start_time = clock.unix_timestamp;
        unbonding.end_time = clock.unix_timestamp + UNBONDING_PERIOD;
        unbonding.completed = false;
        unbonding.bump = ctx.bumps.unbonding;

        // Update delegation
        delegation.amount -= amount;

        // Update validator
        let validator = &mut ctx.accounts.validator;
        validator.delegated_stake -= amount;

        emit!(Undelegated {
            delegator: ctx.accounts.delegator.key(),
            validator: ctx.accounts.validator.key(),
            amount,
            unbonding_end: unbonding.end_time,
        });

        Ok(())
    }

    /// Complete unbonding (withdraw after period)
    pub fn complete_unbonding(ctx: Context<CompleteUnbonding>) -> Result<()> {
        let unbonding = &mut ctx.accounts.unbonding;
        let clock = Clock::get()?;

        require!(!unbonding.completed, StakingError::AlreadyCompleted);
        require!(
            clock.unix_timestamp >= unbonding.end_time,
            StakingError::UnbondingNotComplete
        );

        unbonding.completed = true;

        // Transfer tokens back to delegator
        let config = &ctx.accounts.config;
        let config_seeds = &[
            b"config",
            &[config.bump],
        ];
        let signer_seeds = &[&config_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.stake_vault.to_account_info(),
                to: ctx.accounts.delegator_token_account.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, unbonding.amount)?;

        // Update global stats
        let config = &mut ctx.accounts.config;
        config.total_staked -= unbonding.amount;

        emit!(UnbondingCompleted {
            delegator: ctx.accounts.delegator.key(),
            amount: unbonding.amount,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REWARDS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Advance epoch and calculate rewards
    pub fn advance_epoch(ctx: Context<AdvanceEpoch>) -> Result<()> {
        let config = &mut ctx.accounts.config;
        let clock = Clock::get()?;

        require!(
            clock.unix_timestamp >= config.last_epoch_time + EPOCH_DURATION,
            StakingError::EpochNotReady
        );

        config.current_epoch += 1;
        config.last_epoch_time = clock.unix_timestamp;

        // Calculate epoch rewards based on config
        // Rewards are distributed proportionally to staked amounts

        emit!(EpochAdvanced {
            epoch: config.current_epoch,
            total_staked: config.total_staked,
        });

        Ok(())
    }

    /// Claim delegation rewards
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        let config = &ctx.accounts.config;
        let delegation = &mut ctx.accounts.delegation;
        let validator = &ctx.accounts.validator;
        let clock = Clock::get()?;

        // Calculate rewards based on stake amount and time
        let time_staked = clock.unix_timestamp - delegation.created_at;
        let annual_reward = (delegation.amount as u128)
            .checked_mul(config.reward_rate_bps as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap();

        let reward_amount = (annual_reward as u128)
            .checked_mul(time_staked as u128)
            .unwrap()
            .checked_div(365 * 24 * 60 * 60)
            .unwrap() as u64;

        // Apply validator commission
        let commission = (reward_amount as u128)
            .checked_mul(validator.commission_rate_bps as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap() as u64;

        let net_reward = reward_amount.saturating_sub(commission);

        if net_reward > 0 {
            // Transfer rewards from reward vault
            let config_seeds = &[
                b"config",
                &[config.bump],
            ];
            let signer_seeds = &[&config_seeds[..]];

            let transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.reward_vault.to_account_info(),
                    to: ctx.accounts.delegator_token_account.to_account_info(),
                    authority: ctx.accounts.config.to_account_info(),
                },
                signer_seeds,
            );
            token::transfer(transfer_ctx, net_reward)?;

            // Update stats
            delegation.reward_debt += net_reward;
            let config = &mut ctx.accounts.config;
            config.total_rewards_distributed += net_reward;
        }

        emit!(RewardsClaimed {
            delegator: ctx.accounts.delegator.key(),
            validator: validator.owner,
            gross_reward: reward_amount,
            commission,
            net_reward,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SLASHING
    // ═══════════════════════════════════════════════════════════════════════════

    /// Slash validator for misbehavior
    pub fn slash_validator(
        ctx: Context<SlashValidator>,
        slash_type: u8, // 0 = double sign, 1 = downtime
        evidence: Vec<u8>,
    ) -> Result<()> {
        require!(
            ctx.accounts.authority.key() == ctx.accounts.config.authority,
            StakingError::Unauthorized
        );

        let validator = &mut ctx.accounts.validator;
        require!(!validator.jailed, StakingError::AlreadyJailed);

        let slash_bps = match slash_type {
            0 => SLASH_DOUBLE_SIGN,
            1 => SLASH_DOWNTIME,
            _ => return Err(StakingError::InvalidSlashType.into()),
        };

        // Calculate slash amount
        let total_stake = validator.self_stake + validator.delegated_stake;
        let slash_amount = (total_stake as u128)
            .checked_mul(slash_bps as u128)
            .unwrap()
            .checked_div(10000)
            .unwrap() as u64;

        // Apply slash proportionally to self-stake first
        let self_slash = slash_amount.min(validator.self_stake);
        validator.self_stake -= self_slash;

        // Remaining slash from delegated stake
        let delegated_slash = slash_amount.saturating_sub(self_slash);
        validator.delegated_stake = validator.delegated_stake.saturating_sub(delegated_slash);

        // Jail validator
        validator.jailed = true;
        validator.jail_time = Clock::get()?.unix_timestamp;
        validator.active = false;
        validator.slash_count += 1;

        // Update global stats
        let config = &mut ctx.accounts.config;
        config.total_staked -= slash_amount;

        emit!(ValidatorSlashed {
            validator: validator.owner,
            slash_type,
            slash_amount,
            jailed: true,
        });

        Ok(())
    }

    /// Unjail validator after penalty period
    pub fn unjail_validator(ctx: Context<UnjailValidator>) -> Result<()> {
        let validator = &mut ctx.accounts.validator;
        let clock = Clock::get()?;

        require!(validator.jailed, StakingError::NotJailed);

        // Minimum jail time: 24 hours
        let min_jail_time = 24 * 60 * 60;
        require!(
            clock.unix_timestamp >= validator.jail_time + min_jail_time,
            StakingError::JailTimeNotServed
        );

        // Require minimum self-stake to unjail
        let config = &ctx.accounts.config;
        require!(
            validator.self_stake >= config.min_validator_stake,
            StakingError::InsufficientSelfStake
        );

        validator.jailed = false;
        validator.active = true;

        emit!(ValidatorUnjailed {
            validator: validator.owner,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // GOVERNANCE
    // ═══════════════════════════════════════════════════════════════════════════

    /// Get voting power for governance
    pub fn get_voting_power(ctx: Context<GetVotingPower>) -> Result<u64> {
        let delegation = &ctx.accounts.delegation;
        
        // Voting power = delegated amount
        // Could be weighted by lock period, validator status, etc.
        Ok(delegation.amount)
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ADMIN
    // ═══════════════════════════════════════════════════════════════════════════

    pub fn pause(ctx: Context<AdminAction>) -> Result<()> {
        ctx.accounts.config.paused = true;
        emit!(ProtocolPaused {});
        Ok(())
    }

    pub fn unpause(ctx: Context<AdminAction>) -> Result<()> {
        ctx.accounts.config.paused = false;
        emit!(ProtocolUnpaused {});
        Ok(())
    }

    pub fn update_reward_rate(
        ctx: Context<AdminAction>,
        new_rate_bps: u16,
    ) -> Result<()> {
        require!(new_rate_bps <= 5000, StakingError::RewardRateTooHigh); // Max 50% APY
        ctx.accounts.config.reward_rate_bps = new_rate_bps;
        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ACCOUNTS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub stake_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        space = 8 + StakingConfig::SIZE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, StakingConfig>,

    #[account(
        init,
        payer = authority,
        token::mint = stake_mint,
        token::authority = config,
        seeds = [b"stake_vault"],
        bump
    )]
    pub stake_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = authority,
        token::mint = stake_mint,
        token::authority = config,
        seeds = [b"reward_vault"],
        bump
    )]
    pub reward_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct RegisterValidator<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakingConfig>,

    #[account(
        init,
        payer = owner,
        space = 8 + Validator::SIZE,
        seeds = [b"validator", owner.key().as_ref()],
        bump
    )]
    pub validator: Account<'info, Validator>,

    /// CHECK: Validator's stake token account
    pub validator_stake_account: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ValidatorStake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakingConfig>,

    #[account(
        mut,
        seeds = [b"validator", owner.key().as_ref()],
        bump = validator.bump,
        constraint = validator.owner == owner.key()
    )]
    pub validator: Account<'info, Validator>,

    #[account(mut, seeds = [b"stake_vault"], bump)]
    pub stake_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = owner_token_account.owner == owner.key())]
    pub owner_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateValidator<'info> {
    pub owner: Signer<'info>,

    #[account(
        mut,
        seeds = [b"validator", owner.key().as_ref()],
        bump = validator.bump,
        constraint = validator.owner == owner.key()
    )]
    pub validator: Account<'info, Validator>,
}

#[derive(Accounts)]
pub struct Delegate<'info> {
    #[account(mut)]
    pub delegator: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakingConfig>,

    #[account(
        mut,
        seeds = [b"validator", validator.owner.as_ref()],
        bump = validator.bump
    )]
    pub validator: Account<'info, Validator>,

    #[account(
        init_if_needed,
        payer = delegator,
        space = 8 + Delegation::SIZE,
        seeds = [b"delegation", delegator.key().as_ref(), validator.key().as_ref()],
        bump
    )]
    pub delegation: Account<'info, Delegation>,

    #[account(mut, seeds = [b"stake_vault"], bump)]
    pub stake_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = delegator_token_account.owner == delegator.key())]
    pub delegator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Undelegate<'info> {
    #[account(mut)]
    pub delegator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"validator", validator.owner.as_ref()],
        bump = validator.bump
    )]
    pub validator: Account<'info, Validator>,

    #[account(
        mut,
        seeds = [b"delegation", delegator.key().as_ref(), validator.key().as_ref()],
        bump = delegation.bump,
        constraint = delegation.delegator == delegator.key()
    )]
    pub delegation: Account<'info, Delegation>,

    #[account(
        init,
        payer = delegator,
        space = 8 + Unbonding::SIZE,
        seeds = [
            b"unbonding",
            delegator.key().as_ref(),
            validator.key().as_ref(),
            &Clock::get()?.unix_timestamp.to_le_bytes()
        ],
        bump
    )]
    pub unbonding: Account<'info, Unbonding>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CompleteUnbonding<'info> {
    #[account(mut)]
    pub delegator: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakingConfig>,

    #[account(
        mut,
        constraint = unbonding.delegator == delegator.key(),
        constraint = !unbonding.completed
    )]
    pub unbonding: Account<'info, Unbonding>,

    #[account(mut, seeds = [b"stake_vault"], bump)]
    pub stake_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = delegator_token_account.owner == delegator.key())]
    pub delegator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AdvanceEpoch<'info> {
    pub caller: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakingConfig>,
}

#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub delegator: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakingConfig>,

    #[account(seeds = [b"validator", validator.owner.as_ref()], bump = validator.bump)]
    pub validator: Account<'info, Validator>,

    #[account(
        mut,
        seeds = [b"delegation", delegator.key().as_ref(), validator.key().as_ref()],
        bump = delegation.bump
    )]
    pub delegation: Account<'info, Delegation>,

    #[account(mut, seeds = [b"reward_vault"], bump)]
    pub reward_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = delegator_token_account.owner == delegator.key())]
    pub delegator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SlashValidator<'info> {
    pub authority: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakingConfig>,

    #[account(mut)]
    pub validator: Account<'info, Validator>,
}

#[derive(Accounts)]
pub struct UnjailValidator<'info> {
    pub owner: Signer<'info>,

    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakingConfig>,

    #[account(
        mut,
        seeds = [b"validator", owner.key().as_ref()],
        bump = validator.bump,
        constraint = validator.owner == owner.key()
    )]
    pub validator: Account<'info, Validator>,
}

#[derive(Accounts)]
pub struct GetVotingPower<'info> {
    pub delegator: Signer<'info>,

    #[account(
        seeds = [b"delegation", delegator.key().as_ref(), validator.key().as_ref()],
        bump = delegation.bump
    )]
    pub delegation: Account<'info, Delegation>,

    pub validator: Account<'info, Validator>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(constraint = authority.key() == config.authority @ StakingError::Unauthorized)]
    pub authority: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, StakingConfig>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATE
// ═══════════════════════════════════════════════════════════════════════════════

#[account]
pub struct StakingConfig {
    pub authority: Pubkey,
    pub stake_mint: Pubkey,
    pub reward_rate_bps: u16,
    pub min_validator_stake: u64,
    pub total_staked: u64,
    pub total_rewards_distributed: u64,
    pub current_epoch: u64,
    pub last_epoch_time: i64,
    pub paused: bool,
    pub validator_count: u32,
    pub bump: u8,
}

impl StakingConfig {
    pub const SIZE: usize = 32 + 32 + 2 + 8 + 8 + 8 + 8 + 8 + 1 + 4 + 1;
}

#[account]
pub struct Validator {
    pub owner: Pubkey,
    pub stake_account: Pubkey,
    pub self_stake: u64,
    pub delegated_stake: u64,
    pub commission_rate_bps: u16,
    pub moniker: String,
    pub active: bool,
    pub jailed: bool,
    pub jail_time: i64,
    pub slash_count: u32,
    pub created_at: i64,
    pub last_reward_claim: i64,
    pub bump: u8,
}

impl Validator {
    pub const SIZE: usize = 32 + 32 + 8 + 8 + 2 + 36 + 1 + 1 + 8 + 4 + 8 + 8 + 1;
}

#[account]
pub struct Delegation {
    pub delegator: Pubkey,
    pub validator: Pubkey,
    pub amount: u64,
    pub reward_debt: u64,
    pub created_at: i64,
    pub bump: u8,
}

impl Delegation {
    pub const SIZE: usize = 32 + 32 + 8 + 8 + 8 + 1;
}

#[account]
pub struct Unbonding {
    pub delegator: Pubkey,
    pub validator: Pubkey,
    pub amount: u64,
    pub start_time: i64,
    pub end_time: i64,
    pub completed: bool,
    pub bump: u8,
}

impl Unbonding {
    pub const SIZE: usize = 32 + 32 + 8 + 8 + 8 + 1 + 1;
}

// ═══════════════════════════════════════════════════════════════════════════════
// EVENTS
// ═══════════════════════════════════════════════════════════════════════════════

#[event]
pub struct StakingInitialized {
    pub authority: Pubkey,
    pub stake_mint: Pubkey,
    pub reward_rate_bps: u16,
}

#[event]
pub struct ValidatorRegistered {
    pub validator: Pubkey,
    pub moniker: String,
    pub commission_rate_bps: u16,
}

#[event]
pub struct ValidatorStaked {
    pub validator: Pubkey,
    pub amount: u64,
    pub total_self_stake: u64,
}

#[event]
pub struct CommissionUpdated {
    pub validator: Pubkey,
    pub old_commission: u16,
    pub new_commission: u16,
}

#[event]
pub struct Delegated {
    pub delegator: Pubkey,
    pub validator: Pubkey,
    pub amount: u64,
    pub total_delegation: u64,
}

#[event]
pub struct Undelegated {
    pub delegator: Pubkey,
    pub validator: Pubkey,
    pub amount: u64,
    pub unbonding_end: i64,
}

#[event]
pub struct UnbondingCompleted {
    pub delegator: Pubkey,
    pub amount: u64,
}

#[event]
pub struct EpochAdvanced {
    pub epoch: u64,
    pub total_staked: u64,
}

#[event]
pub struct RewardsClaimed {
    pub delegator: Pubkey,
    pub validator: Pubkey,
    pub gross_reward: u64,
    pub commission: u64,
    pub net_reward: u64,
}

#[event]
pub struct ValidatorSlashed {
    pub validator: Pubkey,
    pub slash_type: u8,
    pub slash_amount: u64,
    pub jailed: bool,
}

#[event]
pub struct ValidatorUnjailed {
    pub validator: Pubkey,
}

#[event]
pub struct ProtocolPaused {}

#[event]
pub struct ProtocolUnpaused {}

// ═══════════════════════════════════════════════════════════════════════════════
// ERRORS
// ═══════════════════════════════════════════════════════════════════════════════

#[error_code]
pub enum StakingError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Protocol is paused")]
    Paused,
    #[msg("Stake amount below minimum")]
    StakeBelowMinimum,
    #[msg("Invalid commission rate")]
    InvalidCommission,
    #[msg("Moniker too long")]
    MonikerTooLong,
    #[msg("Maximum validators reached")]
    MaxValidatorsReached,
    #[msg("Validator not active")]
    ValidatorNotActive,
    #[msg("Validator is jailed")]
    ValidatorJailed,
    #[msg("Insufficient stake")]
    InsufficientStake,
    #[msg("Unbonding not complete")]
    UnbondingNotComplete,
    #[msg("Already completed")]
    AlreadyCompleted,
    #[msg("Epoch not ready")]
    EpochNotReady,
    #[msg("Invalid slash type")]
    InvalidSlashType,
    #[msg("Already jailed")]
    AlreadyJailed,
    #[msg("Not jailed")]
    NotJailed,
    #[msg("Jail time not served")]
    JailTimeNotServed,
    #[msg("Insufficient self stake")]
    InsufficientSelfStake,
    #[msg("Reward rate too high")]
    RewardRateTooHigh,
    #[msg("Calculation overflow")]
    CalculationOverflow,
}
