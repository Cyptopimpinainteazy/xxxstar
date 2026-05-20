/// Gas Relayer — Multi-token fee payment abstraction enabling users to pay transaction fees in any token
/// Implements relayer netting, fee routing, and sponsor mechanisms
use parity_scale_codec::{Decode, DecodeWithMemTracking, Encode};
use sp_std::vec::Vec;

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct RelayerConfig {
    pub relayer_id: [u8; 32],
    pub accepted_tokens: Vec<[u8; 32]>,
    pub fee_share_bps: u32, // Basis points (0-10000)
    pub min_fee_liquidity: u128,
    pub is_active: bool,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct FeeRequest {
    pub request_id: [u8; 32],
    pub transaction_hash: [u8; 32],
    pub payer: [u8; 32],
    pub fee_token: [u8; 32],
    pub fee_amount: u128,
    pub native_fee_equivalent: u128,
    pub relayer: [u8; 32],
    pub status: FeeRequestStatus,
    pub created_at: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub enum FeeRequestStatus {
    Pending,
    Settled,
    Refunded,
    Disputed,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct TokenExchangeRate {
    pub token: [u8; 32],
    pub native_per_token: u128,
    pub updated_at: u64,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct RelayerBalance {
    pub relayer: [u8; 32],
    pub token: [u8; 32],
    pub balance: u128,
}

#[derive(Clone, Encode, Decode, DecodeWithMemTracking, Debug, PartialEq, Eq)]
pub struct SponsorPool {
    pub pool_id: [u8; 32],
    pub sponsor: [u8; 32],
    pub tokens: Vec<[u8; 32]>,
    pub balances: Vec<u128>,
    pub beneficiaries: Vec<[u8; 32]>,
    pub is_active: bool,
}

pub const MAX_FEE_BPS: u32 = 1000; // 10% max relayer fee
pub const NATIVE_TOKEN: [u8; 32] = [0u8; 32]; // Zero address = native X3 token
pub const MIN_RELAYER_STAKE: u128 = 1000 * 10u128.pow(18); // 1000 X3 tokens

pub struct GasRelayer;

impl GasRelayer {
    /// Register a new relayer with accepted token list and fee share
    pub fn register_relayer(
        relayer_id: [u8; 32],
        accepted_tokens: Vec<[u8; 32]>,
        fee_share_bps: u32,
    ) -> Result<RelayerConfig, &'static str> {
        if relayer_id == [0; 32] {
            return Err("Relayer ID cannot be zero");
        }
        if accepted_tokens.is_empty() {
            return Err("Must accept at least one token");
        }
        if fee_share_bps > MAX_FEE_BPS {
            return Err("Fee share exceeds maximum");
        }

        Ok(RelayerConfig {
            relayer_id,
            accepted_tokens,
            fee_share_bps,
            min_fee_liquidity: 0,
            is_active: true,
        })
    }

    /// Create fee request (user pays fee in non-native token)
    pub fn create_fee_request(
        payer: [u8; 32],
        fee_token: [u8; 32],
        fee_amount: u128,
        native_fee_equivalent: u128,
        relayer: [u8; 32],
    ) -> Result<FeeRequest, &'static str> {
        if payer == [0; 32] {
            return Err("Invalid payer");
        }
        if fee_amount == 0 {
            return Err("Fee amount must be positive");
        }
        if native_fee_equivalent == 0 {
            return Err("Native fee equivalent must be positive");
        }

        let request_id = Self::generate_request_id(&payer, &fee_token, fee_amount);

        Ok(FeeRequest {
            request_id,
            transaction_hash: [0; 32],
            payer,
            fee_token,
            fee_amount,
            native_fee_equivalent,
            relayer,
            status: FeeRequestStatus::Pending,
            created_at: 0,
        })
    }

    /// Settle fee request with slippage tolerance
    pub fn settle_fee(
        fee_req: &mut FeeRequest,
        exchange_rate: &TokenExchangeRate,
        max_slippage_bps: u32,
    ) -> Result<u128, &'static str> {
        if fee_req.status != FeeRequestStatus::Pending {
            return Err("Fee request is not in Pending state");
        }
        if exchange_rate.token != fee_req.fee_token {
            return Err("Exchange rate token mismatch");
        }

        // Calculate equivalent native amount at current rate
        let equivalent =
            (fee_req.fee_amount as f64) * (exchange_rate.native_per_token as f64) / 1e18;
        let equivalent_native = equivalent as u128;

        // Check slippage
        let max_acceptable = fee_req
            .native_fee_equivalent
            .saturating_add((fee_req.native_fee_equivalent * max_slippage_bps as u128) / 10000);

        if equivalent_native > max_acceptable {
            return Err("Slippage exceeds tolerance");
        }

        fee_req.status = FeeRequestStatus::Settled;
        Ok(equivalent_native)
    }

    /// Process relayer payout (fee collected minus relayer share)
    pub fn process_relayer_payout(
        fee_req: &FeeRequest,
        relayer_config: &RelayerConfig,
    ) -> Result<(u128, u128), &'static str> {
        if fee_req.status != FeeRequestStatus::Settled {
            return Err("Fee must be settled first");
        }

        // Relayer keeps (fee_share_bps % of native equivalent)
        let relayer_share = (fee_req.native_fee_equivalent as u128)
            .saturating_mul(relayer_config.fee_share_bps as u128)
            .saturating_div(10000u128);

        // Protocol keeps the rest
        let protocol_share = fee_req.native_fee_equivalent.saturating_sub(relayer_share);

        Ok((relayer_share, protocol_share))
    }

    /// Update token exchange rate with new market data
    pub fn update_exchange_rate(
        token: [u8; 32],
        native_per_token: u128,
        current_timestamp: u64,
    ) -> Result<TokenExchangeRate, &'static str> {
        if token == [0; 32] {
            return Err("Cannot update rate for native token");
        }
        if native_per_token == 0 {
            return Err("Exchange rate must be positive");
        }

        Ok(TokenExchangeRate {
            token,
            native_per_token,
            updated_at: current_timestamp,
        })
    }

    /// Create sponsor pool for fee-free transactions to target beneficiaries
    pub fn create_sponsor_pool(
        sponsor: [u8; 32],
        tokens: Vec<[u8; 32]>,
        balances: Vec<u128>,
        beneficiaries: Vec<[u8; 32]>,
    ) -> Result<SponsorPool, &'static str> {
        if sponsor == [0; 32] {
            return Err("Invalid sponsor");
        }
        if tokens.len() != balances.len() {
            return Err("Token/balance count mismatch");
        }
        if tokens.is_empty() {
            return Err("Pool must contain at least one token");
        }
        if beneficiaries.is_empty() {
            return Err("Pool must have beneficiaries");
        }

        let pool_id = Self::generate_pool_id(&sponsor, &tokens);

        Ok(SponsorPool {
            pool_id,
            sponsor,
            tokens,
            balances,
            beneficiaries,
            is_active: true,
        })
    }

    /// Check if user is eligible for sponsored fees
    pub fn is_sponsored_beneficiary(
        pool: &SponsorPool,
        user: &[u8; 32],
    ) -> Result<bool, &'static str> {
        if !pool.is_active {
            return Err("Pool is not active");
        }

        Ok(pool.beneficiaries.contains(user))
    }

    /// Deduct sponsored fee from pool
    pub fn deduct_sponsored_fee(
        pool: &mut SponsorPool,
        fee_token: [u8; 32],
        fee_amount: u128,
    ) -> Result<u128, &'static str> {
        if !pool.is_active {
            return Err("Pool is not active");
        }

        // Find token in pool
        let token_index = pool
            .tokens
            .iter()
            .position(|&t| t == fee_token)
            .ok_or("Token not in pool")?;

        if pool.balances[token_index] < fee_amount {
            return Err("Insufficient pool balance");
        }

        pool.balances[token_index] = pool.balances[token_index].saturating_sub(fee_amount);

        Ok(pool.balances[token_index])
    }

    /// Batch settle multiple fee requests for net settlement
    pub fn batch_settle_fees(
        fee_requests: &[FeeRequest],
        exchange_rates: &[TokenExchangeRate],
    ) -> Result<(u128, u128), &'static str> {
        if fee_requests.is_empty() {
            return Err("No fee requests to settle");
        }

        let mut total_collected = 0u128;
        let mut total_refunded = 0u128;

        for fee_req in fee_requests {
            if fee_req.status != FeeRequestStatus::Settled {
                continue;
            }

            // Find exchange rate for this fee token
            if let Some(rate) = exchange_rates.iter().find(|r| r.token == fee_req.fee_token) {
                let native_equiv =
                    (fee_req.fee_amount as f64) * (rate.native_per_token as f64) / 1e18;
                total_collected = total_collected.saturating_add(native_equiv as u128);
            }
        }

        Ok((total_collected, total_refunded))
    }

    /// Add tokens to relayer's accepted list
    pub fn add_accepted_token(
        config: &mut RelayerConfig,
        token: [u8; 32],
    ) -> Result<(), &'static str> {
        if config.accepted_tokens.contains(&token) {
            return Err("Token already accepted");
        }

        config.accepted_tokens.push(token);
        Ok(())
    }

    /// Remove token from relayer's accepted list
    pub fn remove_accepted_token(
        config: &mut RelayerConfig,
        token: [u8; 32],
    ) -> Result<(), &'static str> {
        if !config.accepted_tokens.contains(&token) {
            return Err("Token not in list");
        }

        config.accepted_tokens.retain(|&t| t != token);
        Ok(())
    }

    /// Pause relayer operations
    pub fn pause_relayer(config: &mut RelayerConfig) -> Result<(), &'static str> {
        config.is_active = false;
        Ok(())
    }

    /// Resume relayer operations
    pub fn resume_relayer(config: &mut RelayerConfig) -> Result<(), &'static str> {
        config.is_active = true;
        Ok(())
    }

    /// Dispute fee settlement (fraud/slippage claim)
    pub fn dispute_fee(fee_req: &mut FeeRequest, reason: &str) -> Result<(), &'static str> {
        if fee_req.status == FeeRequestStatus::Disputed
            || fee_req.status == FeeRequestStatus::Refunded
        {
            return Err("Fee already disputed or refunded");
        }

        fee_req.status = FeeRequestStatus::Disputed;
        Ok(())
    }

    /// Generate deterministic request ID
    fn generate_request_id(payer: &[u8; 32], token: &[u8; 32], amount: u128) -> [u8; 32] {
        let mut id = [0u8; 32];
        let mut hash = 0u64;

        for byte in payer {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        for byte in token {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        hash = hash.wrapping_mul(31).wrapping_add((amount >> 64) as u64);

        id[0..8].copy_from_slice(&hash.to_le_bytes());
        id
    }

    /// Generate deterministic pool ID
    fn generate_pool_id(sponsor: &[u8; 32], tokens: &[[u8; 32]]) -> [u8; 32] {
        let mut id = [0u8; 32];
        let mut hash = 0u64;

        for byte in sponsor {
            hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
        }
        for token in tokens {
            for byte in token {
                hash = hash.wrapping_mul(31).wrapping_add(*byte as u64);
            }
        }

        id[0..8].copy_from_slice(&hash.to_le_bytes());
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_relayer() {
        let relayer_id = [1; 32];
        let tokens = vec![[2; 32], [3; 32]];

        let config = GasRelayer::register_relayer(relayer_id, tokens.clone(), 500).unwrap();

        assert_eq!(config.relayer_id, relayer_id);
        assert_eq!(config.fee_share_bps, 500);
        assert!(config.is_active);
    }

    #[test]
    fn test_register_relayer_fee_exceeds_max() {
        let result = GasRelayer::register_relayer([1; 32], vec![[2; 32]], 2000);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_fee_request() {
        let req =
            GasRelayer::create_fee_request([1; 32], [2; 32], 1000000, 950000, [3; 32]).unwrap();

        assert_eq!(req.payer, [1; 32]);
        assert_eq!(req.fee_token, [2; 32]);
        assert_eq!(req.status, FeeRequestStatus::Pending);
    }

    #[test]
    fn test_create_fee_request_zero_amount() {
        let result = GasRelayer::create_fee_request([1; 32], [2; 32], 0, 100, [3; 32]);
        assert!(result.is_err());
    }

    #[test]
    fn test_settle_fee() {
        let mut req =
            GasRelayer::create_fee_request([1; 32], [2; 32], 1000000, 1000000, [3; 32]).unwrap();

        let rate = TokenExchangeRate {
            token: [2; 32],
            native_per_token: 1e18 as u128,
            updated_at: 1000,
        };

        let result = GasRelayer::settle_fee(&mut req, &rate, 100).unwrap();
        assert_eq!(req.status, FeeRequestStatus::Settled);
    }

    #[test]
    fn test_update_exchange_rate() {
        let rate = GasRelayer::update_exchange_rate([1; 32], 500000000000000000, 1000).unwrap();

        assert_eq!(rate.token, [1; 32]);
        assert_eq!(rate.native_per_token, 500000000000000000);
    }

    #[test]
    fn test_process_relayer_payout() {
        let mut req =
            GasRelayer::create_fee_request([1; 32], [2; 32], 1000000, 1000000, [3; 32]).unwrap();
        req.status = FeeRequestStatus::Settled;

        let config = GasRelayer::register_relayer([3; 32], vec![[2; 32]], 500).unwrap();

        let (relayer_share, protocol_share) =
            GasRelayer::process_relayer_payout(&req, &config).unwrap();

        assert_eq!(relayer_share, 50000); // 5% of 1000000
        assert_eq!(protocol_share, 950000);
    }

    #[test]
    fn test_create_sponsor_pool() {
        let pool = GasRelayer::create_sponsor_pool(
            [1; 32],
            vec![[2; 32]],
            vec![10000000],
            vec![[3; 32], [4; 32]],
        )
        .unwrap();

        assert_eq!(pool.sponsor, [1; 32]);
        assert!(pool.is_active);
    }

    #[test]
    fn test_is_sponsored_beneficiary() {
        let pool =
            GasRelayer::create_sponsor_pool([1; 32], vec![[2; 32]], vec![10000000], vec![[3; 32]])
                .unwrap();

        assert!(GasRelayer::is_sponsored_beneficiary(&pool, &[3; 32]).unwrap());
        assert!(!GasRelayer::is_sponsored_beneficiary(&pool, &[4; 32]).unwrap());
    }

    #[test]
    fn test_deduct_sponsored_fee() {
        let mut pool =
            GasRelayer::create_sponsor_pool([1; 32], vec![[2; 32]], vec![10000000], vec![[3; 32]])
                .unwrap();

        GasRelayer::deduct_sponsored_fee(&mut pool, [2; 32], 1000000).unwrap();
        assert_eq!(pool.balances[0], 9000000);
    }

    #[test]
    fn test_add_accepted_token() {
        let mut config = GasRelayer::register_relayer([1; 32], vec![[2; 32]], 500).unwrap();

        GasRelayer::add_accepted_token(&mut config, [3; 32]).unwrap();
        assert_eq!(config.accepted_tokens.len(), 2);
    }

    #[test]
    fn test_pause_resume_relayer() {
        let mut config = GasRelayer::register_relayer([1; 32], vec![[2; 32]], 500).unwrap();

        assert!(config.is_active);
        GasRelayer::pause_relayer(&mut config).unwrap();
        assert!(!config.is_active);

        GasRelayer::resume_relayer(&mut config).unwrap();
        assert!(config.is_active);
    }

    #[test]
    fn test_dispute_fee() {
        let mut req =
            GasRelayer::create_fee_request([1; 32], [2; 32], 1000000, 950000, [3; 32]).unwrap();

        GasRelayer::dispute_fee(&mut req, "slippage").unwrap();
        assert_eq!(req.status, FeeRequestStatus::Disputed);
    }

    #[test]
    fn test_batch_settle_fees() {
        let mut req =
            GasRelayer::create_fee_request([1; 32], [2; 32], 1000000, 1000000, [3; 32]).unwrap();

        let rate = TokenExchangeRate {
            token: [2; 32],
            native_per_token: 1e18 as u128,
            updated_at: 1000,
        };

        // settle the request first so batch_settle_fees can aggregate it
        GasRelayer::settle_fee(&mut req, &rate, 100).unwrap();

        let result = GasRelayer::batch_settle_fees(&[req], &[rate]).unwrap();
        assert!(result.0 > 0);
    }

    #[test]
    fn test_remove_accepted_token() {
        let mut config =
            GasRelayer::register_relayer([1; 32], vec![[2; 32], [3; 32]], 500).unwrap();

        GasRelayer::remove_accepted_token(&mut config, [2; 32]).unwrap();
        assert_eq!(config.accepted_tokens.len(), 1);
    }
}
