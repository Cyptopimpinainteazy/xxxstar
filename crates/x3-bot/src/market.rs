use anyhow::{anyhow, Result};
use ethers::prelude::*;
use std::sync::Arc;
use tracing::debug;

// Uniswap V2 Pair Interface for on-chain calls
abigen!(
    IUniswapV2Pair,
    r#"[
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        function token0() external view returns (address)
        function token1() external view returns (address)
    ]"#
);

pub struct MarketScanner<M> {
    client: Arc<M>,
}

impl<M: Middleware + 'static> MarketScanner<M> {
    pub fn new(client: Arc<M>) -> Self {
        Self { client }
    }

    pub async fn get_pair_reserves(&self, pair_address: Address) -> Result<(U256, U256)> {
        let pair = IUniswapV2Pair::new(pair_address, self.client.clone());
        let (reserve0, reserve1, _) = pair
            .get_reserves()
            .call()
            .await
            .map_err(|e| anyhow!("Failed to fetch reserves for {}: {}", pair_address, e))?;

        debug!("Reserves for {}: {} / {}", pair_address, reserve0, reserve1);
        Ok((U256::from(reserve0), U256::from(reserve1)))
    }

    pub async fn calculate_price(&self, pair_address: Address) -> Result<U256> {
        let (r0, r1) = self.get_pair_reserves(pair_address).await?;
        if r0.is_zero() {
            return Err(anyhow!("Liquidity is zero"));
        }
        // Price of token0 in terms of token1 (scaled by 1e18 for precision)
        Ok((r1 * U256::exp10(18)) / r0)
    }
}
