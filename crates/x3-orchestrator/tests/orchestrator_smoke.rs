use std::sync::Arc;

use x3_orchestrator::{
    adapters::{evm::EvmAdapter, svm::SvmAdapter, x3vm::X3VmAdapter},
    AdapterRegistry, CanonicalSupplySnapshot, ChainId, CrossVmMessage, ExecutionProof,
    OrchestratorError, OrchestratorRouter, ReplayGuard, VmKind,
};

fn sample_message() -> CrossVmMessage {
    CrossVmMessage {
        source_chain: ChainId::new("ethereum-sepolia"),
        target_chain: ChainId::new("solana-devnet"),
        source_vm: VmKind::Evm,
        target_vm: VmKind::Svm,
        sender: b"evm-user".to_vec(),
        target: b"svm-program".to_vec(),
        payload: b"swap:1000".to_vec(),
        gas_limit: 1_000_000,
        nonce: 1,
        expiry_block: 99_999_999,
    }
}

fn sample_proof(message_id: &str) -> ExecutionProof {
    ExecutionProof {
        source_chain: "ethereum-sepolia".into(),
        message_id: message_id.to_string(),
        block_number: 123,
        state_root: vec![1, 2, 3],
        proof_bytes: vec![9, 9, 9],
    }
}

fn build_router() -> OrchestratorRouter {
    let registry = Arc::new(AdapterRegistry::new());
    registry.register(Arc::new(EvmAdapter::new(ChainId::new("ethereum-sepolia"))));
    registry.register(Arc::new(SvmAdapter::new(ChainId::new("solana-devnet"))));
    registry.register(Arc::new(X3VmAdapter::new(ChainId::new("x3-local"))));
    OrchestratorRouter::new(registry, Arc::new(ReplayGuard::new()))
}

#[test]
fn routes_evm_to_svm_message() {
    let router = build_router();
    let msg = sample_message();
    let id = msg.id().unwrap();
    let proof = sample_proof(&id);

    let routed_id = router.route(&msg, &proof).expect("route should succeed");
    assert_eq!(routed_id, id);
}

#[test]
fn blocks_replayed_message() {
    let router = build_router();
    let msg = sample_message();
    let id = msg.id().unwrap();
    let proof = sample_proof(&id);

    router.route(&msg, &proof).expect("first route succeeds");

    match router.route(&msg, &proof) {
        Err(OrchestratorError::ReplayDetected(replayed_id)) => {
            assert_eq!(replayed_id, id);
        }
        other => panic!("expected ReplayDetected, got {other:?}"),
    }
}

#[test]
fn fails_missing_adapter() {
    let registry = Arc::new(AdapterRegistry::new());
    // Only register the source adapter; target is missing.
    registry.register(Arc::new(EvmAdapter::new(ChainId::new("ethereum-sepolia"))));
    let router = OrchestratorRouter::new(registry, Arc::new(ReplayGuard::new()));

    let msg = sample_message();
    let id = msg.id().unwrap();
    let proof = sample_proof(&id);

    match router.route(&msg, &proof) {
        Err(OrchestratorError::AdapterNotFound(chain)) => {
            assert_eq!(chain, "solana-devnet");
        }
        other => panic!("expected AdapterNotFound, got {other:?}"),
    }
}

#[test]
fn validates_canonical_supply() {
    let snapshot = CanonicalSupplySnapshot {
        native: 1_000_000,
        evm: 200_000,
        svm: 150_000,
        x3vm: 100_000,
        external_locked: 300_000,
        pending: 50_000,
        canonical_supply: 1_800_000,
    };
    snapshot.validate().expect("invariant should hold");
}

#[test]
fn fails_invalid_canonical_supply() {
    let snapshot = CanonicalSupplySnapshot {
        native: 1_000_000,
        evm: 200_000,
        svm: 150_000,
        x3vm: 100_000,
        external_locked: 300_000,
        pending: 50_000,
        canonical_supply: 1_234_567, // wrong on purpose
    };
    assert!(matches!(
        snapshot.validate(),
        Err(OrchestratorError::InvariantFailed)
    ));
}

#[test]
fn rejects_invalid_proof() {
    let router = build_router();
    let msg = sample_message();
    let id = msg.id().unwrap();
    let mut proof = sample_proof(&id);
    proof.proof_bytes.clear();

    assert!(matches!(
        router.route(&msg, &proof),
        Err(OrchestratorError::InvalidProof)
    ));
}
