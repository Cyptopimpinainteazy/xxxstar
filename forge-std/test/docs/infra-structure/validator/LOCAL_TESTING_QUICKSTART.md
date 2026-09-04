# Quick Start: Local E2E Testing (No Docker)

## Simple One-Command Setup

```bash
cd cross-chain-gpu-validator
./scripts/run-local-tests.sh
```

That's it! Tests run locally with mock RPC.

## What Happens

1. Creates Python virtual environment (`.venv/`)
2. Installs dependencies (pytest, coverage, etc.)
3. Runs all E2E security tests with mock RPC
4. Generates coverage report
5. Saves results to `./test-results/`

## Test Results

```
test-results/
├── security-results.xml      # Security tests
├── dashboard-results.xml     # Dashboard tests  
├── rpc-results.xml           # RPC endpoint tests
└── coverage/
    ├── index.html            # Coverage report (open in browser)
    └── ...
```

View coverage:
```bash
open test-results/coverage/index.html
```

## No Dependencies Required

✓ Python 3.10+ (already installed)  
✓ No Docker needed  
✓ No external services  
✓ No network access required  
✓ Runs in 2-5 minutes  

## Environment Variables (Optional)

```bash
# Use default mock RPC
export CCGV_USE_MOCK_RPC=true

# Or run specific test file
python -m pytest tests/test_security_e2e.py -v

# Verbose output
./scripts/run-local-tests.sh
```

## Test Coverage

- **Security** - Input validation, signatures, atomicity
- **Dashboard** - Metrics, real-time updates, XSS protection
- **RPC Endpoints** - Availability, response validation, consistency

## Troubleshooting

**If tests fail:**
```bash
# Check Python version
python3 --version  # Should be 3.10+

# Check dependencies
pip list | grep pytest

# Re-install
pip install -e .
```

**Clear cache and retry:**
```bash
rm -rf .venv test-results
./scripts/run-local-tests.sh
```

**Run single test file:**
```bash
source .venv/bin/activate
python -m pytest tests/test_security_e2e.py -v
```

## That's All!

No Docker, no complexity. Just run the script and get results.
