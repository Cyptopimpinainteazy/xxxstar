//! Real quoting: where a price comes from, and what makes it executable.
//!
//! This module replaces a quote engine that returned the caller's own
//! `min_amount_out` as the `estimated_output`. That is not a quote: it echoed the
//! request and made every swap look executable at exactly the requested floor.
//!
//! The rules here:
//!
//! * **A quote comes from a source that holds liquidity**, priced with the
//!   canonical constant-product math (`x3bc_core::amm`), never from the request.
//! * **A source that cannot answer says so.** No synthetic fallback, no
//!   "estimated" figure presented as executable.
//! * **Every quote carries its provider, its observation time and its expiry**, so
//!   a stale price can be rejected rather than used.
//! * **Executable means backed by reserves.** It does not mean a transaction has
//!   been built or submitted — that is a lifecycle this crate does not implement,
//!   and `AtomicSwapExecutor` refuses rather than claiming otherwise.

use crate::routing::SwapRoute;
use crate::{SwapParams, SwapRouterError, VmType};
use sp_core::{H160, U256};

/// Why a liquidity source could not answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuoteError {
    /// The source does not trade this pair.
    UnsupportedPair,
    /// The source has no reserves for this pair.
    NoLiquidity,
    /// The amount does not fit the source's arithmetic.
    AmountOutOfRange,
    /// The source's observation is older than the caller allows.
    Stale {
        observed_at: u64,
        now: u64,
        max_age: u64,
    },
    /// The source's answer would leave the pool outside its own invariant.
    InsufficientLiquidity,
}

/// One source's answer for an exact-input swap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceQuote {
    /// Who answered.
    pub provider: String,
    /// The chain the liquidity is on.
    pub chain_id: u64,
    /// Output the source would pay, before any router-level cost.
    pub amount_out: U256,
    /// The source's own fee, already taken from the input.
    pub fee: U256,
    /// Price impact in basis points, measured against the no-fee output.
    pub price_impact_bps: u32,
    /// When the reserves behind this quote were observed.
    pub observed_at: u64,
    /// The last moment the caller may act on it.
    pub expires_at: u64,
}

/// A venue that can price an exact-input swap.
///
/// Implementations must answer from real reserves. A source that has no reserves
/// returns `NoLiquidity`; it must never estimate.
pub trait LiquiditySource {
    /// Human-readable name, recorded on every quote so a receipt can attribute it.
    fn name(&self) -> &str;

    /// Chain this source settles on.
    fn chain_id(&self) -> u64;

    /// VM family, so a caller can tell a native route from a foreign one.
    fn vm(&self) -> VmType;

    /// Price `amount_in` of `token_in` into `token_out`.
    fn quote_exact_in(
        &self,
        token_in: H160,
        token_out: H160,
        amount_in: U256,
        now: u64,
    ) -> Result<SourceQuote, QuoteError>;
}

/// A constant-product pool registered with the router.
///
/// The reserves are a snapshot taken at `observed_at`; `max_age` is how long the
/// router will keep honouring that snapshot. Both are explicit because a quote is
/// only as good as the reserve position behind it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativePool {
    pub name: String,
    pub chain_id: u64,
    pub token_a: H160,
    pub token_b: H160,
    pub reserve_a: u128,
    pub reserve_b: u128,
    pub fee_bps: u16,
    /// Block or timestamp at which these reserves were read.
    pub observed_at: u64,
    /// How long this snapshot may be quoted from.
    pub max_age: u64,
}

impl NativePool {
    /// Reserves for a directed pair, or `None` when the pool does not trade it.
    fn oriented_reserves(&self, token_in: H160, token_out: H160) -> Option<(u128, u128)> {
        if token_in == self.token_a && token_out == self.token_b {
            Some((self.reserve_a, self.reserve_b))
        } else if token_in == self.token_b && token_out == self.token_a {
            Some((self.reserve_b, self.reserve_a))
        } else {
            None
        }
    }
}

impl LiquiditySource for NativePool {
    fn name(&self) -> &str {
        &self.name
    }

    fn chain_id(&self) -> u64 {
        self.chain_id
    }

    fn vm(&self) -> VmType {
        VmType::X3Vm
    }

    fn quote_exact_in(
        &self,
        token_in: H160,
        token_out: H160,
        amount_in: U256,
        now: u64,
    ) -> Result<SourceQuote, QuoteError> {
        let (reserve_in, reserve_out) = self
            .oriented_reserves(token_in, token_out)
            .ok_or(QuoteError::UnsupportedPair)?;
        if reserve_in == 0 || reserve_out == 0 {
            return Err(QuoteError::NoLiquidity);
        }
        if now.saturating_sub(self.observed_at) > self.max_age {
            return Err(QuoteError::Stale {
                observed_at: self.observed_at,
                now,
                max_age: self.max_age,
            });
        }

        let amount = u128::try_from(amount_in).map_err(|_| QuoteError::AmountOutOfRange)?;
        let priced = x3bc_core::amm::swap(amount, reserve_in, reserve_out, self.fee_bps)
            .map_err(|_| QuoteError::InsufficientLiquidity)?;

        // Impact is the gap between what the *net* input would buy at the pool's
        // spot price and what it actually buys. Measuring against the no-fee
        // output of the *gross* input would fold the fee into the number and
        // shrink as the trade grew, which is the opposite of an impact.
        let spot_out = x3bc_core::amm::mul_div(priced.net_in, reserve_out, reserve_in)
            .ok_or(QuoteError::AmountOutOfRange)?;
        let impact_bps = x3bc_core::amm::shortfall_bps(spot_out, priced.amount_out)
            .and_then(|bps| u32::try_from(bps).ok())
            .unwrap_or(u32::MAX);

        Ok(SourceQuote {
            provider: self.name.clone(),
            chain_id: self.chain_id,
            amount_out: U256::from(priced.amount_out),
            fee: U256::from(priced.fee),
            price_impact_bps: impact_bps,
            observed_at: self.observed_at,
            expires_at: self.observed_at.saturating_add(self.max_age),
        })
    }
}

/// A route through two or more pools, priced hop by hop.
///
/// The single-pool case is [`NativePool`]; this is what makes the router a
/// *router*: a direct pool may pay less than a path through an intermediate
/// asset, and only pricing both lets a caller see that. Each hop consumes the
/// previous hop's output, so the reported output is what the whole path would
/// actually deliver, not the sum of independent quotes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MultiHopPath {
    /// Human-readable path name, recorded on the quote.
    pub name: String,
    /// Pools in execution order. Each must trade the asset the previous produced.
    pub pools: Vec<NativePool>,
}

impl MultiHopPath {
    /// Build a path, refusing one whose pools do not chain.
    ///
    /// Checking the chain here rather than at quote time means a misconfigured
    /// route cannot sit in the engine looking valid until someone uses it.
    pub fn new(name: impl Into<String>, pools: Vec<NativePool>) -> Result<Self, QuoteError> {
        if pools.is_empty() {
            return Err(QuoteError::UnsupportedPair);
        }
        for pair in pools.windows(2) {
            // Which side is the input depends on the direction the path is
            // traversed, so the only structural requirement here is that
            // consecutive pools can hand value to each other at all.
            let (left, right) = (&pair[0], &pair[1]);
            let shares_an_asset = left.token_a == right.token_a
                || left.token_a == right.token_b
                || left.token_b == right.token_a
                || left.token_b == right.token_b;
            if !shares_an_asset {
                return Err(QuoteError::UnsupportedPair);
            }
        }
        Ok(Self {
            name: name.into(),
            pools,
        })
    }

    /// The assets this path connects, when traversed in declaration order.
    fn ends(&self) -> (H160, H160) {
        (
            self.pools[0].token_a,
            self.pools[self.pools.len() - 1].token_b,
        )
    }
}

impl LiquiditySource for MultiHopPath {
    fn name(&self) -> &str {
        &self.name
    }

    fn chain_id(&self) -> u64 {
        self.pools[0].chain_id
    }

    fn vm(&self) -> VmType {
        VmType::X3Vm
    }

    fn quote_exact_in(
        &self,
        token_in: H160,
        token_out: H160,
        amount_in: U256,
        now: u64,
    ) -> Result<SourceQuote, QuoteError> {
        let (first, last) = self.ends();
        let reverse_ends = (
            self.pools[self.pools.len() - 1].token_b,
            self.pools[0].token_a,
        );
        let forward = (first, last) == (token_in, token_out);
        // A path is usable in either direction; the pools chain either way.
        let reverse = reverse_ends == (token_in, token_out);
        if !forward && !reverse {
            return Err(QuoteError::UnsupportedPair);
        }

        let hops: Vec<&NativePool> = if forward {
            self.pools.iter().collect()
        } else {
            self.pools.iter().rev().collect()
        };

        let mut amount = amount_in;
        let mut total_fee = U256::zero();
        let mut spot_total = U256::zero();
        let mut observed_at = 0u64;
        let mut expires_at = u64::MAX;

        // The asset moving into each hop. After the first hop it is whatever the
        // previous pool paid out, which is the pool's "other" side.
        let mut moving = token_in;
        for (index, pool) in hops.iter().enumerate() {
            if now.saturating_sub(pool.observed_at) > pool.max_age {
                return Err(QuoteError::Stale {
                    observed_at: pool.observed_at,
                    now,
                    max_age: pool.max_age,
                });
            }
            let (reserve_in, reserve_out) = pool
                .oriented_reserves(
                    moving,
                    if moving == pool.token_a {
                        pool.token_b
                    } else {
                        pool.token_a
                    },
                )
                .ok_or(QuoteError::UnsupportedPair)?;
            let amount_u128 = u128::try_from(amount).map_err(|_| QuoteError::AmountOutOfRange)?;
            let priced = x3bc_core::amm::swap(amount_u128, reserve_in, reserve_out, pool.fee_bps)
                .map_err(|_| QuoteError::InsufficientLiquidity)?;

            let spot_out = x3bc_core::amm::mul_div(priced.net_in, reserve_out, reserve_in)
                .ok_or(QuoteError::AmountOutOfRange)?;

            total_fee = total_fee.saturating_add(U256::from(priced.fee));
            spot_total = spot_total.saturating_add(U256::from(spot_out));
            observed_at = observed_at.max(pool.observed_at);
            expires_at = expires_at.min(pool.observed_at.saturating_add(pool.max_age));

            amount = U256::from(priced.amount_out);
            // Next hop's input is this pool's output asset.
            moving = if moving == pool.token_a {
                pool.token_b
            } else {
                pool.token_a
            };

            if index + 1 == hops.len() && moving != token_out {
                return Err(QuoteError::UnsupportedPair);
            }
        }

        let impact_bps = x3bc_core::amm::shortfall_bps(
            u128::try_from(spot_total).unwrap_or(u128::MAX),
            u128::try_from(amount).unwrap_or(u128::MAX),
        )
        .and_then(|bps| u32::try_from(bps).ok())
        .unwrap_or(u32::MAX);

        Ok(SourceQuote {
            provider: self.name.clone(),
            chain_id: self.chain_id(),
            amount_out: amount,
            fee: total_fee,
            price_impact_bps: impact_bps,
            observed_at,
            expires_at,
        })
    }
}

/// One route the engine is willing to offer.
///
/// `PartialEq` only: a route hop carries an `f64` for legacy MEV metrics, which
/// has no `Eq`. Nothing here compares floats.
#[derive(Clone, Debug, PartialEq)]
pub struct QuoteResult {
    pub route: SwapRoute,
    /// Output the source would pay.
    pub estimated_output: U256,
    /// Network cost of executing this route, in the chain's native unit.
    pub gas_cost: U256,
    /// Which source answered.
    pub provider: String,
    /// Chain the route settles on.
    pub chain_id: u64,
    /// VM family of that chain.
    pub vm: VmType,
    /// Source fee, already inside `estimated_output`.
    pub source_fee: U256,
    /// Price impact of this route, in basis points.
    pub price_impact_bps: u32,
    /// Last moment this quote is valid.
    pub expires_at: u64,
    /// `estimated_output - gas_cost`, the figure a caller can actually compare.
    ///
    /// Saturated at zero: a route whose cost exceeds its output is worth nothing,
    /// not a negative number a caller might read as a small positive one.
    pub net_output: U256,
}

/// Prices a swap across every source that can answer.
pub struct QuoteEngine {
    sources: Vec<Box<dyn LiquiditySource>>,
    /// Network cost of one routing hop, in the chain's native unit. Supplied by
    /// the caller because this crate has no chain integration and must not
    /// invent a gas price.
    gas_cost_per_hop: U256,
}

impl QuoteEngine {
    /// Build an engine over `sources`.
    ///
    /// `gas_cost_per_hop` is required rather than defaulted: a router that
    /// guesses gas will rank a cheap route above an expensive one on a figure it
    /// made up.
    pub fn new(sources: Vec<Box<dyn LiquiditySource>>, gas_cost_per_hop: U256) -> Self {
        Self {
            sources,
            gas_cost_per_hop,
        }
    }

    /// Every executable route for `params` at time `now`.
    ///
    /// Returns `RouteNotFound` when no source can price the pair. It never
    /// returns a quote assembled from the request's own `min_amount_out`, and a
    /// quote below `min_amount_out` is filtered out rather than reported.
    pub fn get_comprehensive_quotes(
        &self,
        params: &SwapParams,
        now: u64,
    ) -> Result<Vec<QuoteResult>, SwapRouterError> {
        let mut out = Vec::new();
        for source in &self.sources {
            // A source that cannot answer — wrong pair, no reserves, stale
            // snapshot — contributes no route. Its reason is deliberately not
            // turned into a synthetic quote.
            let Ok(quote) =
                source.quote_exact_in(params.token_in, params.token_out, params.amount_in, now)
            else {
                continue;
            };
            if quote.amount_out < params.min_amount_out {
                continue;
            }
            let gas_cost = self.gas_cost_per_hop;
            let net_output = quote.amount_out.saturating_sub(gas_cost);
            out.push(QuoteResult {
                route: SwapRoute {
                    hops: vec![crate::mev_protection::Hop {
                        dex_address: params.token_out,
                        amount: quote.amount_out,
                        estimated_block: quote.expires_at,
                        slippage: 0.0,
                    }],
                    amount_in: params.amount_in,
                    estimated_output: quote.amount_out,
                    gas_estimate: gas_cost,
                },
                estimated_output: quote.amount_out,
                gas_cost,
                provider: quote.provider,
                chain_id: quote.chain_id,
                vm: source.vm(),
                source_fee: quote.fee,
                price_impact_bps: quote.price_impact_bps,
                expires_at: quote.expires_at,
                net_output,
            });
        }
        if out.is_empty() {
            return Err(SwapRouterError::RouteNotFound);
        }
        // Best net output first. A caller that takes the head gets the route that
        // actually pays most, not the one with the smallest advertised cost.
        out.sort_by(|a, b| {
            b.net_output
                .cmp(&a.net_output)
                .then_with(|| a.gas_cost.cmp(&b.gas_cost))
                .then_with(|| a.provider.cmp(&b.provider))
        });
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const X3: u64 = 1;
    const TOKEN_A: H160 = H160([0xA1; 20]);
    const TOKEN_B: H160 = H160([0xB2; 20]);
    const UNRELATED: H160 = H160([0xCC; 20]);
    const UNIT: u128 = 1_000_000;

    fn pool(name: &str, reserve_a: u128, reserve_b: u128, fee_bps: u16) -> NativePool {
        NativePool {
            name: name.into(),
            chain_id: X3,
            token_a: TOKEN_A,
            token_b: TOKEN_B,
            reserve_a,
            reserve_b,
            fee_bps,
            observed_at: 100,
            max_age: 50,
        }
    }

    fn params(amount_in: u128, min_out: u128) -> SwapParams {
        SwapParams {
            token_in: TOKEN_A,
            token_out: TOKEN_B,
            amount_in: U256::from(amount_in),
            min_amount_out: U256::from(min_out),
            chain_in: X3,
            chain_out: X3,
            deadline: 1_000,
            recipient: H160::zero(),
            slippage_tolerance_bps: 50,
            gas_price_limit: None,
            source_vm: VmType::X3Vm,
            destination_vm: VmType::X3Vm,
        }
    }

    fn engine(sources: Vec<NativePool>, gas: u128) -> QuoteEngine {
        QuoteEngine::new(
            sources
                .into_iter()
                .map(|p| Box::new(p) as Box<dyn LiquiditySource>)
                .collect(),
            U256::from(gas),
        )
    }

    #[test]
    fn a_quote_is_priced_from_the_pool_not_echoed_from_the_request() {
        // The requested minimum is absurd on purpose: 999 whole units out of a
        // 1_000-in pool. The old engine returned exactly this as its estimate.
        let engine = engine(
            vec![pool("x3dex_pool_1", 1_000 * UNIT, 2_000 * UNIT, 30)],
            0,
        );
        let quotes = engine
            .get_comprehensive_quotes(&params(10 * UNIT, 0), 120)
            .expect("a real pool prices this");

        assert_eq!(quotes.len(), 1);
        assert_eq!(quotes[0].provider, "x3dex_pool_1");

        // The figure is the AMM's, not the caller's.
        let expected = x3bc_core::amm::swap(10 * UNIT, 1_000 * UNIT, 2_000 * UNIT, 30).unwrap();
        assert_eq!(quotes[0].estimated_output, U256::from(expected.amount_out));
        assert_ne!(quotes[0].estimated_output, U256::from(999 * UNIT));
    }

    #[test]
    fn a_quote_below_the_minimum_is_filtered_not_reported() {
        let engine = engine(
            vec![pool("x3dex_pool_1", 1_000 * UNIT, 2_000 * UNIT, 30)],
            0,
        );
        // Demand more out than the pool can possibly pay.
        let err = engine
            .get_comprehensive_quotes(&params(10 * UNIT, 1_900 * UNIT), 120)
            .expect_err("nothing clears that floor");
        assert_eq!(err, SwapRouterError::RouteNotFound);
    }

    #[test]
    fn a_pool_that_does_not_trade_the_pair_contributes_nothing() {
        let engine = engine(vec![pool("other", 1_000 * UNIT, 2_000 * UNIT, 30)], 0);
        let mut p = params(10 * UNIT, 0);
        p.token_out = UNRELATED;
        assert_eq!(
            engine.get_comprehensive_quotes(&p, 120),
            Err(SwapRouterError::RouteNotFound)
        );
    }

    #[test]
    fn an_empty_pool_is_no_liquidity_not_a_zero_output_quote() {
        let source = pool("empty", 0, 2_000 * UNIT, 30);
        assert_eq!(
            source.quote_exact_in(TOKEN_A, TOKEN_B, U256::from(UNIT), 120),
            Err(QuoteError::NoLiquidity)
        );
    }

    #[test]
    fn a_stale_snapshot_is_refused() {
        let source = pool("stale", 1_000 * UNIT, 2_000 * UNIT, 30);
        // observed_at 100, max_age 50: 151 is one second too late.
        assert_eq!(
            source.quote_exact_in(TOKEN_A, TOKEN_B, U256::from(UNIT), 151),
            Err(QuoteError::Stale {
                observed_at: 100,
                now: 151,
                max_age: 50
            })
        );
        assert!(source
            .quote_exact_in(TOKEN_A, TOKEN_B, U256::from(UNIT), 150)
            .is_ok());
    }

    #[test]
    fn the_best_net_output_wins_over_a_lower_advertised_cost() {
        // A deep, cheap pool beats a shallow one, whatever gas each claims.
        let deep = pool("deep", 1_000_000 * UNIT, 1_000_000 * UNIT, 5);
        let shallow = pool("shallow", 1_000 * UNIT, 1_000 * UNIT, 30);
        let engine = engine(vec![deep, shallow], 0);
        let quotes = engine
            .get_comprehensive_quotes(&params(100 * UNIT, 0), 120)
            .unwrap();
        assert!(quotes.len() >= 2);
        assert_eq!(quotes[0].provider, "deep");
        assert!(quotes[0].net_output > quotes[1].net_output);
    }

    #[test]
    fn gas_is_subtracted_from_the_net_the_caller_compares() {
        let engine = engine(
            vec![pool("p", 1_000_000 * UNIT, 1_000_000 * UNIT, 30)],
            1_000,
        );
        let quotes = engine
            .get_comprehensive_quotes(&params(10 * UNIT, 0), 120)
            .unwrap();
        assert_eq!(
            quotes[0].net_output,
            quotes[0].estimated_output - U256::from(1_000u64)
        );
    }

    #[test]
    fn every_offered_route_is_ranked_by_all_in_cost() {
        let deep = pool("deep", 5_000_000 * UNIT, 5_000_000 * UNIT, 30);
        let shallow = pool("shallow", 1_000 * UNIT, 1_000 * UNIT, 30);
        let engine = engine(vec![shallow, deep], 500);
        let quotes = engine
            .get_comprehensive_quotes(&params(50 * UNIT, 0), 120)
            .unwrap();
        assert_eq!(quotes[0].provider, "deep");
        for pair in quotes.windows(2) {
            assert!(pair[0].net_output >= pair[1].net_output);
        }
    }

    #[test]
    fn an_amount_beyond_u128_is_refused_rather_than_truncated() {
        let source = pool("p", 1_000 * UNIT, 2_000 * UNIT, 30);
        let huge = U256::from(u128::MAX) + U256::from(1u64);
        assert_eq!(
            source.quote_exact_in(TOKEN_A, TOKEN_B, huge, 120),
            Err(QuoteError::AmountOutOfRange)
        );
    }

    #[test]
    fn price_impact_is_reported_and_grows_with_size() {
        let source = pool("p", 100 * UNIT, 100 * UNIT, 30);
        let small = source
            .quote_exact_in(TOKEN_A, TOKEN_B, U256::from(UNIT), 120)
            .unwrap();
        let large = source
            .quote_exact_in(TOKEN_A, TOKEN_B, U256::from(50 * UNIT), 120)
            .unwrap();
        assert!(
            large.price_impact_bps > small.price_impact_bps,
            "a bigger trade moves the price more: {} vs {}",
            large.price_impact_bps,
            small.price_impact_bps
        );
        assert!(small.price_impact_bps > 0, "any trade moves the price");
    }

    #[test]
    fn a_reversed_pair_prices_from_the_reversed_reserves() {
        let source = pool("p", 1_000 * UNIT, 2_000 * UNIT, 30);
        let forward = source
            .quote_exact_in(TOKEN_A, TOKEN_B, U256::from(10 * UNIT), 120)
            .unwrap();
        let reverse = source
            .quote_exact_in(TOKEN_B, TOKEN_A, U256::from(10 * UNIT), 120)
            .unwrap();
        // 10k A into a 1000k/2000k pool pays ~19.8k B; the reverse pays ~4.9k A.
        assert!(forward.amount_out > reverse.amount_out);
    }
}

#[cfg(test)]
mod multi_hop_tests {
    use super::*;

    const X3: u64 = 1;
    const A: H160 = H160([0xA1; 20]);
    const B: H160 = H160([0xB2; 20]);
    const C: H160 = H160([0xC3; 20]);
    const UNIT: u128 = 1_000_000;

    fn pool(name: &str, a: H160, b: H160, ra: u128, rb: u128, fee: u16) -> NativePool {
        NativePool {
            name: name.into(),
            chain_id: X3,
            token_a: a,
            token_b: b,
            reserve_a: ra,
            reserve_b: rb,
            fee_bps: fee,
            observed_at: 100,
            max_age: 50,
        }
    }

    fn params(token_in: H160, token_out: H160, amount: u128) -> SwapParams {
        SwapParams {
            token_in,
            token_out,
            amount_in: U256::from(amount),
            min_amount_out: U256::zero(),
            chain_in: X3,
            chain_out: X3,
            deadline: 1_000,
            recipient: H160::zero(),
            slippage_tolerance_bps: 50,
            gas_price_limit: None,
            source_vm: VmType::X3Vm,
            destination_vm: VmType::X3Vm,
        }
    }

    fn engine(sources: Vec<Box<dyn LiquiditySource>>, gas: u128) -> QuoteEngine {
        QuoteEngine::new(sources, U256::from(gas))
    }

    #[test]
    fn a_two_hop_path_prices_each_leg_against_the_previous_output() {
        let a_to_b = pool("a/b", A, B, 1_000 * UNIT, 1_000 * UNIT, 30);
        let b_to_c = pool("b/c", B, C, 1_000 * UNIT, 1_000 * UNIT, 30);
        let path = MultiHopPath::new("A->B->C", vec![a_to_b.clone(), b_to_c.clone()]).unwrap();

        let quoted = path
            .quote_exact_in(A, C, U256::from(10 * UNIT), 120)
            .expect("the path prices");

        // Hand-compute: hop 1 pays B, hop 2 consumes exactly that B.
        let leg1 = x3bc_core::amm::swap(10 * UNIT, 1_000 * UNIT, 1_000 * UNIT, 30).unwrap();
        let leg2 = x3bc_core::amm::swap(leg1.amount_out, 1_000 * UNIT, 1_000 * UNIT, 30).unwrap();
        assert_eq!(quoted.amount_out, U256::from(leg2.amount_out));
        assert_eq!(quoted.fee, U256::from(leg1.fee + leg2.fee));
    }

    #[test]
    fn a_path_whose_pools_share_no_asset_is_refused_at_construction() {
        let ab = pool("a/b", A, B, 1_000 * UNIT, 1_000 * UNIT, 30);
        let disconnected = pool(
            "x/y",
            H160([0x11; 20]),
            H160([0x22; 20]),
            1_000 * UNIT,
            1_000 * UNIT,
            30,
        );
        assert_eq!(
            MultiHopPath::new("bad", vec![ab.clone(), disconnected]).unwrap_err(),
            QuoteError::UnsupportedPair
        );

        // A pool that shares an asset is accepted, in either orientation.
        let bc = pool("b/c", B, C, 1_000 * UNIT, 1_000 * UNIT, 30);
        assert!(MultiHopPath::new("a->b->c", vec![ab, bc]).is_ok());
    }

    #[test]
    fn a_multi_hop_route_can_beat_a_thin_direct_pool() {
        // Direct A->C pool is thin: 10k in moves the price hard.
        let direct = pool("direct a/c", A, C, 20_000 * UNIT, 20_000 * UNIT, 30);
        // Two deep pools through B lose less to impact despite paying two fees.
        let a_to_b = pool("a/b", A, B, 1_000_000 * UNIT, 1_000_000 * UNIT, 30);
        let b_to_c = pool("b/c", B, C, 1_000_000 * UNIT, 1_000_000 * UNIT, 30);
        let path = MultiHopPath::new("A->B->C", vec![a_to_b, b_to_c]).unwrap();

        let engine = engine(vec![Box::new(direct), Box::new(path)], 0);
        let quotes = engine
            .get_comprehensive_quotes(&params(A, C, 10_000 * UNIT), 120)
            .unwrap();

        assert_eq!(quotes.len(), 2, "both routes are priced");
        assert_eq!(
            quotes[0].provider, "A->B->C",
            "the two-hop path out-pays the thin direct pool"
        );
        assert!(quotes[0].net_output > quotes[1].net_output);
    }

    #[test]
    fn a_direct_route_wins_when_the_path_pays_two_fees_for_nothing() {
        // Direct pool is deep; the path adds a fee without adding depth.
        let direct = pool("direct a/c", A, C, 1_000_000 * UNIT, 1_000_000 * UNIT, 5);
        let a_to_b = pool("a/b", A, B, 1_000_000 * UNIT, 1_000_000 * UNIT, 30);
        let b_to_c = pool("b/c", B, C, 1_000_000 * UNIT, 1_000_000 * UNIT, 30);
        let path = MultiHopPath::new("A->B->C", vec![a_to_b, b_to_c]).unwrap();

        let engine = engine(vec![Box::new(direct), Box::new(path)], 0);
        let quotes = engine
            .get_comprehensive_quotes(&params(A, C, 100 * UNIT), 120)
            .unwrap();
        assert_eq!(quotes[0].provider, "direct a/c");
    }

    #[test]
    fn a_path_is_quotable_in_reverse() {
        // Deliberately not mirror images: had they been, the two directions would
        // pay identically and the test would prove nothing.
        let a_to_b = pool("a/b", A, B, 1_000 * UNIT, 3_000 * UNIT, 30);
        let b_to_c = pool("b/c", B, C, 2_500 * UNIT, 800 * UNIT, 30);
        let path = MultiHopPath::new("A->B->C", vec![a_to_b, b_to_c]).unwrap();

        let forward = path
            .quote_exact_in(A, C, U256::from(10 * UNIT), 120)
            .unwrap();
        let reverse = path
            .quote_exact_in(C, A, U256::from(10 * UNIT), 120)
            .expect("the path is traversable in reverse");
        // Both directions price, and they are not the same number.
        assert_ne!(
            forward.amount_out, reverse.amount_out,
            "forward {} vs reverse {}",
            forward.amount_out, reverse.amount_out
        );
        assert!(forward.amount_out > U256::zero());
        assert!(reverse.amount_out > U256::zero());
    }

    #[test]
    fn a_path_over_an_unrelated_pair_is_refused() {
        let a_to_b = pool("a/b", A, B, 1_000 * UNIT, 1_000 * UNIT, 30);
        let b_to_c = pool("b/c", B, C, 1_000 * UNIT, 1_000 * UNIT, 30);
        let path = MultiHopPath::new("A->B->C", vec![a_to_b, b_to_c]).unwrap();
        assert_eq!(
            path.quote_exact_in(H160([0x99; 20]), C, U256::from(UNIT), 120),
            Err(QuoteError::UnsupportedPair)
        );
    }

    #[test]
    fn one_stale_hop_makes_the_whole_path_stale() {
        let a_to_b = pool("a/b", A, B, 1_000 * UNIT, 1_000 * UNIT, 30);
        let mut b_to_c = pool("b/c", B, C, 1_000 * UNIT, 1_000 * UNIT, 30);
        b_to_c.observed_at = 10; // observed long before the fresh pool
        let path = MultiHopPath::new("A->B->C", vec![a_to_b, b_to_c]).unwrap();
        assert!(matches!(
            path.quote_exact_in(A, C, U256::from(UNIT), 120),
            Err(QuoteError::Stale { .. })
        ));
    }

    #[test]
    fn an_empty_pool_in_the_middle_is_refused_not_skipped() {
        let a_to_b = pool("a/b", A, B, 1_000 * UNIT, 1_000 * UNIT, 30);
        let empty = pool("empty", B, C, 0, 1_000 * UNIT, 30);
        let path = MultiHopPath::new("A->B->C", vec![a_to_b, empty]).unwrap();
        assert!(matches!(
            path.quote_exact_in(A, C, U256::from(UNIT), 120),
            Err(QuoteError::InsufficientLiquidity) | Err(QuoteError::UnsupportedPair)
        ));
    }

    #[test]
    fn a_path_reports_the_worst_expiry_of_its_hops() {
        let mut a_to_b = pool("a/b", A, B, 1_000 * UNIT, 1_000 * UNIT, 30);
        a_to_b.observed_at = 100;
        a_to_b.max_age = 50;
        let mut b_to_c = pool("b/c", B, C, 1_000 * UNIT, 1_000 * UNIT, 30);
        b_to_c.observed_at = 90;
        b_to_c.max_age = 20; // expires at 110
        let path = MultiHopPath::new("A->B->C", vec![a_to_b, b_to_c]).unwrap();

        let quoted = path
            .quote_exact_in(A, C, U256::from(UNIT), 105)
            .expect("both hops are fresh at 105");
        assert_eq!(quoted.expires_at, 110, "the earliest hop's expiry governs");
    }
}
