## ADDED Requirements

### Requirement: GPU-accelerated EVM verification
The system SHALL provide GPU-accelerated batch verification for secp256k1 signatures and keccak256 hashing with CPU parity checks.

#### Scenario: secp256k1 batch verification
- **WHEN** a batch of EVM signatures is submitted for validation
- **THEN** the GPU pipeline validates the batch and matches CPU results for all signatures

#### Scenario: keccak256 batch hashing
- **WHEN** a batch of EVM state inputs is submitted for hashing
- **THEN** the GPU pipeline produces hashes identical to a CPU reference

### Requirement: EVM state root validation
The system SHALL validate EVM block state roots using the GPU batch pipelines and report validation status per block.

#### Scenario: validate state root
- **WHEN** an EVM block is provided with its expected state root
- **THEN** the validator computes the state root and reports valid or invalid

### Requirement: Atomic cross-chain orchestration
The system SHALL enforce atomic commit/rollback across Solana and Ethereum by validating both sides before approval and rolling back on any failure or timeout.

#### Scenario: atomic commit
- **WHEN** both chain validations succeed within the timeout
- **THEN** the orchestrator approves both sides atomically

#### Scenario: atomic rollback
- **WHEN** either chain validation fails or exceeds timeout
- **THEN** the orchestrator rolls back both sides and records the failure

### Requirement: State synchronization and failover
The system SHALL maintain a shared atomic registry and MUST fail closed if synchronization is unavailable, using CPU-only failover only for validation errors.

#### Scenario: registry unavailable
- **WHEN** the atomic registry cannot be reached
- **THEN** the orchestrator rejects new swaps and reports a critical error

#### Scenario: GPU failure
- **WHEN** GPU validation errors occur
- **THEN** the orchestrator uses CPU validation as failover and preserves atomic guarantees

### Requirement: Testnet deployment and benchmarking
The system SHALL provide deployment scripts and benchmarking reports for Solana and Ethereum testnets, including combined TPS metrics.

#### Scenario: testnet deployment
- **WHEN** deployment scripts are executed in a testnet environment
- **THEN** both validators start and report readiness

#### Scenario: benchmark report
- **WHEN** the benchmark runner completes
- **THEN** it emits TPS, latency, and rollback metrics in a machine-readable report

### Requirement: Operator observability
The system SHALL expose a dashboard with live TPS, atomic success rate, rollback counts, GPU health, and RPC latency.

#### Scenario: dashboard metrics
- **WHEN** the operator opens the dashboard
- **THEN** live metrics for both chains and the orchestrator are displayed
