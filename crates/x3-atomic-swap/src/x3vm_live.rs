//! # Live X3VM atomic-swap transport boundary
//!
//! This module is the production-facing counterpart to `x3vm_htlc`'s offline
//! simulator. It never derives transaction identifiers, block hashes, or proof
//! bytes locally. A host-provided [`X3VmLiveTransport`] must submit/read the
//! native X3 chain and return canonical proof objects sourced from that chain.
//!
//! The adapter validates domain identity and proof completeness before exposing
//! transport results to the relayer. It intentionally keeps the
//! [`X3VmAdapter::is_simulated`] safe default until repository-level live-node,
//! key-separation, and failure-path gates explicitly promote it.

use crate::adapter::{
    AdapterReadinessScore, AssetId, ChainHealth, ChainId, ClaimProof, FeeEstimate, FinalityProof,
    LockProof, RefundProof, TxId, VmType, X3VmAdapter,
};
use crate::error::SwapError;
use crate::intent::{AtomicIntent, IntentId};
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Debug;

/// Host boundary for real X3VM lock/claim/refund execution and proof retrieval.
///
/// Implementations own the actual RPC/indexer connection and signing boundary.
/// Private keys must remain outside this crate. Every lifecycle method must
/// return values taken from finalized chain data; implementations must not
/// fabricate transaction ids, block hashes, or `raw_proof` bytes.
pub trait X3VmLiveTransport: Send + Sync + Debug {
    fn lock(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent: &AtomicIntent,
    ) -> Result<LockProof, SwapError>;

    fn claim(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent_id: IntentId,
        preimage: [u8; 32],
    ) -> Result<ClaimProof, SwapError>;

    fn refund(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent_id: IntentId,
    ) -> Result<RefundProof, SwapError>;

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError>;
    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError>;
    fn verify_refund(&self, proof: &RefundProof) -> Result<bool, SwapError>;

    fn estimate_fee(
        &self,
        chain_id: &ChainId,
        escrow_address: &[u8],
        intent: &AtomicIntent,
    ) -> Result<FeeEstimate, SwapError>;

    fn finality_status(
        &self,
        chain_id: &ChainId,
        tx_id: &TxId,
    ) -> Result<FinalityProof, SwapError>;

    fn chain_health(&self, chain_id: &ChainId) -> Result<ChainHealth, SwapError>;

    /// Capability report from the concrete transport.
    ///
    /// This is informational in this slice: `LiveX3VmAdapter` deliberately does
    /// not override `X3VmAdapter::is_simulated`, so the production relayer gate
    /// remains closed until live-node evidence is wired and audited.
    fn readiness_score(&self) -> AdapterReadinessScore;
}

/// Production-pluggable X3VM adapter.
///
/// Unlike `X3VmAdapterImpl::simulation`, this type contains no proof-generation
/// fallback. If its transport cannot execute or prove an operation, that error
/// is returned to the caller unchanged.
#[derive(Debug, Clone)]
pub struct LiveX3VmAdapter<T: X3VmLiveTransport> {
    chain_id: ChainId,
    escrow_address: Vec<u8>,
    transport: T,
}

impl<T: X3VmLiveTransport> LiveX3VmAdapter<T> {
    pub fn new(chain_id: ChainId, escrow_address: Vec<u8>, transport: T) -> Self {
        Self {
            chain_id,
            escrow_address,
            transport,
        }
    }

    pub fn chain_id(&self) -> &ChainId {
        &self.chain_id
    }

    pub fn escrow_address(&self) -> &[u8] {
        &self.escrow_address
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    fn invalid_proof(kind: &str, reason: &str) -> SwapError {
        SwapError::Internal(alloc::format!(
            "X3VM live {} proof rejected: {}",
            kind,
            reason
        ))
    }

    fn validate_common(
        &self,
        kind: &str,
        chain_id: &ChainId,
        vm_type: VmType,
        tx_id: &str,
        block_hash: &str,
        raw_proof: &[u8],
    ) -> Result<(), SwapError> {
        if vm_type != VmType::X3Vm {
            return Err(Self::invalid_proof(kind, "vm_type is not X3Vm"));
        }
        if chain_id != &self.chain_id {
            return Err(Self::invalid_proof(kind, "chain_id does not match adapter"));
        }
        if tx_id.is_empty() {
            return Err(Self::invalid_proof(kind, "transaction id is empty"));
        }
        if block_hash.is_empty() {
            return Err(Self::invalid_proof(kind, "block hash is empty"));
        }
        if raw_proof.is_empty() {
            return Err(Self::invalid_proof(kind, "raw proof is empty"));
        }
        Ok(())
    }

    fn validate_lock(&self, proof: &LockProof) -> Result<(), SwapError> {
        self.validate_common(
            "lock",
            &proof.chain_id,
            proof.vm_type,
            &proof.tx_id,
            &proof.block_hash,
            &proof.raw_proof,
        )?;
        if proof.lock_address.is_empty() {
            return Err(Self::invalid_proof("lock", "lock address is empty"));
        }
        if proof.locked_amount == 0 {
            return Err(Self::invalid_proof("lock", "locked amount is zero"));
        }
        if proof.timeout == 0 {
            return Err(Self::invalid_proof("lock", "timeout is zero"));
        }
        Ok(())
    }

    fn validate_claim(&self, proof: &ClaimProof) -> Result<(), SwapError> {
        self.validate_common(
            "claim",
            &proof.chain_id,
            proof.vm_type,
            &proof.tx_id,
            &proof.block_hash,
            &proof.raw_proof,
        )
    }

    fn validate_refund(&self, proof: &RefundProof) -> Result<(), SwapError> {
        self.validate_common(
            "refund",
            &proof.chain_id,
            proof.vm_type,
            &proof.tx_id,
            &proof.block_hash,
            &proof.raw_proof,
        )
    }

    fn validate_finality(&self, proof: &FinalityProof, tx_id: &TxId) -> Result<(), SwapError> {
        if proof.vm_type != VmType::X3Vm {
            return Err(Self::invalid_proof("finality", "vm_type is not X3Vm"));
        }
        if proof.chain_id != self.chain_id {
            return Err(Self::invalid_proof(
                "finality",
                "chain_id does not match adapter",
            ));
        }
        if &proof.tx_id != tx_id {
            return Err(Self::invalid_proof(
                "finality",
                "transaction id does not match query",
            ));
        }
        if proof.block_hash.is_empty() {
            return Err(Self::invalid_proof("finality", "block hash is empty"));
        }
        Ok(())
    }

    fn validate_health(&self, health: &ChainHealth) -> Result<(), SwapError> {
        if health.vm_type != VmType::X3Vm {
            return Err(SwapError::Internal(
                "X3VM live health rejected: vm_type is not X3Vm".into(),
            ));
        }
        if health.chain_id != self.chain_id {
            return Err(SwapError::Internal(
                "X3VM live health rejected: chain_id does not match adapter".into(),
            ));
        }
        Ok(())
    }

    fn validate_fee(&self, fee: &FeeEstimate) -> Result<(), SwapError> {
        if fee.vm_type != VmType::X3Vm {
            return Err(SwapError::Internal(
                "X3VM live fee rejected: vm_type is not X3Vm".into(),
            ));
        }
        if fee.chain_id != self.chain_id {
            return Err(SwapError::Internal(
                "X3VM live fee rejected: chain_id does not match adapter".into(),
            ));
        }
        Ok(())
    }
}

impl<T: X3VmLiveTransport> X3VmAdapter for LiveX3VmAdapter<T> {
    fn vm_type(&self) -> VmType {
        VmType::X3Vm
    }

    fn adapter_name(&self) -> &'static str {
        "x3-adapter-x3vm-live"
    }

    fn supported_chains(&self) -> Vec<ChainId> {
        vec![self.chain_id.clone()]
    }

    fn supported_assets(&self) -> Vec<AssetId> {
        vec![String::from("X3"), String::from("aX3")]
    }

    fn lock(&self, intent: &AtomicIntent) -> Result<LockProof, SwapError> {
        let proof = self
            .transport
            .lock(&self.chain_id, &self.escrow_address, intent)?;
        self.validate_lock(&proof)?;
        Ok(proof)
    }

    fn claim(&self, intent_id: IntentId, preimage: [u8; 32]) -> Result<ClaimProof, SwapError> {
        let proof = self.transport.claim(
            &self.chain_id,
            &self.escrow_address,
            intent_id,
            preimage,
        )?;
        self.validate_claim(&proof)?;
        if proof.intent_id != intent_id {
            return Err(Self::invalid_proof(
                "claim",
                "intent id does not match request",
            ));
        }
        if proof.preimage != preimage {
            return Err(Self::invalid_proof(
                "claim",
                "preimage does not match request",
            ));
        }
        Ok(proof)
    }

    fn refund(&self, intent_id: IntentId) -> Result<RefundProof, SwapError> {
        let proof = self
            .transport
            .refund(&self.chain_id, &self.escrow_address, intent_id)?;
        self.validate_refund(&proof)?;
        if proof.intent_id != intent_id {
            return Err(Self::invalid_proof(
                "refund",
                "intent id does not match request",
            ));
        }
        Ok(proof)
    }

    fn verify_lock(&self, proof: &LockProof) -> Result<bool, SwapError> {
        self.validate_lock(proof)?;
        self.transport.verify_lock(proof)
    }

    fn verify_claim(&self, proof: &ClaimProof) -> Result<bool, SwapError> {
        self.validate_claim(proof)?;
        self.transport.verify_claim(proof)
    }

    fn verify_refund(&self, proof: &RefundProof) -> Result<bool, SwapError> {
        self.validate_refund(proof)?;
        self.transport.verify_refund(proof)
    }

    fn estimate_fee(&self, intent: &AtomicIntent) -> Result<FeeEstimate, SwapError> {
        let fee = self
            .transport
            .estimate_fee(&self.chain_id, &self.escrow_address, intent)?;
        self.validate_fee(&fee)?;
        Ok(fee)
    }

    fn finality_status(&self, tx_id: &TxId) -> Result<FinalityProof, SwapError> {
        let proof = self.transport.finality_status(&self.chain_id, tx_id)?;
        self.validate_finality(&proof, tx_id)?;
        Ok(proof)
    }

    fn chain_health(&self) -> Result<ChainHealth, SwapError> {
        let health = self.transport.chain_health(&self.chain_id)?;
        self.validate_health(&health)?;
        Ok(health)
    }

    fn readiness_score(&self) -> AdapterReadinessScore {
        let mut score = self.transport.readiness_score();
        // This adapter is X3VM-specific even if a buggy transport reports a
        // different VM type. Keep the dashboard domain truthful.
        score.adapter_name = "x3-adapter-x3vm-live";
        score.vm_type = VmType::X3Vm;
        score
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intent::{
        AtomicSwapStatus, ChainKind, FinalityLevel, FinalityRequirement, RefundPath, RouteMode,
    };
    use core::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[derive(Debug, Default)]
    struct Counts {
        lock: AtomicUsize,
        claim: AtomicUsize,
        refund: AtomicUsize,
        verify: AtomicUsize,
        fee: AtomicUsize,
        finality: AtomicUsize,
        health: AtomicUsize,
    }

    #[derive(Debug, Clone)]
    struct RecordingTransport {
        counts: Arc<Counts>,
        chain_id: ChainId,
        wrong_vm: bool,
        wrong_chain: bool,
    }

    impl RecordingTransport {
        fn good(chain_id: &str) -> Self {
            Self {
                counts: Arc::new(Counts::default()),
                chain_id: chain_id.into(),
                wrong_vm: false,
                wrong_chain: false,
            }
        }

        fn proof_chain(&self) -> ChainId {
            if self.wrong_chain {
                "x3-wrong".into()
            } else {
                self.chain_id.clone()
            }
        }

        fn proof_vm(&self) -> VmType {
            if self.wrong_vm {
                VmType::Evm
            } else {
                VmType::X3Vm
            }
        }
    }

    impl X3VmLiveTransport for RecordingTransport {
        fn lock(
            &self,
            _chain_id: &ChainId,
            _escrow_address: &[u8],
            intent: &AtomicIntent,
        ) -> Result<LockProof, SwapError> {
            self.counts.lock.fetch_add(1, Ordering::SeqCst);
            Ok(LockProof {
                tx_id: "0xlive-lock".into(),
                chain_id: self.proof_chain(),
                vm_type: self.proof_vm(),
                block_number: 10,
                block_hash: "0xblock10".into(),
                confirmations: 1,
                lock_address: "x3:escrow".into(),
                locked_amount: intent.amount_in,
                hashlock: intent.hashlock,
                receiver: intent.receiver.as_bytes().to_vec(),
                refund_address: intent.refund_path.address.as_bytes().to_vec(),
                timeout: intent.source_timeout,
                raw_proof: vec![1, 2, 3],
            })
        }

        fn claim(
            &self,
            _chain_id: &ChainId,
            _escrow_address: &[u8],
            intent_id: IntentId,
            preimage: [u8; 32],
        ) -> Result<ClaimProof, SwapError> {
            self.counts.claim.fetch_add(1, Ordering::SeqCst);
            Ok(ClaimProof {
                tx_id: "0xlive-claim".into(),
                intent_id,
                chain_id: self.proof_chain(),
                vm_type: self.proof_vm(),
                preimage,
                block_number: 11,
                block_hash: "0xblock11".into(),
                raw_proof: vec![4, 5, 6],
            })
        }

        fn refund(
            &self,
            _chain_id: &ChainId,
            _escrow_address: &[u8],
            intent_id: IntentId,
        ) -> Result<RefundProof, SwapError> {
            self.counts.refund.fetch_add(1, Ordering::SeqCst);
            Ok(RefundProof {
                tx_id: "0xlive-refund".into(),
                intent_id,
                chain_id: self.proof_chain(),
                vm_type: self.proof_vm(),
                block_number: 12,
                block_hash: "0xblock12".into(),
                raw_proof: vec![7, 8, 9],
            })
        }

        fn verify_lock(&self, _proof: &LockProof) -> Result<bool, SwapError> {
            self.counts.verify.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        }

        fn verify_claim(&self, _proof: &ClaimProof) -> Result<bool, SwapError> {
            self.counts.verify.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        }

        fn verify_refund(&self, _proof: &RefundProof) -> Result<bool, SwapError> {
            self.counts.verify.fetch_add(1, Ordering::SeqCst);
            Ok(true)
        }

        fn estimate_fee(
            &self,
            _chain_id: &ChainId,
            _escrow_address: &[u8],
            _intent: &AtomicIntent,
        ) -> Result<FeeEstimate, SwapError> {
            self.counts.fee.fetch_add(1, Ordering::SeqCst);
            Ok(FeeEstimate {
                chain_id: self.proof_chain(),
                vm_type: self.proof_vm(),
                native_fee: 10,
                gas_units: 20,
                gas_price: 1,
                estimated_usd: 0.0,
            })
        }

        fn finality_status(
            &self,
            _chain_id: &ChainId,
            tx_id: &TxId,
        ) -> Result<FinalityProof, SwapError> {
            self.counts.finality.fetch_add(1, Ordering::SeqCst);
            Ok(FinalityProof {
                chain_id: self.proof_chain(),
                vm_type: self.proof_vm(),
                tx_id: tx_id.clone(),
                block_number: 10,
                block_hash: "0xblock10".into(),
                confirmations: 1,
                finalized: true,
                finality_source: "grandpa".into(),
                safe_to_reveal_secret: true,
            })
        }

        fn chain_health(&self, _chain_id: &ChainId) -> Result<ChainHealth, SwapError> {
            self.counts.health.fetch_add(1, Ordering::SeqCst);
            Ok(ChainHealth {
                chain_id: self.proof_chain(),
                vm_type: self.proof_vm(),
                latest_block: 10,
                finalized_block: 10,
                block_delay_ms: 1_000,
                finality_delay_ms: 1_000,
                rpc_quorum_healthy: true,
                gas_price: 1,
                halted: false,
                degraded: false,
                safe_for_new_intents: true,
            })
        }

        fn readiness_score(&self) -> AdapterReadinessScore {
            AdapterReadinessScore {
                adapter_name: "recording-x3vm",
                vm_type: self.proof_vm(),
                interface_implemented: true,
                lock_path: true,
                claim_path: true,
                refund_path: true,
                event_proof_extraction: true,
                finality_proof: true,
                rpc_indexer_support: true,
                timeout_safety: true,
                tests_implemented: true,
                proof_ledger_integration: false,
                ibc_support: false,
                cross_adapter_atomicity_test: false,
            }
        }
    }

    fn intent() -> AtomicIntent {
        AtomicIntent {
            intent_id: 77,
            source_chain: ChainKind::X3,
            destination_chain: ChainKind::Ethereum,
            source_asset: "X3".into(),
            destination_asset: "USDC".into(),
            amount_in: 1000,
            min_amount_out: 900,
            receiver: "receiver".into(),
            hashlock: [9u8; 32],
            source_timeout: 200,
            destination_timeout: 100,
            finality_requirements: vec![FinalityRequirement {
                chain: ChainKind::X3,
                level: FinalityLevel::Bft,
            }],
            refund_path: RefundPath {
                chain: ChainKind::X3,
                address: "refund".into(),
                asset: None,
            },
            route_mode: RouteMode::DirectHtlc,
            max_slippage_bps: 100,
            relayer_quorum_requirement: 1,
            status: AtomicSwapStatus::Pending,
            intent_hash: [0u8; 32],
        }
    }

    #[test]
    fn x3vm_live_delegates_lifecycle_and_observability() {
        let transport = RecordingTransport::good("x3-local");
        let counts = transport.counts.clone();
        let adapter = LiveX3VmAdapter::new("x3-local".into(), vec![0xaa; 32], transport);
        let intent = intent();
        let preimage = [3u8; 32];

        let lock = adapter.lock(&intent).expect("live lock");
        let claim = adapter.claim(intent.intent_id, preimage).expect("live claim");
        let refund = adapter.refund(intent.intent_id).expect("live refund delegation");
        assert!(adapter.verify_lock(&lock).expect("verify lock"));
        assert!(adapter.verify_claim(&claim).expect("verify claim"));
        assert!(adapter.verify_refund(&refund).expect("verify refund"));
        let _ = adapter.estimate_fee(&intent).expect("fee");
        let _ = adapter.finality_status(&lock.tx_id).expect("finality");
        let _ = adapter.chain_health().expect("health");

        assert_eq!(counts.lock.load(Ordering::SeqCst), 1);
        assert_eq!(counts.claim.load(Ordering::SeqCst), 1);
        assert_eq!(counts.refund.load(Ordering::SeqCst), 1);
        assert_eq!(counts.verify.load(Ordering::SeqCst), 3);
        assert_eq!(counts.fee.load(Ordering::SeqCst), 1);
        assert_eq!(counts.finality.load(Ordering::SeqCst), 1);
        assert_eq!(counts.health.load(Ordering::SeqCst), 1);
        assert!(adapter.is_simulated(), "production gate stays fail-closed in Task 1");
    }

    #[test]
    fn x3vm_live_rejects_wrong_vm_proof_before_relayer_sees_it() {
        let mut transport = RecordingTransport::good("x3-local");
        transport.wrong_vm = true;
        let adapter = LiveX3VmAdapter::new("x3-local".into(), vec![0xaa; 32], transport);

        assert!(adapter.lock(&intent()).is_err());
        assert!(adapter.chain_health().is_err());
    }

    #[test]
    fn x3vm_live_rejects_wrong_chain_proof_before_relayer_sees_it() {
        let mut transport = RecordingTransport::good("x3-local");
        transport.wrong_chain = true;
        let adapter = LiveX3VmAdapter::new("x3-local".into(), vec![0xaa; 32], transport);

        assert!(adapter.lock(&intent()).is_err());
        assert!(adapter.estimate_fee(&intent()).is_err());
    }

    #[test]
    fn x3vm_live_rejects_empty_chain_evidence() {
        #[derive(Debug, Clone)]
        struct EmptyProofTransport(RecordingTransport);

        impl X3VmLiveTransport for EmptyProofTransport {
            fn lock(&self, c: &ChainId, e: &[u8], i: &AtomicIntent) -> Result<LockProof, SwapError> {
                let mut proof = self.0.lock(c, e, i)?;
                proof.raw_proof.clear();
                Ok(proof)
            }
            fn claim(&self, c: &ChainId, e: &[u8], id: IntentId, p: [u8; 32]) -> Result<ClaimProof, SwapError> { self.0.claim(c, e, id, p) }
            fn refund(&self, c: &ChainId, e: &[u8], id: IntentId) -> Result<RefundProof, SwapError> { self.0.refund(c, e, id) }
            fn verify_lock(&self, p: &LockProof) -> Result<bool, SwapError> { self.0.verify_lock(p) }
            fn verify_claim(&self, p: &ClaimProof) -> Result<bool, SwapError> { self.0.verify_claim(p) }
            fn verify_refund(&self, p: &RefundProof) -> Result<bool, SwapError> { self.0.verify_refund(p) }
            fn estimate_fee(&self, c: &ChainId, e: &[u8], i: &AtomicIntent) -> Result<FeeEstimate, SwapError> { self.0.estimate_fee(c, e, i) }
            fn finality_status(&self, c: &ChainId, tx: &TxId) -> Result<FinalityProof, SwapError> { self.0.finality_status(c, tx) }
            fn chain_health(&self, c: &ChainId) -> Result<ChainHealth, SwapError> { self.0.chain_health(c) }
            fn readiness_score(&self) -> AdapterReadinessScore { self.0.readiness_score() }
        }

        let transport = EmptyProofTransport(RecordingTransport::good("x3-local"));
        let adapter = LiveX3VmAdapter::new("x3-local".into(), vec![0xaa; 32], transport);
        assert!(adapter.lock(&intent()).is_err());
    }
}
