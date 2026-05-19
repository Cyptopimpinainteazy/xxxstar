//! Test Account Management
//!
//! Provides utilities for creating and managing test accounts
//! with proper funding and permissions for E2E testing.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Test account with associated metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAccount {
    pub address: String,
    pub private_key: String,
    pub name: String,
    pub account_type: AccountType,
    pub balance: u128,
    pub permissions: Vec<String>,
}

/// Type of test account
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AccountType {
    Regular,
    Lender,
    Borrower,
    GPUMiner,
    AITrader,
    Admin,
    Contract,
}

/// Manages test accounts for E2E testing
pub struct TestAccountManager {
    accounts: HashMap<String, TestAccount>,
    network_rpc: String,
}

impl TestAccountManager {
    /// Create a new test account manager
    pub fn new(network_rpc: String) -> Self {
        Self {
            accounts: HashMap::new(),
            network_rpc,
        }
    }

    /// Initialize default test accounts
    pub async fn initialize_default_accounts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Initializing default test accounts");

        // Create standard test accounts
        self.create_test_account("test_user_1", AccountType::Regular, 1000)
            .await?;
        self.create_test_account("test_user_2", AccountType::Regular, 1000)
            .await?;

        // Create DeFi-specific accounts
        self.create_test_account("test_lender", AccountType::Lender, 5000)
            .await?;
        self.create_test_account("test_borrower", AccountType::Borrower, 2000)
            .await?;

        // Create GPU mining accounts
        self.create_test_account("test_gpu_miner_1", AccountType::GPUMiner, 1000)
            .await?;
        self.create_test_account("test_gpu_miner_2", AccountType::GPUMiner, 1000)
            .await?;

        // Create AI trading accounts
        self.create_test_account("test_ai_trader_1", AccountType::AITrader, 3000)
            .await?;
        self.create_test_account("test_ai_trader_2", AccountType::AITrader, 3000)
            .await?;

        // Create admin account
        self.create_test_account("test_admin", AccountType::Admin, 10000)
            .await?;

        Ok(())
    }

    /// Create a single test account
    pub async fn create_test_account(
        &mut self,
        name: &str,
        account_type: AccountType,
        initial_balance: u128,
    ) -> Result<&TestAccount, Box<dyn std::error::Error>> {
        info!("Creating test account: {} ({:?})", name, account_type);

        // Generate account keypair
        let (address, private_key) = self.generate_keypair();

        // Create account metadata
        let permissions = self.get_permissions_for_type(&account_type);

        let account = TestAccount {
            address: address.clone(),
            private_key: private_key.clone(),
            name: name.to_string(),
            account_type,
            balance: initial_balance,
            permissions,
        };

        // Fund the account
        self.fund_account(&address, initial_balance).await?;

        self.accounts.insert(name.to_string(), account);
        Ok(self.accounts.get(name).unwrap())
    }

    /// Generate a keypair for a test account
    fn generate_keypair(&self) -> (String, String) {
        use secp256k1::rand::thread_rng;
        use secp256k1::{KeyPair, Secp256k1};

        let secp = Secp256k1::new();
        let keypair = KeyPair::new(&secp, &mut thread_rng());

        let private_key = format!("0x{}", hex::encode(keypair.secret_key().as_ref()));
        let address = format!("0x{}", hex::encode(keypair.public_key().serialize()));

        (address, private_key)
    }

    /// Fund an account via RPC
    async fn fund_account(
        &self,
        address: &str,
        amount: u128,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();

        let response = client
            .post(&format!("{}/rpc", self.network_rpc))
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "eth_sendTransaction",
                "params": [{
                    "from": "0x0000000000000000000000000000000000000000",
                    "to": address,
                    "value": format!("0x{:x}", amount),
                    "gas": "0x5208",
                    "gasPrice": "0x3b9aca00"
                }],
                "id": 1
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            warn!("Failed to fund account: {}", address);
        }

        Ok(())
    }

    /// Get permissions for account type
    fn get_permissions_for_type(&self, account_type: &AccountType) -> Vec<String> {
        match account_type {
            AccountType::Regular => vec!["basic_transactions".to_string()],
            AccountType::Lender => vec![
                "basic_transactions".to_string(),
                "lending_deposit".to_string(),
                "lending_withdraw".to_string(),
            ],
            AccountType::Borrower => vec![
                "basic_transactions".to_string(),
                "lending_borrow".to_string(),
                "lending_repay".to_string(),
            ],
            AccountType::GPUMiner => vec![
                "basic_transactions".to_string(),
                "gpu_swarm_join".to_string(),
                "gpu_swarm_task_submit".to_string(),
            ],
            AccountType::AITrader => vec![
                "basic_transactions".to_string(),
                "ai_swarm_submit_strategy".to_string(),
                "ai_swarm_execute".to_string(),
            ],
            AccountType::Admin => vec![
                "basic_transactions".to_string(),
                "contract_deploy".to_string(),
                "contract_upgrade".to_string(),
                "network_admin".to_string(),
            ],
            AccountType::Contract => vec!["contract_interaction".to_string()],
        }
    }

    /// Get account by name
    pub fn get_account(&self, name: &str) -> Option<&TestAccount> {
        self.accounts.get(name)
    }

    /// Get account by address
    pub fn get_account_by_address(&self, address: &str) -> Option<&TestAccount> {
        self.accounts.values().find(|acc| acc.address == address)
    }

    /// Get all accounts of a specific type
    pub fn get_accounts_by_type(&self, account_type: &AccountType) -> Vec<&TestAccount> {
        self.accounts
            .values()
            .filter(|acc| {
                std::mem::discriminant(&acc.account_type) == std::mem::discriminant(account_type)
            })
            .collect()
    }

    /// Update account balance
    pub fn update_balance(
        &mut self,
        name: &str,
        new_balance: u128,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(account) = self.accounts.get_mut(name) {
            account.balance = new_balance;
            Ok(())
        } else {
            Err(format!("Account '{}' not found", name).into())
        }
    }

    /// Get all accounts
    pub fn get_all_accounts(&self) -> Vec<&TestAccount> {
        self.accounts.values().collect()
    }

    /// Get random account for testing
    pub fn get_random_account(&self, account_type: Option<AccountType>) -> Option<&TestAccount> {
        let accounts = match account_type {
            Some(t) => self.get_accounts_by_type(&t),
            None => self.get_all_accounts(),
        };

        if accounts.is_empty() {
            None
        } else {
            use rand::Rng;
            let idx = rand::thread_rng().gen_range(0..accounts.len());
            Some(accounts[idx])
        }
    }

    /// Export accounts for use in tests
    pub fn export_for_tests(&self) -> HashMap<String, TestAccount> {
        self.accounts.clone()
    }
}

/// Predefined test account collections for common scenarios
pub struct TestAccountCollections {
    pub lending_scenario: Vec<&'static str>,
    pub gpu_mining_scenario: Vec<&'static str>,
    pub ai_trading_scenario: Vec<&'static str>,
    pub cross_chain_scenario: Vec<&'static str>,
}

impl TestAccountCollections {
    pub fn new() -> Self {
        Self {
            lending_scenario: vec!["test_lender", "test_borrower", "test_user_1"],
            gpu_mining_scenario: vec!["test_gpu_miner_1", "test_gpu_miner_2", "test_user_1"],
            ai_trading_scenario: vec!["test_ai_trader_1", "test_ai_trader_2", "test_user_1"],
            cross_chain_scenario: vec!["test_user_1", "test_user_2", "test_admin"],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_account_creation() {
        let manager = TestAccountManager::new("http://localhost:9933".to_string());

        let result = manager
            .create_test_account("test_account", AccountType::Regular, 1000)
            .await;
        assert!(result.is_ok());

        let account = result.unwrap();
        assert_eq!(account.name, "test_account");
        assert_eq!(account.balance, 1000);
        assert_eq!(account.account_type, AccountType::Regular);
    }

    #[test]
    fn test_permissions() {
        let manager = TestAccountManager::new("http://localhost:9933".to_string());

        let lender_perms = manager.get_permissions_for_type(&AccountType::Lender);
        assert!(lender_perms.contains(&"lending_deposit".to_string()));

        let admin_perms = manager.get_permissions_for_type(&AccountType::Admin);
        assert!(admin_perms.contains(&"contract_deploy".to_string()));
    }
}
