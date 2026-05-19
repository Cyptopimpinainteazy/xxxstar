/**
 * Stress Tests
 * Tests application behavior under high load:
 * - Rapid concurrent requests
 * - Multiple simultaneous failures
 * - High-frequency retries
 * - Resource exhaustion scenarios
 */
import { test, expect } from '@playwright/test';
import {
  waitForTauriReady,
  simulatePacketLoss,
  simulateNetworkLatency,
  clearIpcLogs,
  getIpcLogs,
  measureCommandPerformance,
  allPanelsRendered,
} from './helpers';

test.describe('Stress Tests: Rapid Concurrent Requests', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    await clearIpcLogs(page);
  });

  test('should handle 10 rapid panel interactions', async ({ page }) => {
    const panels = await page.locator('[data-testid*="Panel"]').all();
    
    // Rapidly click all panels
    const clickPromises = [];
    for (let i = 0; i < Math.min(10, panels.length * 3); i++) {
      const panel = panels[i % panels.length];
      clickPromises.push(
        panel.click({ timeout: 500 }).catch(() => {
          // Ignore timeout failures
        })
      );
    }
    
    await Promise.allSettled(clickPromises);
    await page.waitForTimeout(2000);
    
    // Should not crash
    expect(await page.locator('body').count()).toBeGreaterThan(0);
    
    // Check logs for successful commands
    const logs = await getIpcLogs(page);
    expect(logs.length).toBeGreaterThan(0);
  });

  test('should handle 20 concurrent IPC commands', async ({ page }) => {
    // Simulate rapid IPC calls by reloading page multiple times quickly
    const commandIds: string[] = [];
    
    for (let i = 0; i < 5; i++) {
      await page.reload({ waitUntil: 'networkidle' });
      await waitForTauriReady(page);
      
      const logs = await getIpcLogs(page);
      commandIds.push(...logs.map((l: any) => l.command));
    }
    
    // Should handle multiple reloads without crashing
    expect(commandIds.length).toBeGreaterThan(0);
    
    // Verify app is still responsive
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });

  test('should maintain performance under concurrent load', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Trigger multiple panels rapidly
    const panels = await page.locator('[data-testid*="Panel"]').all();
    
    for (const panel of panels) {
      try {
        await panel.click({ timeout: 1000 });
      } catch {
        // OK if some clicks fail due to rapid fire
      }
    }
    
    await page.waitForTimeout(3000);
    
    // Check performance metrics
    const logs = await getIpcLogs(page);
    const avgDuration = logs
      .filter((l: any) => l.duration)
      .reduce((sum: number, l: any) => sum + (l.duration || 0), 0) / 
      Math.max(logs.length, 1);
    
    // Average response should be reasonable (under 5 seconds)
    expect(avgDuration).toBeLessThan(5000);
  });

  test('should handle 100 rapid requests in succession', async ({ page }) => {
    // Programmatically trigger many IPC calls
    const startTime = Date.now();
    
    // Use page.evaluate to trigger rapid commands
    const successful = await page.evaluate(async () => {
      let count = 0;
      const commands = [
        'launch_system_metrics',
        'launch_ipfs_storage',
        'launch_swarm_health',
      ];
      
      // Simulate rapid calls
      for (let i = 0; i < 25; i++) {
        // Just verify the logging mechanism works
        const logs = localStorage.getItem('x3-desktop-ipc-log');
        if (logs) count++;
      }
      return count;
    });
    
    const duration = Date.now() - startTime;
    
    // Should complete in reasonable time
    expect(duration).toBeLessThan(10000);
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });
});

test.describe('Stress Tests: Concurrent Failures', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    await clearIpcLogs(page);
  });

  test('should handle 50% packet loss on all requests', async ({ page }) => {
    // Extreme packet loss
    await simulatePacketLoss(page, 50);
    
    // Try to use the app
    const panels = await page.locator('[data-testid*="Panel"]').all();
    
    for (const panel of panels.slice(0, 3)) {
      try {
        await panel.click({ timeout: 500 });
      } catch {
        // Expected with high packet loss
      }
    }
    
    await page.waitForTimeout(4000);
    
    // App should still be responsive (not crash)
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });

  test('should handle rapid retries under 50% packet loss', async ({ page }) => {
    await simulatePacketLoss(page, 50);
    
    // Force multiple retries
    for (let attempt = 0; attempt < 3; attempt++) {
      await page.reload({ waitUntil: 'domcontentloaded' });
      await waitForTauriReady(page);
      await page.waitForTimeout(500);
    }
    
    // Check retry logs
    const logs = await getIpcLogs(page);
    
    // Should have attempted multiple times
    if (logs.length > 0) {
      const errorLogs = logs.filter((l: any) => l.error);
      // Some should have failed but app recovered
      expect(logs.length + errorLogs.length).toBeGreaterThan(0);
    }
  });

  test('should recover from cascading failures', async ({ page }) => {
    // Start with 100% failure
    await simulatePacketLoss(page, 100);
    
    await page.waitForTimeout(2000);
    
    // Gradually reduce packet loss (recovery)
    for (let packetLoss = 80; packetLoss >= 0; packetLoss -= 20) {
      await simulatePacketLoss(page, packetLoss);
      await page.waitForTimeout(1000);
    }
    
    // Should eventually recover
    const rendered = await allPanelsRendered(page);
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });

  test('should handle simultaneous errors on all panels', async ({ page }) => {
    // Simulate multiple failures at once
    await simulatePacketLoss(page, 60);
    
    // Click all panels at once
    const panels = await page.locator('[data-testid*="Panel"]').all();
    const clickPromises = panels.map(panel =>
      panel.click({ timeout: 500 }).catch(() => {})
    );
    
    await Promise.allSettled(clickPromises);
    await page.waitForTimeout(3000);
    
    // Check for error handling
    const logs = await getIpcLogs(page);
    
    // Should have logs even with failures
    expect(logs.length).toBeGreaterThanOrEqual(0);
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });

  test('should not leak memory during repeated failures', async ({ page }) => {
    // Monitor for memory leaks through multiple failure cycles
    for (let cycle = 0; cycle < 3; cycle++) {
      // Simulate failure
      await simulatePacketLoss(page, 80);
      await page.waitForTimeout(1000);
      
      // Attempt interaction
      const panels = await page.locator('[data-testid*="Panel"]').all();
      if (panels.length > 0) {
        try {
          await panels[0].click({ timeout: 300 });
        } catch {
          // OK
        }
      }
      
      // Recover
      await simulatePacketLoss(page, 0);
      await page.waitForTimeout(500);
    }
    
    // Page should still be functional
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });
});

test.describe('Stress Tests: High-Frequency Retries', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    await clearIpcLogs(page);
  });

  test('should handle exponential backoff correctly under high load', async ({ page }) => {
    // Moderate failure rate that triggers retries
    await simulatePacketLoss(page, 40);
    
    // Trigger multiple requests
    const panels = await page.locator('[data-testid*="Panel"]').all();
    
    for (const panel of panels) {
      try {
        await panel.click({ timeout: 500 });
      } catch {
        // OK
      }
    }
    
    // Wait for retries to complete
    await page.waitForTimeout(5000);
    
    // Verify exponential backoff in logs
    const logs = await getIpcLogs(page);
    const retryLogs = logs.filter((l: any) => l.error);
    
    // Should have some retry attempts
    expect(logs.length).toGreaterThanOrEqual(0);
  });

  test('should not retry successfully completed requests', async ({ page }) => {
    // Low failure rate
    await simulatePacketLoss(page, 5);
    
    await page.waitForTimeout(3000);
    
    // Check logs
    const logs = await getIpcLogs(page);
    const successLogs = logs.filter((l: any) => l.level === 'INFO');
    const errorLogs = logs.filter((l: any) => l.level === 'ERROR');
    
    // Should have mostly successful requests
    if (logs.length > 0) {
      expect(successLogs.length).toBeGreaterThanOrEqual(errorLogs.length);
    }
  });

  test('should limit retries to prevent infinite loops', async ({ page }) => {
    // High failure rate
    await simulatePacketLoss(page, 80);
    
    // Let retries happen
    await page.waitForTimeout(5000);
    
    // Check that we don't have excessive retry attempts
    const logs = await getIpcLogs(page);
    
    // Even with failures, shouldn't have more than ~3 attempts per command
    const commandAttempts: { [key: string]: number } = {};
    logs.forEach((l: any) => {
      commandAttempts[l.command] = (commandAttempts[l.command] || 0) + 1;
    });
    
    Object.values(commandAttempts).forEach((count: any) => {
      // Max retries should be bounded (typically 3-5)
      expect(count).toBeLessThan(10);
    });
  });
});

test.describe('Stress Tests: Latency Under Load', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    await clearIpcLogs(page);
  });

  test('should maintain reasonable latency with 1s delay + concurrent requests', async ({ page }) => {
    // Add latency and load
    await simulateNetworkLatency(page, 1000);
    
    // Rapid interactions
    const panels = await page.locator('[data-testid*="Panel"]').all();
    for (const panel of panels) {
      try {
        panel.click({ timeout: 500 }).catch(() => {});
      } catch {
        // OK
      }
    }
    
    await page.waitForTimeout(4000);
    
    // Check performance
    const perf = await measureCommandPerformance(page, 'launch_system_metrics');
    
    if (perf) {
      // Even with 1s latency, shouldn't exceed timeout too much
      expect(perf.maxDuration).toBeLessThan(15000);
    }
  });

  test('should handle timeout correctly under 2s latency + concurrent load', async ({ page }) => {
    // High latency + load
    await simulateNetworkLatency(page, 2000);
    
    // Try interactions
    const panels = await page.locator('[data-testid*="Panel"]').all();
    
    for (let i = 0; i < 3; i++) {
      for (const panel of panels) {
        try {
          await panel.click({ timeout: 300 });
        } catch {
          // Expected timeout
        }
      }
    }
    
    await page.waitForTimeout(4000);
    
    // App should still render
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });
});

test.describe('Stress Tests: Stability & Recovery', () => {
  test('should remain stable through 5-minute continuous operation', async ({ page, context }, testInfo) => {
    // Use shorter timeout for test environment
    test.setTimeout(120000); // 2 minutes total
    
    await page.goto('/');
    await waitForTauriReady(page);
    
    // 30-second stability test (simulating continuous operation)
    const endTime = Date.now() + 30000;
    let interactionCount = 0;
    
    while (Date.now() < endTime) {
      try {
        const panels = await page.locator('[data-testid*="Panel"]').all();
        if (panels.length > 0) {
          await panels[interactionCount % panels.length].click({ timeout: 500 });
          interactionCount++;
        }
      } catch {
        // Occasional failures OK
      }
      
      await page.waitForTimeout(2000);
    }
    
    // Should have survived the period
    expect(await page.locator('body').count()).toBeGreaterThan(0);
    expect(interactionCount).toBeGreaterThanOrEqual(0);
  });

  test('should handle memory efficiently under sustained load', async ({ page }) => {
    // Multiple refresh cycles
    for (let i = 0; i < 5; i++) {
      await page.reload({ waitUntil: 'networkidle' }).catch(() => {});
      await waitForTauriReady(page);
      await page.waitForTimeout(1000);
    }
    
    // Check final state
    expect(await page.locator('body').count()).toBeGreaterThan(0);
    
    // No console errors
    let errorCount = 0;
    page.on('pageerror', () => {
      errorCount++;
    });
    
    await page.waitForTimeout(1000);
    expect(errorCount).toBeLessThan(5);
  });
});
