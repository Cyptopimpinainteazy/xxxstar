use x3_court::court::{ConsensusBlock, ConsensusChainState};
use x3_court::types::{ChallengePayload, ChallengeType};
use x3_court::{Court, CourtConfig, DisputeType, Verdict};
use x3_flashloan::types::{AssetId, ChainKind};
use x3_flashloan::{BorrowPurpose, BorrowRequest, FlashloanPool, SettlementEngine};
use x3_proof::chain::ProofChain;
use x3_proof::hasher::DeterministicHasher;
use x3_proof::{AgentIdentity, ProofEngine, ProofEngineConfig};

fn main() {
    // 1) Prepare a pool and perform a borrow + repay
    let mut pool = FlashloanPool::new();
    pool.deposit(ChainKind::Evm(1), AssetId::new("USDC"), 1_000_000_000);

    let borrow = BorrowRequest {
        id: x3_flashloan::types::FlashloanId::new(),
        chain: ChainKind::Evm(1),
        asset: AssetId::new("USDC"),
        amount: 100_000_000,
        purpose: BorrowPurpose::ArbExecution,
    };

    let receipt = pool.borrow(&borrow).expect("borrow should succeed");
    println!(
        "Borrow receipt: id={} principal={} premium={}",
        receipt.id.0, receipt.principal, receipt.premium
    );

    // Repay
    let repay_amount = receipt.principal + receipt.premium;
    let ok = pool
        .repay(&receipt.id, repay_amount)
        .expect("repay should succeed");
    println!("Repay ok: {}", ok);

    // 2) Generate a proof chain using the ProofEngine
    let mut engine = ProofEngine::new(
        ProofEngineConfig::default(),
        100u64,
        AgentIdentity {
            pubkey: [0x42; 32],
            ephemeral: false,
        },
        [0xAA; 32],
    );

    // Emit two proofs to simulate execution
    engine
        .record_state_write(b"k1".to_vec(), None, Some(b"v1".to_vec()))
        .unwrap();
    let _p1 = engine.emit_proof(1000, 10, None).unwrap();

    engine
        .record_state_write(b"k2".to_vec(), None, Some(b"v2".to_vec()))
        .unwrap();
    let _p2 = engine.emit_proof(500, 5, None).unwrap();

    let (chain, receipt_info) = engine.finalize().unwrap();
    println!(
        "Generated proof chain hash: {}",
        hex::encode(chain.chain_hash())
    );
    println!(
        "Execution receipt: total_gas={}, total_fees={}",
        receipt_info.total_gas, receipt_info.total_fees
    );

    // 3) Create a tampered replay chain (mutate first proof post_state_hash)
    let block = chain.proofs().get(0).map(|p| p.block_height).unwrap_or(100);
    let program = chain
        .proofs()
        .get(0)
        .map(|p| p.program_hash)
        .unwrap_or([0u8; 32]);
    let mut tampered = ProofChain::new(block, program);

    for (i, orig) in chain.proofs().iter().enumerate() {
        let mut copy = orig.clone();
        if i == 0 {
            // Flip a byte to simulate divergence
            copy.post_state_hash[0] ^= 1;
            copy.proof_hash = DeterministicHasher::hash_execution_proof(&copy);
        }
        tampered.append(copy).unwrap();
    }

    println!(
        "Tampered proof chain hash: {}",
        hex::encode(tampered.chain_hash())
    );

    // 4) File a dispute and adjudicate
    let court_config = CourtConfig::default();
    let dispute_bond = court_config.dispute_bond;
    let mut court = Court::new(court_config);
    let dispute_id = court
        .file_dispute(
            DisputeType::ExecutionDivergence {
                proof_chain_hash: chain.chain_hash(),
            },
            AgentIdentity {
                pubkey: [0x42; 32],
                ephemeral: false,
            },
            200u64,
            dispute_bond,
        )
        .unwrap();

    let disputed_block = ConsensusBlock::default();
    let pre_state = ConsensusChainState::default();
    let challenge_payload = ChallengePayload::ReceiptMismatch {
        action_id: 1,
        expected: [0u8; 32],
        observed: [0u8; 32],
    };

    match court.adjudicate(
        dispute_id,
        &disputed_block,
        &pre_state,
        &ChallengeType::InvalidExecution,
        &challenge_payload,
        205u64,
    ) {
        Ok(verdict) => {
            println!("Verdict outcome: {:?}", verdict.outcome);
            println!("Verdict summary: {}", Verdict::summary(&verdict));
        }
        Err(err) => {
            println!("Adjudication unavailable in demo: {err}");
        }
    }

    // 5) Settlement record demonstration
    let settlement_engine = SettlementEngine::new();
    let srec = settlement_engine
        .settle(
            &receipt,
            repay_amount,
            &ProofEngine::new(
                ProofEngineConfig::default(),
                100,
                AgentIdentity {
                    pubkey: [0xFF; 32],
                    ephemeral: true,
                },
                [0x00; 32],
            ),
        )
        .unwrap();
    println!(
        "Settlement record: id={} status={:?} proof_hash={}",
        srec.flashloan_id.0, srec.status, srec.proof_hash
    );
}
