/**
 * Proof Relayer Index
 * Main exports for the proof relayer library
 */

export { EventManager } from './event-listener';
export {
  BitcoinEventListener,
  EthereumEventListener,
  SolanaEventListener,
  X3VMEventListener,
  ChainEventListener,
} from './event-listener';
export type { HTLCEvent, EventListenerConfig, EventFilter } from './event-listener';

export { SPVProofGenerator } from './spv-proof-generator';
export type {
  MerkleProof,
  BlockHeader,
  SPVProof,
  ProofGeneratorConfig,
} from './spv-proof-generator';

export { ProofRelayer } from './proof-relayer';
export type { RelayTask, RelayResult, RelayConfig } from './proof-relayer';

export { SettlementVerifier } from './settlement-verifier';
export type {
  SettlementRecord,
  VerificationResult,
  VerifierConfig,
} from './settlement-verifier';

// Default exports
export default ProofRelayer;
