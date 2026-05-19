## ADDED Requirements

### Requirement: Canonical X3 asset
The system SHALL define X3 as a runtime-native canonical asset with fixed total supply and governance-controlled issuance rules.

#### Scenario: Canonical supply at genesis
- **WHEN** the chain boots from genesis
- **THEN** the X3 total supply matches the configured allocation and is recorded in the canonical ledger

#### Scenario: Unauthorized mint is rejected
- **WHEN** a non-governance actor attempts to mint X3
- **THEN** the transaction is rejected and no supply change occurs

### Requirement: Canonical parameters are fixed
The system SHALL configure canonical X3 parameters as Asset ID **1000**, symbol **X3**, decimals **18**, and total supply **8,888,888,888**.

#### Scenario: Parameter query
- **WHEN** a client queries the X3 asset metadata
- **THEN** the returned asset ID, symbol, decimals, and total supply match the canonical parameters

### Requirement: Genesis allocation splits
The system SHALL allocate the total supply at genesis according to the approved splits and amounts.

#### Scenario: Genesis allocation correctness
- **WHEN** the chain boots from genesis
- **THEN** treasury (2,222,222,222), validators (1,777,777,778), ecosystem (1,777,777,778), presale (1,333,333,333), bonus pool (888,888,889), and team (888,888,888) balances match the approved allocation amounts

#### Scenario: Recipient mapping
- **WHEN** genesis allocation recipients are configured from wallet outputs
- **THEN** the on-chain balances are credited to those exact recipient accounts

#### Testnet recipient mapping (2026-02-18)
- Treasury multisig (4-of-7): `5Gm3cbEyVgrzTJghqvePTFkfP5Gyw8JxpQPv73r2ycWjc7DL`
- Validators / Staking: `5DCNYZ4PPFZyCKZZkqFn5oiWd78zRAyG41L6SzkugAVE5uHV`
- Ecosystem / Grants: `5G472MxRo1XGtWudyXtq2oh9uKGACRqFqv2QoH2ygTnNoDBR`
- Presale / Early Investors: `5DPXVzc36fT9sEtHLb4Xo5e28xCVAx5exytFLxPAwrKbLBfL`
- Bonus Pool: `5CqSUtcWaxen8F5Q4XPhDPzFmPqVRyuPKMzrvnw932k2bdcT`
- Team / Core Contributors: `5CzThhqqAD52z4qzdZE1Gk9DgfpnzoZfYZBNLSZcukjbJQep`

### Requirement: Bonus pool sub-allocation
The system SHALL subdivide the bonus pool according to the approved percentages for GPU, validators, presale/traders, and auditors (40/20/10/30).

#### Scenario: Bonus pool composition
- **WHEN** the bonus pool ledger is queried
- **THEN** its sub-allocations match the approved percentage breakdown

### Requirement: Canonical balance API
The runtime SHALL expose a deterministic API to query X3 balances and total supply.

#### Scenario: Balance query
- **WHEN** a client queries the X3 balance for a valid account
- **THEN** the runtime returns the canonical balance from on-chain state
