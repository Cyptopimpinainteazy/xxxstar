//! # Token Escrow Program (SVM/Solana)
//!
//! Multi-party escrow with milestone releases and dispute resolution.
//! Supports cross-VM escrow operations.
//!
//! ## Features
//! - Multi-party escrow with configurable parties
//! - Milestone-based releases
//! - Dispute resolution with arbitration
//! - Time-locked releases
//! - Cross-VM integration

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};

declare_id!("X3EsCrOw1111111111111111111111111111111111");

/// Maximum number of parties in multi-party escrow
pub const MAX_PARTIES: usize = 5;

/// Maximum number of milestones
pub const MAX_MILESTONES: usize = 10;

/// Minimum escrow duration (1 hour)
pub const MIN_DURATION: i64 = 3600;

/// Maximum escrow duration (1 year)
pub const MAX_DURATION: i64 = 365 * 24 * 3600;

#[program]
pub mod token_escrow {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // ESCROW CREATION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Create a simple two-party escrow
    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        escrow_id: [u8; 32],
        recipient: Pubkey,
        amount: u64,
        release_time: i64,
        conditions_hash: [u8; 32],
    ) -> Result<()> {
        let clock = Clock::get()?;

        require!(amount > 0, EscrowError::InvalidAmount);
        require!(
            release_time > clock.unix_timestamp + MIN_DURATION,
            EscrowError::ReleaseTooSoon
        );
        require!(
            release_time < clock.unix_timestamp + MAX_DURATION,
            EscrowError::ReleaseTooFar
        );

        let escrow = &mut ctx.accounts.escrow;
        escrow.escrow_id = escrow_id;
        escrow.depositor = ctx.accounts.depositor.key();
        escrow.recipient = recipient;
        escrow.token_mint = ctx.accounts.token_mint.key();
        escrow.amount = amount;
        escrow.release_time = release_time;
        escrow.conditions_hash = conditions_hash;
        escrow.status = EscrowStatus::Active as u8;
        escrow.created_at = clock.unix_timestamp;
        escrow.released_at = 0;
        escrow.dispute_initiated_at = 0;
        escrow.arbitrator = Pubkey::default();
        escrow.bump = ctx.bumps.escrow;

        // Transfer tokens to escrow vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.depositor_token_account.to_account_info(),
                to: ctx.accounts.escrow_vault.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        emit!(EscrowCreated {
            escrow_id,
            depositor: ctx.accounts.depositor.key(),
            recipient,
            token_mint: ctx.accounts.token_mint.key(),
            amount,
            release_time,
        });

        Ok(())
    }

    /// Create a milestone-based escrow
    pub fn create_milestone_escrow(
        ctx: Context<CreateMilestoneEscrow>,
        escrow_id: [u8; 32],
        recipient: Pubkey,
        total_amount: u64,
        milestones: Vec<MilestoneInput>,
        arbitrator: Pubkey,
    ) -> Result<()> {
        let clock = Clock::get()?;

        require!(total_amount > 0, EscrowError::InvalidAmount);
        require!(
            milestones.len() > 0 && milestones.len() <= MAX_MILESTONES,
            EscrowError::InvalidMilestones
        );

        // Verify milestone amounts sum to total
        let milestone_sum: u64 = milestones.iter().map(|m| m.amount).sum();
        require!(milestone_sum == total_amount, EscrowError::MilestoneAmountMismatch);

        let escrow = &mut ctx.accounts.milestone_escrow;
        escrow.escrow_id = escrow_id;
        escrow.depositor = ctx.accounts.depositor.key();
        escrow.recipient = recipient;
        escrow.token_mint = ctx.accounts.token_mint.key();
        escrow.total_amount = total_amount;
        escrow.released_amount = 0;
        escrow.arbitrator = arbitrator;
        escrow.status = EscrowStatus::Active as u8;
        escrow.created_at = clock.unix_timestamp;
        escrow.bump = ctx.bumps.milestone_escrow;

        // Initialize milestones
        escrow.milestone_count = milestones.len() as u8;
        for (i, m) in milestones.iter().enumerate() {
            escrow.milestones[i] = Milestone {
                description_hash: m.description_hash,
                amount: m.amount,
                deadline: m.deadline,
                completed: false,
                disputed: false,
                released_at: 0,
            };
        }

        // Transfer tokens to escrow vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.depositor_token_account.to_account_info(),
                to: ctx.accounts.escrow_vault.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, total_amount)?;

        emit!(MilestoneEscrowCreated {
            escrow_id,
            depositor: ctx.accounts.depositor.key(),
            recipient,
            total_amount,
            milestone_count: milestones.len() as u8,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // RELEASE OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Release escrow funds to recipient (by depositor)
    pub fn release_escrow(ctx: Context<ReleaseEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let clock = Clock::get()?;

        require!(
            escrow.status == EscrowStatus::Active as u8,
            EscrowError::EscrowNotActive
        );

        escrow.status = EscrowStatus::Released as u8;
        escrow.released_at = clock.unix_timestamp;

        // Transfer tokens to recipient
        let escrow_seeds = &[
            b"escrow",
            escrow.escrow_id.as_ref(),
            &[escrow.bump],
        ];
        let signer_seeds = &[&escrow_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: escrow.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, escrow.amount)?;

        emit!(EscrowReleased {
            escrow_id: escrow.escrow_id,
            recipient: escrow.recipient,
            amount: escrow.amount,
        });

        Ok(())
    }

    /// Claim escrow after release time (by recipient)
    pub fn claim_escrow(ctx: Context<ClaimEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let clock = Clock::get()?;

        require!(
            escrow.status == EscrowStatus::Active as u8,
            EscrowError::EscrowNotActive
        );
        require!(
            clock.unix_timestamp >= escrow.release_time,
            EscrowError::ReleaseTimeNotReached
        );

        escrow.status = EscrowStatus::Claimed as u8;
        escrow.released_at = clock.unix_timestamp;

        // Transfer tokens to recipient
        let escrow_seeds = &[
            b"escrow",
            escrow.escrow_id.as_ref(),
            &[escrow.bump],
        ];
        let signer_seeds = &[&escrow_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: escrow.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, escrow.amount)?;

        emit!(EscrowClaimed {
            escrow_id: escrow.escrow_id,
            recipient: escrow.recipient,
            amount: escrow.amount,
        });

        Ok(())
    }

    /// Mark milestone as complete and release funds
    pub fn complete_milestone(
        ctx: Context<CompleteMilestone>,
        milestone_index: u8,
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.milestone_escrow;
        let clock = Clock::get()?;

        require!(
            escrow.status == EscrowStatus::Active as u8,
            EscrowError::EscrowNotActive
        );
        require!(
            milestone_index < escrow.milestone_count,
            EscrowError::InvalidMilestoneIndex
        );

        let milestone = &mut escrow.milestones[milestone_index as usize];
        require!(!milestone.completed, EscrowError::MilestoneAlreadyCompleted);
        require!(!milestone.disputed, EscrowError::MilestoneDisputed);

        milestone.completed = true;
        milestone.released_at = clock.unix_timestamp;

        let release_amount = milestone.amount;
        escrow.released_amount += release_amount;

        // Transfer milestone amount to recipient
        let escrow_seeds = &[
            b"milestone_escrow",
            escrow.escrow_id.as_ref(),
            &[escrow.bump],
        ];
        let signer_seeds = &[&escrow_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: escrow.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, release_amount)?;

        // Check if all milestones completed
        let all_completed = escrow.milestones[..escrow.milestone_count as usize]
            .iter()
            .all(|m| m.completed);
        
        if all_completed {
            escrow.status = EscrowStatus::Completed as u8;
        }

        emit!(MilestoneCompleted {
            escrow_id: escrow.escrow_id,
            milestone_index,
            amount: release_amount,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // DISPUTE RESOLUTION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Initiate a dispute on escrow
    pub fn initiate_dispute(ctx: Context<InitiateDispute>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let clock = Clock::get()?;

        require!(
            escrow.status == EscrowStatus::Active as u8,
            EscrowError::EscrowNotActive
        );

        escrow.status = EscrowStatus::Disputed as u8;
        escrow.dispute_initiated_at = clock.unix_timestamp;

        emit!(DisputeInitiated {
            escrow_id: escrow.escrow_id,
            initiated_by: ctx.accounts.party.key(),
        });

        Ok(())
    }

    /// Resolve dispute (arbitrator only)
    pub fn resolve_dispute(
        ctx: Context<ResolveDispute>,
        recipient_share_bps: u16, // 0-10000 (0-100%)
    ) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let clock = Clock::get()?;

        require!(
            escrow.status == EscrowStatus::Disputed as u8,
            EscrowError::NotInDispute
        );
        require!(recipient_share_bps <= 10000, EscrowError::InvalidShare);

        escrow.status = EscrowStatus::Resolved as u8;
        escrow.released_at = clock.unix_timestamp;

        let recipient_amount = (escrow.amount as u128 * recipient_share_bps as u128 / 10000) as u64;
        let depositor_amount = escrow.amount - recipient_amount;

        let escrow_seeds = &[
            b"escrow",
            escrow.escrow_id.as_ref(),
            &[escrow.bump],
        ];
        let signer_seeds = &[&escrow_seeds[..]];

        // Transfer to recipient
        if recipient_amount > 0 {
            let transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.escrow_vault.to_account_info(),
                    to: ctx.accounts.recipient_token_account.to_account_info(),
                    authority: escrow.to_account_info(),
                },
                signer_seeds,
            );
            token::transfer(transfer_ctx, recipient_amount)?;
        }

        // Transfer to depositor
        if depositor_amount > 0 {
            let transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.escrow_vault.to_account_info(),
                    to: ctx.accounts.depositor_token_account.to_account_info(),
                    authority: escrow.to_account_info(),
                },
                signer_seeds,
            );
            token::transfer(transfer_ctx, depositor_amount)?;
        }

        emit!(DisputeResolved {
            escrow_id: escrow.escrow_id,
            recipient_amount,
            depositor_amount,
            arbitrator: ctx.accounts.arbitrator.key(),
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // REFUND
    // ═══════════════════════════════════════════════════════════════════════════

    /// Cancel escrow and refund (mutual agreement required)
    pub fn cancel_escrow(ctx: Context<CancelEscrow>) -> Result<()> {
        let escrow = &mut ctx.accounts.escrow;
        let clock = Clock::get()?;

        require!(
            escrow.status == EscrowStatus::Active as u8,
            EscrowError::EscrowNotActive
        );

        escrow.status = EscrowStatus::Cancelled as u8;
        escrow.released_at = clock.unix_timestamp;

        // Refund to depositor
        let escrow_seeds = &[
            b"escrow",
            escrow.escrow_id.as_ref(),
            &[escrow.bump],
        ];
        let signer_seeds = &[&escrow_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.escrow_vault.to_account_info(),
                to: ctx.accounts.depositor_token_account.to_account_info(),
                authority: escrow.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, escrow.amount)?;

        emit!(EscrowCancelled {
            escrow_id: escrow.escrow_id,
            refund_amount: escrow.amount,
        });

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// ACCOUNTS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Accounts)]
#[instruction(escrow_id: [u8; 32])]
pub struct CreateEscrow<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = depositor,
        space = 8 + Escrow::SIZE,
        seeds = [b"escrow", escrow_id.as_ref()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        init,
        payer = depositor,
        token::mint = token_mint,
        token::authority = escrow,
        seeds = [b"escrow_vault", escrow_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = depositor_token_account.owner == depositor.key()
    )]
    pub depositor_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(escrow_id: [u8; 32])]
pub struct CreateMilestoneEscrow<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = depositor,
        space = 8 + MilestoneEscrow::SIZE,
        seeds = [b"milestone_escrow", escrow_id.as_ref()],
        bump
    )]
    pub milestone_escrow: Account<'info, MilestoneEscrow>,

    #[account(
        init,
        payer = depositor,
        token::mint = token_mint,
        token::authority = milestone_escrow,
        seeds = [b"milestone_vault", escrow_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = depositor_token_account.owner == depositor.key()
    )]
    pub depositor_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct ReleaseEscrow<'info> {
    #[account(
        constraint = escrow.depositor == depositor.key() @ EscrowError::Unauthorized
    )]
    pub depositor: Signer<'info>,

    #[account(
        mut,
        seeds = [b"escrow", escrow.escrow_id.as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        seeds = [b"escrow_vault", escrow.escrow_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_token_account.owner == escrow.recipient
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimEscrow<'info> {
    #[account(
        constraint = escrow.recipient == recipient.key() @ EscrowError::Unauthorized
    )]
    pub recipient: Signer<'info>,

    #[account(
        mut,
        seeds = [b"escrow", escrow.escrow_id.as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        seeds = [b"escrow_vault", escrow.escrow_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_token_account.owner == recipient.key()
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CompleteMilestone<'info> {
    /// Depositor must approve milestone completion
    #[account(
        constraint = milestone_escrow.depositor == depositor.key() @ EscrowError::Unauthorized
    )]
    pub depositor: Signer<'info>,

    #[account(
        mut,
        seeds = [b"milestone_escrow", milestone_escrow.escrow_id.as_ref()],
        bump = milestone_escrow.bump
    )]
    pub milestone_escrow: Account<'info, MilestoneEscrow>,

    #[account(
        mut,
        seeds = [b"milestone_vault", milestone_escrow.escrow_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_token_account.owner == milestone_escrow.recipient
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct InitiateDispute<'info> {
    /// Either depositor or recipient can initiate
    pub party: Signer<'info>,

    #[account(
        mut,
        seeds = [b"escrow", escrow.escrow_id.as_ref()],
        bump = escrow.bump,
        constraint = escrow.depositor == party.key() || escrow.recipient == party.key() @ EscrowError::Unauthorized
    )]
    pub escrow: Account<'info, Escrow>,
}

#[derive(Accounts)]
pub struct ResolveDispute<'info> {
    #[account(
        constraint = escrow.arbitrator == arbitrator.key() @ EscrowError::Unauthorized
    )]
    pub arbitrator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"escrow", escrow.escrow_id.as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        seeds = [b"escrow_vault", escrow.escrow_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = recipient_token_account.owner == escrow.recipient
    )]
    pub recipient_token_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = depositor_token_account.owner == escrow.depositor
    )]
    pub depositor_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CancelEscrow<'info> {
    /// Both parties must sign to cancel
    #[account(
        constraint = escrow.depositor == depositor.key() @ EscrowError::Unauthorized
    )]
    pub depositor: Signer<'info>,

    #[account(
        constraint = escrow.recipient == recipient.key() @ EscrowError::Unauthorized
    )]
    pub recipient: Signer<'info>,

    #[account(
        mut,
        seeds = [b"escrow", escrow.escrow_id.as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,

    #[account(
        mut,
        seeds = [b"escrow_vault", escrow.escrow_id.as_ref()],
        bump
    )]
    pub escrow_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = depositor_token_account.owner == depositor.key()
    )]
    pub depositor_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATE
// ═══════════════════════════════════════════════════════════════════════════════

#[account]
pub struct Escrow {
    pub escrow_id: [u8; 32],
    pub depositor: Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub release_time: i64,
    pub conditions_hash: [u8; 32],
    pub status: u8,
    pub created_at: i64,
    pub released_at: i64,
    pub dispute_initiated_at: i64,
    pub arbitrator: Pubkey,
    pub bump: u8,
}

impl Escrow {
    pub const SIZE: usize = 32 + 32 + 32 + 32 + 8 + 8 + 32 + 1 + 8 + 8 + 8 + 32 + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default)]
pub struct Milestone {
    pub description_hash: [u8; 32],
    pub amount: u64,
    pub deadline: i64,
    pub completed: bool,
    pub disputed: bool,
    pub released_at: i64,
}

impl Milestone {
    pub const SIZE: usize = 32 + 8 + 8 + 1 + 1 + 8;
}

#[account]
pub struct MilestoneEscrow {
    pub escrow_id: [u8; 32],
    pub depositor: Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey,
    pub total_amount: u64,
    pub released_amount: u64,
    pub arbitrator: Pubkey,
    pub status: u8,
    pub created_at: i64,
    pub milestone_count: u8,
    pub milestones: [Milestone; MAX_MILESTONES],
    pub bump: u8,
}

impl MilestoneEscrow {
    pub const SIZE: usize = 32 + 32 + 32 + 32 + 8 + 8 + 32 + 1 + 8 + 1 + (Milestone::SIZE * MAX_MILESTONES) + 1;
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MilestoneInput {
    pub description_hash: [u8; 32],
    pub amount: u64,
    pub deadline: i64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EscrowStatus {
    Active = 0,
    Released = 1,
    Claimed = 2,
    Disputed = 3,
    Resolved = 4,
    Cancelled = 5,
    Completed = 6,
}

// ═══════════════════════════════════════════════════════════════════════════════
// EVENTS
// ═══════════════════════════════════════════════════════════════════════════════

#[event]
pub struct EscrowCreated {
    pub escrow_id: [u8; 32],
    pub depositor: Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey,
    pub amount: u64,
    pub release_time: i64,
}

#[event]
pub struct MilestoneEscrowCreated {
    pub escrow_id: [u8; 32],
    pub depositor: Pubkey,
    pub recipient: Pubkey,
    pub total_amount: u64,
    pub milestone_count: u8,
}

#[event]
pub struct EscrowReleased {
    pub escrow_id: [u8; 32],
    pub recipient: Pubkey,
    pub amount: u64,
}

#[event]
pub struct EscrowClaimed {
    pub escrow_id: [u8; 32],
    pub recipient: Pubkey,
    pub amount: u64,
}

#[event]
pub struct MilestoneCompleted {
    pub escrow_id: [u8; 32],
    pub milestone_index: u8,
    pub amount: u64,
}

#[event]
pub struct DisputeInitiated {
    pub escrow_id: [u8; 32],
    pub initiated_by: Pubkey,
}

#[event]
pub struct DisputeResolved {
    pub escrow_id: [u8; 32],
    pub recipient_amount: u64,
    pub depositor_amount: u64,
    pub arbitrator: Pubkey,
}

#[event]
pub struct EscrowCancelled {
    pub escrow_id: [u8; 32],
    pub refund_amount: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ERRORS
// ═══════════════════════════════════════════════════════════════════════════════

#[error_code]
pub enum EscrowError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Release time too soon")]
    ReleaseTooSoon,
    #[msg("Release time too far in future")]
    ReleaseTooFar,
    #[msg("Escrow not active")]
    EscrowNotActive,
    #[msg("Release time not reached")]
    ReleaseTimeNotReached,
    #[msg("Not in dispute")]
    NotInDispute,
    #[msg("Invalid share percentage")]
    InvalidShare,
    #[msg("Invalid milestones")]
    InvalidMilestones,
    #[msg("Milestone amounts don't match total")]
    MilestoneAmountMismatch,
    #[msg("Invalid milestone index")]
    InvalidMilestoneIndex,
    #[msg("Milestone already completed")]
    MilestoneAlreadyCompleted,
    #[msg("Milestone is disputed")]
    MilestoneDisputed,
}
