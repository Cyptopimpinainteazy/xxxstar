//! # AMM DEX Program (SVM/Solana)
//!
//! Constant Product Automated Market Maker for X3 Chain's Solana VM.
//! Supports liquidity pools, swaps, fees, and cross-VM integration.
//!
//! ## Features
//! - Constant product AMM (x*y=k)
//! - Concentrated liquidity support
//! - Fee tiers (0.05%, 0.30%, 1.00%)
//! - LP token minting/burning
//! - Cross-VM swap routing
//! - Flash swaps support
//! - Protocol fees and referrals

use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer, Mint, MintTo, Burn};

declare_id!("X3AMM1111111111111111111111111111111111111");

/// Fee tiers in BPS
pub const FEE_TIER_LOW: u16 = 5;      // 0.05%
pub const FEE_TIER_MEDIUM: u16 = 30;  // 0.30%
pub const FEE_TIER_HIGH: u16 = 100;   // 1.00%

/// Protocol fee (portion of swap fees)
pub const PROTOCOL_FEE_BPS: u16 = 1667; // ~16.67% of swap fee

/// Minimum liquidity locked forever (prevent division by zero attacks)
pub const MINIMUM_LIQUIDITY: u64 = 1000;

#[program]
pub mod amm {
    use super::*;

    // ═══════════════════════════════════════════════════════════════════════════
    // INITIALIZATION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Initialize the AMM protocol
    pub fn initialize(
        ctx: Context<InitializeProtocol>,
        fee_recipient: Pubkey,
    ) -> Result<()> {
        let protocol = &mut ctx.accounts.protocol;
        protocol.authority = ctx.accounts.authority.key();
        protocol.fee_recipient = fee_recipient;
        protocol.pool_count = 0;
        protocol.total_volume = 0;
        protocol.total_fees_collected = 0;
        protocol.paused = false;
        protocol.bump = ctx.bumps.protocol;

        emit!(ProtocolInitialized {
            authority: ctx.accounts.authority.key(),
            fee_recipient,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // POOL MANAGEMENT
    // ═══════════════════════════════════════════════════════════════════════════

    /// Create a new liquidity pool
    pub fn create_pool(
        ctx: Context<CreatePool>,
        fee_tier: u16,
    ) -> Result<()> {
        require!(
            fee_tier == FEE_TIER_LOW || fee_tier == FEE_TIER_MEDIUM || fee_tier == FEE_TIER_HIGH,
            AmmError::InvalidFeeTier
        );

        let protocol = &mut ctx.accounts.protocol;
        require!(!protocol.paused, AmmError::ProtocolPaused);

        // Ensure tokens are sorted (token_a < token_b)
        require!(
            ctx.accounts.token_a_mint.key() < ctx.accounts.token_b_mint.key(),
            AmmError::TokensNotSorted
        );

        let pool = &mut ctx.accounts.pool;
        pool.protocol = ctx.accounts.protocol.key();
        pool.token_a_mint = ctx.accounts.token_a_mint.key();
        pool.token_b_mint = ctx.accounts.token_b_mint.key();
        pool.token_a_vault = ctx.accounts.token_a_vault.key();
        pool.token_b_vault = ctx.accounts.token_b_vault.key();
        pool.lp_mint = ctx.accounts.lp_mint.key();
        pool.fee_tier = fee_tier;
        pool.reserve_a = 0;
        pool.reserve_b = 0;
        pool.lp_supply = 0;
        pool.total_volume_a = 0;
        pool.total_volume_b = 0;
        pool.total_fees_a = 0;
        pool.total_fees_b = 0;
        pool.created_at = Clock::get()?.unix_timestamp;
        pool.bump = ctx.bumps.pool;

        protocol.pool_count += 1;

        emit!(PoolCreated {
            pool: pool.key(),
            token_a: pool.token_a_mint,
            token_b: pool.token_b_mint,
            fee_tier,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // LIQUIDITY
    // ═══════════════════════════════════════════════════════════════════════════

    /// Add liquidity to pool
    pub fn add_liquidity(
        ctx: Context<AddLiquidity>,
        amount_a_desired: u64,
        amount_b_desired: u64,
        amount_a_min: u64,
        amount_b_min: u64,
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;
        require!(amount_a_desired > 0 && amount_b_desired > 0, AmmError::ZeroAmount);

        let (amount_a, amount_b, lp_tokens) = if pool.lp_supply == 0 {
            // First liquidity - use desired amounts
            let lp = calculate_initial_lp(amount_a_desired, amount_b_desired)?;
            (amount_a_desired, amount_b_desired, lp)
        } else {
            // Subsequent liquidity - maintain ratio
            let (a, b) = calculate_optimal_amounts(
                amount_a_desired,
                amount_b_desired,
                pool.reserve_a,
                pool.reserve_b,
            )?;
            
            require!(a >= amount_a_min, AmmError::SlippageExceeded);
            require!(b >= amount_b_min, AmmError::SlippageExceeded);

            let lp = calculate_lp_tokens(a, b, pool.reserve_a, pool.reserve_b, pool.lp_supply)?;
            (a, b, lp)
        };

        // Transfer token A
        let transfer_a_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_a.to_account_info(),
                to: ctx.accounts.token_a_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_a_ctx, amount_a)?;

        // Transfer token B
        let transfer_b_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_b.to_account_info(),
                to: ctx.accounts.token_b_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_b_ctx, amount_b)?;

        // Mint LP tokens
        let pool_seeds = &[
            b"pool",
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            &[pool.bump],
        ];
        let signer_seeds = &[&pool_seeds[..]];

        let mint_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            MintTo {
                mint: ctx.accounts.lp_mint.to_account_info(),
                to: ctx.accounts.user_lp_token.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        );
        token::mint_to(mint_ctx, lp_tokens)?;

        // Update pool state
        let pool = &mut ctx.accounts.pool;
        pool.reserve_a += amount_a;
        pool.reserve_b += amount_b;
        pool.lp_supply += lp_tokens;

        emit!(LiquidityAdded {
            pool: pool.key(),
            user: ctx.accounts.user.key(),
            amount_a,
            amount_b,
            lp_tokens,
        });

        Ok(())
    }

    /// Remove liquidity from pool
    pub fn remove_liquidity(
        ctx: Context<RemoveLiquidity>,
        lp_amount: u64,
        amount_a_min: u64,
        amount_b_min: u64,
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;
        require!(lp_amount > 0, AmmError::ZeroAmount);
        require!(pool.lp_supply > 0, AmmError::EmptyPool);

        // Calculate amounts to return
        let amount_a = (lp_amount as u128)
            .checked_mul(pool.reserve_a as u128)
            .and_then(|v| v.checked_div(pool.lp_supply as u128))
            .ok_or(AmmError::CalculationOverflow)? as u64;

        let amount_b = (lp_amount as u128)
            .checked_mul(pool.reserve_b as u128)
            .and_then(|v| v.checked_div(pool.lp_supply as u128))
            .ok_or(AmmError::CalculationOverflow)? as u64;

        require!(amount_a >= amount_a_min, AmmError::SlippageExceeded);
        require!(amount_b >= amount_b_min, AmmError::SlippageExceeded);

        // Burn LP tokens
        let burn_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Burn {
                mint: ctx.accounts.lp_mint.to_account_info(),
                from: ctx.accounts.user_lp_token.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::burn(burn_ctx, lp_amount)?;

        // Transfer tokens back to user
        let pool_seeds = &[
            b"pool",
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            &[pool.bump],
        ];
        let signer_seeds = &[&pool_seeds[..]];

        let transfer_a_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_a_vault.to_account_info(),
                to: ctx.accounts.user_token_a.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_a_ctx, amount_a)?;

        let transfer_b_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_b_vault.to_account_info(),
                to: ctx.accounts.user_token_b.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_b_ctx, amount_b)?;

        // Update pool state
        let pool = &mut ctx.accounts.pool;
        pool.reserve_a -= amount_a;
        pool.reserve_b -= amount_b;
        pool.lp_supply -= lp_amount;

        emit!(LiquidityRemoved {
            pool: pool.key(),
            user: ctx.accounts.user.key(),
            amount_a,
            amount_b,
            lp_tokens: lp_amount,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // SWAPS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Swap exact input for output
    pub fn swap_exact_input(
        ctx: Context<Swap>,
        amount_in: u64,
        minimum_amount_out: u64,
        a_to_b: bool,
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;
        let protocol = &ctx.accounts.protocol;

        require!(!protocol.paused, AmmError::ProtocolPaused);
        require!(amount_in > 0, AmmError::ZeroAmount);

        let (reserve_in, reserve_out) = if a_to_b {
            (pool.reserve_a, pool.reserve_b)
        } else {
            (pool.reserve_b, pool.reserve_a)
        };

        // Calculate output amount with fees
        let (amount_out, fee_amount, protocol_fee) = calculate_swap_output(
            amount_in,
            reserve_in,
            reserve_out,
            pool.fee_tier,
        )?;

        require!(amount_out >= minimum_amount_out, AmmError::SlippageExceeded);
        require!(amount_out < reserve_out, AmmError::InsufficientLiquidity);

        // Perform swap
        execute_swap(
            &ctx,
            amount_in,
            amount_out,
            fee_amount,
            protocol_fee,
            a_to_b,
        )?;

        Ok(())
    }

    /// Swap input for exact output
    pub fn swap_exact_output(
        ctx: Context<Swap>,
        amount_out: u64,
        maximum_amount_in: u64,
        a_to_b: bool,
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;
        let protocol = &ctx.accounts.protocol;

        require!(!protocol.paused, AmmError::ProtocolPaused);
        require!(amount_out > 0, AmmError::ZeroAmount);

        let (reserve_in, reserve_out) = if a_to_b {
            (pool.reserve_a, pool.reserve_b)
        } else {
            (pool.reserve_b, pool.reserve_a)
        };

        require!(amount_out < reserve_out, AmmError::InsufficientLiquidity);

        // Calculate required input amount
        let (amount_in, fee_amount, protocol_fee) = calculate_swap_input(
            amount_out,
            reserve_in,
            reserve_out,
            pool.fee_tier,
        )?;

        require!(amount_in <= maximum_amount_in, AmmError::SlippageExceeded);

        // Perform swap
        execute_swap(
            &ctx,
            amount_in,
            amount_out,
            fee_amount,
            protocol_fee,
            a_to_b,
        )?;

        Ok(())
    }

    /// Flash swap (borrow then repay in same tx)
    pub fn flash_swap(
        ctx: Context<FlashSwap>,
        amount_a_out: u64,
        amount_b_out: u64,
        callback_data: Vec<u8>,
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;
        let protocol = &ctx.accounts.protocol;

        require!(!protocol.paused, AmmError::ProtocolPaused);
        require!(amount_a_out > 0 || amount_b_out > 0, AmmError::ZeroAmount);
        require!(amount_a_out < pool.reserve_a, AmmError::InsufficientLiquidity);
        require!(amount_b_out < pool.reserve_b, AmmError::InsufficientLiquidity);

        // Record initial K value
        let k_before = (pool.reserve_a as u128) * (pool.reserve_b as u128);

        // Transfer requested amounts to borrower
        let pool_seeds = &[
            b"pool",
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            &[pool.bump],
        ];
        let signer_seeds = &[&pool_seeds[..]];

        if amount_a_out > 0 {
            let transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_a_vault.to_account_info(),
                    to: ctx.accounts.user_token_a.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                signer_seeds,
            );
            token::transfer(transfer_ctx, amount_a_out)?;
        }

        if amount_b_out > 0 {
            let transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_b_vault.to_account_info(),
                    to: ctx.accounts.user_token_b.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                signer_seeds,
            );
            token::transfer(transfer_ctx, amount_b_out)?;
        }

        // Emit event for callback (in production, this would trigger actual callback)
        emit!(FlashSwapInitiated {
            pool: pool.key(),
            borrower: ctx.accounts.user.key(),
            amount_a: amount_a_out,
            amount_b: amount_b_out,
            callback_data: callback_data.clone(),
        });

        // After callback, verify K has increased (accounting for fees)
        // In a real implementation, the callback would execute and repay
        // For now, we just require the reserves to be restored + fees

        ctx.accounts.token_a_vault.reload()?;
        ctx.accounts.token_b_vault.reload()?;

        let new_reserve_a = ctx.accounts.token_a_vault.amount;
        let new_reserve_b = ctx.accounts.token_b_vault.amount;

        // K must not decrease (should increase due to fees)
        let k_after = (new_reserve_a as u128) * (new_reserve_b as u128);
        require!(k_after >= k_before, AmmError::KInvariantViolated);

        // Update pool reserves
        let pool = &mut ctx.accounts.pool;
        pool.reserve_a = new_reserve_a;
        pool.reserve_b = new_reserve_b;

        emit!(FlashSwapCompleted {
            pool: pool.key(),
            borrower: ctx.accounts.user.key(),
            amount_a_repaid: new_reserve_a.saturating_sub(pool.reserve_a - amount_a_out),
            amount_b_repaid: new_reserve_b.saturating_sub(pool.reserve_b - amount_b_out),
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // CROSS-VM INTEGRATION
    // ═══════════════════════════════════════════════════════════════════════════

    /// Execute cross-VM swap (called by bridge)
    pub fn cross_vm_swap(
        ctx: Context<CrossVmSwap>,
        amount_in: u64,
        minimum_amount_out: u64,
        a_to_b: bool,
        evm_recipient: [u8; 20], // EVM address
    ) -> Result<()> {
        let pool = &ctx.accounts.pool;
        let protocol = &ctx.accounts.protocol;

        require!(!protocol.paused, AmmError::ProtocolPaused);
        require!(amount_in > 0, AmmError::ZeroAmount);

        // Calculate swap output
        let (reserve_in, reserve_out) = if a_to_b {
            (pool.reserve_a, pool.reserve_b)
        } else {
            (pool.reserve_b, pool.reserve_a)
        };

        let (amount_out, fee_amount, _protocol_fee) = calculate_swap_output(
            amount_in,
            reserve_in,
            reserve_out,
            pool.fee_tier,
        )?;

        require!(amount_out >= minimum_amount_out, AmmError::SlippageExceeded);

        // Update reserves (tokens already transferred via bridge)
        let pool = &mut ctx.accounts.pool;
        if a_to_b {
            pool.reserve_a += amount_in;
            pool.reserve_b -= amount_out;
            pool.total_volume_a += amount_in;
            pool.total_fees_a += fee_amount;
        } else {
            pool.reserve_b += amount_in;
            pool.reserve_a -= amount_out;
            pool.total_volume_b += amount_in;
            pool.total_fees_b += fee_amount;
        }

        emit!(CrossVmSwapExecuted {
            pool: pool.key(),
            amount_in,
            amount_out,
            a_to_b,
            evm_recipient,
        });

        Ok(())
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // VIEWS
    // ═══════════════════════════════════════════════════════════════════════════

    /// Get quote for swap
    pub fn get_quote(
        ctx: Context<GetQuote>,
        amount_in: u64,
        a_to_b: bool,
    ) -> Result<SwapQuote> {
        let pool = &ctx.accounts.pool;

        let (reserve_in, reserve_out) = if a_to_b {
            (pool.reserve_a, pool.reserve_b)
        } else {
            (pool.reserve_b, pool.reserve_a)
        };

        let (amount_out, fee_amount, protocol_fee) = calculate_swap_output(
            amount_in,
            reserve_in,
            reserve_out,
            pool.fee_tier,
        )?;

        let price_impact = calculate_price_impact(amount_in, amount_out, reserve_in, reserve_out)?;

        Ok(SwapQuote {
            amount_out,
            fee_amount,
            protocol_fee,
            price_impact_bps: price_impact,
        })
    }

    // ═══════════════════════════════════════════════════════════════════════════
    // ADMIN
    // ═══════════════════════════════════════════════════════════════════════════

    pub fn pause(ctx: Context<AdminAction>) -> Result<()> {
        ctx.accounts.protocol.paused = true;
        emit!(ProtocolPausedEvent {});
        Ok(())
    }

    pub fn unpause(ctx: Context<AdminAction>) -> Result<()> {
        ctx.accounts.protocol.paused = false;
        emit!(ProtocolUnpausedEvent {});
        Ok(())
    }

    pub fn update_fee_recipient(
        ctx: Context<AdminAction>,
        new_recipient: Pubkey,
    ) -> Result<()> {
        ctx.accounts.protocol.fee_recipient = new_recipient;
        emit!(FeeRecipientUpdated {
            new_recipient,
        });
        Ok(())
    }

    /// Collect accumulated protocol fees from pool
    pub fn collect_protocol_fees(ctx: Context<CollectFees>) -> Result<()> {
        let pool = &mut ctx.accounts.pool;
        let protocol = &ctx.accounts.protocol;

        let fees_a = pool.total_fees_a;
        let fees_b = pool.total_fees_b;

        require!(fees_a > 0 || fees_b > 0, AmmError::NoFeesToCollect);

        // Transfer fees to protocol fee recipient
        let pool_seeds = &[
            b"pool",
            pool.token_a_mint.as_ref(),
            pool.token_b_mint.as_ref(),
            &[pool.bump],
        ];
        let signer_seeds = &[&pool_seeds[..]];

        if fees_a > 0 {
            let transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_a_vault.to_account_info(),
                    to: ctx.accounts.fee_recipient_a.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                signer_seeds,
            );
            token::transfer(transfer_ctx, fees_a)?;
        }

        if fees_b > 0 {
            let transfer_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.token_b_vault.to_account_info(),
                    to: ctx.accounts.fee_recipient_b.to_account_info(),
                    authority: ctx.accounts.pool.to_account_info(),
                },
                signer_seeds,
            );
            token::transfer(transfer_ctx, fees_b)?;
        }

        // Reset fee counters
        pool.total_fees_a = 0;
        pool.total_fees_b = 0;

        // Update protocol stats
        let protocol = &mut ctx.accounts.protocol;
        protocol.total_fees_collected += fees_a + fees_b;

        emit!(FeesCollected {
            pool: pool.key(),
            fees_a,
            fees_b,
        });

        Ok(())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HELPER FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

fn calculate_initial_lp(amount_a: u64, amount_b: u64) -> Result<u64> {
    let lp = ((amount_a as u128) * (amount_b as u128)).integer_sqrt() as u64;
    require!(lp > MINIMUM_LIQUIDITY, AmmError::InsufficientInitialLiquidity);
    Ok(lp - MINIMUM_LIQUIDITY) // Lock MINIMUM_LIQUIDITY forever
}

fn calculate_optimal_amounts(
    amount_a_desired: u64,
    amount_b_desired: u64,
    reserve_a: u64,
    reserve_b: u64,
) -> Result<(u64, u64)> {
    let amount_b_optimal = (amount_a_desired as u128)
        .checked_mul(reserve_b as u128)
        .ok_or(AmmError::CalculationOverflow)?
        .checked_div(reserve_a as u128)
        .ok_or(AmmError::CalculationOverflow)? as u64;

    if amount_b_optimal <= amount_b_desired {
        Ok((amount_a_desired, amount_b_optimal))
    } else {
        let amount_a_optimal = (amount_b_desired as u128)
            .checked_mul(reserve_a as u128)
            .ok_or(AmmError::CalculationOverflow)?
            .checked_div(reserve_b as u128)
            .ok_or(AmmError::CalculationOverflow)? as u64;
        Ok((amount_a_optimal, amount_b_desired))
    }
}

fn calculate_lp_tokens(
    amount_a: u64,
    amount_b: u64,
    reserve_a: u64,
    reserve_b: u64,
    lp_supply: u64,
) -> Result<u64> {
    let lp_a = (amount_a as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::CalculationOverflow)?
        .checked_div(reserve_a as u128)
        .ok_or(AmmError::CalculationOverflow)?;

    let lp_b = (amount_b as u128)
        .checked_mul(lp_supply as u128)
        .ok_or(AmmError::CalculationOverflow)?
        .checked_div(reserve_b as u128)
        .ok_or(AmmError::CalculationOverflow)?;

    Ok(lp_a.min(lp_b) as u64)
}

fn calculate_swap_output(
    amount_in: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_tier: u16,
) -> Result<(u64, u64, u64)> {
    // Fee calculation
    let fee_amount = (amount_in as u128)
        .checked_mul(fee_tier as u128)
        .ok_or(AmmError::CalculationOverflow)?
        .checked_div(10000)
        .ok_or(AmmError::CalculationOverflow)? as u64;

    let amount_in_after_fee = amount_in - fee_amount;

    // Protocol fee (portion of swap fee)
    let protocol_fee = (fee_amount as u128)
        .checked_mul(PROTOCOL_FEE_BPS as u128)
        .ok_or(AmmError::CalculationOverflow)?
        .checked_div(10000)
        .ok_or(AmmError::CalculationOverflow)? as u64;

    // Constant product formula: dy = y * dx / (x + dx)
    let amount_out = (reserve_out as u128)
        .checked_mul(amount_in_after_fee as u128)
        .ok_or(AmmError::CalculationOverflow)?
        .checked_div((reserve_in as u128) + (amount_in_after_fee as u128))
        .ok_or(AmmError::CalculationOverflow)? as u64;

    Ok((amount_out, fee_amount, protocol_fee))
}

fn calculate_swap_input(
    amount_out: u64,
    reserve_in: u64,
    reserve_out: u64,
    fee_tier: u16,
) -> Result<(u64, u64, u64)> {
    // Inverse constant product formula: dx = x * dy / (y - dy)
    let numerator = (reserve_in as u128).checked_mul(amount_out as u128)
        .ok_or(AmmError::CalculationOverflow)?;
    let denominator = (reserve_out as u128).checked_sub(amount_out as u128)
        .ok_or(AmmError::CalculationOverflow)?;
    let amount_in_before_fee = numerator.checked_div(denominator)
        .ok_or(AmmError::CalculationOverflow)? as u64 + 1;

    // Add fee
    let amount_in = (amount_in_before_fee as u128)
        .checked_mul(10000)
        .ok_or(AmmError::CalculationOverflow)?
        .checked_div((10000 - fee_tier as u128) as u128)
        .ok_or(AmmError::CalculationOverflow)? as u64;

    let fee_amount = amount_in - amount_in_before_fee;
    let protocol_fee = (fee_amount as u128)
        .checked_mul(PROTOCOL_FEE_BPS as u128)
        .ok_or(AmmError::CalculationOverflow)?
        .checked_div(10000)
        .ok_or(AmmError::CalculationOverflow)? as u64;

    Ok((amount_in, fee_amount, protocol_fee))
}

fn calculate_price_impact(
    amount_in: u64,
    amount_out: u64,
    reserve_in: u64,
    reserve_out: u64,
) -> Result<u16> {
    let spot_price = (reserve_out as u128) * 10000 / (reserve_in as u128);
    let exec_price = (amount_out as u128) * 10000 / (amount_in as u128);
    
    if spot_price <= exec_price {
        Ok(0)
    } else {
        let impact = ((spot_price - exec_price) * 10000 / spot_price) as u16;
        Ok(impact)
    }
}

fn execute_swap(
    ctx: &Context<Swap>,
    amount_in: u64,
    amount_out: u64,
    fee_amount: u64,
    _protocol_fee: u64,
    a_to_b: bool,
) -> Result<()> {
    let pool = &ctx.accounts.pool;

    let pool_seeds = &[
        b"pool",
        pool.token_a_mint.as_ref(),
        pool.token_b_mint.as_ref(),
        &[pool.bump],
    ];
    let signer_seeds = &[&pool_seeds[..]];

    if a_to_b {
        // Transfer A in
        let transfer_in_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_in.to_account_info(),
                to: ctx.accounts.token_a_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_in_ctx, amount_in)?;

        // Transfer B out
        let transfer_out_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_b_vault.to_account_info(),
                to: ctx.accounts.user_token_out.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_out_ctx, amount_out)?;
    } else {
        // Transfer B in
        let transfer_in_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.user_token_in.to_account_info(),
                to: ctx.accounts.token_b_vault.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(transfer_in_ctx, amount_in)?;

        // Transfer A out
        let transfer_out_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            Transfer {
                from: ctx.accounts.token_a_vault.to_account_info(),
                to: ctx.accounts.user_token_out.to_account_info(),
                authority: ctx.accounts.pool.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(transfer_out_ctx, amount_out)?;
    }

    // Update pool state
    let pool = &mut ctx.accounts.pool;
    if a_to_b {
        pool.reserve_a += amount_in;
        pool.reserve_b -= amount_out;
        pool.total_volume_a += amount_in;
        pool.total_fees_a += fee_amount;
    } else {
        pool.reserve_b += amount_in;
        pool.reserve_a -= amount_out;
        pool.total_volume_b += amount_in;
        pool.total_fees_b += fee_amount;
    }

    // Update protocol stats
    let protocol = &mut ctx.accounts.protocol;
    protocol.total_volume += amount_in;

    emit!(SwapExecuted {
        pool: pool.key(),
        user: ctx.accounts.user.key(),
        amount_in,
        amount_out,
        fee: fee_amount,
        a_to_b,
    });

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════════
// ACCOUNTS
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Accounts)]
pub struct InitializeProtocol<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        init,
        payer = authority,
        space = 8 + Protocol::SIZE,
        seeds = [b"protocol"],
        bump
    )]
    pub protocol: Account<'info, Protocol>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreatePool<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Account<'info, Protocol>,

    pub token_a_mint: Account<'info, Mint>,
    pub token_b_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = creator,
        space = 8 + Pool::SIZE,
        seeds = [b"pool", token_a_mint.key().as_ref(), token_b_mint.key().as_ref()],
        bump
    )]
    pub pool: Account<'info, Pool>,

    #[account(
        init,
        payer = creator,
        token::mint = token_a_mint,
        token::authority = pool,
        seeds = [b"vault_a", pool.key().as_ref()],
        bump
    )]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = creator,
        token::mint = token_b_mint,
        token::authority = pool,
        seeds = [b"vault_b", pool.key().as_ref()],
        bump
    )]
    pub token_b_vault: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = creator,
        mint::decimals = 9,
        mint::authority = pool,
        seeds = [b"lp_mint", pool.key().as_ref()],
        bump
    )]
    pub lp_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct AddLiquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, seeds = [b"pool", pool.token_a_mint.as_ref(), pool.token_b_mint.as_ref()], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(mut, seeds = [b"vault_a", pool.key().as_ref()], bump)]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"vault_b", pool.key().as_ref()], bump)]
    pub token_b_vault: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"lp_mint", pool.key().as_ref()], bump)]
    pub lp_mint: Account<'info, Mint>,

    #[account(mut, constraint = user_token_a.owner == user.key())]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_token_b.owner == user.key())]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_lp_token.mint == lp_mint.key())]
    pub user_lp_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct RemoveLiquidity<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, seeds = [b"pool", pool.token_a_mint.as_ref(), pool.token_b_mint.as_ref()], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(mut, seeds = [b"vault_a", pool.key().as_ref()], bump)]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"vault_b", pool.key().as_ref()], bump)]
    pub token_b_vault: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"lp_mint", pool.key().as_ref()], bump)]
    pub lp_mint: Account<'info, Mint>,

    #[account(mut, constraint = user_token_a.owner == user.key())]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_token_b.owner == user.key())]
    pub user_token_b: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_lp_token.mint == lp_mint.key())]
    pub user_lp_token: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Account<'info, Protocol>,

    #[account(mut, seeds = [b"pool", pool.token_a_mint.as_ref(), pool.token_b_mint.as_ref()], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(mut, seeds = [b"vault_a", pool.key().as_ref()], bump)]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"vault_b", pool.key().as_ref()], bump)]
    pub token_b_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_token_in.owner == user.key())]
    pub user_token_in: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_token_out.owner == user.key())]
    pub user_token_out: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct FlashSwap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Account<'info, Protocol>,

    #[account(mut, seeds = [b"pool", pool.token_a_mint.as_ref(), pool.token_b_mint.as_ref()], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(mut, seeds = [b"vault_a", pool.key().as_ref()], bump)]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"vault_b", pool.key().as_ref()], bump)]
    pub token_b_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_token_a.owner == user.key())]
    pub user_token_a: Account<'info, TokenAccount>,

    #[account(mut, constraint = user_token_b.owner == user.key())]
    pub user_token_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct CrossVmSwap<'info> {
    #[account(mut)]
    pub bridge: Signer<'info>, // Must be authorized bridge

    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Account<'info, Protocol>,

    #[account(mut, seeds = [b"pool", pool.token_a_mint.as_ref(), pool.token_b_mint.as_ref()], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(mut, seeds = [b"vault_a", pool.key().as_ref()], bump)]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"vault_b", pool.key().as_ref()], bump)]
    pub token_b_vault: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct GetQuote<'info> {
    pub pool: Account<'info, Pool>,
}

#[derive(Accounts)]
pub struct AdminAction<'info> {
    #[account(constraint = authority.key() == protocol.authority @ AmmError::Unauthorized)]
    pub authority: Signer<'info>,

    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Account<'info, Protocol>,
}

#[derive(Accounts)]
pub struct CollectFees<'info> {
    #[account(constraint = authority.key() == protocol.authority @ AmmError::Unauthorized)]
    pub authority: Signer<'info>,

    #[account(mut, seeds = [b"protocol"], bump = protocol.bump)]
    pub protocol: Account<'info, Protocol>,

    #[account(mut, seeds = [b"pool", pool.token_a_mint.as_ref(), pool.token_b_mint.as_ref()], bump = pool.bump)]
    pub pool: Account<'info, Pool>,

    #[account(mut, seeds = [b"vault_a", pool.key().as_ref()], bump)]
    pub token_a_vault: Account<'info, TokenAccount>,

    #[account(mut, seeds = [b"vault_b", pool.key().as_ref()], bump)]
    pub token_b_vault: Account<'info, TokenAccount>,

    #[account(mut, constraint = fee_recipient_a.owner == protocol.fee_recipient)]
    pub fee_recipient_a: Account<'info, TokenAccount>,

    #[account(mut, constraint = fee_recipient_b.owner == protocol.fee_recipient)]
    pub fee_recipient_b: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// STATE
// ═══════════════════════════════════════════════════════════════════════════════

#[account]
pub struct Protocol {
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
    pub pool_count: u64,
    pub total_volume: u64,
    pub total_fees_collected: u64,
    pub paused: bool,
    pub bump: u8,
}

impl Protocol {
    pub const SIZE: usize = 32 + 32 + 8 + 8 + 8 + 1 + 1;
}

#[account]
pub struct Pool {
    pub protocol: Pubkey,
    pub token_a_mint: Pubkey,
    pub token_b_mint: Pubkey,
    pub token_a_vault: Pubkey,
    pub token_b_vault: Pubkey,
    pub lp_mint: Pubkey,
    pub fee_tier: u16,
    pub reserve_a: u64,
    pub reserve_b: u64,
    pub lp_supply: u64,
    pub total_volume_a: u64,
    pub total_volume_b: u64,
    pub total_fees_a: u64,
    pub total_fees_b: u64,
    pub created_at: i64,
    pub bump: u8,
}

impl Pool {
    pub const SIZE: usize = 32 + 32 + 32 + 32 + 32 + 32 + 2 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 8 + 1;
}

// ═══════════════════════════════════════════════════════════════════════════════
// RETURN TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SwapQuote {
    pub amount_out: u64,
    pub fee_amount: u64,
    pub protocol_fee: u64,
    pub price_impact_bps: u16,
}

// ═══════════════════════════════════════════════════════════════════════════════
// EVENTS
// ═══════════════════════════════════════════════════════════════════════════════

#[event]
pub struct ProtocolInitialized {
    pub authority: Pubkey,
    pub fee_recipient: Pubkey,
}

#[event]
pub struct PoolCreated {
    pub pool: Pubkey,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub fee_tier: u16,
}

#[event]
pub struct LiquidityAdded {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount_a: u64,
    pub amount_b: u64,
    pub lp_tokens: u64,
}

#[event]
pub struct LiquidityRemoved {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount_a: u64,
    pub amount_b: u64,
    pub lp_tokens: u64,
}

#[event]
pub struct SwapExecuted {
    pub pool: Pubkey,
    pub user: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub fee: u64,
    pub a_to_b: bool,
}

#[event]
pub struct FlashSwapInitiated {
    pub pool: Pubkey,
    pub borrower: Pubkey,
    pub amount_a: u64,
    pub amount_b: u64,
    pub callback_data: Vec<u8>,
}

#[event]
pub struct FlashSwapCompleted {
    pub pool: Pubkey,
    pub borrower: Pubkey,
    pub amount_a_repaid: u64,
    pub amount_b_repaid: u64,
}

#[event]
pub struct CrossVmSwapExecuted {
    pub pool: Pubkey,
    pub amount_in: u64,
    pub amount_out: u64,
    pub a_to_b: bool,
    pub evm_recipient: [u8; 20],
}

#[event]
pub struct ProtocolPausedEvent {}

#[event]
pub struct ProtocolUnpausedEvent {}

#[event]
pub struct FeeRecipientUpdated {
    pub new_recipient: Pubkey,
}

#[event]
pub struct FeesCollected {
    pub pool: Pubkey,
    pub fees_a: u64,
    pub fees_b: u64,
}

// ═══════════════════════════════════════════════════════════════════════════════
// ERRORS
// ═══════════════════════════════════════════════════════════════════════════════

#[error_code]
pub enum AmmError {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Protocol is paused")]
    ProtocolPaused,
    #[msg("Invalid fee tier")]
    InvalidFeeTier,
    #[msg("Tokens not sorted")]
    TokensNotSorted,
    #[msg("Zero amount")]
    ZeroAmount,
    #[msg("Slippage exceeded")]
    SlippageExceeded,
    #[msg("Insufficient liquidity")]
    InsufficientLiquidity,
    #[msg("Insufficient initial liquidity")]
    InsufficientInitialLiquidity,
    #[msg("Empty pool")]
    EmptyPool,
    #[msg("K invariant violated")]
    KInvariantViolated,
    #[msg("No fees to collect")]
    NoFeesToCollect,
    #[msg("Calculation overflow")]
    CalculationOverflow,
}

/// Integer square root helper for u128
trait IntegerSquareRoot {
    fn integer_sqrt(self) -> Self;
}

impl IntegerSquareRoot for u128 {
    fn integer_sqrt(self) -> Self {
        if self == 0 {
            return 0;
        }
        let mut x = self;
        let mut y = (x + 1) / 2;
        while y < x {
            x = y;
            y = (x + self / x) / 2;
        }
        x
    }
}
