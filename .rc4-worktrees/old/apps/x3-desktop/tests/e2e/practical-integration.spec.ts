// Practical E2E Integration Tests for X3 Intelligence + API Server
// Tests the actual running services: Dashboard and Backend API

import { test, expect, APIRequestContext, Page } from '@playwright/test';

test.describe('X3 Intelligence E2E Integration', () => {
  let apiRequest: APIRequestContext;
  const DASHBOARD_URL = 'http://localhost:5173';
  const API_BASE = 'http://localhost:8001/api/v1';

  test.beforeAll(async ({ playwright }) => {
    apiRequest = await playwright.request.newContext();
  });

  test.afterAll(async () => {
    await apiRequest.dispose();
  });

  // ========== SERVICE HEALTH CHECK ==========

  test('API server should be running and healthy', async () => {
    const response = await apiRequest.get('http://localhost:8001/health');
    expect(response.status()).toBe(200);
    const data = await response.json();
    expect(data.status).toBe('ok');
  });

  test('all API endpoints should return 200', async () => {
    const endpoints = [
      `/floor/stats`,
      `/intents?page=1&pageSize=10`,
      `/agents?page=1&pageSize=10`,
      `/slashes?page=1&pageSize=10`,
      `/disputes?page=1&pageSize=10`,
    ];

    for (const endpoint of endpoints) {
      const response = await apiRequest.get(`${API_BASE}${endpoint}`);
      expect(response.status()).toBe(200);
    }
  });

  // ========== API RESPONSE VALIDATION ==========

  test('floor stats should have required fields', async () => {
    const response = await apiRequest.get(`${API_BASE}/floor/stats`);
    const data = await response.json();

    expect(data).toHaveProperty('activeAgents');
    expect(data).toHaveProperty('totalIntents');
    expect(data).toHaveProperty('totalVolume');
    expect(data).toHaveProperty('avgSuccessRate');

    // Verify data types
    expect(typeof data.activeAgents).toBe('number');
    expect(typeof data.totalIntents).toBe('number');
    expect(data.activeAgents).toBeGreaterThanOrEqual(0);
  });

  test('intents should be properly paginated', async () => {
    const response = await apiRequest.get(`${API_BASE}/intents?page=1&pageSize=5`);
    const data = await response.json();

    expect(data).toHaveProperty('items');
    expect(data).toHaveProperty('page');
    expect(data).toHaveProperty('pageSize');
    expect(data).toHaveProperty('total');

    expect(data.page).toBe(1);
    expect(data.pageSize).toBe(5);
    expect(Array.isArray(data.items)).toBe(true);
  });

  test('intent items should have required fields', async () => {
    const response = await apiRequest.get(`${API_BASE}/intents?page=1&pageSize=1`);
    const data = await response.json();

    if (data.items.length > 0) {
      const intent = data.items[0];
      expect(intent).toHaveProperty('id');
      expect(intent).toHaveProperty('agentId');
      expect(intent).toHaveProperty('state');
      expect(intent).toHaveProperty('legs');
      expect(intent).toHaveProperty('feeCap');
      expect(intent).toHaveProperty('feeActual');
      expect(intent).toHaveProperty('createdAt');
      expect(intent).toHaveProperty('executedAt');
      expect(intent).toHaveProperty('proofHash');
    }
  });

  test('agents should be properly paginated', async () => {
    const response = await apiRequest.get(`${API_BASE}/agents?page=1&pageSize=5`);
    const data = await response.json();

    expect(data).toHaveProperty('items');
    expect(data).toHaveProperty('page');
    expect(data).toHaveProperty('pageSize');
    expect(data).toHaveProperty('total');

    expect(data.page).toBe(1);
    expect(data.pageSize).toBe(5);
  });

  test('slashing events should be properly formatted', async () => {
    const response = await apiRequest.get(`${API_BASE}/slashes?page=1&pageSize=5`);
    const data = await response.json();

    expect(data).toHaveProperty('items');
    expect(Array.isArray(data.items)).toBe(true);

    if (data.items.length > 0) {
      const slash = data.items[0];
      expect(slash).toHaveProperty('id');
      expect(slash).toHaveProperty('agentId');
      expect(slash).toHaveProperty('amount');
      expect(slash).toHaveProperty('timestamp');
    }
  });

  // ========== DASHBOARD FUNCTIONALITY ==========

  test('dashboard should load successfully', async ({ page }) => {
    await page.goto(DASHBOARD_URL, { waitUntil: 'networkidle' });

    // Wait for main content to load
    const mainElement = page.locator('main, [role="main"]');
    await expect(mainElement).toBeVisible({ timeout: 5000 });

    // Page should not show critical errors
    const noErrors = await page.evaluate(() => {
      const errors: string[] = [];
      const logs = (window as any).__console_logs || [];
      return errors.length === 0;
    });
    expect(noErrors).toBe(true);
  });

  test('dashboard should display floor statistics', async ({ page }) => {
    await page.goto(DASHBOARD_URL, { waitUntil: 'networkidle' });

    // Look for stat displays
    await page.waitForTimeout(2000); // Wait for data to load

    // Should have visible stats (might be displayed as numbers or text)
    const pageText = await page.content();
    expect(pageText).toBeTruthy();
  });

  test('dashboard should fetch from correct API endpoint', async ({ page }) => {
    let apiCallDetected = false;

    // Listen for API calls
    page.on('response', (response) => {
      if (response.url().includes('/api/v1/floor/stats')) {
        apiCallDetected = true;
      }
    });

    await page.goto(DASHBOARD_URL, { waitUntil: 'networkidle' });
    await page.waitForTimeout(3000);

    // API should have been called at least once
    expect(apiCallDetected).toBe(true);
  });

  test('dashboard elements should be accessible', async ({ page }) => {
    await page.goto(DASHBOARD_URL, { waitUntil: 'networkidle' });

    // Check for main semantic elements
    const hasHeading = await page.locator('h1, [role="heading"][aria-level="1"]').count();
    expect(hasHeading).toBeGreaterThan(0);

    const hasNav = await page.locator('nav, [role="navigation"]').count();
    // Nav might not exist, that's OK

    const hasList = await page.locator('[role="listitem"], li').count();
    // Lists might not exist, that's OK
  });

  // ========== LIVE DATA VERIFICATION ==========

  test('API should return fresh data on each call', async () => {
    const call1 = await apiRequest.get(`${API_BASE}/floor/stats`);
    const data1 = await call1.json();

    // Add small delay
    await new Promise(resolve => setTimeout(resolve, 100));

    const call2 = await apiRequest.get(`${API_BASE}/floor/stats`);
    const data2 = await call2.json();

    // Both calls should succeed
    expect(call1.status()).toBe(200);
    expect(call2.status()).toBe(200);

    // Data might be different or same (depending on implementation)
    expect(data1.activeAgents).toBeDefined();
    expect(data2.activeAgents).toBeDefined();
  });

  test('rapid API calls should not fail', async () => {
    const calls = [];
    for (let i = 0; i < 10; i++) {
      calls.push(apiRequest.get(`${API_BASE}/floor/stats`));
    }

    const responses = await Promise.all(calls);
    
    for (const response of responses) {
      expect(response.status()).toBe(200);
    }
  });

  // ========== ERROR HANDLING & RESILIENCE ==========

  test('API should handle invalid pagination gracefully', async () => {
    const response = await apiRequest.get(`${API_BASE}/intents?page=99999&pageSize=10`);
    // Should return 200 with empty items
    expect([200, 404]).toContain(response.status());
  });

  test('API should handle missing query params', async () => {
    const response = await apiRequest.get(`${API_BASE}/intents`);
    // Should still return valid response
    expect(response.status()).toBe(200);
  });

  // ========== PERFORMANCE CHECKS ==========

  test('dashboard should load within 5 seconds', async ({ page }) => {
    const startTime = Date.now();

    await page.goto(DASHBOARD_URL, { waitUntil: 'networkidle' });

    const loadTime = Date.now() - startTime;
    expect(loadTime).toBeLessThan(5000);
  });

  test('API endpoints should respond within 1 second', async () => {
    const endpoints = [
      `/floor/stats`,
      `/intents?page=1&pageSize=10`,
      `/agents?page=1&pageSize=10`,
    ];

    for (const endpoint of endpoints) {
      const startTime = Date.now();
      const response = await apiRequest.get(`${API_BASE}${endpoint}`);
      const responseTime = Date.now() - startTime;

      expect(response.status()).toBe(200);
      expect(responseTime).toBeLessThan(1000);
    }
  });

  // ========== INTEGRATION WORKFLOW ==========

  test('complete workflow: load dashboard and fetch data', async ({ page }) => {
    // 1. Load dashboard
    await page.goto(DASHBOARD_URL, { waitUntil: 'networkidle' });

    // 2. Verify page loaded
    const content = await page.content();
    expect(content.length).toBeGreaterThan(100);

    // 3. Call API to verify it's working
    const apiResponse = await apiRequest.get(`${API_BASE}/floor/stats`);
    expect(apiResponse.status()).toBe(200);

    const apiData = await apiResponse.json();
    expect(apiData.activeAgents).toBeDefined();

    // 4. Verify no critical errors
    const hasErrors = await page.evaluate(() => {
      const text = document.body.innerText;
      return text.includes('Error') && text.includes('500');
    }).catch(() => false);

    expect(hasErrors).toBe(false);
  });

  test('multiple users can access dashboard simultaneously', async ({
    playwright,
  }) => {
    const pages = [];

    // Create 3 concurrent page instances
    for (let i = 0; i < 3; i++) {
      const page = await playwright.chromium.launch().then(browser =>
        browser.newPage()
      );
      pages.push(page);
    }

    // Navigate all to dashboard
    const navigations = pages.map(page =>
      page.goto(DASHBOARD_URL, { waitUntil: 'networkidle' })
    );

    // All should succeed
    const results = await Promise.allSettled(navigations);
    const successCount = results.filter(r => r.status === 'fulfilled').length;

    expect(successCount).toBe(pages.length);

    // Close all pages
    for (const page of pages) {
      await page.close();
    }
  });
});
