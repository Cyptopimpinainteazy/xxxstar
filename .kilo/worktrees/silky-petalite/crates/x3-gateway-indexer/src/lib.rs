use std::collections::HashMap;
use x3_circuit_breaker::{CircuitBreakerRecord, CircuitBreakerScope};
use x3_external_route_registry::{AssetId, GatewayRouteConfig, RouteId, X3Domain};
use x3_gateway_insurance::{FundId, InsuranceFund, RouteCoverage};
use x3_gateway_risk_engine::{GatewayRiskStatus, GatewayRouteRiskReport};
use x3_proof_dispute::{DisputeStatus, DisputeWindow};
use x3_proof_envelope::ProofId;
use x3_verification_router::{ExternalAssetRef, ExternalChainId, VerificationResult};

pub type TransferId = [u8; 32];
pub type BlockNumber = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GatewayTransferStatus {
    Observed,
    WaitingFinality,
    ProofSubmitted,
    Verified,
    DisputeWindowOpen,
    X3Credited,
    X3Burned,
    ExternalReleased,
    Expired,
    Refunded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalAssetIndexRecord {
    pub x3_asset_id: AssetId,
    pub external_chain_id: ExternalChainId,
    pub external_asset: ExternalAssetRef,
    pub symbol: String,
    pub decimals: u8,
    pub route_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayTransferIndexRecord {
    pub transfer_id: TransferId,
    pub source_chain: ExternalChainId,
    pub destination_domain: X3Domain,
    pub external_asset: ExternalAssetRef,
    pub x3_asset_id: AssetId,
    pub sender: String,
    pub recipient: String,
    pub amount: u128,
    pub status: GatewayTransferStatus,
    pub source_tx_hash: [u8; 32],
    pub proof_id: ProofId,
    pub created_block: BlockNumber,
    pub finalized_block: BlockNumber,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayRouteIndexRecord {
    pub route_id: RouteId,
    pub external_chain_id: ExternalChainId,
    pub x3_asset_id: AssetId,
    pub enabled: bool,
    pub min_amount: u128,
    pub max_amount: u128,
    pub daily_limit: u128,
    pub volume_used: u128,
    pub pending_count: u32,
    pub finality_requirement: u64,
    pub verification_level: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationIndexRecord {
    pub proof_id: ProofId,
    pub verified: bool,
    pub strategy: String,
    pub confidence_bps: u16,
    pub checked_block: BlockNumber,
    pub failure_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DisputeIndexRecord {
    pub proof_id: ProofId,
    pub opens_at_block: BlockNumber,
    pub closes_at_block: BlockNumber,
    pub status: DisputeStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayRiskIndexRecord {
    pub route_id: RouteId,
    pub status: GatewayRiskStatus,
    pub allow_transfer: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CircuitBreakerIndexRecord {
    pub scope: CircuitBreakerScope,
    pub status: String,
    pub reason: String,
    pub tripped_at_block: Option<BlockNumber>,
    pub reset_at_block: Option<BlockNumber>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsuranceFundIndexRecord {
    pub fund_id: FundId,
    pub asset_id: AssetId,
    pub balance: u128,
    pub coverage_limit: u128,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCoverageIndexRecord {
    pub route_id: RouteId,
    pub fund_id: FundId,
    pub max_covered_amount: u128,
    pub premium_bps: u16,
}

#[derive(Debug, Default)]
pub struct GatewayIndexer {
    assets: HashMap<AssetId, ExternalAssetIndexRecord>,
    routes: HashMap<RouteId, GatewayRouteIndexRecord>,
    transfers: HashMap<TransferId, GatewayTransferIndexRecord>,
    verifications: HashMap<ProofId, VerificationIndexRecord>,
    disputes: HashMap<ProofId, DisputeIndexRecord>,
    risks: HashMap<RouteId, GatewayRiskIndexRecord>,
    circuit_breakers: HashMap<CircuitBreakerScope, CircuitBreakerIndexRecord>,
    insurance_funds: HashMap<FundId, InsuranceFundIndexRecord>,
    route_coverage: HashMap<RouteId, RouteCoverageIndexRecord>,
}

impl GatewayIndexer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn index_external_asset(
        &mut self,
        x3_asset_id: AssetId,
        external_asset: ExternalAssetRef,
        route_status: impl Into<String>,
    ) {
        self.assets.insert(
            x3_asset_id,
            ExternalAssetIndexRecord {
                x3_asset_id,
                external_chain_id: external_asset.chain_id,
                symbol: external_asset.symbol.clone(),
                decimals: external_asset.decimals,
                external_asset,
                route_status: route_status.into(),
            },
        );
    }

    pub fn index_gateway_route(&mut self, config: &GatewayRouteConfig) {
        self.routes.insert(
            config.route_id,
            GatewayRouteIndexRecord {
                route_id: config.route_id,
                external_chain_id: config.external_chain_id,
                x3_asset_id: config.x3_asset_id,
                enabled: config.enabled,
                min_amount: config.min_amount,
                max_amount: config.max_amount,
                daily_limit: config.daily_limit,
                volume_used: 0,
                pending_count: 0,
                finality_requirement: config.finality_requirement,
                verification_level: format!("{:?}", config.verification_level),
            },
        );
    }

    pub fn index_gateway_transfer(&mut self, record: GatewayTransferIndexRecord) {
        self.transfers.insert(record.transfer_id, record);
    }

    pub fn update_transfer_status(
        &mut self,
        transfer_id: TransferId,
        status: GatewayTransferStatus,
    ) {
        if let Some(record) = self.transfers.get_mut(&transfer_id) {
            record.status = status;
        }
    }

    pub fn index_verification_result(&mut self, result: &VerificationResult) {
        self.verifications.insert(
            result.proof_id,
            VerificationIndexRecord {
                proof_id: result.proof_id,
                verified: result.verified,
                strategy: format!("{:?}", result.strategy),
                confidence_bps: result.confidence_bps,
                checked_block: result.verified_at_block,
                failure_reason: result.failure_reason.clone(),
            },
        );
    }

    pub fn index_dispute_window(&mut self, window: &DisputeWindow) {
        self.disputes.insert(
            window.proof_id,
            DisputeIndexRecord {
                proof_id: window.proof_id,
                opens_at_block: window.opens_at_block,
                closes_at_block: window.closes_at_block,
                status: window.status,
            },
        );
    }

    pub fn update_dispute_status(&mut self, proof_id: ProofId, status: DisputeStatus) {
        if let Some(record) = self.disputes.get_mut(&proof_id) {
            record.status = status;
        }
    }

    pub fn index_gateway_risk_report(&mut self, report: &GatewayRouteRiskReport) {
        self.risks.insert(
            report.route_id,
            GatewayRiskIndexRecord {
                route_id: report.route_id,
                status: report.status,
                allow_transfer: report.allow_transfer,
                reasons: report.reasons.clone(),
            },
        );
    }

    pub fn index_circuit_breaker(&mut self, record: &CircuitBreakerRecord) {
        self.circuit_breakers.insert(
            record.scope,
            CircuitBreakerIndexRecord {
                scope: record.scope,
                status: format!("{:?}", record.status),
                reason: record.reason.clone(),
                tripped_at_block: record.tripped_at_block,
                reset_at_block: record.reset_at_block,
            },
        );
    }

    pub fn index_insurance_fund(&mut self, fund: &InsuranceFund) {
        self.insurance_funds.insert(
            fund.fund_id,
            InsuranceFundIndexRecord {
                fund_id: fund.fund_id,
                asset_id: fund.asset_id,
                balance: fund.balance,
                coverage_limit: fund.coverage_limit,
                status: format!("{:?}", fund.status),
            },
        );
    }

    pub fn index_route_coverage(&mut self, coverage: &RouteCoverage) {
        self.route_coverage.insert(
            coverage.route_id,
            RouteCoverageIndexRecord {
                route_id: coverage.route_id,
                fund_id: coverage.fund_id,
                max_covered_amount: coverage.max_covered_amount,
                premium_bps: coverage.premium_bps,
            },
        );
    }

    pub fn get_external_asset(&self, x3_asset_id: AssetId) -> Option<&ExternalAssetIndexRecord> {
        self.assets.get(&x3_asset_id)
    }

    pub fn get_gateway_route(&self, route_id: RouteId) -> Option<&GatewayRouteIndexRecord> {
        self.routes.get(&route_id)
    }

    pub fn get_gateway_transfer(
        &self,
        transfer_id: TransferId,
    ) -> Option<&GatewayTransferIndexRecord> {
        self.transfers.get(&transfer_id)
    }

    pub fn get_gateway_transfers_by_asset(
        &self,
        x3_asset_id: AssetId,
    ) -> Vec<&GatewayTransferIndexRecord> {
        self.transfers
            .values()
            .filter(|record| record.x3_asset_id == x3_asset_id)
            .collect()
    }

    pub fn get_gateway_transfers_by_account(
        &self,
        account: &str,
    ) -> Vec<&GatewayTransferIndexRecord> {
        self.transfers
            .values()
            .filter(|record| record.sender == account || record.recipient == account)
            .collect()
    }

    pub fn get_pending_gateway_transfers(&self) -> Vec<&GatewayTransferIndexRecord> {
        self.transfers
            .values()
            .filter(|record| {
                matches!(
                    record.status,
                    GatewayTransferStatus::Observed
                        | GatewayTransferStatus::WaitingFinality
                        | GatewayTransferStatus::ProofSubmitted
                        | GatewayTransferStatus::Verified
                        | GatewayTransferStatus::DisputeWindowOpen
                )
            })
            .collect()
    }

    pub fn get_failed_gateway_transfers(&self) -> Vec<&GatewayTransferIndexRecord> {
        self.transfers
            .values()
            .filter(|record| record.status == GatewayTransferStatus::Failed)
            .collect()
    }

    pub fn get_verification_result(&self, proof_id: ProofId) -> Option<&VerificationIndexRecord> {
        self.verifications.get(&proof_id)
    }

    pub fn get_dispute_window(&self, proof_id: ProofId) -> Option<&DisputeIndexRecord> {
        self.disputes.get(&proof_id)
    }

    pub fn get_gateway_risk_report(&self, route_id: RouteId) -> Option<&GatewayRiskIndexRecord> {
        self.risks.get(&route_id)
    }

    pub fn get_circuit_breaker(
        &self,
        scope: CircuitBreakerScope,
    ) -> Option<&CircuitBreakerIndexRecord> {
        self.circuit_breakers.get(&scope)
    }

    pub fn get_insurance_fund(&self, fund_id: FundId) -> Option<&InsuranceFundIndexRecord> {
        self.insurance_funds.get(&fund_id)
    }

    pub fn get_route_coverage(&self, route_id: RouteId) -> Option<&RouteCoverageIndexRecord> {
        self.route_coverage.get(&route_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_external_route_registry::{GatewayMode, GatewayRouteConfig};
    use x3_verification_router::VerificationStrategy;

    fn asset() -> ExternalAssetRef {
        ExternalAssetRef {
            chain_id: ExternalChainId::BaseSepolia,
            token_address_or_mint: "0xmock".to_string(),
            decimals: 18,
            symbol: "MOCK".to_string(),
        }
    }

    fn route() -> GatewayRouteConfig {
        GatewayRouteConfig {
            route_id: [1; 32],
            external_chain_id: ExternalChainId::BaseSepolia,
            external_asset: asset(),
            x3_asset_id: [9; 32],
            destination_domain: X3Domain::Native,
            enabled: true,
            min_amount: 1,
            max_amount: 1_000,
            daily_limit: 10_000,
            pending_limit: 10,
            finality_requirement: 32,
            verification_level: VerificationStrategy::ValidatorQuorum,
            fee_bps: 10,
            mode: GatewayMode::TestnetLive,
            require_dispute_window: false,
        }
    }

    #[test]
    fn indexes_asset_route_and_transfer_queries() {
        let mut indexer = GatewayIndexer::new();
        indexer.index_external_asset([9; 32], asset(), "enabled");
        indexer.index_gateway_route(&route());
        indexer.index_gateway_transfer(GatewayTransferIndexRecord {
            transfer_id: [7; 32],
            source_chain: ExternalChainId::BaseSepolia,
            destination_domain: X3Domain::Native,
            external_asset: asset(),
            x3_asset_id: [9; 32],
            sender: "alice".to_string(),
            recipient: "bob".to_string(),
            amount: 100,
            status: GatewayTransferStatus::ProofSubmitted,
            source_tx_hash: [8; 32],
            proof_id: [7; 32],
            created_block: 1,
            finalized_block: 2,
        });

        assert!(indexer.get_external_asset([9; 32]).is_some());
        assert!(indexer.get_gateway_route([1; 32]).is_some());
        assert_eq!(indexer.get_pending_gateway_transfers().len(), 1);
        indexer.update_transfer_status([7; 32], GatewayTransferStatus::Failed);
        assert_eq!(indexer.get_failed_gateway_transfers().len(), 1);
        assert_eq!(indexer.get_gateway_transfers_by_account("bob").len(), 1);
    }

    #[test]
    fn indexes_safety_records() {
        let mut indexer = GatewayIndexer::new();
        indexer.index_verification_result(&VerificationResult {
            proof_id: [1; 32],
            strategy: VerificationStrategy::ValidatorQuorum,
            verified: true,
            confidence_bps: 8_000,
            verified_at_block: 5,
            failure_reason: None,
        });
        indexer.index_dispute_window(&DisputeWindow {
            proof_id: [1; 32],
            opens_at_block: 5,
            closes_at_block: 15,
            status: DisputeStatus::Open,
        });
        indexer.index_gateway_risk_report(&GatewayRouteRiskReport {
            route_id: [2; 32],
            status: GatewayRiskStatus::Low,
            allow_transfer: true,
            reasons: vec!["low_risk".to_string()],
        });

        assert!(indexer.get_verification_result([1; 32]).unwrap().verified);
        assert_eq!(
            indexer.get_dispute_window([1; 32]).unwrap().status,
            DisputeStatus::Open
        );
        assert_eq!(
            indexer.get_gateway_risk_report([2; 32]).unwrap().status,
            GatewayRiskStatus::Low
        );
    }
}
