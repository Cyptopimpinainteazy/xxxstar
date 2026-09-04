# X3 Desktop E2E Testing Comprehensive Guide

## Overview

This document describes the complete end-to-end testing strategy for the X3 ecosystem, which includes:

1. **Tauri Desktop Application** (`/apps/x3-desktop`)
2. **X3 Intelligence Dashboard** (`/apps/x3-intelligence`)
3. **Backend API Server** (`/apps/x3-intelligence/server.js`)
4. **GPU Validator** (Python orchestrator)
5. **All 16+ Tauri Plugins** (shell, process, notification, clipboard, etc.)

## System Architecture Under Test

```
┌─────────────────────────────────────────────────────────────┐
│                   User's Browser/Desktop                      │
├─────────────────────────────────────────────────────────────┤
│  ┌──────────────┐              ┌──────────────────────────┐  │
│  │Tauri Desktop │              │X3 Intelligence Dashboard │  │
│  │(Rust + Vite) │              │   (React + Vite)        │  │
│  │ :7913        │              │    :5173                │  │
│  └──────┬───────┘              └────────────┬─────────────┘  │
│         │                                    │                │
│         └────────────────┬───────────────────┘                │
│                          │ (HTTP/IPC)                         │
└──────────────────────────┼──────────────────────────────────┘
                           │
        ┌──────────────────┴─────────────────────┐
        │  X3 Intelligence API Server            │
        │  Node.js/Express                       │
        │  :8001/api/v1/*                        │
        ├────────────────────────────────────────┤
        │ • Floor Stats Generator                │
        │ • Intent Data Provider                 │
        │ • Agent Metrics                        │
        │ • Slashing Events                      │
        │ • Dispute Tracking                     │
        └────────────────────────────────────────┘
```

## Test Suites Organization

### 1. **Full Integration Testing** (NEW)
**File**: `tests/e2e/full-integration.spec.ts`  
**Purpose**: End-to-end verification of entire system  
**Test Count**: 25+

#### Coverage Areas:

| Category | Tests | Verifies |
|----------|-------|----------|
| System Health | 1 | All services running (Redis, API, Validator) |
| Desktop Functionality | 2 | Window loads, navigation menu present |
| Data Flow | 3 | Floor stats, execution feed, live updates |
| IPC Communication | 2 | Desktop-browser sync, error handling |
| Performance | 2 | < 3s load time, rapid requests handled |
| API Validation | 2 | All endpoints healthy, correct schemas |
| Plugin Integration | 1 | No critical console errors |
| State Consistency | 1 | Data integrity across calls |
| Accessibility | 2 | Semantic HTML, error boundaries |
| User Workflow | 1 | Navigation, interaction, scrolling |

### 2. **Smoke Tests**
**File**: `tests/e2e/smoke-tests.spec.ts`  
**Purpose**: Basic functionality verification  
**Test Count**: Variable (TIER 6 & 7 CRM tests)  
**Coverage**: Contact management, data import/export, CRUD operations

### 3. **Backend Integration Tests**
**File**: `tests/e2e/tauri-backend.spec.ts`  
**Purpose**: Tauri IPC command execution  
**Test Count**: 18  
**Coverage**: Command execution, event handling, plugin responses

### 4. **Network Edge Cases**
**File**: `tests/e2e/network-edge-cases.spec.ts`  
**Purpose**: Resilience and error recovery  
**Test Count**: 19  
**Coverage**: Failures, latency, packet loss, timeouts

### 5. **Stress Testing**
**File**: `tests/e2e/stress-tests.spec.ts`  
**Purpose**: Load handling and concurrency  
**Test Count**: 17  
**Coverage**: Concurrent requests, high throughput, memory stability

## Quick Start

### Prerequisites

```bash
# Required services must be running
npm run dev                 # From /apps/x3-intelligence
# OR
bash scripts/start-beast.sh # Starts all services
```

### Running All Tests

```bash
cd /apps/x3-desktop
npm run test:e2e
```

### Running Specific Test Suite

```bash
# Just full integration tests
npx playwright test tests/e2e/full-integration.spec.ts

# Just smoke tests
npx playwright test tests/e2e/smoke-tests.spec.ts

# Just network edge cases
npx playwright test tests/e2e/network-edge-cases.spec.ts

# With heading for visual inspection
npx playwright test --headed
```

### Running with Debugging

```bash
# Visual debugging mode
npx playwright test --headed --debug

# Slow motion (500ms between actions)
npx playwright test --headed --slow-mo=500

# Single test only
npx playwright test -g "should fetch real-time floor stats"
```

### Generating Reports

```bash
# HTML report (opens automatically)
npx playwright test --reporter=html

# JSON report for CI/CD integration
npx playwright test --reporter=json > test-results.json

# JUnit XML for Jenkins
npx playwright test --reporter=junit
```

## Test Execution Workflow

### 1. **Pre-Test Verification**
```bash
# Check all services are running
curl http://localhost:8001/health
curl http://localhost:6379/ping
ps aux | grep -E "node|python|redis"
```

### 2. **Run Master E2E Script**
```bash
bash scripts/run-e2e-tests.sh
```

This script:
- ✓ Verifies system state (all services)
- ✓ Tests API health check
- ✓ Prepares test environment
- ✓ Runs complete E2E suite
- ✓ Generates detailed reports
- ✓ Provides deployment sign-off

### 3. **Review Test Results**

**HTML Report** (User-friendly):
```
open test-results/e2e-html/index.html
```

**JSON Report** (Machine-readable):
```bash
jq '.stats' e2e-results.json
```

**Execution Log**:
```bash
cat test-results/e2e-execution.log
```

## Test Data Flow

### Architecture Under Test

```
Browser/Desktop Tests
       │
       ├─ Playwright Browser Context
       │    └─ Navigate to http://localhost:5173
       │         └─ X3 Intelligence Dashboard loads
       │              └─ React app initializes
       │                   └─ useEffect hooks fire
       │                       └─ API Service calls
       │                           └─ fetch() to http://localhost:8001
       │
       └─ Playwright API Requests
            └─ Direct HTTP calls to
                 ├─ http://localhost:8001/api/v1/floor/stats
                 ├─ http://localhost:8001/api/v1/intents
                 ├─ http://localhost:8001/api/v1/agents
                 ├─ http://localhost:8001/api/v1/slashes
                 └─ http://localhost:8001/api/v1/disputes
```

### Expected Data Handling

1. **On First Load**:
   - Dashboard mounts
   - useEffect calls `getFloorStats()`
   - API returns real-time data
   - Component setState with proper TypeScript types
   - UI updates with actual metrics

2. **Every 3 Seconds**:
   - Interval tick calls `getFloorStats()` again
   - API returns new data (different values)
   - Component updates with fresh metrics
   - Browser shows live updates

3. **On Navigation**:
   - User clicks menu item (Intents, Agents, etc.)
   - New component mounts
   - useEffect fetches relevant data from API
   - Table/list populates with results
   - Pagination works correctly

## API Endpoints Covered

| Endpoint | Method | Purpose | Response Schema |
|----------|--------|---------|-----------------|
| `/health` | GET | Server status | `{ status: "ok" }` |
| `/api/v1/floor/stats` | GET | Floor metrics | `{ activeAgents, totalIntents, totalVolume, avgSuccessRate }` |
| `/api/v1/intents` | GET | Intent listings | `{ items: Intent[], page, pageSize, total }` |
| `/api/v1/agents` | GET | Agent listings | `{ items: Agent[], page, pageSize, total }` |
| `/api/v1/agents/:id` | GET | Agent details | `{ id, address, balance, performance, ... }` |
| `/api/v1/slashes` | GET | Slashing events | `{ items: SlashEvent[], page, pageSize, total }` |
| `/api/v1/disputes` | GET | Dispute listings | `{ items: Dispute[], page, pageSize, total }` |

## Test Helpers & Utilities

Located in `tests/e2e/helpers.ts`:

```typescript
// Tauri Integration
waitForTauriReady(page)          // Waits for Tauri CLI to initialize
waitForIpcCommand(page, cmd)    // Waits for specific IPC command

// Network Simulation
simulateNetworkLatency(ms)      // Adds latency to requests
simulateNetworkFailure()        // Simulates connection failure
simulatePacketLoss(percent)     // Simulates packet loss

// Performance
measureCommandPerformance(fn)   // Measures execution time
waitForIpcLogs()               // Gets IPC command log

// UI Verification
allPanelsRendered(page)        // Checks all UI panels
getErrorMessage(page)          // Extracts error text
```

## Performance Benchmarks

Tests verify these performance targets:

| Metric | Target | Actual |
|--------|--------|--------|
| Dashboard Load Time | < 3 seconds | ✓ Verified |
| API Response Time | < 500ms | ✓ Verified |
| State Refresh | 3 seconds | ✓ Verified |
| Concurrent Requests | 50+ | ✓ Verified |
| Panel Render Time | < 1 second | ✓ Verified |

## Plugin Testing Coverage

The test suite validates these Tauri plugins:

- ✓ Shell command execution
- ✓ Process management (spawn, kill, stdout/stderr)
- ✓ Notifications (create, close)
- ✓ Clipboard (read, write)
- ✓ Global shortcuts (register, unregister)
- ✓ Window state (minimize, maximize, fullscreen)
- ✓ File system (read, write, list)
- ✓ Logging (trace, debug, info, warn, error)
- ✓ Data storage (key-value persistence)

## Continuous Integration

For GitHub Actions / CI/CD:

```yaml
# .github/workflows/e2e-tests.yml
name: E2E Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-node@v3
      - run: npm ci
      - run: bash scripts/start-beast.sh &
      - run: npm run test:e2e
      - uses: actions/upload-artifact@v3
        if: always()
        with:
          name: playwright-report
          path: test-results/e2e-html/
```

## Troubleshooting

### Tests Timing Out

```bash
# Increase timeout for all tests
npx playwright test --timeout=60000

# Increase for specific test file
npx playwright test tests/e2e/network-edge-cases.spec.ts --timeout=120000
```

### API Not Responding

```bash
# Verify API is running
curl http://localhost:8001/health

# Check API logs
tail -f apps/x3-intelligence/api.log

# Restart API server
killall node
cd apps/x3-intelligence && node server.js &
```

### Dashboard Not Loading

```bash
# Check if dev server is running
curl http://localhost:5173

# Start dev server
cd apps/x3-intelligence && npm run dev &

# Check for port conflicts
lsof -i :5173
```

### Plugin Test Failures

```bash
# Verify Tauri runtime
cargo --version
npm list @tauri-apps/api

# Rebuild Tauri
cd apps/x3-desktop
cargo build --release

# Clear Tauri cache
rm -rf src-tauri/target
```

## Example Test Runs

### Scenario 1: New Deployment Verification

```bash
# 1. Start all services
bash scripts/start-beast.sh

# 2. Wait for services to be ready
sleep 10

# 3. Run comprehensive E2E suite
bash scripts/run-e2e-tests.sh

# 4. Review report
open test-results/e2e-html/index.html
```

**Expected Result**: All 54+ tests passing, zero critical errors

### Scenario 2: Bug Investigation

```bash
# 1. Run specific failing test with debug output
npx playwright test -g "should fetch real-time floor stats" --headed --debug

# 2. Step through with debugger
# 3. Inspect network tab for API calls
# 4. Check browser console for errors
# 5. Run with trace for post-mortem
npx playwright test -g "specific test" --trace on
```

### Scenario 3: Performance Regression Check

```bash
# 1. Run stress tests
npx playwright test tests/e2e/stress-tests.spec.ts --reporter=json > results.json

# 2. Compare with baseline
jq '.stats' results.json

# 3. Check for increased execution time
# 4. Review memory usage during test
```

## Success Criteria

A test run is considered successful when:

- ✓ All 54+ E2E tests pass
- ✓ Zero critical console errors
- ✓ API endpoints return HTTP 200 with valid schemas
- ✓ Dashboard updates in real-time
- ✓ All plugins initialize without errors
- ✓ Network edge cases handled gracefully
- ✓ Performance within benchmarks
- ✓ No memory leaks detected
- ✓ All panel renders complete

## Support & Documentation

- **Playwright Docs**: https://playwright.dev
- **Tauri Docs**: https://tauri.app
- **Vite Docs**: https://vitejs.dev
- **React Testing**: https://testing-library.com

## Summary

The E2E test suite provides comprehensive validation of the entire X3 ecosystem:

- **54+ tests** across 5 test suites
- **5 major systems** under test (desktop, dashboard, API, validator, plugins)
- **3-5 minute** typical execution
- **Automated reporting** (HTML, JSON, JUnit)
- **Network resilience** tested
- **Performance tracked** continuously
- **CI/CD ready** for automation

Run `bash scripts/run-e2e-tests.sh` to execute the full suite and validate system readiness.
