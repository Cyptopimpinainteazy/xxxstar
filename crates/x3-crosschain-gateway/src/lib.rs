use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

use x3_asset_kernel_types::{
    traits::{SupplyLedgerGovern, SupplyLedgerWrite},
    DomainId,
};
use x3_circuit_breaker::{CircuitBreakerEngine, CircuitBreakerRecord, CircuitBreakerScope};
use x3_external_route_registry::{
    AssetId, ExternalRouteRegistry, GatewayMode, GatewayRouteConfig, RegistryError, RouteId,
};
use x3_gateway_indexer::{
    GatewayIndexer, GatewayTransferIndexRecord, GatewayTransferStatus, TransferId,
};
use x3_gateway_insurance::{GatewayInsuranceEngine, InsuranceFund, RouteCoverage};
use x3_gateway_risk_engine::{
    GatewayRiskConfig, GatewayRiskEngine, GatewayRouteRiskInput, RiskPolicy,
};
use x3_proof_dispute::{DisputeError, ProofDisputeEngine};
use x3_proof_envelope::{ProofEnvelope, ProofId};
use x3_validator_attestation::{
    AttestationError, GatewayAttestationSet, ValidatorAttestationEngine, ValidatorSet,
};
use x3_verification_router::{
    ExternalChainId, VerificationRequest, VerificationResult, VerificationRouter,
    VerificationStrategy,
};

pub type Balance = u128;
pub type BlockNumber = u64;
pub type AccountId = String;
pub type WithdrawalId = [u8; 32];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayTransfer {
    pub transfer_id: TransferId,
    pub route_id: RouteId,
    pub proof_id: ProofId,
    pub x3_asset_id: AssetId,
    pub sender: AccountId,
    pub recipient: AccountId,
    pub amount: Balance,
    pub status: GatewayTransferStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawalRecord {
    pub withdrawal_id: WithdrawalId,
    pub x3_asset_id: AssetId,
    pub source_domain: String,
    pub destination_chain: ExternalChainId,
    pub recipient: AccountId,
    pub amount: Balance,
    pub burned: bool,
    pub released: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayError {
    Registry(RegistryError),
    CircuitTripped,
    RiskBlocked(Vec<String>),
    ProofReplay,
    ExternalNonceReplay,
    WrongChain,
    WrongToken,
    WrongAmount,
    WrongRecipient,
    UnfinalizedProof,
    VerificationFailed(String),
    Dispute(DisputeError),
    DisputeWindowOpen,
    MissingTransfer,
    MissingWithdrawal,
    InsufficientLedgerBalance,
    InvariantViolation,
    ReleaseReplay,
    Attestation(AttestationError),
}

impl From<RegistryError> for GatewayError {
    fn from(value: RegistryError) -> Self {
        Self::Registry(value)
    }
}

impl From<DisputeError> for GatewayError {
    fn from(value: DisputeError) -> Self {
        Self::Dispute(value)
    }
}

impl From<AttestationError> for GatewayError {
    fn from(value: AttestationError) -> Self {
        Self::Attestation(value)
    }
}

pub trait SupplyLedgerGateway {
    fn credit_x3(
        &mut self,
        asset_id: AssetId,
        account: &str,
        amount: Balance,
    ) -> Result<(), GatewayError>;
    fn burn_x3(
        &mut self,
        asset_id: AssetId,
        account: &str,
        amount: Balance,
    ) -> Result<(), GatewayError>;
    fn balance(&self, asset_id: AssetId, account: &str) -> Balance;
    fn represented_supply(&self, asset_id: AssetId) -> Balance;
}

#[derive(Debug, Default)]
pub struct InMemoryGatewayLedger {
    balances: HashMap<(AssetId, AccountId), Balance>,
    represented: HashMap<AssetId, Balance>,
}

pub struct RuntimeSupplyLedgerAdapter<G>(PhantomData<G>);

impl<G> Default for RuntimeSupplyLedgerAdapter<G> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<G> SupplyLedgerGateway for RuntimeSupplyLedgerAdapter<G>
where
    G: SupplyLedgerGovern + SupplyLedgerWrite,
{
    fn credit_x3(
        &mut self,
        asset_id: AssetId,
        _account: &str,
        amount: Balance,
    ) -> Result<(), GatewayError> {
        G::do_mint_canonical(&asset_id.into(), DomainId::X3Native, amount)
            .map_err(|_| GatewayError::InvariantViolation)
    }

    fn burn_x3(
        &mut self,
        asset_id: AssetId,
        _account: &str,
        amount: Balance,
    ) -> Result<(), GatewayError> {
        G::do_burn_canonical(&asset_id.into(), DomainId::X3Native, amount)
            .map_err(|_| GatewayError::InvariantViolation)
    }

    fn balance(&self, asset_id: AssetId, _account: &str) -> Balance {
        G::ledger(&asset_id.into())
            .map(|ledger| ledger.native_supply)
            .unwrap_or(0)
    }

    fn represented_supply(&self, asset_id: AssetId) -> Balance {
        G::ledger(&asset_id.into())
            .and_then(|ledger| ledger.represented())
            .unwrap_or(0)
    }
}

impl SupplyLedgerGateway for InMemoryGatewayLedger {
    fn credit_x3(
        &mut self,
        asset_id: AssetId,
        account: &str,
        amount: Balance,
    ) -> Result<(), GatewayError> {
        let key = (asset_id, account.to_string());
        let next_balance = self
            .balance(asset_id, account)
            .checked_add(amount)
            .ok_or(GatewayError::InvariantViolation)?;
        let next_supply = self
            .represented_supply(asset_id)
            .checked_add(amount)
            .ok_or(GatewayError::InvariantViolation)?;
        self.balances.insert(key, next_balance);
        self.represented.insert(asset_id, next_supply);
        Ok(())
    }

    fn burn_x3(
        &mut self,
        asset_id: AssetId,
        account: &str,
        amount: Balance,
    ) -> Result<(), GatewayError> {
        let current = self.balance(asset_id, account);
        if current < amount {
            return Err(GatewayError::InsufficientLedgerBalance);
        }
        let supply = self.represented_supply(asset_id);
        if supply < amount {
            return Err(GatewayError::InvariantViolation);
        }
        self.balances
            .insert((asset_id, account.to_string()), current - amount);
        self.represented.insert(asset_id, supply - amount);
        Ok(())
    }

    fn balance(&self, asset_id: AssetId, account: &str) -> Balance {
        self.balances
            .get(&(asset_id, account.to_string()))
            .copied()
            .unwrap_or(0)
    }

    fn represented_supply(&self, asset_id: AssetId) -> Balance {
        self.represented.get(&asset_id).copied().unwrap_or(0)
    }
}

pub struct CrosschainGateway<L: SupplyLedgerGateway> {
    pub registry: ExternalRouteRegistry,
    pub verification_router: VerificationRouter,
    pub attestation_engine: ValidatorAttestationEngine,
    pub risk_engine: GatewayRiskEngine,
    pub circuit_breakers: CircuitBreakerEngine,
    pub dispute_engine: ProofDisputeEngine,
    pub insurance_engine: GatewayInsuranceEngine,
    pub indexer: GatewayIndexer,
    pub ledger: L,
    transfers: HashMap<TransferId, GatewayTransfer>,
    withdrawals: HashMap<WithdrawalId, WithdrawalRecord>,
    used_proofs: HashSet<ProofId>,
    used_external_nonces: HashSet<(ExternalChainId, String, u64)>,
    external_locked: HashMap<AssetId, Balance>,
    pending_withdrawals: HashMap<AssetId, Balance>,
    current_block: BlockNumber,
}

impl<L: SupplyLedgerGateway> CrosschainGateway<L> {
    pub fn new(ledger: L, current_block: BlockNumber) -> Self {
        Self {
            registry: ExternalRouteRegistry::new(),
            verification_router: VerificationRouter::at_block(current_block),
            attestation_engine: ValidatorAttestationEngine::new(),
            risk_engine: GatewayRiskEngine::new(RiskPolicy::default()),
            circuit_breakers: CircuitBreakerEngine::new(),
            dispute_engine: ProofDisputeEngine::new(10).expect("non-zero dispute window"),
            insurance_engine: GatewayInsuranceEngine::new(),
            indexer: GatewayIndexer::new(),
            ledger,
            transfers: HashMap::new(),
            withdrawals: HashMap::new(),
            used_proofs: HashSet::new(),
            used_external_nonces: HashSet::new(),
            external_locked: HashMap::new(),
            pending_withdrawals: HashMap::new(),
            current_block,
        }
    }

    pub fn register_external_asset(
        &mut self,
        external_asset: x3_verification_router::ExternalAssetRef,
        x3_asset_id: AssetId,
    ) {
        self.registry
            .register_external_asset(external_asset.clone(), x3_asset_id);
        self.indexer
            .index_external_asset(x3_asset_id, external_asset, "registered");
    }

    pub fn enable_gateway_route(
        &mut self,
        route_config: GatewayRouteConfig,
    ) -> Result<(), GatewayError> {
        self.risk_engine
            .set_gateway_route_config(GatewayRiskConfig {
                route_id: route_config.route_id,
                max_single_deposit: route_config.max_amount,
                daily_limit: route_config.daily_limit,
                rolling_window_limit: route_config.daily_limit,
                max_pending_transfers: route_config.pending_limit,
                max_unverified_transfers: route_config.pending_limit,
                dispute_window_blocks: 10,
                auto_pause_on_failure_count: 3,
                auto_pause_on_large_deviation: true,
                max_price_deviation_bps: 500,
            });
        self.registry.enable_gateway_route(route_config.clone())?;
        let indexed = self
            .registry
            .get_gateway_route(route_config.route_id)
            .expect("route just inserted");
        self.indexer.index_gateway_route(indexed);
        Ok(())
    }

    pub fn disable_gateway_route(&mut self, route_id: RouteId) -> Result<(), GatewayError> {
        self.registry.disable_gateway_route(route_id)?;
        Ok(())
    }

    pub fn register_validator_set(&mut self, validator_set: ValidatorSet) {
        self.attestation_engine
            .register_validator_set(validator_set);
    }

    pub fn index_circuit_breaker_record(&mut self, record: &CircuitBreakerRecord) {
        self.indexer.index_circuit_breaker(record);
    }

    pub fn index_insurance_fund(&mut self, fund: &InsuranceFund) {
        self.indexer.index_insurance_fund(fund);
    }

    pub fn index_route_coverage(&mut self, coverage: &RouteCoverage) {
        self.indexer.index_route_coverage(coverage);
    }

    pub fn submit_attested_deposit_proof(
        &mut self,
        route_id: RouteId,
        proof_envelope: ProofEnvelope,
        validator_set_id: u64,
        attestation: GatewayAttestationSet,
    ) -> Result<GatewayTransfer, GatewayError> {
        if attestation.proof_id != proof_envelope.proof_id {
            return Err(GatewayError::Attestation(AttestationError::WrongEventHash));
        }
        let route = self
            .registry
            .get_gateway_route(route_id)
            .ok_or(RegistryError::RouteNotFound)?;
        if route.verification_level != VerificationStrategy::ValidatorQuorum {
            return Err(GatewayError::VerificationFailed(
                "route_does_not_accept_validator_quorum".to_string(),
            ));
        }
        let status = self.attestation_engine.submit_attestation(
            validator_set_id,
            attestation,
            self.current_block,
        )?;
        if !matches!(
            status,
            x3_validator_attestation::AttestationStatus::Verified
                | x3_validator_attestation::AttestationStatus::QuorumReached
        ) {
            return Err(GatewayError::Attestation(AttestationError::BelowThreshold));
        }
        self.submit_deposit_proof(route_id, proof_envelope)
    }

    pub fn submit_deposit_proof(
        &mut self,
        route_id: RouteId,
        proof_envelope: ProofEnvelope,
    ) -> Result<GatewayTransfer, GatewayError> {
        if self.used_proofs.contains(&proof_envelope.proof_id) {
            return Err(GatewayError::ProofReplay);
        }
        if self
            .used_external_nonces
            .contains(&proof_envelope.external_nonce_key())
        {
            return Err(GatewayError::ExternalNonceReplay);
        }
        self.circuit_breakers
            .enforce_deposit_allowed(route_id)
            .map_err(|_| GatewayError::CircuitTripped)?;

        let route = self
            .registry
            .enforce_route(route_id, proof_envelope.amount)?
            .clone();
        Self::validate_proof_against_route(&route, &proof_envelope)?;

        self.risk_engine.set_gateway_route_input(
            route_id,
            GatewayRouteRiskInput {
                deposit_amount: proof_envelope.amount,
                validator_confidence_bps: 8_000,
                ..Default::default()
            },
        );
        self.risk_engine
            .enforce_gateway_risk_limits(route_id, proof_envelope.amount)
            .map_err(|report| {
                self.indexer.index_gateway_risk_report(&report);
                GatewayError::RiskBlocked(report.reasons)
            })?;
        let report = self.risk_engine.evaluate_gateway_route_risk(route_id);
        self.indexer.index_gateway_risk_report(&report);

        let verification = self.verify_deposit_proof(&route, &proof_envelope);
        self.indexer.index_verification_result(&verification);
        if !verification.verified {
            return Err(GatewayError::VerificationFailed(
                verification
                    .failure_reason
                    .unwrap_or_else(|| "unverified_proof".to_string()),
            ));
        }

        let status = if route.require_dispute_window {
            let window = self
                .dispute_engine
                .open_dispute_window(proof_envelope.proof_id, self.current_block);
            self.indexer.index_dispute_window(&window);
            GatewayTransferStatus::DisputeWindowOpen
        } else {
            GatewayTransferStatus::Verified
        };

        let external_nonce_key = proof_envelope.external_nonce_key();
        let sender = proof_envelope.sender.clone();
        let recipient = proof_envelope.recipient.clone();

        let transfer = GatewayTransfer {
            transfer_id: proof_envelope.proof_id,
            route_id,
            proof_id: proof_envelope.proof_id,
            x3_asset_id: route.x3_asset_id,
            sender: sender.clone(),
            recipient: recipient.clone(),
            amount: proof_envelope.amount,
            status,
        };
        self.indexer
            .index_gateway_transfer(GatewayTransferIndexRecord {
                transfer_id: transfer.transfer_id,
                source_chain: proof_envelope.source_chain,
                destination_domain: route.destination_domain,
                external_asset: proof_envelope.external_asset.clone(),
                x3_asset_id: route.x3_asset_id,
                sender,
                recipient,
                amount: proof_envelope.amount,
                status,
                source_tx_hash: proof_envelope.source_tx_hash,
                proof_id: proof_envelope.proof_id,
                created_block: self.current_block,
                finalized_block: proof_envelope.finalized_at_block,
            });
        self.used_proofs.insert(transfer.proof_id);
        self.used_external_nonces.insert(external_nonce_key);
        self.external_locked
            .entry(transfer.x3_asset_id)
            .and_modify(|value| *value = value.saturating_add(transfer.amount))
            .or_insert(transfer.amount);
        self.transfers
            .insert(transfer.transfer_id, transfer.clone());
        Ok(transfer)
    }

    pub fn credit_x3_representation(
        &mut self,
        transfer_id: TransferId,
    ) -> Result<(), GatewayError> {
        let (asset_id, recipient, amount) = {
            let transfer = self
                .transfers
                .get(&transfer_id)
                .ok_or(GatewayError::MissingTransfer)?;
            if transfer.status == GatewayTransferStatus::DisputeWindowOpen {
                return Err(GatewayError::DisputeWindowOpen);
            }
            if transfer.status != GatewayTransferStatus::Verified {
                return Err(GatewayError::VerificationFailed(
                    "transfer_not_verified".to_string(),
                ));
            }
            (
                transfer.x3_asset_id,
                transfer.recipient.clone(),
                transfer.amount,
            )
        };
        self.ledger.credit_x3(asset_id, &recipient, amount)?;
        if let Some(transfer) = self.transfers.get_mut(&transfer_id) {
            transfer.status = GatewayTransferStatus::X3Credited;
        }
        self.indexer
            .update_transfer_status(transfer_id, GatewayTransferStatus::X3Credited);
        self.check_external_collateral_invariant(asset_id)?;
        Ok(())
    }

    pub fn finalize_after_dispute_window(
        &mut self,
        transfer_id: TransferId,
        now: BlockNumber,
    ) -> Result<(), GatewayError> {
        self.dispute_engine
            .finalize_after_dispute_window(transfer_id, now)?;
        if let Some(status) = self.dispute_engine.get_dispute_status(transfer_id) {
            self.indexer.update_dispute_status(transfer_id, status);
        }
        let transfer = self
            .transfers
            .get_mut(&transfer_id)
            .ok_or(GatewayError::MissingTransfer)?;
        transfer.status = GatewayTransferStatus::Verified;
        self.indexer
            .update_transfer_status(transfer_id, GatewayTransferStatus::Verified);
        Ok(())
    }

    pub fn request_external_withdrawal(
        &mut self,
        x3_asset_id: AssetId,
        source_domain: impl Into<String>,
        destination_chain: ExternalChainId,
        recipient: impl Into<String>,
        amount: Balance,
    ) -> WithdrawalId {
        let recipient = recipient.into();
        let withdrawal_id =
            Self::derive_withdrawal_id(x3_asset_id, &recipient, amount, self.current_block);
        self.withdrawals.insert(
            withdrawal_id,
            WithdrawalRecord {
                withdrawal_id,
                x3_asset_id,
                source_domain: source_domain.into(),
                destination_chain,
                recipient,
                amount,
                burned: false,
                released: false,
            },
        );
        withdrawal_id
    }

    pub fn burn_x3_representation(
        &mut self,
        withdrawal_id: WithdrawalId,
        owner: &str,
    ) -> Result<(), GatewayError> {
        let (asset_id, amount) = {
            let withdrawal = self
                .withdrawals
                .get(&withdrawal_id)
                .ok_or(GatewayError::MissingWithdrawal)?;
            (withdrawal.x3_asset_id, withdrawal.amount)
        };
        self.ledger.burn_x3(asset_id, owner, amount)?;
        if let Some(withdrawal) = self.withdrawals.get_mut(&withdrawal_id) {
            withdrawal.burned = true;
        }
        self.pending_withdrawals
            .entry(asset_id)
            .and_modify(|value| *value = value.saturating_add(amount))
            .or_insert(amount);
        self.check_external_collateral_invariant(asset_id)?;
        Ok(())
    }

    pub fn finalize_external_release(
        &mut self,
        withdrawal_id: WithdrawalId,
    ) -> Result<(), GatewayError> {
        let (asset_id, amount) = {
            let withdrawal = self
                .withdrawals
                .get(&withdrawal_id)
                .ok_or(GatewayError::MissingWithdrawal)?;
            if withdrawal.released {
                return Err(GatewayError::ReleaseReplay);
            }
            if !withdrawal.burned {
                return Err(GatewayError::VerificationFailed(
                    "withdrawal_not_burned".to_string(),
                ));
            }
            (withdrawal.x3_asset_id, withdrawal.amount)
        };
        if let Some(withdrawal) = self.withdrawals.get_mut(&withdrawal_id) {
            withdrawal.released = true;
        }
        self.external_locked
            .entry(asset_id)
            .and_modify(|value| *value = value.saturating_sub(amount));
        self.pending_withdrawals
            .entry(asset_id)
            .and_modify(|value| *value = value.saturating_sub(amount));
        self.check_external_collateral_invariant(asset_id)?;
        Ok(())
    }

    pub fn get_gateway_transfer(&self, transfer_id: TransferId) -> Option<&GatewayTransfer> {
        self.transfers.get(&transfer_id)
    }

    pub fn get_withdrawal(&self, withdrawal_id: WithdrawalId) -> Option<&WithdrawalRecord> {
        self.withdrawals.get(&withdrawal_id)
    }

    pub fn external_locked(&self, asset_id: AssetId) -> Balance {
        self.external_locked.get(&asset_id).copied().unwrap_or(0)
    }

    fn verify_deposit_proof(
        &mut self,
        route: &GatewayRouteConfig,
        proof: &ProofEnvelope,
    ) -> VerificationResult {
        self.verification_router
            .route_verification_request(VerificationRequest {
                proof_id: proof.proof_id,
                source_chain: proof.source_chain,
                source_block: proof.source_block,
                source_tx_hash: proof.source_tx_hash,
                external_asset: proof.external_asset.clone(),
                sender: proof.sender.clone(),
                recipient: proof.recipient.clone(),
                amount: proof.amount,
                nonce: proof.nonce,
                proof_payload: proof.proof_payload.clone(),
                strategy: route.verification_level,
            })
    }

    fn validate_proof_against_route(
        route: &GatewayRouteConfig,
        proof: &ProofEnvelope,
    ) -> Result<(), GatewayError> {
        if proof.source_chain != route.external_chain_id {
            return Err(GatewayError::WrongChain);
        }
        if proof.external_asset.token_address_or_mint != route.external_asset.token_address_or_mint
        {
            return Err(GatewayError::WrongToken);
        }
        if proof.amount < route.min_amount || proof.amount > route.max_amount {
            return Err(GatewayError::WrongAmount);
        }
        if proof.recipient.is_empty() {
            return Err(GatewayError::WrongRecipient);
        }
        if proof.finalized_at_block == 0 || proof.finalized_at_block < proof.source_block {
            return Err(GatewayError::UnfinalizedProof);
        }
        if route.mode == GatewayMode::DryRun {
            return Err(GatewayError::Registry(RegistryError::DryRunCannotCredit));
        }
        Ok(())
    }

    fn check_external_collateral_invariant(&self, asset_id: AssetId) -> Result<(), GatewayError> {
        let external_locked = self.external_locked(asset_id);
        let represented = self.ledger.represented_supply(asset_id);
        let pending = self
            .pending_withdrawals
            .get(&asset_id)
            .copied()
            .unwrap_or(0);
        if external_locked < represented.saturating_add(pending) {
            return Err(GatewayError::InvariantViolation);
        }
        Ok(())
    }

    fn derive_withdrawal_id(
        asset_id: AssetId,
        recipient: &str,
        amount: Balance,
        block: BlockNumber,
    ) -> WithdrawalId {
        let mut out = asset_id;
        for (idx, byte) in recipient.as_bytes().iter().enumerate() {
            out[idx % 32] ^= *byte;
        }
        for (idx, byte) in amount.to_be_bytes().iter().enumerate() {
            out[idx] ^= *byte;
        }
        for (idx, byte) in block.to_be_bytes().iter().enumerate() {
            out[24 + idx] ^= *byte;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_circuit_breaker::CircuitBreakerScope;
    use x3_external_route_registry::{GatewayMode, X3Domain};
    use x3_gateway_insurance::InsuranceFundStatus;
    use x3_validator_attestation::ValidatorId;
    use x3_verification_router::{ExternalAssetRef, VerificationStrategy};

    fn asset() -> ExternalAssetRef {
        ExternalAssetRef {
            chain_id: ExternalChainId::BaseSepolia,
            token_address_or_mint: "0xmock".to_string(),
            decimals: 18,
            symbol: "MOCK".to_string(),
        }
    }

    fn route(mode: GatewayMode) -> GatewayRouteConfig {
        GatewayRouteConfig {
            route_id: [1; 32],
            external_chain_id: ExternalChainId::BaseSepolia,
            external_asset: asset(),
            x3_asset_id: [9; 32],
            destination_domain: X3Domain::Native,
            enabled: false,
            min_amount: 1,
            max_amount: 1_000,
            daily_limit: 10_000,
            pending_limit: 10,
            finality_requirement: 32,
            verification_level: VerificationStrategy::ValidatorQuorum,
            fee_bps: 10,
            mode,
            require_dispute_window: false,
        }
    }

    fn proof(amount: u128, nonce: u64) -> ProofEnvelope {
        ProofEnvelope {
            version: 1,
            proof_id: ProofEnvelope::deterministic_proof_id(
                ExternalChainId::BaseSepolia,
                [7; 32],
                0,
                nonce,
            ),
            source_chain: ExternalChainId::BaseSepolia,
            source_block: 10,
            source_tx_hash: [7; 32],
            event_index: 0,
            external_asset: asset(),
            sender: "external-alice".to_string(),
            recipient: "bob".to_string(),
            amount,
            nonce,
            observed_at_block: 20,
            finalized_at_block: 42,
            proof_payload: vec![1],
        }
    }

    fn gateway() -> CrosschainGateway<InMemoryGatewayLedger> {
        let mut gateway = CrosschainGateway::new(InMemoryGatewayLedger::default(), 50);
        gateway.register_external_asset(asset(), [9; 32]);
        gateway
            .enable_gateway_route(route(GatewayMode::TestnetLive))
            .unwrap();
        gateway
    }

    fn validator_set() -> ValidatorSet {
        ValidatorSet {
            set_id: 1,
            validators: vec![
                ValidatorId("alice".to_string()),
                ValidatorId("bob".to_string()),
                ValidatorId("carol".to_string()),
            ],
            threshold: 2,
            active_from_block: 1,
            active_until_block: 1_000,
        }
    }

    fn attestation(proof_id: ProofId) -> GatewayAttestationSet {
        GatewayAttestationSet {
            proof_id,
            source_chain: 1,
            source_tx_hash: [7; 32],
            event_hash: [6; 32],
            signers: vec![
                ValidatorId("alice".to_string()),
                ValidatorId("bob".to_string()),
            ],
            signatures: vec![vec![1], vec![2]],
            threshold: 2,
            created_at_block: 50,
        }
    }

    #[test]
    fn verified_deposit_credits_through_ledger() {
        let mut gateway = gateway();
        let transfer = gateway
            .submit_deposit_proof([1; 32], proof(100, 1))
            .unwrap();
        gateway
            .credit_x3_representation(transfer.transfer_id)
            .unwrap();

        assert_eq!(gateway.ledger.balance([9; 32], "bob"), 100);
        assert_eq!(gateway.external_locked([9; 32]), 100);
        assert_eq!(
            gateway
                .get_gateway_transfer(transfer.transfer_id)
                .unwrap()
                .status,
            GatewayTransferStatus::X3Credited
        );
        assert!(
            gateway
                .indexer
                .get_verification_result(transfer.proof_id)
                .unwrap()
                .verified
        );
        assert!(gateway.indexer.get_gateway_risk_report([1; 32]).is_some());
    }

    #[test]
    fn validator_attestation_is_required_directly_for_attested_deposit() {
        let mut gateway = gateway();
        gateway.register_validator_set(validator_set());
        let proof = proof(100, 1);
        let proof_id = proof.proof_id;
        let transfer = gateway
            .submit_attested_deposit_proof([1; 32], proof, 1, attestation(proof_id))
            .unwrap();

        assert_eq!(transfer.status, GatewayTransferStatus::Verified);
        assert!(gateway
            .attestation_engine
            .get_attestation_status(proof_id)
            .is_some());
    }

    #[test]
    fn below_threshold_attestation_blocks_deposit() {
        let mut gateway = gateway();
        gateway.register_validator_set(validator_set());
        let proof = proof(100, 1);
        let mut attestation = attestation(proof.proof_id);
        attestation.signers.pop();
        attestation.signatures.pop();

        assert!(matches!(
            gateway.submit_attested_deposit_proof([1; 32], proof, 1, attestation),
            Err(GatewayError::Attestation(AttestationError::BelowThreshold))
        ));
    }

    #[test]
    fn replayed_proof_and_nonce_fail() {
        let mut gateway = gateway();
        gateway
            .submit_deposit_proof([1; 32], proof(100, 1))
            .unwrap();

        assert_eq!(
            gateway
                .submit_deposit_proof([1; 32], proof(100, 1))
                .unwrap_err(),
            GatewayError::ProofReplay
        );

        let mut second = proof(100, 1);
        second.proof_id = [2; 32];
        assert_eq!(
            gateway.submit_deposit_proof([1; 32], second).unwrap_err(),
            GatewayError::ExternalNonceReplay
        );
    }

    #[test]
    fn wrong_chain_token_amount_and_unfinalized_fail() {
        let mut gateway = gateway();
        let mut wrong_chain = proof(100, 2);
        wrong_chain.source_chain = ExternalChainId::EthereumSepolia;
        assert_eq!(
            gateway
                .submit_deposit_proof([1; 32], wrong_chain)
                .unwrap_err(),
            GatewayError::WrongChain
        );

        let mut wrong_token = proof(100, 3);
        wrong_token.external_asset.token_address_or_mint = "0xbad".to_string();
        assert_eq!(
            gateway
                .submit_deposit_proof([1; 32], wrong_token)
                .unwrap_err(),
            GatewayError::WrongToken
        );

        assert_eq!(
            gateway
                .submit_deposit_proof([1; 32], proof(1_001, 4))
                .unwrap_err(),
            GatewayError::Registry(RegistryError::AmountAboveMaximum)
        );

        let mut unfinalized = proof(100, 5);
        unfinalized.finalized_at_block = 0;
        assert_eq!(
            gateway
                .submit_deposit_proof([1; 32], unfinalized)
                .unwrap_err(),
            GatewayError::UnfinalizedProof
        );
    }

    #[test]
    fn dry_run_cannot_credit() {
        let mut gateway = CrosschainGateway::new(InMemoryGatewayLedger::default(), 50);
        gateway.register_external_asset(asset(), [9; 32]);
        gateway
            .enable_gateway_route(route(GatewayMode::DryRun))
            .unwrap();

        assert_eq!(
            gateway
                .submit_deposit_proof([1; 32], proof(100, 1))
                .unwrap_err(),
            GatewayError::Registry(RegistryError::DryRunCannotCredit)
        );
    }

    #[test]
    fn dispute_window_delays_credit() {
        let mut gateway = CrosschainGateway::new(InMemoryGatewayLedger::default(), 50);
        let mut dispute_route = route(GatewayMode::TestnetLive);
        dispute_route.require_dispute_window = true;
        gateway.register_external_asset(asset(), [9; 32]);
        gateway.enable_gateway_route(dispute_route).unwrap();
        let transfer = gateway
            .submit_deposit_proof([1; 32], proof(100, 1))
            .unwrap();

        assert_eq!(
            gateway
                .credit_x3_representation(transfer.transfer_id)
                .unwrap_err(),
            GatewayError::DisputeWindowOpen
        );
        gateway
            .finalize_after_dispute_window(transfer.transfer_id, 60)
            .unwrap();
        gateway
            .credit_x3_representation(transfer.transfer_id)
            .unwrap();
    }

    #[test]
    fn withdrawal_burn_and_release_preserve_collateral_model() {
        let mut gateway = gateway();
        let transfer = gateway
            .submit_deposit_proof([1; 32], proof(100, 1))
            .unwrap();
        gateway
            .credit_x3_representation(transfer.transfer_id)
            .unwrap();

        let withdrawal_id = gateway.request_external_withdrawal(
            [9; 32],
            "Native",
            ExternalChainId::BaseSepolia,
            "external-bob",
            40,
        );
        gateway
            .burn_x3_representation(withdrawal_id, "bob")
            .expect("burn should debit ledger");
        gateway.finalize_external_release(withdrawal_id).unwrap();

        assert_eq!(gateway.ledger.balance([9; 32], "bob"), 60);
        assert_eq!(gateway.external_locked([9; 32]), 60);
        assert!(gateway.get_withdrawal(withdrawal_id).unwrap().released);
    }

    #[test]
    fn circuit_breaker_blocks_deposit() {
        let mut gateway = gateway();
        gateway.circuit_breakers.trip_circuit_breaker(
            CircuitBreakerScope::Route([1; 32]),
            "manual_pause",
            51,
        );

        assert_eq!(
            gateway
                .submit_deposit_proof([1; 32], proof(100, 1))
                .unwrap_err(),
            GatewayError::CircuitTripped
        );
    }

    #[test]
    fn indexes_circuit_and_insurance_records() {
        let mut gateway = gateway();
        let record = gateway.circuit_breakers.trip_circuit_breaker(
            CircuitBreakerScope::Route([1; 32]),
            "manual_pause",
            51,
        );
        gateway.index_circuit_breaker_record(&record);
        gateway
            .insurance_engine
            .create_fund([3; 32], [9; 32], 1_000);
        let fund = gateway
            .insurance_engine
            .fund_insurance([3; 32], 500)
            .unwrap();
        gateway.index_insurance_fund(&fund);
        gateway.index_route_coverage(&RouteCoverage {
            route_id: [1; 32],
            fund_id: [3; 32],
            max_covered_amount: 500,
            premium_bps: 25,
        });

        assert!(gateway
            .indexer
            .get_circuit_breaker(CircuitBreakerScope::Route([1; 32]))
            .is_some());
        assert_eq!(
            gateway.indexer.get_insurance_fund([3; 32]).unwrap().status,
            format!("{:?}", InsuranceFundStatus::Active)
        );
        assert!(gateway.indexer.get_route_coverage([1; 32]).is_some());
    }

    #[test]
    fn fuzz_gateway_state_machine_invariants() {
        for seed in 0..128u64 {
            let mut rng = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
            let mut gateway = gateway();
            let mut credited = Vec::new();
            for step in 0..120u64 {
                rng = rng.wrapping_mul(6364136223846793005).wrapping_add(1);
                let action = (rng >> 61) as u8;
                let amount = ((rng % 250) + 1) as u128;
                let nonce = seed.saturating_mul(1_000).saturating_add(step);
                match action {
                    0 | 1 => {
                        let p = proof(amount, nonce);
                        if let Ok(transfer) = gateway.submit_deposit_proof([1; 32], p) {
                            credited.push(transfer.transfer_id);
                        }
                    }
                    2 => {
                        if let Some(id) = credited.last().copied() {
                            let _ = gateway.credit_x3_representation(id);
                        }
                    }
                    3 => {
                        let p = proof(amount, nonce);
                        let _ = gateway.submit_deposit_proof([1; 32], p.clone());
                        let _ = gateway.submit_deposit_proof([1; 32], p);
                    }
                    4 => {
                        let amount = amount.min(gateway.ledger.balance([9; 32], "bob"));
                        let id = gateway.request_external_withdrawal(
                            [9; 32],
                            "Native",
                            ExternalChainId::BaseSepolia,
                            "external-bob",
                            amount,
                        );
                        let _ = gateway.burn_x3_representation(id, "bob");
                        let _ = gateway.finalize_external_release(id);
                    }
                    5 => {
                        let record = gateway.circuit_breakers.trip_circuit_breaker(
                            CircuitBreakerScope::Route([2; 32]),
                            "fuzz_unused_route",
                            step,
                        );
                        gateway.index_circuit_breaker_record(&record);
                    }
                    _ => {
                        let _ = gateway.disable_gateway_route([1; 32]);
                    }
                }
                let external_locked = gateway.external_locked([9; 32]);
                let represented = gateway.ledger.represented_supply([9; 32]);
                let pending = gateway
                    .pending_withdrawals
                    .get(&[9; 32])
                    .copied()
                    .unwrap_or(0);
                assert!(
                    external_locked >= represented.saturating_add(pending),
                    "seed {seed} step {step} broke collateral invariant"
                );
                for withdrawal in gateway.withdrawals.values() {
                    assert!(
                        !(withdrawal.released && !withdrawal.burned),
                        "seed {seed} step {step} released without burn"
                    );
                }
            }
        }
    }
}
