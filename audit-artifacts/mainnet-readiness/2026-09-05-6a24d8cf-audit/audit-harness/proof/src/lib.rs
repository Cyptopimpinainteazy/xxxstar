#[cfg(test)]
mod audit_tests {
 use x3_verification_router::*;
 fn envelope(chain: ChainKind, strategy: VerificationStrategy) -> ProofEnvelope {
  ProofEnvelope { proof_id:[1;32], strategy, source_chain:chain, destination_chain:ChainKind::X3, payload:vec![1], expected_asset_id:[2;32], expected_amount:100, expected_sender:vec![3;20], expected_recipient:vec![4;20] }
 }
 #[test]
 fn reject_unsigned_one_byte_quorum_proof_in_production() {
  let proof=envelope(ChainKind::Evm{chain_id:1}, VerificationStrategy::ValidatorQuorum{threshold:3,total:5});
  assert!(ValidatorQuorumVerifier::new(3,5).verify(&proof).is_err(), "one-byte unsigned proof accepted by production quorum verifier");
 }
 #[test]
 fn reject_unsigned_one_byte_solana_proof_in_production() {
  let proof=envelope(ChainKind::Solana, VerificationStrategy::SolanaFinalizedProof);
  assert!(SolanaFinalizedVerifier.verify(&proof).is_err(), "one-byte unsigned proof accepted by production Solana verifier");
 }
 #[test]
 fn reject_legacy_structural_evm_proof_in_production() {
  let mut proof=envelope(ChainKind::Evm{chain_id:1},VerificationStrategy::EvmReceiptProof); proof.payload=vec![1;64];
  assert!(EvmReceiptVerifier::new(12).verify(&proof).is_err(), "64 arbitrary bytes accepted by legacy verifier with production feature");
 }
}
