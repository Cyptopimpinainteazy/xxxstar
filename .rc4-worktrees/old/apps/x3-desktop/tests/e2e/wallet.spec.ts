import { test, expect } from '@playwright/test';
import { 
  waitForTauriReady, 
  simulateNetworkLatency,
  allPanelsRendered 
} from './helpers';

test.describe('Wallet Flow', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForTauriReady(page);
  });

  test('should show wallet setup view initially when not connected', async ({ page }) => {
    // Navigate to wallet panel if not already there
    // (Assuming there's a way to click the wallet icon in the dock/taskbar)
    // For now, assume it's the active panel or we can trigger it.
    
    // Check for the setup view
    const setupTitle = page.getByText(/DIGITAL COMMAND CENTER/i);
    await expect(setupTitle).toBeVisible();
    
    const initBtn = page.getByText(/Initialize Swarm Wallet/i);
    await expect(initBtn).toBeVisible();
  });

  test('should generate a new wallet and show dashboard', async ({ page }) => {
    const initBtn = page.getByText(/Initialize Swarm Wallet/i);
    await initBtn.click();
    
    // Check for "X3 Universal Wallet Created" modal
    const modalTitle = page.getByText(/X3 Universal Wallet Created/i);
    await expect(modalTitle).toBeVisible();
    
    // Close modal (assuming there's a close button or we click away)
    // If it automatically connects, we should see the dashboard
    await page.keyboard.press('Escape'); // Try closing modal
    
    // Check for dashboard elements
    const netWorth = page.getByText(/Net Worth/i);
    await expect(netWorth).toBeVisible();
  });

  test('should navigate between wallet views', async ({ page }) => {
    // Connect first if needed (usually handled by state persistence in dev)
    // But for clean test, we might need to initialize
    if (await page.getByText(/Initialize Swarm Wallet/i).isVisible()) {
      await page.getByText(/Initialize Swarm Wallet/i).click();
      await page.keyboard.press('Escape');
    }

    // Click Security in sidebar
    const securityLink = page.getByRole('button', { name: /Security/i });
    await securityLink.click();
    
    // Check for Security view
    const securityTitle = page.getByText(/ENCLAVE FIREWALL/i);
    await expect(securityTitle).toBeVisible();
    
    // Click DApps in sidebar
    const dappsLink = page.getByRole('button', { name: /Ecosystem/i }); // Sidebar label is "Ecosystem"
    await dappsLink.click();
    
    // Check for DApps view
    const dappsTitle = page.getByText(/X3 App Ecosystem/i);
    await expect(dappsTitle).toBeVisible();
  });

  test('should disconnect correctly', async ({ page }) => {
    // Ensure connected
    if (await page.getByText(/Initialize Swarm Wallet/i).isVisible()) {
      await page.getByText(/Initialize Swarm Wallet/i).click();
      await page.keyboard.press('Escape');
    }

    const logoutBtn = page.getByTestId('logout-btn');
    await logoutBtn.click();
    
    // Should be back to setup view
    const setupTitle = page.getByText(/DIGITAL COMMAND CENTER/i);
    await expect(setupTitle).toBeVisible();
  });
});
