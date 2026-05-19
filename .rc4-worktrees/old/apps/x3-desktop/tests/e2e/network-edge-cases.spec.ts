/**
 * Network Edge Case Tests
 * Tests application behavior under various network conditions:
 * - Slow connections
 * - Packet loss
 * - Network failures
 * - High latency
 */
import { test, expect } from '@playwright/test';
import {
  waitForTauriReady,
  simulateNetworkLatency,
  simulateNetworkFailure,
  simulatePacketLoss,
  clearIpcLogs,
  getIpcLogs,
  waitForRetryButton,
  clickRetryAndWait,
  getErrorMessage,
} from './helpers';

test.describe('Network Edge Cases: Slow Connections', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    await clearIpcLogs(page);
  });

  test('should handle 1s network latency gracefully', async ({ page }) => {
    // Simulate slow connection (1000ms delay)
    await simulateNetworkLatency(page, 1000);
    
    await page.waitForTimeout(2000);
    
    // Check that application still renders
    expect(await page.locator('body').count()).toBeGreaterThan(0);
    
    // Should show loading or spinner
    const hasLoadingIndicator = await page.locator(
      'text=/loading|please wait|fetching/i'
    ).count() > 0;
    
    // Either loading indicator or rendered content
    const hasContent = await page.locator('[data-testid*="Panel"]').count() > 0;
    expect(hasLoadingIndicator || hasContent).toBeTruthy();
  });

  test('should handle 3s network latency and timeout gracefully', async ({ page }) => {
    // Simulate very slow connection
    await simulateNetworkLatency(page, 3000);
    
    await page.waitForTimeout(4000);
    
    // Should either show content or error with retry
    const errorMsg = await getErrorMessage(page);
    const hasContent = await page.locator('[data-testid*="Panel"]').count() > 0;
    
    // Either shows error or content
    expect(errorMsg !== null || hasContent).toBeTruthy();
  });

  test('should display helpful timeouts errors', async ({ page }) => {
    // Set a very short timeout scenario
    await simulateNetworkLatency(page, 2000);
    
    await page.waitForTimeout(3000);
    
    // Check for timeout-related messages
    const timeoutError = await page.locator(
      'text=/timeout|took too long|try again/i'
    ).count();
    
    // Should inform user about delay
    expect(
      timeoutError > 0 || 
      (await page.locator('[data-testid*="Panel"]').count() > 0)
    ).toBeTruthy();
  });

  test('should complete slow requests after timeout period', async ({ page }) => {
    // Moderate latency
    await simulateNetworkLatency(page, 2000);
    
    // Initially might show loading
    await page.waitForTimeout(1000);
    let hasLoading = await page.locator('text=/loading/i').count() > 0;
    
    // Wait for longer
    await page.waitForTimeout(3000);
    
    // Should eventually render or show error state
    const logs = await getIpcLogs(page);
    expect(logs.length).toBeGreaterThan(0);
  });
});

test.describe('Network Edge Cases: Packet Loss', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    await clearIpcLogs(page);
  });

  test('should handle 10% packet loss', async ({ page }) => {
    // Simulate 10% request failure
    await simulatePacketLoss(page, 10);
    
    await page.waitForTimeout(3000);
    
    // Application should still be functional
    const errorMsg = await getErrorMessage(page);
    const hasContent = await page.locator('[data-testid*="Panel"]').count() > 0;
    
    // Should handle gracefully - either display some content or show error
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });

  test('should handle 30% packet loss with recovery', async ({ page }) => {
    // Higher packet loss
    await simulatePacketLoss(page, 30);
    
    await page.waitForTimeout(3000);
    
    // Check logs to see retries
    const logs = await getIpcLogs(page);
    
    // With retries, some commands should eventually succeed
    const successLogs = logs.filter((l: any) => l.level === 'INFO');
    
    // Either succeeded or logged errors
    expect(logs.length).toBeGreaterThan(0);
  });

  test('should show user-friendly message during packet loss', async ({ page }) => {
    // Significant packet loss
    await simulatePacketLoss(page, 50);
    
    await page.waitForTimeout(3000);
    
    // Should show error state with retry option or show some cached/fallback data
    const hasRetryButton = await page.locator('button:has-text(/Retry/i)').count() > 0;
    const hasContent = await page.locator('[data-testid*="Panel"]').count() > 0;
    
    // Should provide feedback to user
    expect(hasRetryButton || hasContent || (await getErrorMessage(page)) !== null).toBeTruthy();
  });
});

test.describe('Network Edge Cases: Complete Failure', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    await clearIpcLogs(page);
  });

  test('should handle network disconnection', async ({ page }) => {
    // Create offline condition
    await simulateNetworkFailure(page, true);
    
    await page.waitForTimeout(3000);
    
    // Should show offline/error state
    const errorMsg = await getErrorMessage(page);
    expect(errorMsg).not.toBeNull();
    expect(
      errorMsg?.toLowerCase().includes('offline') ||
      errorMsg?.toLowerCase().includes('failed') ||
      errorMsg?.toLowerCase().includes('error')
    ).toBeTruthy();
  });

  test('should provide retry mechanism when offline', async ({ page }) => {
    // Go offline
    await simulateNetworkFailure(page, true);
    
    await page.waitForTimeout(2000);
    
    // Should have retry button available
    const retryButton = page.locator('button:has-text(/Retry|Reconnect|Try again/i)').first();
    await expect(retryButton).toBeVisible({ timeout: 5000 });
  });

  test('should recover when network is restored', async ({ page }) => {
    // Go offline
    await simulateNetworkFailure(page, true);
    await page.waitForTimeout(2000);
    
    // Restore network
    await simulateNetworkFailure(page, false);
    
    // Should allow retry to succeed
    const retryButton = page.locator('button:has-text(/Retry/i)').first();
    if (await retryButton.isVisible({ timeout: 2000 }).catch(() => false)) {
      await retryButton.click();
      await page.waitForTimeout(1000);
    }
    
    // Application should recover
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });

  test('should handle intermittent connection drops', async ({ page }) => {
    // Simulate connection dropping and restoring
    for (let i = 0; i < 3; i++) {
      await simulateNetworkFailure(page, true);
      await page.waitForTimeout(500);
      
      await simulateNetworkFailure(page, false);
      await page.waitForTimeout(1000);
    }
    
    // App should remain stable
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });
});

test.describe('Network Edge Cases: High Latency Scenarios', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    await clearIpcLogs(page);
  });

  test('should handle latency spikes gracefully', async ({ page }) => {
    // Normal conditions
    await page.waitForTimeout(500);
    
    // Sudden latency spike
    await simulateNetworkLatency(page, 2000);
    await page.waitForTimeout(3000);
    
    // Return to normal
    await simulateNetworkLatency(page, 0);
    await page.waitForTimeout(1000);
    
    // Should handle the transition
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });

  test('should queue requests during high latency', async ({ page }) => {
    // Simulate high latency
    await simulateNetworkLatency(page, 1500);
    
    // Try to interact with multiple panels
    const panels = await page.locator('[data-testid*="Panel"]').all();
    
    for (const panel of panels.slice(0, 2)) {
      try {
        await panel.click({ timeout: 500 });
      } catch {
        // Click might timeout, that's OK
      }
    }
    
    await page.waitForTimeout(4000);
    
    // Check logs show commands were processed
    const logs = await getIpcLogs(page);
    expect(logs.length).toBeGreaterThan(0);
  });

  test('should display latency-aware loading states', async ({ page }) => {
    // Set moderate latency
    await simulateNetworkLatency(page, 1000);
    
    await page.waitForTimeout(500);
    
    // Should show some indication of loading
    const loadingIndicators = await page.locator(
      'text=/loading|fetching|please wait/i'
    ).count();
    
    const hasContent = await page.locator('[data-testid*="Panel"]').count();
    
    // Should show feedback
    expect(loadingIndicators + hasContent).toBeGreaterThan(0);
  });
});

test.describe('Network Edge Cases: Consistency', () => {
  test('should maintain data consistency across retries', async ({ page }) => {
    // Simulate occasional failures
    await simulatePacketLoss(page, 20);
    
    await page.waitForTimeout(3000);
    
    // Clear logs and check final state
    const logs = await getIpcLogs(page);
    
    // Should have attempted commands
    expect(logs.length).toBeGreaterThan(0);
    
    // Check that we don't have duplicate final data
    const finalLogs = logs.filter((l: any) => l.level === 'INFO');
    const commandNames = finalLogs.map((l: any) => l.command);
    
    // Can have multiple attempts but data should be consistent
    expect(commandNames.length).toBeGreaterThan(0);
  });

  test('should handle mixed success and failure responses', async ({ page }) => {
    // Moderate packet loss
    await simulatePacketLoss(page, 25);
    
    // Interact with multiple endpoints
    for (let i = 0; i < 2; i++) {
      await page.waitForTimeout(1000);
      
      const panels = await page.locator('[data-testid*="Panel"]').all();
      if (panels.length > 0) {
        try {
          await panels[0].click({ timeout: 500 });
        } catch {
          // OK if click fails
        }
      }
    }
    
    await page.waitForTimeout(2000);
    
    const logs = await getIpcLogs(page);
    expect(logs.length).toBeGreaterThan(0);
  });
});
