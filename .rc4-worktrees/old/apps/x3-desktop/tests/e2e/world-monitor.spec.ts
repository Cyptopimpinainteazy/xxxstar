import { test, expect } from '@playwright/test';
import { waitForTauriReady } from './helpers';

test.describe('World Monitor integration (App Store)', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/appstore');
    await waitForTauriReady(page);
  });

  test('opens World Monitor window and renders iframe', async ({ page }) => {
    // Open the World Monitor card
    await page.locator('text=World Monitor').first().click();

    // Click Launch App button in the detail modal
    await page.locator('button:has-text("Launch App")').first().click();

    // Wait briefly for window to open
    const win = page.locator('[role="dialog"][aria-label="World Monitor"]');
    await expect(win).toHaveCount(1);

    // The panel should render an iframe (iframe src will be local dev or cloud fallback)
    const iframe = win.locator('iframe').first();
    await expect(iframe).toHaveCount(1);

    // src should point to known World Monitor host or localhost dev server
    const src = await iframe.getAttribute('src');
    expect(src).toBeTruthy();
    expect(src).toMatch(/worldmonitor|localhost|127\.0\.0\.1/);
  });
});
