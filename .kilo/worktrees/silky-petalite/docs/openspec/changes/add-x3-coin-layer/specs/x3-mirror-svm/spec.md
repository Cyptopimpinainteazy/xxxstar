## ADDED Requirements

### Requirement: Proof-gated SVM mirror mint
The system SHALL mint the SVM mirror token only when a valid threshold-signed X3 proof is provided.

#### Scenario: Valid proof mints
- **WHEN** a valid proof is submitted to the SVM mirror program
- **THEN** the program mints the exact amount to the recipient and records the proof hash

#### Scenario: Replay is rejected
- **WHEN** a previously accepted proof is submitted again
- **THEN** the transaction fails and no mint occurs

### Requirement: Hashlock and timeout enforcement
The system SHALL enforce hashlock unlocks and timeout refunds for SVM mirror escrows.

#### Scenario: Hashlock unlock
- **WHEN** a valid preimage is provided before timeout
- **THEN** the escrow releases to the recipient and emits a proof receipt

#### Scenario: Timeout refund
- **WHEN** the timeout expires without a valid preimage
- **THEN** the escrow refunds and emits a refund receipt
