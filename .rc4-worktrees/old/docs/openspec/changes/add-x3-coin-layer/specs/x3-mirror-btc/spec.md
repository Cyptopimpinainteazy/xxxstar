## ADDED Requirements

### Requirement: Proof-gated BTC HTLC unlock
The system SHALL unlock BTC HTLC outputs only when a valid threshold-signed X3 proof and matching preimage are provided.

#### Scenario: Valid unlock
- **WHEN** a valid proof and preimage are submitted before timeout
- **THEN** the HTLC releases to the recipient and emits an unlock receipt

#### Scenario: Replay is rejected
- **WHEN** a previously accepted proof is submitted again
- **THEN** the unlock is rejected and no funds move

### Requirement: BTC timeout refund
The system SHALL support timeout-based refunds for BTC HTLCs.

#### Scenario: Timeout refund
- **WHEN** the timeout height is reached without a valid preimage
- **THEN** the HTLC can be refunded and a refund receipt is emitted
