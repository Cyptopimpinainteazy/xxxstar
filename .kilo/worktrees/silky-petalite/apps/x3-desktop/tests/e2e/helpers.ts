/**
 * E2E Test Helpers
 * Utilities for testing the Tauri desktop application
 */
import { Page, expect } from '@playwright/test';

/**
 * Wait for the Tauri window to be fully loaded
 */
export async function waitForTauriReady(page: Page) {
  // Wait for the app to hydrate
  await page.waitForFunction(
    () => typeof (window as any).__TAURI__ !== 'undefined',
    { timeout: 30000 }
  );
  
  // Give the app a moment to render
  await page.waitForTimeout(500);
}

/**
 * Simulate network latency by intercepting requests
 * @param page - Playwright page
 * @param delayMs - Delay to add to all requests
 */
export async function simulateNetworkLatency(page: Page, delayMs: number) {
  await page.route('**/*', async (route) => {
    await new Promise((resolve) => setTimeout(resolve, delayMs));
    await route.continue();
  });
}

/**
 * Simulate network failure
 * @param page - Playwright page
 * @param shouldFail - Whether to fail the requests
 */
export async function simulateNetworkFailure(page: Page, shouldFail: boolean) {
  if (shouldFail) {
    await page.route('**/*', (route) => {
      route.abort('failed');
    });
  } else {
    await page.unroute('**/*');
  }
}

/**
 * Simulate packet loss by randomly failing requests
 * @param page - Playwright page
 * @param percentFailure - Percentage of requests to fail (0-100)
 */
export async function simulatePacketLoss(page: Page, percentFailure: number) {
  await page.route('**/*', async (route) => {
    if (Math.random() * 100 < percentFailure) {
      route.abort('failed');
    } else {
      await route.continue();
    }
  });
}

/**
 * Wait for IPC command to complete
 * Watches for Tauri IPC logs or error states
 */
export async function waitForIpcCommand(
  page: Page,
  commandName: string,
  timeoutMs: number = 10000
) {
  const startTime = Date.now();
  
  while (Date.now() - startTime < timeoutMs) {
    // Check if command had error or completed
    const hasError = await page.evaluate(() => {
      const logs = localStorage.getItem('x3-desktop-ipc-log');
      if (!logs) return false;
      const entries = JSON.parse(logs);
      return entries.some(
        (e: any) => e.command === commandName && (e.error || e.level === 'INFO')
      );
    });
    
    if (hasError) return true;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  
  throw new Error(`Timeout waiting for IPC command: ${commandName}`);
}

/**
 * Get IPC log entries
 */
export async function getIpcLogs(page: Page) {
  return await page.evaluate(() => {
    const logs = localStorage.getItem('x3-desktop-ipc-log');
    return logs ? JSON.parse(logs) : [];
  });
}

/**
 * Clear IPC logs
 */
export async function clearIpcLogs(page: Page) {
  await page.evaluate(() => {
    localStorage.removeItem('x3-desktop-ipc-log');
  });
}

/**
 * Check if element has error state
 */
export async function hasErrorState(page: Page, selector: string): Promise<boolean> {
  try {
    const element = await page.locator(selector).first();
    return await element.evaluate((el) => {
      return (
        el.textContent?.toLowerCase().includes('error') ||
        el.getAttribute('aria-label')?.toLowerCase().includes('error') ||
        el.getAttribute('data-error') === 'true'
      );
    });
  } catch {
    return false;
  }
}

/**
 * Wait for retry button to appear (indicates error state with retry)
 */
export async function waitForRetryButton(page: Page, timeoutMs: number = 5000) {
  await page.locator('button:has-text(/Retry|Reload|Try again/i)').first().waitFor({
    timeout: timeoutMs,
  });
}

/**
 * Click retry button and wait for recovery
 */
export async function clickRetryAndWait(page: Page) {
  await page.locator('button:has-text(/Retry|Reload|Try again/i)').first().click();
  await page.waitForTimeout(500);
}

/**
 * Measure IPC command performance
 */
export async function measureCommandPerformance(
  page: Page,
  commandName: string
) {
  const logs = await getIpcLogs(page);
  const commandLogs = logs.filter((l: any) => l.command === commandName);
  
  if (commandLogs.length === 0) {
    return null;
  }
  
  const durations = commandLogs
    .filter((l: any) => l.duration)
    .map((l: any) => l.duration);
  
  return {
    count: commandLogs.length,
    avgDuration: durations.reduce((a, b) => a + b, 0) / durations.length,
    minDuration: Math.min(...durations),
    maxDuration: Math.max(...durations),
  };
}

/**
 * Check if all panels are rendered
 */
export async function allPanelsRendered(page: Page): Promise<boolean> {
  const panels = ['SystemMetricsPanel', 'IpfsStoragePanel', 'SwarmHealthPanel'];
  
  for (const panel of panels) {
    const element = await page.locator(`[data-testid="${panel}"]`).count();
    if (element === 0) return false;
  }
  
  return true;
}

/**
 * Get current error message from page
 */
export async function getErrorMessage(page: Page): Promise<string | null> {
  try {
    const errorText = await page.locator('[role="alert"]').first().textContent();
    return errorText || null;
  } catch {
    return null;
  }
}
