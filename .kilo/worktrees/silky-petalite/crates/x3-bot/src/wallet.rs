use anyhow::{anyhow, Result};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::Address;
use std::str::FromStr;

pub struct BotWallet {
    pub wallet: LocalWallet,
    pub address: Address,
}

impl BotWallet {
    pub fn new(key: &str, chain_id: u64) -> Result<Self> {
        let wallet = LocalWallet::from_str(key)?.with_chain_id(chain_id);
        let address = wallet.address();
        Ok(Self { wallet, address })
    }

    pub async fn verify_signing(&self) -> Result<()> {
        let msg = "☢️ X3 Bot Signing Test";
        let sig = self.wallet.sign_message(msg).await?;
        sig.verify(msg, self.address)
            .map_err(|e| anyhow!("Signing verification failed: {}", e))?;
        Ok(())
    }
}
