//! Pool creation and initial liquidity bootstrap.
//!
//! Wraps `x3_dex::AMMPool::create_pool` and adds basic sanity checks that
//! the raw DEX layer does not enforce.

/// Request descriptor for creating a new AMM pool.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchRequest {
    pub token_a: u64,
    pub token_b: u64,
    /// Initial liquidity for token A side (raw units).
    pub initial_a: u128,
    /// Initial liquidity for token B side (raw units).
    pub initial_b: u128,
    /// Fee in basis points (max 1000 = 10 %).
    pub fee_bps: u32,
}

/// Errors returned by `Launchpad`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LaunchError {
    /// Both token IDs are the same.
    SameToken,
    /// At least one initial liquidity value is zero.
    ZeroInitialLiquidity,
    /// Fee exceeds the 1000 bps ceiling.
    FeeTooHigh,
}

/// Default fee used when none is supplied: 30 bps (0.30 %).
pub const DEFAULT_FEE_BPS: u32 = 30;
/// Maximum allowed pool fee: 1000 bps (10 %).
pub const MAX_FEE_BPS: u32 = 1_000;

/// Configuration for when a pool should graduate from bonding curve to AMM
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraduationConfig {
    /// Target market capitalization threshold (in base units)
    pub target_market_cap: u128,
    /// Minimum total liquidity required
    pub min_liquidity: u128,
    /// Minimum number of unique holders required
    pub min_holders: u32,
    /// Minimum time since launch (in blocks)
    pub min_age_blocks: u32,
}

/// Current state of a launch pool
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LaunchPoolState {
    /// Total tokens sold through bonding curve
    pub tokens_sold: u128,
    /// Total ETH/X3 received
    pub total_raised: u128,
    /// Current token price from bonding curve
    pub current_price: u128,
    /// Number of unique contributors
    pub contributor_count: u32,
    /// Block when pool was launched
    pub launch_block: u32,
    /// Current block
    pub current_block: u32,
}

/// AMM pool launch helper.
pub struct Launchpad;

impl Launchpad {
    /// Build a launch request with the default fee.
    pub fn build(
        token_a: u64,
        token_b: u64,
        initial_a: u128,
        initial_b: u128,
    ) -> Result<LaunchRequest, LaunchError> {
        Self::build_with_fee(token_a, token_b, initial_a, initial_b, DEFAULT_FEE_BPS)
    }

    /// Build a launch request with an explicit fee.
    pub fn build_with_fee(
        token_a: u64,
        token_b: u64,
        initial_a: u128,
        initial_b: u128,
        fee_bps: u32,
    ) -> Result<LaunchRequest, LaunchError> {
        if token_a == token_b {
            return Err(LaunchError::SameToken);
        }
        if initial_a == 0 || initial_b == 0 {
            return Err(LaunchError::ZeroInitialLiquidity);
        }
        if fee_bps > MAX_FEE_BPS {
            return Err(LaunchError::FeeTooHigh);
        }
        Ok(LaunchRequest {
            token_a,
            token_b,
            initial_a,
            initial_b,
            fee_bps,
        })
    }

    /// Check if a launch pool meets graduation criteria
    pub fn check_graduation(
        state: &LaunchPoolState,
        config: &GraduationConfig,
    ) -> Result<bool, &'static str> {
        // Check minimum age
        let age_blocks = state.current_block.saturating_sub(state.launch_block);
        if age_blocks < config.min_age_blocks {
            return Ok(false);
        }

        // Check minimum liquidity raised
        if state.total_raised < config.min_liquidity {
            return Ok(false);
        }

        // Check minimum holder count
        if state.contributor_count < config.min_holders {
            return Ok(false);
        }

        // Check market cap target (simplified - would need token supply and price data)
        // For now, just check if we've raised enough capital
        let estimated_market_cap = state.tokens_sold.saturating_mul(state.current_price);
        if estimated_market_cap < config.target_market_cap {
            return Ok(false);
        }

        Ok(true)
    }

    /// Execute graduation: transition from bonding curve to AMM pool
    pub fn execute_graduation(
        state: &LaunchPoolState,
        config: &GraduationConfig,
    ) -> Result<GraduationResult, &'static str> {
        // Validate graduation criteria
        if !Self::check_graduation(state, config)? {
            return Err("Graduation criteria not met");
        }

        // Calculate AMM pool parameters
        let amm_fee_bps = 30; // Standard 0.3% fee
        let initial_liquidity_a = state.total_raised;
        let initial_liquidity_b = state.tokens_sold;

        // Create the AMM pool request
        let amm_request = LaunchRequest {
            token_a: 0, // X3 native token
            token_b: 1, // Launched token (placeholder ID)
            initial_a: initial_liquidity_a,
            initial_b: initial_liquidity_b,
            fee_bps: amm_fee_bps,
        };

        Ok(GraduationResult {
            amm_request,
            burned_bonding_curve_tokens: 0, // Would calculate based on unsold tokens
            lp_tokens_distributed: initial_liquidity_a.saturating_add(initial_liquidity_b),
        })
    }
}

/// Result of a successful graduation
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraduationResult {
    /// AMM pool creation request
    pub amm_request: LaunchRequest,
    /// Number of bonding curve tokens burned
    pub burned_bonding_curve_tokens: u128,
    /// LP tokens distributed to contributors
    pub lp_tokens_distributed: u128,
}
