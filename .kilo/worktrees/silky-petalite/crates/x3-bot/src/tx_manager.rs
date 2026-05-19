use anyhow::{anyhow, Result};
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use tracing::{info, warn};

pub struct TxManager<M> {
    client: Arc<M>,
    chain_id: u64,
}

impl<M: Middleware + 'static> TxManager<M> {
    pub fn new(client: Arc<M>, chain_id: u64) -> Self {
        Self { client, chain_id }
    }

    pub async fn build_signed_tx(
        &self,
        wallet: &LocalWallet,
        to: Address,
        data: Bytes,
        gas_limit: U256,
    ) -> Result<Bytes> {
        let from = wallet.address();
        let nonce = self.client.get_transaction_count(from, None).await?;
        let gas_price = self.client.get_gas_price().await?;

        let tx = TransactionRequest::new()
            .to(to)
            .from(from)
            .data(data)
            .gas(gas_limit)
            .gas_price(gas_price)
            .nonce(nonce)
            .chain_id(self.chain_id);

        let typed_tx: TypedTransaction = tx.into();
        let signature = wallet.sign_transaction(&typed_tx).await?;

        Ok(typed_tx.rlp_signed(&signature))
    }

    pub async fn execute_arb(&self, signed_tx: Bytes) -> Result<TxHash> {
        let pending_tx = self.client.send_raw_transaction(signed_tx).await?;
        info!(
            "Arbitrage transaction submitted: {:?}",
            pending_tx.tx_hash()
        );

        let receipt = pending_tx
            .await?
            .ok_or_else(|| anyhow!("Transaction dropped or failed to mine"))?;

        if receipt.status == Some(U64::from(1)) {
            info!("✅ Arbitrage SUCCESS: {:?}", receipt.transaction_hash);
        } else {
            warn!("❌ Arbitrage REVERTED: {:?}", receipt.transaction_hash);
        }

        Ok(receipt.transaction_hash)
    }
}
