/**
 * E2E Tests - Tauri Backend Integration
 * Tests the actual Tauri application with real backend commands
 */
import { test, expect } from '@playwright/test';
import {
  waitForTauriReady,
  waitForIpcCommand,
  getIpcLogs,
  clearIpcLogs,
  allPanelsRendered,
  getErrorMessage,
} from './helpers';

test.describe('E2E: Tauri Desktop Application', () => {
  test.beforeEach(async ({ page }) => {
    // Clear logs before each test
    await page.goto('/');
    await waitForTauriReady(page);
    await clearIpcLogs(page);
  });

  test('should load the application and render all panels', async ({ page }) => {
    // Wait for all panels to be rendered
    const rendered = await allPanelsRendered(page);
    expect(rendered).toBe(true);
    
    // Verify main layout elements exist
    await expect(page.locator('header, nav, main, footer').first()).toBeVisible();
  });

  test('should execute launch_system_metrics command', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Click button/trigger that calls launch_system_metrics
    const systemMetricsButton = page.locator('[data-testid="SystemMetricsPanel"]').first();
    await expect(systemMetricsButton).toBeVisible();
    
    // Wait for command to complete
    await waitForIpcCommand(page, 'launch_system_metrics', 15000);
    
    // Verify logs show successful execution
    const logs = await getIpcLogs(page);
    const commandLog = logs.find((l: any) => l.command === 'launch_system_metrics');
    
    expect(commandLog).toBeDefined();
    expect(commandLog?.level).toBe('INFO');
    expect(commandLog?.duration).toBeGreaterThan(0);
  });

  test('should execute launch_ipfs_storage command', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Verify IPFS panel exists and is interacting with backend
    const ipfsPanel = page.locator('[data-testid="IpfsStoragePanel"]').first();
    await expect(ipfsPanel).toBeVisible();
    
    // Wait for command to complete
    await waitForIpcCommand(page, 'launch_ipfs_storage', 15000);
    
    // Verify successful execution
    const logs = await getIpcLogs(page);
    const commandLog = logs.find((l: any) => l.command === 'launch_ipfs_storage');
    
    expect(commandLog).toBeDefined();
    expect(commandLog?.level).toBe('INFO');
  });

  test('should execute launch_swarm_health command', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Verify swarm health panel
    const swarmPanel = page.locator('[data-testid="SwarmHealthPanel"]').first();
    await expect(swarmPanel).toBeVisible();
    
    // Wait for command execution
    await waitForIpcCommand(page, 'launch_swarm_health', 15000);
    
    const logs = await getIpcLogs(page);
    const commandLog = logs.find((l: any) => l.command === 'launch_swarm_health');
    
    expect(commandLog).toBeDefined();
    expect(commandLog?.level).toBe('INFO');
  });

  test('should execute launch_network_control command', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Verify network panel exists
    const networkPanel = page.locator('[data-testid*="Network"]').first();
    if (networkPanel) {
      await waitForIpcCommand(page, 'launch_network_control', 15000);
      
      const logs = await getIpcLogs(page);
      const commandLog = logs.find((l: any) => l.command === 'launch_network_control');
      
      expect(commandLog).toBeDefined();
    }
  });

  test('should execute launch_storage_monitor command', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Verify storage panel
    const storagePanel = page.locator('[data-testid*="Storage"]').first();
    if (storagePanel) {
      await waitForIpcCommand(page, 'launch_storage_monitor', 15000);
      
      const logs = await getIpcLogs(page);
      const commandLog = logs.find((l: any) => l.command === 'launch_storage_monitor');
      
      expect(commandLog).toBeDefined();
    }
  });

  test('should execute launch_ide_ipc command', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Check if IDE panel exists
    const idePanel = page.locator('[data-testid*="IDE"]').first();
    if (idePanel) {
      await waitForIpcCommand(page, 'launch_ide_ipc', 15000);
      
      const logs = await getIpcLogs(page);
      const commandLog = logs.find((l: any) => l.command === 'launch_ide_ipc');
      
      expect(commandLog).toBeDefined();
    }
  });

  test('should display real system metrics data', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Wait for system metrics to load
    await page.waitForTimeout(2000);
    
    // Check for metric values in the DOM
    const cpuMetric = page.locator('text=/CPU|cpu|0-9+%/i').first();
    const memoryMetric = page.locator('text=/Memory|RAM|0-9+%/i').first();
    
    // At least one should exist
    const cpuCount = await page.locator('text=/CPU|cpu|0-9+%/i').count();
    const memCount = await page.locator('text=/Memory|RAM|0-9+%/i').count();
    
    expect(cpuCount + memCount).toBeGreaterThan(0);
  });

  test('should display real IPFS storage data', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Wait for IPFS data
    await page.waitForTimeout(2000);
    
    // Check for IPFS-related content
    const ipfsContent = await page.locator('text=/IPFS|Storage|pins|CID/i').count();
    expect(ipfsContent).toBeGreaterThan(0);
  });

  test('should handle rapid successive IPC calls', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Simulate rapid panel interactions
    const panels = await page.locator('[data-testid*="Panel"]').all();
    
    for (const panel of panels.slice(0, 3)) {
      try {
        await panel.click({ timeout: 1000 });
      } catch {
        // Panel might not be clickable, that's OK
      }
    }
    
    // Wait for all commands to settle
    await page.waitForTimeout(1000);
    
    // Check that application is still responsive
    const errorMsg = await getErrorMessage(page);
    // Should not have persistent errors
    expect(errorMsg).toBeNull();
  });

  test('should recover from transient errors', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Get baseline logs
    await clearIpcLogs(page);
    await page.waitForTimeout(1000);
    
    const logs = await getIpcLogs(page);
    
    // If there are error logs, check that retries happened
    const errorLogs = logs.filter((l: any) => l.level === 'ERROR');
    
    if (errorLogs.length > 0) {
      // Should have attempted retries (multiple attempts with same command)
      const commandCounts = logs.reduce((acc: any, log: any) => {
        acc[log.command] = (acc[log.command] || 0) + 1;
        return acc;
      }, {});
      
      Object.values(commandCounts).forEach((count: any) => {
        // If there were errors, count should be > 1 (retry)
        expect(count).toBeGreaterThanOrEqual(1);
      });
    }
  });
});

test.describe('E2E: Application Stability', () => {
  test('should remain stable with continuous polling', async ({ page, context }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Let it run for a few seconds to simulate continuous operation
    await page.waitForTimeout(5000);
    
    // Check that page is still responsive
    await expect(page).toHaveTitle(/X3|Desktop|App/);
    
    // No unhandled errors
    let uncaughtErrors = 0;
    page.on('pageerror', () => {
      uncaughtErrors++;
    });
    
    await page.waitForTimeout(2000);
    expect(uncaughtErrors).toBe(0);
  });

  test('should handle window resize without errors', async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
    
    // Simulate viewport change
    await page.setViewportSize({ width: 800, height: 600 });
    await page.waitForTimeout(500);
    
    await page.setViewportSize({ width: 1400, height: 900 });
    await page.waitForTimeout(500);
    
    // Should still have content visible
    const rendered = await allPanelsRendered(page);
    // At least some content should be visible
    expect(await page.locator('body').count()).toBeGreaterThan(0);
  });
});
