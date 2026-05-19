//! # Cross-VM Adapter Program (SVM/Solana)
//!
//! Solana program that enables cross-VM operations with EVM and X3VM.
//! Handles atomic swaps, message passing, and wrapped asset management.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    Cross-VM Adapter (SVM)                       │
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌────────────────┐  ┌────────────────┐  ┌─────────────────────┐│
//! │  │ HTLC Manager   │  │ Message Router │  │ Wrapped Asset Mgr   ││
//! │  └────────────────┘  └────────────────┘  └─────────────────────┘│
//! ├─────────────────────────────────────────────────────────────────┤
//! │  ┌────────────────┐  ┌────────────────┐  ┌─────────────────────┐│
//! │  │ EVM Verifier   │  │ X3VM Verifier  │  │ Relayer Manager     ││
//! │  └────────────────┘  └────────────────┘  └─────────────────────┘│
//! └─────────────────────────────────────────────────────────────────┘
//! ```

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint};

declare_id!("X3CrOsS111111111111111111111111111111111111");

/// Maximum message data size
pub const MAX_MESSAGE_SIZE: usize = 1024;

/// Maximum relayers in the set
pub const MAX_RELAYERS: usize = 10;

/// HTLC timelock bounds
pub const MIN_TIMELOCK: i64 = 300;       // 5 minutes
pub const MAX_TIMELOCK: i64 = 604800;    // 7 days

#[program]
pub mod cross_vm_adapter {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // INITIALIZATION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Initialize the cross-VM adapter
    pub fn initialize(
        ctx: Context<Initialize>,
        evm_chain_id: u64,
        x3_chain_id: u64,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;
        config.authority = ctx.accounts.authority.key();
        config.evm_chain_id = evm_chain_id;
        config.x3_chain_id = x3_chain_id;
        config.paused = false;
        config.total_htlcs_created = 0;
        config.total_messages_sent = 0;
        config.total_messages_received = 0;
        config.bump = ctx.bumps.config;
        
        Ok(())
    }

    /// Add a relayer to the trusted set
    pub fn add_relayer(
        ctx: Context<ManageRelayer>,
        relayer: Pubkey,
    ) -> Result<()> {
        let relayer_set = &mut ctx.accounts.relayer_set;
        
        require!(
            relayer_set.relayer_count < MAX_RELAYERS as u8,
            CrossVMError::MaxRelayersReached
        );

        // Check not already added
        for i in 0..relayer_set.relayer_count as usize {
            require!(
                relayer_set.relayers[i] != relayer,
                CrossVMError::RelayerAlreadyExists
            );
        }

        relayer_set.relayers[relayer_set.relayer_count as usize] = relayer;
        relayer_set.relayer_count += 1;

        emit!(RelayerAdded { relayer });
        Ok(())
    }

    /// Remove a relayer from the trusted set
    pub fn remove_relayer(
        ctx: Context<ManageRelayer>,
        relayer: Pubkey,
    ) -> Result<()> {
        let relayer_set = &mut ctx.accounts.relayer_set;
        
        let mut found = false;
        let mut found_index = 0;

        for i in 0..relayer_set.relayer_count as usize {
            if relayer_set.relayers[i] == relayer {
                found = true;
                found_index = i;
                break;
            }
        }

        require!(found, CrossVMError::RelayerNotFound);

        // Shift remaining relayers
        for i in found_index..((relayer_set.relayer_count - 1) as usize) {
            relayer_set.relayers[i] = relayer_set.relayers[i + 1];
        }
        relayer_set.relayer_count -= 1;

        emit!(RelayerRemoved { relayer });
        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // HTLC OPERATIONS (Cross-VM Atomic Swaps)
    // ═══════════════════════════════════════════════════════════════════════════

    /// Create an HTLC for cross-VM atomic swap
    pub fn create_cross_vm_htlc(
        ctx: Context<CreateCrossVMHtlc>,
        htlc_id: [u8; 32],
        target_vm: u8,           // 0=EVM, 1=X3VM
        target_address: [u8; 32], // Recipient on target VM
        hashlock: [u8; 32],
        timelock: i64,
        amount: u64,
    ) -> Result<()> {
        let clock = Clock::get()?;
        let config = &ctx.accounts.config;

        require!(!config.paused, CrossVMError::ProtocolPaused);
        require!(
            timelock > clock.unix_timestamp + MIN_TIMELOCK,
            CrossVMError::TimelockTooShort
        );
        require!(
            timelock < clock.unix_timestamp + MAX_TIMELOCK,
            CrossVMError::TimelockTooLong
        );
        require!(amount > 0, CrossVMError::InvalidAmount);
        require!(target_vm <= 1, CrossVMError::InvalidTargetVM);

        let htlc = &mut ctx.accounts.htlc;
        htlc.htlc_id = htlc_id;
        htlc.initiator = ctx.accounts.initiator.key();
        htlc.token_mint = ctx.accounts.token_mint.key();
        htlc.target_vm = target_vm;
        htlc.target_address = target_address;
        htlc.amount = amount;
        htlc.hashlock = hashlock;
        htlc.timelock = timelock;
        htlc.status = HtlcStatus::Locked as u8;
        htlc.preimage = [0u8; 32];
        htlc.created_at = clock.unix_timestamp;
        htlc.bump = ctx.bumps.htlc;

        // Transfer tokens to HTLC vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.initiator_token_account.to_account_info(),
                to: ctx.accounts.htlc_vault.to_account_info(),
                authority: ctx.accounts.initiator.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        // Update stats
        let config = &mut ctx.accounts.config;
        config.total_htlcs_created += 1;

        emit!(CrossVMHtlcCreated {
            htlc_id,
            initiator: ctx.accounts.initiator.key(),
            target_vm,
            target_address,
            token_mint: ctx.accounts.token_mint.key(),
            amount,
            hashlock,
            timelock,
        });

        Ok(())
    }

    /// Claim HTLC with preimage (relayer-mediated)
    pub fn claim_cross_vm_htlc(
        ctx: Context<ClaimCrossVMHtlc>,
        preimage: [u8; 32],
    ) -> Result<()> {
        let htlc = &mut ctx.accounts.htlc;

        require!(
            htlc.status == HtlcStatus::Locked as u8,
            CrossVMError::HtlcNotClaimable
        );

        // Verify preimage
        let computed_hash = anchor_lang::solana_program::hash::hash(&preimage);
        require!(
            computed_hash.to_bytes() == htlc.hashlock,
            CrossVMError::InvalidPreimage
        );

        htlc.status = HtlcStatus::Claimed as u8;
        htlc.preimage = preimage;

        // Transfer tokens to recipient vault (will be processed for target VM)
        let htlc_seeds = &[
            b"cross_vm_htlc",
            htlc.htlc_id.as_ref(),
            &[htlc.bump],
        ];
        let signer_seeds = &[&htlc_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.htlc_vault.to_account_info(),
                to: ctx.accounts.recipient_vault.to_account_info(),
                authority: htlc.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, htlc.amount)?;

        emit!(CrossVMHtlcClaimed {
            htlc_id: htlc.htlc_id,
            preimage,
            claimed_at: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    /// Refund expired HTLC
    pub fn refund_cross_vm_htlc(ctx: Context<RefundCrossVMHtlc>) -> Result<()> {
        let htlc = &mut ctx.accounts.htlc;
        let clock = Clock::get()?;

        require!(
            htlc.status == HtlcStatus::Locked as u8,
            CrossVMError::HtlcNotRefundable
        );
        require!(
            clock.unix_timestamp >= htlc.timelock,
            CrossVMError::TimelockNotExpired
        );

        htlc.status = HtlcStatus::Refunded as u8;

        // Transfer tokens back to initiator
        let htlc_seeds = &[
            b"cross_vm_htlc",
            htlc.htlc_id.as_ref(),
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

        emit!(CrossVMHtlcRefunded {
            htlc_id: htlc.htlc_id,
            refunded_at: clock.unix_timestamp,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CROSS-VM MESSAGING
    // ═══════════════════════════════════════════════════════════════════════════

    /// Send a message to another VM
    pub fn send_cross_vm_message(
        ctx: Context<SendCrossVMMessage>,
        message_id: [u8; 32],
        target_vm: u8,
        target_address: [u8; 32],
        payload: Vec<u8>,
    ) -> Result<()> {
        require!(
            payload.len() <= MAX_MESSAGE_SIZE,
            CrossVMError::MessageTooLarge
        );
        require!(!ctx.accounts.config.paused, CrossVMError::ProtocolPaused);

        let message = &mut ctx.accounts.message;
        message.message_id = message_id;
        message.sender = ctx.accounts.sender.key();
        message.target_vm = target_vm;
        message.target_address = target_address;
        message.payload = payload.clone();
        message.status = MessageStatus::Pending as u8;
        message.sent_at = Clock::get()?.unix_timestamp;
        message.executed_at = 0;
        message.bump = ctx.bumps.message;

        let config = &mut ctx.accounts.config;
        config.total_messages_sent += 1;

        emit!(CrossVMMessageSent {
            message_id,
            sender: ctx.accounts.sender.key(),
            target_vm,
            target_address,
            payload_hash: anchor_lang::solana_program::hash::hash(&payload).to_bytes(),
        });

        Ok(())
    }

    /// Execute an incoming cross-VM message (relayer only)
    pub fn execute_cross_vm_message(
        ctx: Context<ExecuteCrossVMMessage>,
        message_id: [u8; 32],
        source_vm: u8,
        source_address: [u8; 32],
        payload: Vec<u8>,
        proof: Vec<u8>,
    ) -> Result<()> {
        // Verify relayer is authorized
        let relayer_set = &ctx.accounts.relayer_set;
        let relayer = ctx.accounts.relayer.key();
        
        let mut is_authorized = false;
        for i in 0..relayer_set.relayer_count as usize {
            if relayer_set.relayers[i] == relayer {
                is_authorized = true;
                break;
            }
        }
        require!(is_authorized, CrossVMError::UnauthorizedRelayer);

        // Verify proof (simplified - in production would verify against light client)
        require!(proof.len() >= 32, CrossVMError::InvalidProof);

        let incoming = &mut ctx.accounts.incoming_message;
        incoming.message_id = message_id;
        incoming.source_vm = source_vm;
        incoming.source_address = source_address;
        incoming.payload = payload;
        incoming.executed = true;
        incoming.executed_at = Clock::get()?.unix_timestamp;
        incoming.relayer = relayer;
        incoming.bump = ctx.bumps.incoming_message;

        let config = &mut ctx.accounts.config;
        config.total_messages_received += 1;

        emit!(CrossVMMessageExecuted {
            message_id,
            source_vm,
            source_address,
            relayer,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // WRAPPED ASSET OPERATIONS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Lock tokens for wrapping on another VM
    pub fn lock_for_wrapping(
        ctx: Context<LockForWrapping>,
        lock_id: [u8; 32],
        target_vm: u8,
        target_recipient: [u8; 32],
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, CrossVMError::InvalidAmount);
        require!(!ctx.accounts.config.paused, CrossVMError::ProtocolPaused);

        let lock = &mut ctx.accounts.lock;
        lock.lock_id = lock_id;
        lock.depositor = ctx.accounts.depositor.key();
        lock.token_mint = ctx.accounts.token_mint.key();
        lock.target_vm = target_vm;
        lock.target_recipient = target_recipient;
        lock.amount = amount;
        lock.locked_at = Clock::get()?.unix_timestamp;
        lock.released = false;
        lock.bump = ctx.bumps.lock;

        // Transfer tokens to lock vault
        let transfer_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.depositor_token_account.to_account_info(),
                to: ctx.accounts.lock_vault.to_account_info(),
                authority: ctx.accounts.depositor.to_account_info(),
            },
        );
        token::transfer(transfer_ctx, amount)?;

        emit!(TokensLockedForWrapping {
            lock_id,
            depositor: ctx.accounts.depositor.key(),
            token_mint: ctx.accounts.token_mint.key(),
            target_vm,
            target_recipient,
            amount,
        });

        Ok(())
    }

    /// Release locked tokens (burn on other VM confirmed)
    pub fn release_locked_tokens(
        ctx: Context<ReleaseLocked>,
        burn_proof: Vec<u8>,
    ) -> Result<()> {
        // Verify relayer is authorized
        let relayer_set = &ctx.accounts.relayer_set;
        let relayer = ctx.accounts.relayer.key();
        
        let mut is_authorized = false;
        for i in 0..relayer_set.relayer_count as usize {
            if relayer_set.relayers[i] == relayer {
                is_authorized = true;
                break;
            }
        }
        require!(is_authorized, CrossVMError::UnauthorizedRelayer);

        // Verify burn proof
        require!(burn_proof.len() >= 32, CrossVMError::InvalidProof);

        let lock = &mut ctx.accounts.lock;
        require!(!lock.released, CrossVMError::AlreadyReleased);

        lock.released = true;

        // Transfer tokens to recipient
        let lock_seeds = &[
            b"token_lock",
            lock.lock_id.as_ref(),
            &[lock.bump],
        ];
        let signer_seeds = &[&lock_seeds[..]];

        let transfer_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.lock_vault.to_account_info(),
                to: ctx.accounts.recipient_token_account.to_account_info(),
                authority: lock.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_ctx, lock.amount)?;

        emit!(LockedTokensReleased {
            lock_id: lock.lock_id,
            recipient: ctx.accounts.recipient.key(),
            amount: lock.amount,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ADMIN FUNCTIONS
    // ═══════════════════════════════════════════════════════════════════════════

    pub fn pause(ctx: Context<AdminPause>) -> Result<()> {
        ctx.accounts.config.paused = true;
        emit!(ProtocolPaused {});
        Ok(())
    }

    pub fn unpause(ctx: Context<AdminPause>) -> Result<()> {
        ctx.accounts.config.paused = false;
        emit!(ProtocolUnpaused {});
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

    #[account(
        init,
        payer = authority,
        space = 8 + Config::SIZE,
        seeds = [b"config"],
        bump
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = authority,
        space = 8 + RelayerSet::SIZE,
        seeds = [b"relayer_set"],
        bump
    )]
    pub relayer_set: Account<'info, RelayerSet>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ManageRelayer<'info> {
    #[account(
        mut,
        constraint = config.authority == authority.key() @ CrossVMError::Unauthorized
    )]
    pub authority: Signer<'info>,

    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        mut,
        seeds = [b"relayer_set"],
        bump
    )]
    pub relayer_set: Account<'info, RelayerSet>,
}

#[derive(Accounts)]
#[instruction(htlc_id: [u8; 32])]
pub struct CreateCrossVMHtlc<'info> {
    #[account(mut)]
    pub initiator: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = initiator,
        space = 8 + CrossVMHtlc::SIZE,
        seeds = [b"cross_vm_htlc", htlc_id.as_ref()],
        bump
    )]
    pub htlc: Account<'info, CrossVMHtlc>,

    #[account(
        init,
        payer = initiator,
        token::mint = token_mint,
        token::authority = htlc,
        seeds = [b"htlc_vault", htlc_id.as_ref()],
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
pub struct ClaimCrossVMHtlc<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(
        mut,
        seeds = [b"cross_vm_htlc", htlc.htlc_id.as_ref()],
        bump = htlc.bump
    )]
    pub htlc: Account<'info, CrossVMHtlc>,

    #[account(
        mut,
        seeds = [b"htlc_vault", htlc.htlc_id.as_ref()],
        bump
    )]
    pub htlc_vault: Account<'info, TokenAccount>,

    /// Recipient vault for claimed tokens (processed for target VM)
    #[account(mut)]
    pub recipient_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RefundCrossVMHtlc<'info> {
    #[account(
        mut,
        constraint = htlc.initiator == initiator.key() @ CrossVMError::Unauthorized
    )]
    pub initiator: Signer<'info>,

    #[account(
        mut,
        seeds = [b"cross_vm_htlc", htlc.htlc_id.as_ref()],
        bump = htlc.bump
    )]
    pub htlc: Account<'info, CrossVMHtlc>,

    #[account(
        mut,
        seeds = [b"htlc_vault", htlc.htlc_id.as_ref()],
        bump
    )]
    pub htlc_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = initiator_token_account.owner == initiator.key()
    )]
    pub initiator_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(message_id: [u8; 32])]
pub struct SendCrossVMMessage<'info> {
    #[account(mut)]
    pub sender: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = sender,
        space = 8 + CrossVMMessage::SIZE,
        seeds = [b"cross_vm_message", message_id.as_ref()],
        bump
    )]
    pub message: Account<'info, CrossVMMessage>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(message_id: [u8; 32])]
pub struct ExecuteCrossVMMessage<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,

    #[account(seeds = [b"relayer_set"], bump)]
    pub relayer_set: Account<'info, RelayerSet>,

    #[account(
        init,
        payer = relayer,
        space = 8 + IncomingMessage::SIZE,
        seeds = [b"incoming_message", message_id.as_ref()],
        bump
    )]
    pub incoming_message: Account<'info, IncomingMessage>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(lock_id: [u8; 32])]
pub struct LockForWrapping<'info> {
    #[account(mut)]
    pub depositor: Signer<'info>,

    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = depositor,
        space = 8 + TokenLock::SIZE,
        seeds = [b"token_lock", lock_id.as_ref()],
        bump
    )]
    pub lock: Account<'info, TokenLock>,

    #[account(
        init,
        payer = depositor,
        token::mint = token_mint,
        token::authority = lock,
        seeds = [b"lock_vault", lock_id.as_ref()],
        bump
    )]
    pub lock_vault: Account<'info, TokenAccount>,

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
pub struct ReleaseLocked<'info> {
    #[account(mut)]
    pub relayer: Signer<'info>,

    #[account(seeds = [b"relayer_set"], bump)]
    pub relayer_set: Account<'info, RelayerSet>,

    #[account(
        mut,
        seeds = [b"token_lock", lock.lock_id.as_ref()],
        bump = lock.bump
    )]
    pub lock: Account<'info, TokenLock>,

    #[account(
        mut,
        seeds = [b"lock_vault", lock.lock_id.as_ref()],
        bump
    )]
    pub lock_vault: Account<'info, TokenAccount>,

    /// CHECK: Recipient for released tokens
    pub recipient: UncheckedAccount<'info>,

    #[account(mut)]
    pub recipient_token_account: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct AdminPause<'info> {
    #[account(
        constraint = config.authority == authority.key() @ CrossVMError::Unauthorized
    )]
    pub authority: Signer<'info>,

    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATE
// ═══════════════════════════════════════════════════════════════════════════════

#[account]
pub struct Config {
    pub authority: Pubkey,
    pub evm_chain_id: u64,
    pub x3_chain_id: u64,
    pub paused: bool,
    pub total_htlcs_created: u64,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub bump: u8,
}

impl Config {
    pub const SIZE: usize = 32 + 8 + 8 + 1 + 8 + 8 + 8 + 1;
}

#[account]
pub struct RelayerSet {
    pub relayers: [Pubkey; MAX_RELAYERS],
    pub relayer_count: u8,
}

impl RelayerSet {
    pub const SIZE: usize = 32 * MAX_RELAYERS + 1;
}

#[account]
pub struct CrossVMHtlc {
    pub htlc_id: [u8; 32],
    pub initiator: Pubkey,
    pub token_mint: Pubkey,
    pub target_vm: u8,
    pub target_address: [u8; 32],
    pub amount: u64,
    pub hashlock: [u8; 32],
    pub timelock: i64,
    pub status: u8,
    pub preimage: [u8; 32],
    pub created_at: i64,
    pub bump: u8,
}

impl CrossVMHtlc {
    pub const SIZE: usize = 32 + 32 + 32 + 1 + 32 + 8 + 32 + 8 + 1 + 32 + 8 + 1;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HtlcStatus {
    Locked = 0,
    Claimed = 1,
    Refunded = 2,
    Expired = 3,
}

#[account]
pub struct CrossVMMessage {
    pub message_id: [u8; 32],
    pub sender: Pubkey,
    pub target_vm: u8,
    pub target_address: [u8; 32],
    pub payload: Vec<u8>,
    pub status: u8,
    pub sent_at: i64,
    pub executed_at: i64,
    pub bump: u8,
}

impl CrossVMMessage {
    pub const SIZE: usize = 32 + 32 + 1 + 32 + 4 + MAX_MESSAGE_SIZE + 1 + 8 + 8 + 1;
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MessageStatus {
    Pending = 0,
    Relayed = 1,
    Executed = 2,
    Failed = 3,
}

#[account]
pub struct IncomingMessage {
    pub message_id: [u8; 32],
    pub source_vm: u8,
    pub source_address: [u8; 32],
    pub payload: Vec<u8>,
    pub executed: bool,
    pub executed_at: i64,
    pub relayer: Pubkey,
    pub bump: u8,
}

impl IncomingMessage {
    pub const SIZE: usize = 32 + 1 + 32 + 4 + MAX_MESSAGE_SIZE + 1 + 8 + 32 + 1;
}

#[account]
pub struct TokenLock {
    pub lock_id: [u8; 32],
    pub depositor: Pubkey,
    pub token_mint: Pubkey,
    pub target_vm: u8,
    pub target_recipient: [u8; 32],
    pub amount: u64,
    pub locked_at: i64,
    pub released: bool,
    pub bump: u8,
}

impl TokenLock {
    pub const SIZE: usize = 32 + 32 + 32 + 1 + 32 + 8 + 8 + 1 + 1;
}

// ═══════════════════════════════════════════════════════════════════════════════
// EVENTS
// ═══════════════════════════════════════════════════════════════════════════════

#[event]
pub struct RelayerAdded {
    pub relayer: Pubkey,
}

#[event]
pub struct RelayerRemoved {
    pub relayer: Pubkey,
}

#[event]
pub struct CrossVMHtlcCreated {
    pub htlc_id: [u8; 32],
    pub initiator: Pubkey,
    pub target_vm: u8,
    pub target_address: [u8; 32],
    pub token_mint: Pubkey,
    pub amount: u64,
    pub hashlock: [u8; 32],
    pub timelock: i64,
}

#[event]
pub struct CrossVMHtlcClaimed {
    pub htlc_id: [u8; 32],
    pub preimage: [u8; 32],
    pub claimed_at: i64,
}

#[event]
pub struct CrossVMHtlcRefunded {
    pub htlc_id: [u8; 32],
    pub refunded_at: i64,
}

#[event]
pub struct CrossVMMessageSent {
    pub message_id: [u8; 32],
    pub sender: Pubkey,
    pub target_vm: u8,
    pub target_address: [u8; 32],
    pub payload_hash: [u8; 32],
}

#[event]
pub struct CrossVMMessageExecuted {
    pub message_id: [u8; 32],
    pub source_vm: u8,
    pub source_address: [u8; 32],
    pub relayer: Pubkey,
}

#[event]
pub struct TokensLockedForWrapping {
    pub lock_id: [u8; 32],
    pub depositor: Pubkey,
    pub token_mint: Pubkey,
    pub target_vm: u8,
    pub target_recipient: [u8; 32],
    pub amount: u64,
}

#[event]
pub struct LockedTokensReleased {
    pub lock_id: [u8; 32],
    pub recipient: Pubkey,
    pub amount: u64,
}

#[event]
pub struct ProtocolPaused {}

#[event]
pub struct ProtocolUnpaused {}

// ═══════════════════════════════════════════════════════════════════════════════
// ERRORS
// ═══════════════════════════════════════════════════════════════════════════════

#[error_code]
pub enum CrossVMError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Protocol is paused")]
    ProtocolPaused,
    #[msg("Timelock too short")]
    TimelockTooShort,
    #[msg("Timelock too long")]
    TimelockTooLong,
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid target VM")]
    InvalidTargetVM,
    #[msg("HTLC not claimable")]
    HtlcNotClaimable,
    #[msg("HTLC not refundable")]
    HtlcNotRefundable,
    #[msg("Invalid preimage")]
    InvalidPreimage,
    #[msg("Timelock not expired")]
    TimelockNotExpired,
    #[msg("Message too large")]
    MessageTooLarge,
    #[msg("Max relayers reached")]
    MaxRelayersReached,
    #[msg("Relayer already exists")]
    RelayerAlreadyExists,
    #[msg("Relayer not found")]
    RelayerNotFound,
    #[msg("Unauthorized relayer")]
    UnauthorizedRelayer,
    #[msg("Invalid proof")]
    InvalidProof,
    #[msg("Already released")]
    AlreadyReleased,
}
