# E2E Security Testing Guide - Cross-Chain GPU Validator

## Overview

This document describes the comprehensive end-to-end (E2E) security testing framework for the cross-chain GPU validator, covering:

- **Security Validation** - Input sanitization, signature handling, RPC security, atomicity guarantees
- **Dashboard Testing** - Metrics endpoint validation, real-time updates, security and XSS protection
- **Multi-Scenario RPC Testing** - Live testnet, mock responses, containerized environment
- **Debugging Infrastructure** - Logs, metrics, error tracking

## Invariants Tested

### Security (INV-SEC-001 through INV-SEC-004)

#### INV-SEC-001: Input Validation & Sanitization
- Chain IDs must be alphanumeric (no SQL injection, path traversal)
- Transaction signatures must have correct size for algorithm (secp256k1: 65 bytes, ed25519: 64 bytes)
- Public keys must have correct size (secp256k1: 64 bytes uncompressed, ed25519: 32 bytes)
- Payloads must not exceed maximum size (1MB limit)
- All inputs validated before processing

**Test File:** `tests/test_security_e2e.py::TestInputValidationSanitization`

#### INV-SEC-002: Private Key & Signature Handling
- Private keys never logged or printed in string form
- Signature verification fails with wrong public key
- Batch signature order preserved with transaction pairing
- Same message with different signers produces different signatures

**Test File:** `tests/test_security_e2e.py::TestPrivateKeySignatureHandling`

#### INV-SEC-003: RPC Endpoint Security
- RPC URLs must use HTTPS in production (HTTP allowed for localhost)
- Requests timeout after configured duration (default 5s)
- Malformed RPC responses rejected
- RPC error responses properly handled without crashes
- Endpoint availability verified before use

**Test File:** `tests/test_security_e2e.py::TestRpcEndpointSecurity`

#### INV-SEC-004: Transaction Atomicity Guarantees
- Atomic swaps cannot proceed with any invalid transaction
- Rollback restores all chains to pre-swap state
- Timeout triggers automatic rollback
- All-or-nothing semantics across N chains
- Concurrent swaps remain isolated
- State transitions follow valid rules (PENDING → LOCKED → CONFIRMED → COMPLETED)

**Test File:** `tests/test_security_e2e.py::TestTransactionAtomicityGuarantees`

### Dashboard (INV-DASH-001 through INV-DASH-004)

#### INV-DASH-001: Metrics Endpoint Validation
- `/metrics.json` endpoint returns valid JSON
- All required fields present
- TPS values non-negative
- Success rate between 0-1
- GPU utilization 0-100%
- VRAM not oversubscribed
- Timestamps recent (within 1 minute)

**Test File:** `tests/test_dashboard_e2e.py::TestDashboardMetricsEndpoint`

#### INV-DASH-002: Real-Time Updates
- Dashboard refreshes every 1000ms
- Pending swaps count updates
- Rollback counter increments
- GPU utilization reflects current load
- Latency metrics update

**Test File:** `tests/test_dashboard_e2e.py::TestDashboardRealTimeUpdates`

#### INV-DASH-003: No Sensitive Data Exposure
- Private keys never in metrics
- RPC credentials not exposed
- Chain state secrets excluded
- No wallet balances or mnemonics

**Test File:** `tests/test_dashboard_e2e.py::TestDashboardSecurityNoSensitiveData`

#### INV-DASH-004: XSS & Injection Protection
- Script tags escaped in output
- Metric values properly escaped
- GPU labels from dynamic content escaped
- DOM updates use `textContent` not `innerHTML`
- Metrics endpoint returns `application/json`
- CORS headers properly configured

**Test File:** `tests/test_dashboard_e2e.py::TestDashboardXssInjectionProtection`

### RPC Endpoints (INV-RPC-001 through INV-RPC-004)

#### INV-RPC-001: Endpoint Availability & Latency
- RPC endpoint reachable with valid response
- Latency measured for performance monitoring
- Unavailable endpoints logged and retried
- Timeouts respected after configured duration
- Rate-limited endpoints (429) handled

**Test File:** `tests/test_rpc_endpoints_e2e.py::TestRpcEndpointAvailability`

#### INV-RPC-002: Response Validation
- Valid JSON-RPC response has result or error field
- Block numbers returned as hex
- Balance queries return hex wei
- Error responses have code and message
- Null result distinguished from error

**Test File:** `tests/test_rpc_endpoints_e2e.py::TestRpcResponseValidation`

#### INV-RPC-003: Transaction Broadcasting
- `eth_sendTransaction` returns 66-char transaction hash
- Transaction receipts confirm inclusion in block
- Status field distinguishes success (0x1) from failure (0x0)
- Solana commitment levels recognized
- Transaction fee estimation enums validated

**Test File:** `tests/test_rpc_endpoints_e2e.py::TestTransactionBroadcastAndConfirmation`

#### INV-RPC-004: Mock vs Real RPC Consistency
- Mock and real RPC responses have same format
- Error responses follow JSON-RPC standard
- Mock RPC is deterministic
- Both return valid structures

**Test File:** `tests/test_rpc_endpoints_e2e.py::TestMockVsRealRpcConsistency`

## Running E2E Tests

### Quick Start (Recommended - Docker)

```bash
cd /path/to/cross-chain-gpu-validator
./scripts/run-e2e-tests.sh docker
```

This launches:
- Ethereum testnet (anvil/Foundry)
- Solana validator
- Redis instance
- Validator service
- Test runner
- Prometheus metrics server

Results are saved to `/tmp/ccgv-test-results/`

### Testing Modes

#### 1. Docker Mode (Recommended)
**Most reliable and reproducible.**

```bash
./scripts/run-e2e-tests.sh docker [--cleanup]
```

**Includes:**
- Isolated containerized environment
- Deterministic test conditions
- Automatic health checks
- Full logging and metrics
- Optional cleanup of containers

**Output:**
```
/tmp/ccgv-test-results/
├── coverage/           # HTML coverage report
├── validator.log       # Service logs
└── junit.xml          # Test results XML
```

#### 2. Mock Mode
**Fast, deterministic, offline.**

```bash
./scripts/run-e2e-tests.sh mock
```

**Environment:**
```bash
export CCGV_USE_MOCK_RPC=true
export CCGV_EVM_RPC="mock://ethereum"
export CCGV_SVM_RPC="mock://solana"
```

**Pros:**
- No network dependencies
- Repeatable results
- Fast execution
- Good for CI/CD

**Cons:**
- Doesn't test actual RPC servers
- May not catch real-world RPC issues

#### 3. Live Testnet Mode
**Tests against actual blockchain testnets.**

```bash
export CCGV_EVM_RPC="https://sepolia.infura.io/v3/YOUR_KEY"
export CCGV_SVM_RPC="https://api.testnet.solana.com"
./scripts/run-e2e-tests.sh live
```

**Requirements:**
- Active internet connection
- Valid RPC endpoint credentials
- Testnet faucets for funding (if needed)

**Pros:**
- Tests real RPC behavior
- Catches real-world issues
- Validates network integration

**Cons:**
- Slower execution
- Dependent on testnet availability
- Non-deterministic results

## Test Structure

```
tests/
├── test_security_e2e.py          # Security validation tests
│   ├── TestInputValidationSanitization
│   ├── TestPrivateKeySignatureHandling
│   ├── TestRpcEndpointSecurity
│   └── TestTransactionAtomicityGuarantees
│
├── test_dashboard_e2e.py          # Dashboard tests
│   ├── TestDashboardMetricsEndpoint
│   ├── TestDashboardRealTimeUpdates
│   ├── TestDashboardSecurityNoSensitiveData
│   ├── TestDashboardXssInjectionProtection
│   └── TestDashboardErrorHandling
│
├── test_rpc_endpoints_e2e.py      # RPC endpoint tests
│   ├── TestRpcEndpointAvailability
│   ├── TestRpcResponseValidation
│   ├── TestTransactionBroadcastAndConfirmation
│   ├── TestMockVsRealRpcConsistency
│   └── TestRpcEndpointSecurityValidation
│
└── resources/                      # Test fixtures
    ├── mock_rpc_responses.json
    └── deterministic_fixtures.toml
```

## Debugging & Troubleshooting

### View Containerized Service Logs

```bash
# Real-time logs from validator service
docker-compose -f docker-compose.testnet.yml logs -f ccgv-validator

# Ethereum testnet logs
docker-compose -f docker-compose.testnet.yml logs ethereum-testnet

# Solana validator logs
docker-compose -f docker-compose.testnet.yml logs solana-testnet
```

### Access Services Directly

```bash
# Ethereum RPC (HTTP)
curl -X POST http://localhost:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"eth_blockNumber","id":1}'

# Solana RPC (HTTP)
curl -X POST http://localhost:8899 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"getLatestBlockhash","params":[],"id":1}'

# Redis
redis-cli -h localhost -p 6379 PING

# Metrics endpoint
curl http://localhost:8000/metrics.json | jq .

# Prometheus
open http://localhost:9090
```

### Common Issues

#### 1. Services Won't Start
```bash
# Check Docker resources
docker system df

# Clean up and rebuild
docker-compose -f docker-compose.testnet.yml down -v
docker-compose -f docker-compose.testnet.yml build --no-cache
```

#### 2. Tests Timeout
```bash
# Increase wait time for services
docker-compose -f docker-compose.testnet.yml up -d
sleep 30  # Wait longer for all services
./scripts/run-e2e-tests.sh docker
```

#### 3. RPC Request Failures
```bash
# Check RPC endpoint health
curl -v http://localhost:8545/

# Verify container networking
docker network inspect ccgv_ccgv-testnet
```

## Security Considerations

### Input Validation
- All transaction inputs validated before processing
- Chain IDs restricted to alphanumeric characters
- Signature and pubkey sizes enforced per algorithm
- Payload size limits prevent memory attacks

### Cryptographic Operations
- Private keys never logged or serialized
- Signature verification fails gracefully on invalid input
- Batch processing preserves signature-transaction pairing
- GPU acceleration validated against CPU reference

### RPC Security
- HTTPS required in production (HTTP for localhost only)
- Request timeouts prevent hanging connections
- Response validation prevents injection attacks
- Rate limiting respected automatically

### Atomicity Guarantees
- All-or-nothing semantics across N chains
- Automatic rollback on any validation failure
- Timeout-based safety net
- State machine enforces valid transitions

## Continuous Integration

Example GitHub Actions workflow:

```yaml
name: E2E Security Tests
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Run E2E tests (Docker)
        run: |
          cd cross-chain-gpu-validator
          ./scripts/run-e2e-tests.sh docker --cleanup
      
      - name: Upload coverage
        uses: codecov/codecov-action@v3
        with:
          files: /tmp/ccgv-test-results/coverage/.coverage.xml
```

## Performance Baselines

Expected test execution times:

- **Docker mode:** 3-5 minutes (includes startup)
- **Mock mode:** 30-60 seconds
- **Live testnet:** 5-10 minutes (depends on network)

GPU requirements: None (tests run on CPU with mock)

## Documentation and Compliance

All tests reference invariants in `tests/invariants/registry.toml`:
- INV-SEC-001 through INV-SEC-004
- INV-DASH-001 through INV-DASH-004
- INV-RPC-001 through INV-RPC-004
- INV-GPU-001 through INV-GPU-004 (existing GPU tests)
- INV-SWAP-001 through INV-SWAP-003 (existing swap tests)

Every test case includes docstring with:
- What invariant it tests
- Why it matters
- How it validates the invariant

## Further Reading

- [Chain Registry Import](../docs/chain_registry_import.md)
- [Atomic Swap State Machine](../docs/atomic_swaps.md)
- [GPU Kernel Profiles](../docs/gpu_kernels.md)
- [Validator Architecture](../docs/architecture.md)
