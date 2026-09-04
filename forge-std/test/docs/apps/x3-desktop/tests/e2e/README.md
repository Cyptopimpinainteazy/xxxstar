# E2E Tests Quick Reference

## Quick Start

```bash
cd apps/x3-desktop

# Run all E2E tests
npm run test:e2e

# Run specific test file
npx playwright test tests/e2e/tauri-backend.spec.ts

# Run in UI mode (interactive)
npx playwright test --ui

# Debug specific test
npx playwright test --debug tests/e2e/tauri-backend.spec.ts

# Show HTML report
npx playwright show-report
```

## Test Files Overview

| File | Tests | Focus | Duration |
|------|-------|-------|----------|
| `tauri-backend.spec.ts` | 18 | Tauri command execution | ~30s |
| `network-edge-cases.spec.ts` | 19 | Network failures, delays, packet loss | ~60s |
| `stress-tests.spec.ts` | 17 | Concurrent requests, high load | ~90s |
| **Total** | **54** | **Full application coverage** | **~3min** |

## Common Commands

### Run Single Test
```bash
npx playwright test -g "should load the application"
```

### Run Tests Matching Pattern
```bash
npx playwright test -g "network"  # Run all network edge case tests
npx playwright test -g "stress"   # Run all stress tests
```

### Headed Mode (See Browser)
```bash
npx playwright test --headed
```

### Debug Mode
```bash
npx playwright test --debug  # Step through test code
```

### Generate Report
```bash
npx playwright test
npx playwright show-report   # Open HTML report
```

### CI Mode (CI=1)
```bash
CI=1 npm run test:e2e  # Runs with 1 worker, 2 retries
```

## Test Helpers

All E2E tests have access to these helpers (from `helpers.ts`):

```typescript
// Wait for Tauri to initialize
await waitForTauriReady(page);

// Simulate network conditions
await simulateNetworkLatency(page, 1000);      // Add 1s delay
await simulateNetworkFailure(page, true);      // Go offline
await simulatePacketLoss(page, 50);            // 50% request failure

// IPC command testing
await waitForIpcCommand(page, 'launch_system_metrics');
const logs = await getIpcLogs(page);
await clearIpcLogs(page);

// Performance measurement
const perf = await measureCommandPerformance(page, 'launch_system_metrics');
console.log(perf.avgDuration);

// UI checks
const rendered = await allPanelsRendered(page);
const errorMsg = await getErrorMessage(page);
await waitForRetryButton(page);
await clickRetryAndWait(page);
```

## Test Categories

### Category 1: Backend Integration (18 tests)
Tests that the Tauri backend commands work correctly:
- System metrics retrieval
- IPFS storage management
- Swarm health monitoring
- Network control
- Storage monitoring
- IDE telemetry

**Expected**: All commands should complete within 15s and return data

### Category 2: Network Edge Cases (19 tests)
Tests how the app handles network issues:
- Slow connections (1-3s latency)
- Packet loss (10-50%)
- Complete offline (no connection)
- Intermittent failures
- Spike patterns

**Expected**: App should show errors gracefully and allow retry

### Category 3: Stress (17 tests)
Tests app stability under load:
- 10-100 rapid concurrent requests
- 50-80% packet loss with multiple failures
- Exponential backoff under stress
- Memory stability
- Continuous operation (30+ seconds)

**Expected**: App should not crash and should recover

## Environment Variables

```bash
# Enable CI mode (fewer workers, more retries)
export CI=1

# Run only specific project
PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1  # If already installed

# Verbose logging
DEBUG=pw:api
```

## Troubleshooting

### Tests hang or timeout
**Check**: Is `npm run tauri:dev` running?
**Fix**: Start the dev server in another terminal before running tests

### "page.route" not working
**Fix**: Add routes BEFORE page navigation
```typescript
await page.route('**/*', route => route.continue());
await page.goto('/');  // AFTER route setup
```

### Timed out waiting for "getByText"
**Fix**: Use `waitFor` with appropriate timeout
```typescript
await expect(element).toBeVisible({ timeout: 10000 });
```

### Test passes locally but fails in CI
**Check**: 
- Base URL matches CI environment
- CI has sufficient timeout (check playwright.config.ts)
- No hardcoded localhost references

## Writing New Tests

### Template
```typescript
import { test, expect } from '@playwright/test';
import { waitForTauriReady, getIpcLogs } from './helpers';

test('my new test', async ({ page }) => {
  // 1. Navigate and wait for ready
  await page.goto('/');
  await waitForTauriReady(page);
  
  // 2. Interact with app
  await page.click('[data-testid="button"]');
  
  // 3. Wait for async work
  await page.waitForTimeout(1000);
  
  // 4. Assert
  const logs = await getIpcLogs(page);
  expect(logs.length).toBeGreaterThan(0);
});
```

### Best Practices
- Always call `waitForTauriReady()` after first navigation
- Use `data-testid` attributes for reliable selectors
- Clear IPC logs before test: `await clearIpcLogs(page)`
- Use appropriate timeouts: 15s for IPC commands
- Wrap risky operations in try-catch if needed
- Avoid hardcoded delays - use `waitFor()` instead

## Performance Baselines

These are expected performance benchmarks:

| Scenario | Expected Duration | Max Duration |
|----------|-------------------|--------------|
| Single command | 100-500ms | 1s |
| Panel load | 200-800ms | 2s |
| Full dashboard | 1-3s | 5s |
| 10 concurrent | 500-2000ms | 5s |
| With 50% loss + retries | 3-10s | 15s |
| Under 1s latency | 1s + delay | 5s |

## Reports

After running tests:
- HTML Report: `test-results/e2e-html/index.html`
- JSON Report: `test-results/e2e-results.json`
- JUnit XML: `test-results/e2e-junit.xml`

View HTML report:
```bash
npx playwright show-report test-results/e2e-html
```

## CI Integration

Tests automatically run on:
- PR creation
- PRmerge requests with changes to `apps/x3-desktop`

See `.github/workflows/` for CI configuration.

---

**Ready for production ship! 🚀**
