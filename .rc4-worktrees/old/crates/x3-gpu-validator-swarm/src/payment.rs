//! Payment System for X3 GPU Validator Swarm
//!
//! Handles provider compensation, reward distribution, and wallet integration.

use crate::config::SwarmConfig;
use crate::error::{SwarmError, SwarmResult};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::time::{Duration, Instant};

/// Payment unit (lamports-like)
pub type PaymentAmount = u64;

/// Reward rate per unit of work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardRate {
    /// Base reward per task
    pub base_rate: PaymentAmount,
    /// Bonus multiplier for verification
    pub verification_bonus: f64,
    /// Penalty for divergence
    pub divergence_penalty: f64,
    /// Minimum stake for rewards
    pub min_stake: PaymentAmount,
}

impl Default for RewardRate {
    fn default() -> Self {
        Self {
            base_rate: 1000, // 0.001 X3
            verification_bonus: 1.5,
            divergence_penalty: 0.5,
            min_stake: 1000000, // 1 X3
        }
    }
}

/// Provider work record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkRecord {
    /// Provider ID
    pub provider_id: String,
    /// Task ID
    pub task_id: String,
    /// Work type
    pub work_type: WorkType,
    /// Amount of work (compute units)
    pub work_units: u64,
    /// Verification status
    pub verified: bool,
    /// Divergence detected
    pub divergent: bool,
    /// Timestamp
    pub timestamp: i64,
    /// Block height
    pub block_height: u64,
}

/// Work types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkType {
    /// Hash computation
    Hash,
    /// Signature verification
    Sign,
    /// Custom computation
    Custom,
    /// Verification (CPU check)
    Verification,
}

/// Pending payment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingPayment {
    /// Payment ID
    pub payment_id: String,
    /// Provider ID
    pub provider_id: String,
    /// Amount
    pub amount: PaymentAmount,
    /// Work record IDs
    pub work_record_ids: Vec<String>,
    /// Created at
    pub created_at: i64,
    /// Settled at (None if pending)
    pub settled_at: Option<i64>,
}

/// Provider account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAccount {
    /// Provider ID
    pub provider_id: String,
    /// Wallet address
    pub wallet_address: String,
    /// Total earned
    pub total_earned: PaymentAmount,
    /// Total pending
    pub total_pending: PaymentAmount,
    /// Total withdrawn
    pub total_withdrawn: PaymentAmount,
    /// Stake amount
    pub stake: PaymentAmount,
    /// Reputation score
    pub reputation: f64,
    /// Total tasks completed
    pub tasks_completed: u64,
    /// Total verifications
    pub verifications: u64,
    /// Divergence count
    pub divergence_count: u64,
    /// Registered at
    pub registered_at: i64,
    /// Last activity
    pub last_activity: i64,
    /// Status
    pub status: ProviderStatus,
}

/// Provider status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProviderStatus {
    /// Not registered
    Unregistered,
    /// Pending registration
    Pending,
    /// Active provider
    Active,
    /// Paused
    Paused,
    /// Slashed
    Slashed,
}

/// Payment settlement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settlement {
    /// Settlement ID
    pub settlement_id: String,
    /// Provider ID
    pub provider_id: String,
    /// Amount
    pub amount: PaymentAmount,
    /// Transaction hash
    pub tx_hash: Option<String>,
    /// Status
    pub status: SettlementStatus,
    /// Created at
    pub created_at: i64,
    /// Confirmed at
    pub confirmed_at: Option<i64>,
}

/// Settlement status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementStatus {
    /// Pending
    Pending,
    /// Submitted
    Submitted,
    /// Confirmed
    Confirmed,
    /// Failed
    Failed,
}

/// Payment system
pub struct PaymentSystem {
    /// Reward rate
    reward_rate: RwLock<RewardRate>,
    /// Provider accounts
    providers: RwLock<HashMap<String, ProviderAccount>>,
    /// Work records
    work_records: RwLock<Vec<WorkRecord>>,
    /// Pending payments
    pending_payments: RwLock<Vec<PendingPayment>>,
    /// Settlements
    settlements: RwLock<Vec<Settlement>>,
    /// Config
    _config: SwarmConfig,
    /// Settlement interval
    settlement_interval: Duration,
    /// Last settlement
    last_settlement: RwLock<Option<Instant>>,
}

impl PaymentSystem {
    /// Create a new payment system
    pub fn new(config: SwarmConfig) -> Self {
        Self {
            reward_rate: RwLock::new(RewardRate::default()),
            providers: RwLock::new(HashMap::new()),
            work_records: RwLock::new(Vec::new()),
            pending_payments: RwLock::new(Vec::new()),
            settlements: RwLock::new(Vec::new()),
            _config: config,
            settlement_interval: Duration::from_secs(3600), // 1 hour
            last_settlement: RwLock::new(None),
        }
    }

    /// Validate provider ID format
    fn is_valid_provider_id(id: &str) -> bool {
        !id.is_empty()
            && id.len() <= 64
            && id.len() >= 3
            && id.chars().all(|c| c.is_alphanumeric() || c == '_')
    }

    /// Validate wallet address format (Ethereum-style)
    fn is_valid_wallet_address(addr: &str) -> bool {
        addr.starts_with("0x")
            && addr.len() == 42
            && addr[2..].chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Register a provider
    pub fn register_provider(
        &self,
        provider_id: String,
        wallet_address: String,
        stake: PaymentAmount,
    ) -> SwarmResult<ProviderAccount> {
        // Validate provider ID format
        if !Self::is_valid_provider_id(&provider_id) {
            return Err(SwarmError::InvalidInput(
                "Invalid provider ID format. Must be alphanumeric with underscores, 3-64 chars"
                    .to_string(),
            ));
        }

        // Validate wallet address format
        if !Self::is_valid_wallet_address(&wallet_address) {
            return Err(SwarmError::InvalidInput(
                "Invalid wallet address format".to_string(),
            ));
        }

        let mut providers = self.providers.write();

        if providers.contains_key(&provider_id) {
            return Err(SwarmError::InvalidInput(
                "Provider already registered".to_string(),
            ));
        }

        // Check minimum stake
        let reward_rate = self.reward_rate.read();
        if stake < reward_rate.min_stake {
            return Err(SwarmError::InvalidInput(format!(
                "Minimum stake is {}",
                reward_rate.min_stake
            )));
        }

        let account = ProviderAccount {
            provider_id: provider_id.clone(),
            wallet_address,
            total_earned: 0,
            total_pending: 0,
            total_withdrawn: 0,
            stake,
            reputation: 100.0,
            tasks_completed: 0,
            verifications: 0,
            divergence_count: 0,
            registered_at: chrono::Utc::now().timestamp(),
            last_activity: chrono::Utc::now().timestamp(),
            status: ProviderStatus::Active,
        };

        providers.insert(provider_id, account.clone());

        Ok(account)
    }

    /// Record work
    pub fn record_work(&self, record: WorkRecord) -> SwarmResult<()> {
        let provider_id = record.provider_id.clone();

        // Update provider stats
        {
            let mut providers = self.providers.write();
            if let Some(provider) = providers.get_mut(&provider_id) {
                provider.tasks_completed += 1;
                provider.last_activity = chrono::Utc::now().timestamp();

                if record.verified {
                    provider.verifications += 1;
                }
                if record.divergent {
                    provider.divergence_count += 1;
                }
            }
        }

        // Calculate reward
        let reward = self.calculate_reward(&record)?;

        // Add to pending payment
        {
            let mut pending = self.pending_payments.write();
            let payment = PendingPayment {
                payment_id: uuid::Uuid::new_v4().to_string(),
                provider_id: provider_id.clone(),
                amount: reward,
                work_record_ids: vec![record.task_id.clone()],
                created_at: chrono::Utc::now().timestamp(),
                settled_at: None,
            };
            pending.push(payment);
        }

        // Store work record
        self.work_records.write().push(record);

        Ok(())
    }

    /// Calculate reward for work (with overflow protection)
    fn calculate_reward(&self, record: &WorkRecord) -> SwarmResult<PaymentAmount> {
        let rate = self.reward_rate.read();

        // Use checked arithmetic for base reward
        let mut reward = rate
            .base_rate
            .checked_mul(record.work_units)
            .ok_or_else(|| SwarmError::InvalidInput("Reward calculation overflow".to_string()))?;

        // Apply verification bonus with overflow protection
        if record.verified {
            let bonus_reward = (reward as u128)
                .checked_mul((rate.verification_bonus * 1000.0) as u128)
                .and_then(|r| (r / 1000).try_into().ok())
                .ok_or_else(|| {
                    SwarmError::InvalidInput("Reward bonus calculation overflow".to_string())
                })?;
            reward = bonus_reward;
        }

        // Apply divergence penalty with underflow protection
        if record.divergent {
            let penalty_reward = (reward as u128)
                .checked_mul((rate.divergence_penalty * 1000.0) as u128)
                .and_then(|r| (r / 1000).try_into().ok())
                .unwrap_or(0); // Penalty can reduce to 0
            reward = penalty_reward;
        }

        // Sanity check: cap maximum reward per task
        const MAX_REWARD_PER_TASK: PaymentAmount = 1_000_000_000; // 1000 X3
        if reward > MAX_REWARD_PER_TASK {
            return Err(SwarmError::InvalidInput(format!(
                "Reward {} exceeds maximum per-task limit",
                reward
            )));
        }

        Ok(reward)
    }

    /// Process settlements
    pub fn process_settlements(&self) -> SwarmResult<Vec<Settlement>> {
        let mut settlements = Vec::new();

        // Check if it's time to settle
        {
            let last = *self.last_settlement.read();
            if let Some(last_time) = last {
                if last_time.elapsed() < self.settlement_interval {
                    return Ok(settlements);
                }
            }
        }

        // Process pending payments
        let mut pending = self.pending_payments.write();
        let to_settle: Vec<PendingPayment> = pending.drain(..).collect();

        for mut payment in to_settle {
            // Mark as settled
            payment.settled_at = Some(chrono::Utc::now().timestamp());

            // Update provider account
            {
                let mut providers = self.providers.write();
                if let Some(provider) = providers.get_mut(&payment.provider_id) {
                    provider.total_pending += payment.amount;
                    provider.total_earned += payment.amount;
                }
            }

            // Create settlement
            let settlement = Settlement {
                settlement_id: uuid::Uuid::new_v4().to_string(),
                provider_id: payment.provider_id.clone(),
                amount: payment.amount,
                tx_hash: None,
                status: SettlementStatus::Pending,
                created_at: chrono::Utc::now().timestamp(),
                confirmed_at: None,
            };

            settlements.push(settlement.clone());
            self.settlements.write().push(settlement);
        }

        *self.last_settlement.write() = Some(Instant::now());

        Ok(settlements)
    }

    /// Confirm settlement
    pub fn confirm_settlement(&self, settlement_id: &str, tx_hash: String) -> SwarmResult<()> {
        let mut settlements = self.settlements.write();

        if let Some(settlement) = settlements
            .iter_mut()
            .find(|s| s.settlement_id == settlement_id)
        {
            settlement.tx_hash = Some(tx_hash);
            settlement.status = SettlementStatus::Confirmed;
            settlement.confirmed_at = Some(chrono::Utc::now().timestamp());

            // Update provider
            let mut providers = self.providers.write();
            if let Some(provider) = providers.get_mut(&settlement.provider_id) {
                provider.total_pending -= settlement.amount;
                provider.total_withdrawn += settlement.amount;
            }

            Ok(())
        } else {
            Err(SwarmError::InvalidInput("Settlement not found".to_string()))
        }
    }

    /// Get provider account
    pub fn get_provider(&self, provider_id: &str) -> Option<ProviderAccount> {
        self.providers.read().get(provider_id).cloned()
    }

    /// Get pending payments
    pub fn get_pending_payments(&self, provider_id: &str) -> Vec<PendingPayment> {
        self.pending_payments
            .read()
            .iter()
            .filter(|p| p.provider_id == provider_id)
            .cloned()
            .collect()
    }

    /// Get total pending for provider
    pub fn get_total_pending(&self, provider_id: &str) -> PaymentAmount {
        self.pending_payments
            .read()
            .iter()
            .filter(|p| p.provider_id == provider_id)
            .map(|p| p.amount)
            .sum()
    }

    /// Update reputation
    pub fn update_reputation(&self, provider_id: &str) -> SwarmResult<()> {
        let mut providers = self.providers.write();

        if let Some(provider) = providers.get_mut(provider_id) {
            // Calculate reputation based on performance
            let total = provider.tasks_completed.max(1) as f64;
            let success_rate = 1.0 - (provider.divergence_count as f64 / total);
            let verification_rate = provider.verifications as f64 / total;

            // Reputation formula: success_rate * 0.7 + verification_rate * 0.3
            provider.reputation = (success_rate * 0.7 + verification_rate * 0.3) * 100.0;

            // Slash if too many divergences
            if provider.divergence_count > 10 {
                provider.status = ProviderStatus::Slashed;
            }

            Ok(())
        } else {
            Err(SwarmError::InvalidInput("Provider not found".to_string()))
        }
    }

    /// Get all providers
    pub fn get_all_providers(&self) -> Vec<ProviderAccount> {
        self.providers.read().values().cloned().collect()
    }

    /// Export payment state
    pub fn export_state(&self) -> SwarmResult<String> {
        let state = serde_json::json!({
            "providers": self.get_all_providers(),
            "pending_payments": *self.pending_payments.read(),
            "settlements": *self.settlements.read(),
            "reward_rate": *self.reward_rate.read(),
        });

        serde_json::to_string_pretty(&state).map_err(|e| e.into())
    }
}

/// Wallet GPU Sync Module
pub mod wallet_sync {
    use super::*;

    /// GPU capability report
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GpuCapabilityReport {
        /// Provider ID
        pub provider_id: String,
        /// GPU model
        pub gpu_model: String,
        /// GPU memory (MB)
        pub memory_mb: u64,
        /// Compute capability
        pub compute_capability: (u32, u32),
        /// CUDA cores
        pub cuda_cores: u32,
        /// Benchmark score
        pub benchmark_score: u64,
        /// Supported operations
        pub supported_ops: Vec<String>,
        /// Timestamp
        pub timestamp: i64,
    }

    /// Wallet sync request
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WalletSyncRequest {
        /// Wallet address
        pub wallet_address: String,
        /// Signature for authentication
        pub signature: Vec<u8>,
        /// GPU capability report
        pub gpu_report: GpuCapabilityReport,
    }

    /// Wallet sync response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct WalletSyncResponse {
        /// Success
        pub success: bool,
        /// Provider ID (if successful)
        pub provider_id: Option<String>,
        /// Error message (if failed)
        pub error: Option<String>,
        /// Node connection info
        pub node_info: Option<NodeConnectionInfo>,
    }

    /// Node connection information
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NodeConnectionInfo {
        /// Node address
        pub address: String,
        /// Port
        pub port: u16,
        /// Authentication token
        pub auth_token: String,
    }

    /// GPU detector
    pub fn detect_gpu() -> Option<GpuCapabilityReport> {
        // In production, this would use CUDA/OpenCL to detect
        // For now, return simulated data
        Some(GpuCapabilityReport {
            provider_id: uuid::Uuid::new_v4().to_string(),
            gpu_model: "NVIDIA GPU".to_string(),
            memory_mb: 8192,
            compute_capability: (8, 6),
            cuda_cores: 4096,
            benchmark_score: 10000,
            supported_ops: vec!["hash".to_string(), "sign".to_string()],
            timestamp: chrono::Utc::now().timestamp(),
        })
    }

    /// Run benchmark
    pub fn run_benchmark() -> u64 {
        // Simple benchmark - hash operations per second
        use crate::crypto::keccak256;

        let start = Instant::now();
        let mut count = 0u64;

        while start.elapsed().as_secs() < 1 {
            keccak256(b"benchmark");
            count += 1;
        }

        count
    }

    /// Sync wallet with swarm
    pub fn sync_wallet(request: WalletSyncRequest) -> WalletSyncResponse {
        // Verify signature (simplified)
        if request.signature.is_empty() {
            return WalletSyncResponse {
                success: false,
                provider_id: None,
                error: Some("Invalid signature".to_string()),
                node_info: None,
            };
        }

        // Create provider
        let provider_id = request.gpu_report.provider_id.clone();

        WalletSyncResponse {
            success: true,
            provider_id: Some(provider_id),
            error: None,
            node_info: Some(NodeConnectionInfo {
                address: "localhost".to_string(),
                port: 30334,
                auth_token: uuid::Uuid::new_v4().to_string(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_registration() {
        let config = SwarmConfig::default();
        let payment = PaymentSystem::new(config);

        let result =
            payment.register_provider("provider1".to_string(), "0x1234".to_string(), 1000000);

        assert!(result.is_ok());
        assert_eq!(payment.get_all_providers().len(), 1);
    }

    #[test]
    fn test_work_recording() {
        let config = SwarmConfig::default();
        let payment = PaymentSystem::new(config);

        payment
            .register_provider("provider1".to_string(), "0x1234".to_string(), 1000000)
            .unwrap();

        let record = WorkRecord {
            provider_id: "provider1".to_string(),
            task_id: "task1".to_string(),
            work_type: WorkType::Hash,
            work_units: 10,
            verified: true,
            divergent: false,
            timestamp: chrono::Utc::now().timestamp(),
            block_height: 1,
        };

        payment.record_work(record).unwrap();

        assert!(payment.get_pending_payments("provider1").len() > 0);
    }

    #[test]
    fn test_gpu_detection() {
        let report = wallet_sync::detect_gpu();
        assert!(report.is_some());
    }

    #[test]
    fn test_benchmark() {
        let score = wallet_sync::run_benchmark();
        assert!(score > 0);
    }
}
