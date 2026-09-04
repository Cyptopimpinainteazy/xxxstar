## ADDED Requirements

### Requirement: Bonus pool claim verification
The system SHALL require verifiable proofs for all bonus pool claims.

#### Scenario: GPU contributor claim
- **WHEN** a GPU contributor submits a proof‑of‑contribution
- **THEN** the claim is accepted only if the proof verifies against on‑chain records

#### Scenario: Auditor claim
- **WHEN** an auditor submits a verified bug or audit milestone
- **THEN** the claim is accepted only if the milestone is recorded on‑chain

### Requirement: Bonus pool claim categories
The system SHALL support claims for GPU/hardware contributors, validators/staking buyers, presale/traders, and auditors/bug hunters.

#### Scenario: Category enforcement
- **WHEN** a claim is submitted
- **THEN** it is attributed to a single approved category and accounted against its allocation

### Requirement: Optional vesting for bonus claims
The system SHALL allow vesting schedules to be applied to bonus pool claims.

#### Scenario: Vesting applied
- **WHEN** a claim is configured with vesting
- **THEN** the claimant can only unlock balances according to the vesting schedule
