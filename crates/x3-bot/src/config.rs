use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct BotConfig {
    pub rpc_evm: Vec<String>,
    pub rpc_svm: Vec<String>,
    pub wallet_key_evm: String,
    pub wallet_key_svm: String,
    pub evm_chain_id: u64,
    pub arb_threshold_bps: u64,
    pub slippage_pct: f64,
    pub evm_router: String,
    pub svm_program_id: String,
    pub flashbots_relay: Option<String>,
    pub prometheus_port: u16,
}

impl BotConfig {
    pub fn from_env() -> Result<Self> {
        let rpc_evm = env::var("RPC_EVM")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let rpc_svm = env::var("RPC_SVM")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let config = BotConfig {
            rpc_evm,
            rpc_svm,
            wallet_key_evm: env::var("WALLET_KEY_EVM").unwrap_or_default(),
            wallet_key_svm: env::var("WALLET_KEY_SVM").unwrap_or_default(),
            evm_chain_id: env::var("EVM_CHAIN_ID")
                .unwrap_or_else(|_| "1".to_string())
                .parse()?,
            arb_threshold_bps: env::var("ARB_THRESHOLD_BPS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()?,
            slippage_pct: env::var("SLIPPAGE_PCT")
                .unwrap_or_else(|_| "0.5".to_string())
                .parse()?,
            evm_router: env::var("EVM_ROUTER").unwrap_or_default(),
            svm_program_id: env::var("SVM_PROGRAM_ID").unwrap_or_default(),
            flashbots_relay: env::var("FLASHBOTS_RELAY").ok(),
            prometheus_port: env::var("PROMETHEUS_PORT")
                .unwrap_or_else(|_| "9090".to_string())
                .parse()?,
        };

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.rpc_evm.is_empty() {
            return Err(anyhow!("RPC_EVM cannot be empty"));
        }
        if self.rpc_svm.is_empty() {
            return Err(anyhow!("RPC_SVM cannot be empty"));
        }
        if self.wallet_key_evm.is_empty() {
            return Err(anyhow!("WALLET_KEY_EVM is missing"));
        }
        if self.wallet_key_svm.is_empty() {
            return Err(anyhow!("WALLET_KEY_SVM is missing"));
        }
        Ok(())
    }
}
