//! # X3 HTLC Program (SVM/Solana)
//!
//! Hash Time-Locked Contract for atomic swaps on Solana/SVM.
//! Counterpart to AtlasHTLC.sol on EVM chains.
//!
//! ## Cross-Chain Atomic Swap Flow
//!
//! ```text
//! 1. Initiator creates HTLC on Source Chain (locks funds with hashlock)
//! 2. Responder sees hashlock, creates HTLC on Destination Chain
//! 3. Initiator claims on Destination (reveals preimage)
//! 4. Responder uses revealed preimage to claim on Source
//! ```

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("X3HtLc111111111111111111111111111111111111111");

/// Maximum timelock duration (7 days in seconds)
pub const MAX_TIMELOCK: i64 = 7 * 24 * 60 * 60;

/// Minimum timelock duration (1 hour in seconds)  
pub const MIN_TIMELOCK: i64 = 60 * 60;

/// HTLC account size calculation
pub const HTLC_SIZE: usize = 8 +  // discriminator
    32 +                          // initiator
    32 +                          // recipient
    32 +                          // token_mint
    8 +                           // amount
    32 +                          // hashlock
    8 +                           // timelock
    1 +                           // status (enum)
    32 +                          // preimage (optional, stored on claim)
    8 +                           // created_at
    1 +                           // bump
    64;                           // padding for future use

#[program]
pub mod x3_htlc {
    use super::*;

    /// Create a new Hash Time-Locked Contract
    ///
    /// # Arguments
    /// * `hashlock` - SHA256 hash of the secret preimage
    /// * `timelock` - Unix timestamp after which initiator can refund
    /// * `amount` - Amount of tokens to lock
    pub fn create_htlc(
        ctx: Context<CreateHtlc>,
        hashlock: [u8; 32],
        timelock: i64,
        amount: u64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;

        // Validate timelock
        require!(
            timelock > current_time + MIN_TIMELOCK,
            HtlcError::TimelockTooShort
        );
        require!(
            timelock < current_time + MAX_TIMELOCK,
            HtlcError::TimelockTooLong
        );
        require!(amount > 0, HtlcError::InvalidAmount);

        // Initialize HTLC state
        let htlc = &mut ctx.accounts.htlc;
        htlc.initiator = ctx.accounts.initiator.key();
        htlc.recipient = ctx.accounts.recipient.key();
        htlc.token_mint = ctx.accounts.token_mint.key();
        htlc.amount = amount;
        htlc.hashlock = hashlock;
        htlc.timelock = timelock;
        htlc.status = HtlcStatus::Funded;
        htlc.preimage = [0u8; 32];
        htlc.created_at = current_time;
        htlc.bump = ctx.bumps.htlc;

        // Transfer tokens from initiator to HTLC vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.initiator_token_account.to_account_info(),
                to: ctx.accounts.htlc_vault.to_account_info(),
                authority: ctx.accounts.initiator.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        emit!(HtlcCreated {
            htlc_id: htlc.key(),
            initiator: htlc.initiator,
            recipient: htlc.recipient,
            token_mint: htlc.token_mint,
            amount,
            hashlock,
            timelock,
        });

        Ok(())
    }

    /// Claim the HTLC by revealing the preimage
    ///
    /// # Arguments
    /// * `preimage` - The secret that hashes to the hashlock
    pub fn claim_htlc(ctx: Context<ClaimHtlc>, preimage: [u8; 32]) -> Result<()> {
        let htlc = &mut ctx.accounts.htlc;
        let clock = Clock::get()?;

        // Validate state
        require!(htlc.status == HtlcStatus::Funded, HtlcError::HtlcNotClaimable);

        // Verify preimage matches hashlock (SHA256)
        let computed_hash = anchor_lang::solana_program::hash::hash(&preimage);
        require!(
            computed_hash.to_bytes() == htlc.hashlock,
            HtlcError::InvalidPreimage
        );

        // Update state
        htlc.status = HtlcStatus::Claimed;
        htlc.preimage = preimage;

        // Transfer tokens from vault to recipient
        let htlc_seeds = &[
            b"htlc",
            htlc.initiator.as_ref(),
            htlc.recipient.as_ref(),
            &htlc.hashlock,
            &[htlc.bump],
        ];
        let signer_seeds = &[&htlc_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.htlc_vault.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: htlc.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, htlc.amount)?;

        emit!(HtlcClaimed {
            htlc_id: htlc.key(),
            recipient: htlc.recipient,
            preimage,
            claimed_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Refund the HTLC after timelock expires
    pub fn refund_htlc(ctx: Context<RefundHtlc>) -> Result<()> {
        let htlc = &mut ctx.accounts.htlc;
        let clock = Clock::get()?;

        // Validate state
        require!(htlc.status == HtlcStatus::Funded, HtlcError::HtlcNotRefundable);
        require!(
            clock.unix_timestamp >= htlc.timelock,
            HtlcError::TimelockNotExpired
        );

        // Update state
        htlc.status = HtlcStatus::Refunded;

        // Transfer tokens back to initiator
        let htlc_seeds = &[
            b"htlc",
            htlc.initiator.as_ref(),
            htlc.recipient.as_ref(),
            &htlc.hashlock,
            &[htlc.bump],
        ];
        let signer_seeds = &[&htlc_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.htlc_vault.to_account_info(),
                to: ctx.accounts.initiator_token_account.to_account_info(),
                authority: htlc.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, htlc.amount)?;

        emit!(HtlcRefunded {
            htlc_id: htlc.key(),
            initiator: htlc.initiator,
            refunded_at: clock.unix_timestamp,
        });

        Ok(())
    }

    /// Get HTLC status (view function)
    pub fn get_htlc_status(ctx: Context<GetHtlcStatus>) -> Result<HtlcStatusResponse> {
        let htlc = &ctx.accounts.htlc;
        let clock = Clock::get()?;

        let effective_status = if htlc.status == HtlcStatus::Funded 
            && clock.unix_timestamp >= htlc.timelock 
        {
            HtlcStatus::Expired
        } else {
            htlc.status
        };

        Ok(HtlcStatusResponse {
            status: effective_status,
            amount: htlc.amount,
            timelock: htlc.timelock,
            initiator: htlc.initiator,
            recipient: htlc.recipient,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ACCOUNTS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Accounts)]
#[instruction(hashlock: [u8; 32], timelock: i64, amount: u64)]
pub struct CreateHtlc<'info> {
    #[account(mut)]
    pub initiator: Signer<'info>,

    /// CHECK: Recipient public key, validated in instruction
    pub recipient: UncheckedAccount<'info>,

    /// The token mint for this HTLC
    pub token_mint: Account<'info, token::Mint>,

    #[account(
        init,
        payer = initiator,
        space = HTLC_SIZE,
        seeds = [b"htlc", initiator.key().as_ref(), recipient.key().as_ref(), &hashlock],
        bump
    )]
    pub htlc: Account<'info, Htlc>,

    #[account(
        init,
        payer = initiator,
        token::mint = token_mint,
        token::authority = htlc,
        seeds = [b"htlc_vault", htlc.key().as_ref()],
        bump
    )]
    pub htlc_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = initiator_token_account.owner == initiator.key(),
        constraint = initiator_token_account.mint == token_mint.key()
    )]
    pub initiator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ClaimHtlc<'info> {
    #[account(mut)]
    pub recipient: Signer<'info>,

    #[account(
        mut,
        constraint = htlc.recipient == recipient.key() @ HtlcError::NotRecipient,
        seeds = [b"htlc", htlc.initiator.as_ref(), htlc.recipient.as_ref(), &htlc.hashlock],
        bump = htlc.bump
    )]
    pub htlc: Account<'info, Htlc>,

    #[account(
        mut,
        seeds = [b"htlc_vault", htlc.key().as_ref()],
        bump
    )]
    pub htlc_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_token_account.owner == recipient.key(),
        constraint = recipient_token_account.mint == htlc.token_mint
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RefundHtlc<'info> {
    #[account(mut)]
    pub initiator: Signer<'info>,

    #[account(
        mut,
        constraint = htlc.initiator == initiator.key() @ HtlcError::NotInitiator,
        seeds = [b"htlc", htlc.initiator.as_ref(), htlc.recipient.as_ref(), &htlc.hashlock],
        bump = htlc.bump
    )]
    pub htlc: Account<'info, Htlc>,

    #[account(
        mut,
        seeds = [b"htlc_vault", htlc.key().as_ref()],
        bump
    )]
    pub htlc_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = initiator_token_account.owner == initiator.key(),
        constraint = initiator_token_account.mint == htlc.token_mint
    )]
    pub initiator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct GetHtlcStatus<'info> {
    pub htlc: Account<'info, Htlc>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATE
// ═══════════════════════════════════════════════════════════════════════════════

#[account]
#[derive(Default)]
pub struct Htlc {
    /// Address that locked the funds
    pub initiator: Pubkey,
    /// Address that can claim with preimage
    pub recipient: Pubkey,
    /// SPL token mint
    pub token_mint: Pubkey,
    /// Amount of tokens locked
    pub amount: u64,
    /// SHA256 hash of the secret preimage
    pub hashlock: [u8; 32],
    /// Unix timestamp after which initiator can refund
    pub timelock: i64,
    /// Current HTLC status
    pub status: HtlcStatus,
    /// Revealed preimage (zeroed until claimed)
    pub preimage: [u8; 32],
    /// Timestamp when HTLC was created
    pub created_at: i64,
    /// PDA bump seed
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Default)]
pub enum HtlcStatus {
    #[default]
    Pending = 0,
    Funded = 1,
    Claimed = 2,
    Refunded = 3,
    Expired = 4,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct HtlcStatusResponse {
    pub status: HtlcStatus,
    pub amount: u64,
    pub timelock: i64,
    pub initiator: Pubkey,
    pub recipient: Pubkey,
}

// ═══════════════════════════════════════════════════════════════════════════════
// EVENTS
// ═══════════════════════════════════════════════════════════════════════════════

#[event]
pub struct HtlcCreated {
    pub htlc_id: Pubkey,
    pub initiator: Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub hashlock: [u8; 32],
    pub timelock: i64,
}

#[event]
pub struct HtlcClaimed {
    pub htlc_id: Pubkey,
    pub recipient: Pubkey,
    pub preimage: [u8; 32],
    pub claimed_at: i64,
}

#[event]
pub struct HtlcRefunded {
    pub htlc_id: Pubkey,
    pub initiator: Pubkey,
    pub refunded_at: i64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ERRORS
// ═══════════════════════════════════════════════════════════════════════════════

#[error_code]
pub enum HtlcError {
    #[msg("Timelock must be at least 1 hour in the future")]
    TimelockTooShort,
    #[msg("Timelock cannot exceed 7 days")]
    TimelockTooLong,
    #[msg("Amount must be greater than 0")]
    InvalidAmount,
    #[msg("Invalid preimage - does not match hashlock")]
    InvalidPreimage,
    #[msg("HTLC is not in claimable state")]
    HtlcNotClaimable,
    #[msg("HTLC is not in refundable state")]
    HtlcNotRefundable,
    #[msg("Timelock has not expired yet")]
    TimelockNotExpired,
    #[msg("Only the recipient can claim")]
    NotRecipient,
    #[msg("Only the initiator can refund")]
    NotInitiator,
}
