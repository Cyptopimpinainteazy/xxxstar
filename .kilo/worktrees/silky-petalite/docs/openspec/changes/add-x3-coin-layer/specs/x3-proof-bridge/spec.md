## ADDED Requirements

### Requirement: Deterministic proof encoding
The system SHALL serialize mirror proofs using a canonical, deterministic encoding with domain separation.

#### Scenario: Cross-platform encoding
- **WHEN** two nodes on different hardware encode the same proof inputs
- **THEN** the output bytes are identical

### Requirement: BLS threshold verification
The system SHALL verify mirror proofs using BLS aggregated signatures with a 2/3 quorum of the active signer set.

#### Scenario: Quorum required
- **WHEN** a proof signature is validated
- **THEN** verification succeeds only if the aggregated signature meets the 2/3 threshold

### Requirement: Proof replay protection
The system SHALL reject any proof whose hash was previously accepted for the same mirror domain.

#### Scenario: Duplicate proof hash
- **WHEN** a proof hash already exists in the registry
- **THEN** the proof is rejected and no state changes occur

### Requirement: Per-chain finality adapters
The system SHALL use explicit finality adapters per chain type (EVM, SVM, BTC) before emitting or accepting mirror proofs.

#### Scenario: Finality depth enforced
- **WHEN** a chain-specific finality threshold is not met
- **THEN** proof emission or acceptance is delayed until the threshold is satisfied

### Requirement: Signer set management
The system SHALL manage the mirror proof signer set via a dedicated pallet with explicit activation and rotation controls.

#### Scenario: Signer rotation
- **WHEN** the signer set is updated by governance
- **THEN** the new set becomes active for proof verification without invalidating prior proofs
