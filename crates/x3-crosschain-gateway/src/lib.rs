use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

use x3_asset_kernel_types::{
    traits::{SupplyLedgerGovern, SupplyLedgerWrite},
    DomainId,
};
use x3_circuit_breaker::{CircuitBreakerEngine, CircuitBreakerRecord};
use x3_external_route_registry::{
    AssetId, ExternalRouteRegistry, GatewayMode, GatewayRouteConfig, RegistryError, RouteId,
};
use x3_gateway_indexer::{
    GatewayIndexer, GatewayTransferIndexRecord, GatewayTransferStatus, TransferId,
};
use x3_gateway_insurance::{GatewayInsuranceEngine, InsuranceFund, RouteCoverage};
use x3_gateway_risk_engine::{
    GatewayRiskEngine, GatewayRiskStatus, GatewayRouteRiskReport, RiskPolicy, RouteRiskInput,
};
use x3_proof_dispute::{DisputeError, DisputeStatus, DisputeTracker, DisputeWindow};
use x3_proof_envelope::{ProofEnvelope, ProofId};
use x3_validator_attestation::ValidatorId;
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

/// A registered external validator set used for proof attestation quorum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatorSet {
    pub set_id: u64,
    pub validators: Vec<ValidatorId>,
    pub threshold: u64,
    pub active_from_block: u64,
    pub active_until_block: u64,
}

/// Signer attestations accompanying an attested deposit proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GatewayAttestationSet {
    pub proof_id: ProofId,
    pub source_chain: u64,
    pub source_tx_hash: [u8; 32],
    pub event_hash: [u8; 32],
    pub signers: Vec<ValidatorId>,
    pub signatures: Vec<Vec<u8>>,
    pub threshold: u64,
    pub created_at_block: u64,
}

/// Verdict recorded for an attested proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttestationStatus {
    Pending,
    Verified,
    QuorumReached,
    BelowThreshold,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttestationError {
    EmptySignature,
    DuplicateValidator,
    WrongEventHash,
    BelowThreshold,
}

/// Tracks registered validator sets and per-proof attestation quorum.
#[derive(Debug, Default)]
pub struct ValidatorAttestationEngine {
    sets: HashMap<u64, ValidatorSet>,
    statuses: HashMap<ProofId, AttestationStatus>,
}

impl ValidatorAttestationEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_validator_set(&mut self, vs: ValidatorSet) {
        self.sets.insert(vs.set_id, vs);
    }

    pub fn get_validator_set(&self, set_id: u64) -> Option<&ValidatorSet> {
        self.sets.get(&set_id)
    }

    pub fn get_attestation_status(&self, proof_id: ProofId) -> Option<AttestationStatus> {
        self.statuses.get(&proof_id).copied()
    }

    /// Verify a signer list reaches quorum against the registered set.
    pub fn verify_quorum(
        &mut self,
        set_id: u64,
        proof_id: ProofId,
        attestation: &GatewayAttestationSet,
    ) -> Result<(), AttestationError> {
        if attestation.proof_id != proof_id {
            return Err(AttestationError::WrongEventHash);
        }
        let set = self
            .sets
            .get(&set_id)
            .cloned()
            .ok_or(AttestationError::BelowThreshold)?;
        if attestation.signers.len() != attestation.signatures.len() {
            return Err(AttestationError::EmptySignature);
        }
        let mut signers = HashSet::new();
        let threshold = set.threshold.max(attestation.threshold);
        for (signer, sig) in attestation.signers.iter().zip(attestation.signatures.iter()) {
            if sig.is_empty() {
                return Err(AttestationError::EmptySignature);
            }
            if !set.validators.contains(signer) {
                return Err(AttestationError::DuplicateValidator);
            }
            if !signers.insert(signer.0.clone()) {
                return Err(AttestationError::DuplicateValidator);
            }
        }
        if (signers.len() as u64) < threshold {
            self.statuses.insert(proof_id, AttestationStatus::BelowThreshold);
            return Err(AttestationError::BelowThreshold);
        }
        self.statuses.insert(proof_id, AttestationStatus::QuorumReached);
        Ok(())
    }

    pub fn mark_verified(&mut self, proof_id: ProofId) {
        self.statuses.insert(proof_id, AttestationStatus::Verified);
    }
}

pub struct CrosschainGateway<L: SupplyLedgerGateway> {
    pub registry: ExternalRouteRegistry,
    pub verification_router: VerificationRouter,
    pub attestation_engine: ValidatorAttestationEngine,
    pub risk_engine: GatewayRiskEngine,
    pub circuit_breakers: CircuitBreakerEngine,
    pub insurance_engine: GatewayInsuranceEngine,
    pub indexer: GatewayIndexer,
    pub ledger: L,
    transfers: HashMap<TransferId, GatewayTransfer>,
    withdrawals: HashMap<WithdrawalId, WithdrawalRecord>,
    used_proofs: HashSet<ProofId>,
    used_external_nonces: HashSet<(ExternalChainId, String, u64)>,
    external_locked: HashMap<AssetId, Balance>,
    pending_withdrawals: HashMap<AssetId, Balance>,
    // Per-route risk limits (thin risk engine is global-policy; static route
    // limits are enforced here against the authoritative GatewayRouteConfig).
    route_risk_limits: HashMap<RouteId, RouteRiskLimit>,
    // Per-proof dispute trackers (thin upstream API is one tracker per proof).
    disputes: HashMap<ProofId, DisputeTracker>,
    current_block: BlockNumber,
}

/// Static route risk limits derived from a `GatewayRouteConfig`.
#[derive(Clone, Debug)]
struct RouteRiskLimit {
    max_single: u128,
    per_route_pending: u32,
    route_value_usd: u128,
}

impl<L: SupplyLedgerGateway> CrosschainGateway<L> {
    pub fn new(ledger: L, current_block: BlockNumber) -> Self {
        Self {
            registry: ExternalRouteRegistry::new(),
            verification_router: VerificationRouter::at_block(current_block),
            attestation_engine: ValidatorAttestationEngine::new(),
            risk_engine: GatewayRiskEngine::new(RiskPolicy::default()),
            circuit_breakers: CircuitBreakerEngine::new(),
            insurance_engine: GatewayInsuranceEngine::new(),
            indexer: GatewayIndexer::new(),
            ledger,
            transfers: HashMap::new(),
            withdrawals: HashMap::new(),
            used_proofs: HashSet::new(),
            used_external_nonces: HashSet::new(),
            external_locked: HashMap::new(),
            pending_withdrawals: HashMap::new(),
            route_risk_limits: HashMap::new(),
            disputes: HashMap::new(),
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
        // Static per-route limits are enforced by the gateway against the
        // authoritative route config; the thin upstream risk engine only
        // applies a global policy at transfer time (see submit_deposit_proof).
        self.route_risk_limits.insert(
            route_config.route_id,
            RouteRiskLimit {
                max_single: route_config.max_amount,
                per_route_pending: route_config.pending_limit,
                route_value_usd: route_config.daily_limit,
            },
        );
        self.registry.enable_gateway_route(route_config.clone())?;
        let indexed = self
            .registry
            .get_gateway_route(route_config.route_id)
            .expect("route just inserted");
        self.indexer.index_gateway_route(indexed);
        Ok(())
    }

    pub fn disable_gateway_route(&mut self, route_id: RouteId) -> Result<(), GatewayError> {
        self.route_risk_limits.remove(&route_id);
        self.registry.disable_gateway_route(route_id)?;
        Ok(())
    }

    pub fn register_validator_set(&mut self, validator_set: ValidatorSet) {
        self.attestation_engine.register_validator_set(validator_set);
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
        let route = self
            .registry
            .get_gateway_route(route_id)
            .ok_or(RegistryError::RouteNotFound)?;
        if !matches!(route.verification_level, VerificationStrategy::ValidatorQuorum { .. }) {
            return Err(GatewayError::VerificationFailed(
                "route_does_not_accept_validator_quorum".to_string(),
            ));
        }
        // Verify quorum before accepting the proven deposit.
        self.attestation_engine.verify_quorum(
            validator_set_id,
            proof_envelope.proof_id,
            &attestation,
        )?;
        self.attestation_engine.mark_verified(proof_envelope.proof_id);
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

        // Route risk check. The upstream risk engine is a thin global-policy
        // evaluator; the gateway enforces its per-route static limits and
        // delegates the policy decision (value / failures) to it.
        let route_limit = self.route_risk_limits.get(&route_id).cloned();
        if let Some(limit) = &route_limit {
            if proof_envelope.amount > limit.max_single {
                return Err(GatewayError::RiskBlocked(vec![
                    "amount exceeds per-route maximum".to_string(),
                ]));
            }
            // Enforce the per-route pending cap (in-flight + dispute-open
            // transfers) and the running value cap recorded on the route.
            let mut pending = 0u32;
            let mut outstanding_value = 0u128;
            for t in self.transfers.values() {
                if t.route_id != route_id {
                    continue;
                }
                match t.status {
                    GatewayTransferStatus::X3Credited
                    | GatewayTransferStatus::X3Burned
                    | GatewayTransferStatus::ExternalReleased
                    | GatewayTransferStatus::Expired
                    | GatewayTransferStatus::Refunded
                    | GatewayTransferStatus::Failed => {}
                    _ => {
                        pending = pending.saturating_add(1);
                        outstanding_value =
                            outstanding_value.saturating_add(t.amount);
                    }
                }
            }
            if pending >= limit.per_route_pending {
                return Err(GatewayError::RiskBlocked(vec![format!(
                    "route pending transfer cap reached ({})",
                    limit.per_route_pending
                )]));
            }
            if outstanding_value.saturating_add(proof_envelope.amount)
                > limit.route_value_usd
            {
                return Err(GatewayError::RiskBlocked(vec![format!(
                    "route outstanding value would exceed cap ({})",
                    limit.route_value_usd
                )]));
            }
        }
        let decision = self.risk_engine.evaluate(RouteRiskInput {
            value_usd: u64::try_from(proof_envelope.amount).unwrap_or(u64::MAX / 4),
            recent_failures: 0,
            verifier_quorum_met: true,
        });
        if !decision.allow_route {
            self.indexer.index_gateway_risk_report(&GatewayRouteRiskReport {
                route_id,
                status: GatewayRiskStatus::High,
                allow_transfer: false,
                reasons: vec![decision.reason.clone()],
            });
            return Err(GatewayError::RiskBlocked(vec![decision.reason]));
        }
        self.indexer.index_gateway_risk_report(&GatewayRouteRiskReport {
            route_id,
            status: GatewayRiskStatus::Low,
            allow_transfer: true,
            reasons: Vec::new(),
        });

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
            let window_blocks: u64 = 10;
            let tracker = DisputeTracker::new(
                proof_envelope.proof_id,
                self.current_block,
                window_blocks,
            )
            .map_err(GatewayError::from)?;
            self.disputes
                .insert(proof_envelope.proof_id, tracker);
            self.indexer.index_dispute_window(&DisputeWindow {
                proof_id: proof_envelope.proof_id,
                opens_at_block: self.current_block,
                closes_at_block: self
                    .current_block
                    .saturating_add(window_blocks),
                status: DisputeStatus::Open,
            });
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
        // If a dispute tracker was opened for this proof, it must be closed
        // once its window has elapsed for the transfer to finalize.
        if let Some(mut tracker) = self.disputes.remove(&transfer_id) {
            // No dispute votes recorded by default => an undisputed window
            // closes cleanly once `now` reaches `close_after`.
            let _ = tracker.close(now, 0).map_err(GatewayError::from)?;
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
    use std::sync::Arc;
    use x3_circuit_breaker::CircuitBreakerScope;
    use x3_external_route_registry::{GatewayMode, X3Domain};
    use x3_gateway_insurance::InsuranceFundStatus;
    use x3_validator_attestation::ValidatorId;
    use x3_verification_router::{ExternalAssetRef, ValidatorQuorumVerifier, VerificationStrategy};

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
            verification_level: VerificationStrategy::ValidatorQuorum { threshold: 2, total: 3 },
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
        register_default_verifiers(&mut gateway);
        gateway.register_external_asset(asset(), [9; 32]);
        gateway
            .enable_gateway_route(route(GatewayMode::TestnetLive))
            .unwrap();
        gateway
    }

    /// The production caller supplies a pre-configured verification router with
    /// verifiers registered for the route strategies. Tests emulate that here so
    /// deposits on the default ValidatorQuorum route can actually verify.
    fn register_default_verifiers(gateway: &mut CrosschainGateway<InMemoryGatewayLedger>) {
        gateway.verification_router.register_verifier(Arc::new(
            ValidatorQuorumVerifier::new(2, 3),
        ));
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
        register_default_verifiers(&mut gateway);
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
                        !withdrawal.released || withdrawal.burned,
                        "seed {seed} step {step} released without burn"
                    );
                }
            }
        }
    }
}
