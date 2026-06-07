## ADDED Requirements

### Requirement: Proof-gated EVM mirror mint
The system SHALL mint the EVM mirror token only when a valid threshold-signed X3 proof is provided.

#### Scenario: Valid proof mints
- **WHEN** a valid proof is submitted to the EVM mirror contract
- **THEN** the contract mints the exact amount to the recipient and records the proof hash

#### Scenario: Replay is rejected
- **WHEN** a previously accepted proof is submitted again
- **THEN** the transaction reverts and no mint occurs

### Requirement: Supported EVM chain IDs
The system SHALL deploy mirror tokens for the approved EVM chain IDs.

#### Scenario: Chain list
- **WHEN** a client queries the mirror deployment registry
- **THEN** Ethereum (1), BSC (56), Polygon (137), and Base (8453) are present as supported chain IDs

### Requirement: Proof-gated EVM mirror burn
The system SHALL burn the EVM mirror token only when a valid burn proof is submitted.

#### Scenario: Valid burn proof
- **WHEN** a valid burn proof is submitted
- **THEN** the contract burns the specified amount and emits a burn receipt event
