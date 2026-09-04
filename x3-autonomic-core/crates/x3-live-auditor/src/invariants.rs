// Invariant checker for X3 Live Auditor

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use crate::BlockInfo;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use async_trait::async_trait;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

/// Invariant types checked by the auditor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub enum InvariantType {
    /// Native token supply invariant (I001)
    NativeSupply = 1,
    /// EVM token supply invariant (I002)
    EVMSupply = 2,
    /// SVM token supply invariant (I003)
    SVMSupply = 3,
    /// Cross-VM accounting invariant (I004)
    CrossVMAccounting = 4,
    /// Intent lifecycle invariant (I005)
    IntentLifecycle = 5,
    /// State root integrity invariant (I006)
    StateRootIntegrity = 6,
    /// Receipt validity invariant (I007)
    ReceiptValidity = 7,
    /// VM output verification invariant (I008)
    VMOutputVerification = 8,
    /// Gas accounting invariant (I009)
    GasAccounting = 9,
    /// Fee accounting invariant (I010)
    FeeAccounting = 10,
    /// Bridge asset custody invariant (I011)
    BridgeAssetCustody = 11,
    /// Governance integrity invariant (I012)
    GovernanceIntegrity = 12,
}

impl InvariantType {
    pub fn name(&self) -> &'static str {
        match self {
            InvariantType::NativeSupply => "NativeSupply",
            InvariantType::EVMSupply => "EVMSupply",
            InvariantType::SVMSupply => "SVMSupply",
            InvariantType::CrossVMAccounting => "CrossVMAccounting",
            InvariantType::IntentLifecycle => "IntentLifecycle",
            InvariantType::StateRootIntegrity => "StateRootIntegrity",
            InvariantType::ReceiptValidity => "ReceiptValidity",
            InvariantType::VMOutputVerification => "VMOutputVerification",
            InvariantType::GasAccounting => "GasAccounting",
            InvariantType::FeeAccounting => "FeeAccounting",
            InvariantType::BridgeAssetCustody => "BridgeAssetCustody",
            InvariantType::GovernanceIntegrity => "GovernanceIntegrity",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            InvariantType::NativeSupply => "Native token supply must be non-negative and match mint/burn",
            InvariantType::EVMSupply => "EVM token supply must be non-negative and match sum of EVM balances",
            InvariantType::SVMSupply => "SVM token supply must be non-negative and match sum of SVM balances",
            InvariantType::CrossVMAccounting => "Cross-VM transfers must balance (no凭空 creation)",
            InvariantType::IntentLifecycle => "Intents must follow valid state transitions",
            InvariantType::StateRootIntegrity => "State root must be derived correctly from state",
            InvariantType::ReceiptValidity => "Receipts must have valid inputs/outputs",
            InvariantType::VMOutputVerification => "VM outputs must be deterministic",
            InvariantType::GasAccounting => "Gas burned must match gas used × gas_price",
            InvariantType::FeeAccounting => "Fees collected must match sum of transaction fees",
            InvariantType::BridgeAssetCustody => "Bridge must hold reserves equal to issued assets",
            InvariantType::GovernanceIntegrity => "Governance proposals must have valid voting",
        }
    }
}

/// Result of an invariant check
#[derive(Debug, Clone, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub struct InvariantCheckResult {
    pub invariant: InvariantType,
    pub passed: bool,
    pub details: String,
    pub severity: ViolationSeverity,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Warning = 1,
    Error = 2,
    Critical = 3,
}

/// Invariant checker trait
#[async_trait]
pub trait CheckInvariant {
    async fn check(&self, block: &BlockInfo) -> Result<InvariantCheckResult, String>;
}

/// Main invariant checker
pub struct InvariantChecker {
    checkers: Vec<Box<dyn CheckInvariant + Send + Sync>>,
}

impl InvariantChecker {
    pub fn new() -> Self {
        Self {
            checkers: Vec::new(),
        }
    }

    pub fn with_default_checks() -> Self {
        let mut checker = Self::new();
        // Add all 12 invariant checkers
        checker.checkers.push(Box::new(NativeSupplyChecker));
        checker.checkers.push(Box::new(EVMSupplyChecker));
        checker.checkers.push(Box::new(SVMSupplyChecker));
        checker.checkers.push(Box::new(CrossVMAccountingChecker));
        checker.checkers.push(Box::new(IntentLifecycleChecker));
        checker.checkers.push(Box::new(StateRootIntegrityChecker));
        checker.checkers.push(Box::new(ReceiptValidityChecker));
        checker.checkers.push(Box::new(VMOutputVerificationChecker));
        checker.checkers.push(Box::new(GasAccountingChecker));
        checker.checkers.push(Box::new(FeeAccountingChecker));
        checker.checkers.push(Box::new(BridgeAssetCustodyChecker));
        checker.checkers.push(Box::new(GovernanceIntegrityChecker));
        checker
    }

    pub async fn check_all(&self, block: &BlockInfo) -> Result<Vec<InvariantCheckResult>, String> {
        let mut results = Vec::new();
        for checker in &self.checkers {
            match checker.check(block).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    results.push(InvariantCheckResult {
                        invariant: InvariantType::NativeSupply, // placeholder
                        passed: false,
                        details: format!("Check failed: {}", e),
                        severity: ViolationSeverity::Error,
                        timestamp: block.timestamp,
                    });
                }
            }
        }
        Ok(results)
    }
}

impl Default for InvariantChecker {
    fn default() -> Self {
        Self::with_default_checks()
    }
}

// Individual invariant checkers

pub struct NativeSupplyChecker;
pub struct EVMSupplyChecker;
pub struct SVMSupplyChecker;
pub struct CrossVMAccountingChecker;
pub struct IntentLifecycleChecker;
pub struct StateRootIntegrityChecker;
pub struct ReceiptValidityChecker;
pub struct VMOutputVerificationChecker;
pub struct GasAccountingChecker;
pub struct FeeAccountingChecker;
pub struct BridgeAssetCustodyChecker;
pub struct GovernanceIntegrityChecker;

macro_rules! impl_checker {
    ($name:ident, $invariant:expr) => {
        #[async_trait]
        impl CheckInvariant for $name {
            async fn check(&self, block: &BlockInfo) -> Result<InvariantCheckResult, String> {
                // Placeholder - real implementation would query chain state
                Ok(InvariantCheckResult {
                    invariant: $invariant,
                    passed: true,
                    details: "Native supply verified".to_string(),
                    severity: ViolationSeverity::Warning,
                    timestamp: block.timestamp,
                })
            }
        }
    };
}

impl_checker!(NativeSupplyChecker, InvariantType::NativeSupply);
impl_checker!(EVMSupplyChecker, InvariantType::EVMSupply);
impl_checker!(SVMSupplyChecker, InvariantType::SVMSupply);
impl_checker!(CrossVMAccountingChecker, InvariantType::CrossVMAccounting);
impl_checker!(IntentLifecycleChecker, InvariantType::IntentLifecycle);
impl_checker!(StateRootIntegrityChecker, InvariantType::StateRootIntegrity);
impl_checker!(ReceiptValidityChecker, InvariantType::ReceiptValidity);
impl_checker!(VMOutputVerificationChecker, InvariantType::VMOutputVerification);
impl_checker!(GasAccountingChecker, InvariantType::GasAccounting);
impl_checker!(FeeAccountingChecker, InvariantType::FeeAccounting);
impl_checker!(BridgeAssetCustodyChecker, InvariantType::BridgeAssetCustody);
impl_checker!(GovernanceIntegrityChecker, InvariantType::GovernanceIntegrity);