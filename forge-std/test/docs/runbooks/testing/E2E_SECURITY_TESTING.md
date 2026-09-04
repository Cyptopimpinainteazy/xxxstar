# 🛡️ E2E Security Testing Guide

## Quick Start - Run All Tests

```bash
bash cross-chain-gpu-validator/scripts/run-local-tests.sh
```

Output:
```
✓ Running 68 security and validation tests...
✓ All tests passed in 120 seconds
✓ Coverage report: 87% of tested code
```

## Test Suite Overview

| Suite | Tests | Coverage | Purpose |
|-------|-------|----------|---------|
| **Security** | 24 | Input validation, signatures, RPC, atomicity | Validate security invariants |
| **Dashboard** | 23 | Metrics, real-time, XSS protection | Verify dashboard integrity |
| **RPC Endpoints** | 21 | Availability, validation, consistency | Validate RPC behavior |

**Total: 68 tests, all passing ✓**

## 1. Security Tests (test_security_e2e.py)

### Input Validation & Sanitization
Ensures all external input is validated before processing.

```python
# What's tested:
✓ Empty signatures rejected
✓ Oversized payloads rejected
✓ Invalid chain IDs rejected
✓ Malformed transactions rejected
✓ SQL injection attempts sanitized
✓ XSS payloads escaped
```

### Private Key & Signature Handling
Validates sensitive data protection.

```python
# What's tested:
✓ Private keys NEVER logged to console
✓ Keys not passed in function signatures
✓ Batch operations preserve order
✓ Signature verification works
✓ Invalid signatures rejected
✓ Duplicate transactions detected
```

### RPC Endpoint Security
Validates network communication safety.

```python
# What's tested:
✓ HTTPS endpoints required
✓ Request timeouts enforced
✓ Response validation happens
✓ Error responses handled
✓ Null responses rejected
✓ Malformed JSON rejected
```

### Transaction Atomicity Guarantees
Ensures all-or-nothing transaction semantics.

```python
# What's tested:
✓ Multiple transactions succeed together
✓ One failure rolls back all
✓ Concurrent transactions isolated
✓ Timeout handling works
✓ No partial state changes
```

### Running Only Security Tests
```bash
cd cross-chain-gpu-validator
source .venv/bin/activate
pytest tests/test_security_e2e.py -v
```

## 2. Dashboard Tests (test_dashboard_e2e.py)

### Metrics Endpoint Validation
Validates that metrics endpoint returns correct data.

```python
# What's tested:
✓ JSON structure valid
✓ All required fields present
✓ Numbers in valid ranges
✓ Timestamps reasonable
✓ Status codes correct
```

### Real-Time Updates
Validates that metrics refresh live.

```python
# What's tested:
✓ Refresh interval respected
✓ Counters increment correctly
✓ Status changes reflected
✓ No data corruption
✓ Concurrent updates handled
```

### Sensitive Data Protection
Validates that private info never exposed.

```python
# What's tested:
✓ No private keys in metrics
✓ No RPC credentials exposed
✓ No database passwords shown
✓ No user API keys leaked
```

### XSS & Injection Protection
Validates that malicious input is neutralized.

```python
# What's tested:
✓ Script tags escaped
✓ Event handlers removed
✓ SQL injection prevention
✓ Command injection prevention
✓ Content-Type headers correct
```

### Running Only Dashboard Tests
```bash
cd cross-chain-gpu-validator
source .venv/bin/activate
pytest tests/test_dashboard_e2e.py -v
```

## 3. RPC Endpoint Tests (test_rpc_endpoints_e2e.py)

### Endpoint Availability & Latency
Validates RPC uptime and performance.

```python
# What's tested:
✓ Endpoints respond within 5s
✓ No connection refused errors
✓ Retry logic works
✓ Latency tracking accurate
✓ Status codes correct (200, 500, etc)
```

### JSON-RPC Response Validation
Validates response format and structure.

```python
# What's tested:
✓ JSON parses correctly
✓ "result" or "error" present
✓ Request IDs match
✓ No null-only responses
✓ Version field correct
```

### Transaction Broadcasting & Confirmation
Validates transaction lifecycle.

```python
# What's tested:
✓ Broadcast succeeds
✓ Transaction ID returned
✓ Confirmation received
✓ Status tracking works
✓ Timeout handling works
```

### Mock vs Real RPC Consistency
Validates mock RPC matches real behavior.

```python
# What's tested:
✓ Response format identical
✓ Error responses same
✓ Status codes consistent
✓ Timing similar
✓ Data validation identical
```

### Running Only RPC Tests
```bash
cd cross-chain-gpu-validator
source .venv/bin/activate
pytest tests/test_rpc_endpoints_e2e.py -v
```

## 4. Running Individual Tests

### Run a specific test class
```bash
pytest tests/test_security_e2e.py::TestInputValidationSanitization -v
```

### Run a specific test method
```bash
pytest tests/test_security_e2e.py::TestInputValidationSanitization::test_empty_signature_rejected -v
```

### Run with coverage
```bash
pytest --cov=cross_chain_gpu_validator --cov-report=html tests/
```

### Run with output capture disabled (see print statements)
```bash
pytest -s tests/test_dashboard_e2e.py
```

## 5. Test Output Interpretation

### Passing Test
```
test_empty_signature_rejected PASSED [5%]
```
✓ Test logic executed correctly
✓ Assertion passed
✓ No exceptions raised

### Failing Test
```
test_empty_signature_rejected FAILED [5%]
AssertionError: Expected ValidationError but got success
```
❌ Test assertion failed
❌ Need to fix validator logic

### Error Test
```
test_empty_signature_rejected ERROR [5%]
ImportError: No module named 'chain_adapter'
```
❌ Test can't run (import/setup issue)
❌ Need to install dependencies or fix imports

## 6. Debugging Failed Tests

### Step 1: Run test with verbose output
```bash
pytest tests/test_security_e2e.py::TestInputValidationSanitization::test_empty_signature_rejected -vv -s
```

### Step 2: Add print statements to understand flow
```python
def test_example():
    print("Step 1: Creating transaction")
    tx = create_transaction()
    print(f"Step 2: Transaction = {tx}")
    assert tx is not None
```

### Step 3: Run with pdb debugger
```bash
pytest tests/test_security_e2e.py -vv --pdb
```

### Step 4: Check test assumptions in mock code
```bash
grep -n "MockRpcServer" tests/test_*.py
```

## 7. Coverage Report

Generate and view coverage:

```bash
# Generate HTML report
pytest --cov=cross_chain_gpu_validator --cov-report=html tests/

# View report
open htmlcov/index.html
```

Look for:
- 🟢 Green lines = covered by tests
- 🔴 Red lines = not covered (untested code)
- 🟠 Yellow = partially covered

Target: **>80% coverage** for security-critical code

## 8. Test Fixtures & Mocks

### Mock RPC Server
What it provides:
- Fast responses (no network latency)
- Deterministic behavior (same input = same output)
- No external dependencies
- Full control over responses

Located in: `test_rpc_endpoints_e2e.py::MockRpcServer`

### Example Mock Response
```python
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "status": "success",
        "transaction_hash": "0x123abc...",
        "confirmation": 12
    }
}
```

## 9. CI/CD Integration

### Local Pre-Commit Hook
```bash
# Add to .git/hooks/pre-commit
#!/bin/bash
pytest tests/ -x --tb=short

# Make executable
chmod +x .git/hooks/pre-commit
```

### GitHub Actions
Automatically runs on push:
```yaml
- name: Run E2E Tests
  run: bash cross-chain-gpu-validator/scripts/run-local-tests.sh
```

## 10. Adding New Tests

### Template
```python
import pytest

class TestNewFeature:
    """Test description for new security feature."""
    
    def test_specific_case(self):
        """
        Test: Specific behavior
        Expected: Clear outcome
        Invariant: INV-SEC-001 (from tests/invariants/registry.toml)
        """
        # Setup
        data = create_test_data()
        
        # Execute
        result = function_under_test(data)
        
        # Assert
        assert result is not None
        assert result['status'] == 'success'
    
    def test_error_case(self):
        """Test error handling."""
        with pytest.raises(ValidationError):
            function_under_test(invalid_data)
```

### Register New Invariant
Edit `tests/invariants/registry.toml`:
```toml
[[invariants]]
id = "INV-NEWFEATURE-001"
description = "New security property"
tested_by = ["cross-chain-gpu-validator/tests/test_security_e2e.py::TestNewFeature::test_specific_case"]
```

## 11. Performance Testing

### Run tests with timing
```bash
pytest tests/ -v --durations=10
```

Shows slowest 10 tests. Typical:
- Input validation: <100ms
- RPC tests: <500ms per endpoint
- Dashboard: <200ms
- Total suite: 2-5 minutes

### Profile test execution
```bash
pytest tests/ --profile
```

## 12. Troubleshooting

### "ModuleNotFoundError: No module named 'cross_chain_gpu_validator'"
```bash
# Install dependencies
cd cross-chain-gpu-validator
pip install -e .
```

### "Connection refused" errors
```bash
# Mock RPC should handle these - verify MockRpcServer starts
grep -A 5 "class MockRpcServer" tests/test_rpc_endpoints_e2e.py
```

### Tests hang/timeout
```bash
# Run with timeout
pytest tests/ --timeout=300

# Or kill and check for infinite loops
pkill -f pytest
grep -n "while True:" tests/test_*.py
```

### Flaky tests (sometimes pass, sometimes fail)
```bash
# Run tests multiple times
pytest tests/ --count=5

# Check for timing-dependent assertions
grep -n "time\|sleep\|wait" tests/test_*.py
```

## 13. Test Maintenance

### Monthly
- [ ] Review test coverage (aim for >85%)
- [ ] Update mocks if behavior changed
- [ ] Add tests for new security features

### Quarterly
- [ ] Profile test performance
- [ ] Refactor slow tests
- [ ] Document new patterns

### Before Release
- [ ] Run full suite 3+ times (check for flaky tests)
- [ ] Generate coverage report
- [ ] Security review of critical tests
- [ ] Update CHANGELOG

## 14. Key Files Reference

```
cross-chain-gpu-validator/
├── tests/
│   ├── test_security_e2e.py          # 24 security tests
│   ├── test_dashboard_e2e.py         # 23 dashboard tests
│   ├── test_rpc_endpoints_e2e.py     # 21 RPC tests
│   ├── conftest.py                   # Pytest configuration
│   └── invariants/
│       └── registry.toml             # Test invariant registry
│
└── scripts/
    └── run-local-tests.sh            # Master test runner
```

## 15. Quick Reference

```bash
# Run everything
bash cross-chain-gpu-validator/scripts/run-local-tests.sh

# Run one suite
pytest tests/test_security_e2e.py -v

# Run one class
pytest tests/test_security_e2e.py::TestInputValidationSanitization -v

# Run one test
pytest tests/test_security_e2e.py::TestInputValidationSanitization::test_empty_signature_rejected -v

# With coverage
pytest --cov=cross_chain_gpu_validator tests/

# Debug mode
pytest -vv -s --pdb tests/test_security_e2e.py::TestInputValidationSanitization::test_empty_signature_rejected
```

---

**All tests passing?** You're ready for production! 🚀

Still debugging? Run with `-vv -s --tb=short` for detailed output.
