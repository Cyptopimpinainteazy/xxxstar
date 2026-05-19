// Comprehensive Smoke Test Suite for X3 Desktop TIER 6 & 7
// Uses Playwright for E2E, Jest for unit, and custom tests for integration

import { test, expect } from '@playwright/test';

// ============================================
// TIER 6: CRM System Smoke Tests
// ============================================

test.describe('TIER 6 - CRM System', () => {
  test.beforeEach(async ({ page }) => {
    // Navigate to CRM section
    await page.goto('http://localhost:5173');
    // Wait for app to load
    await page.waitForLoadState('networkidle');
  });

  // ========== Contact Management ==========

  test('Create new contact', async ({ page }) => {
    // Click "New Contact" button
    await page.click('button:has-text("New Contact")');
    
    // Fill form
    await page.fill('input[name="firstName"]', 'John');
    await page.fill('input[name="lastName"]', 'Doe');
    await page.fill('input[name="email"]', 'john.doe@example.com');
    await page.fill('input[name="company"]', 'Tech Corp');
    
    // Submit
    await page.click('button:has-text("Save Contact")');
    
    // Verify success message
    await expect(page.locator('text=Contact created successfully')).toBeVisible();
  });

  test('View all contacts', async ({ page }) => {
    // Navigate to contacts list
    await page.click('nav >> text=Contacts');
    
    // Verify contacts table loads
    await expect(page.locator('table')).toBeVisible();
    await expect(page.locator('thead')).toContainText('Name');
    await expect(page.locator('thead')).toContainText('Email');
  });

  test('Search contact by email', async ({ page }) => {
    await page.click('nav >> text=Contacts');
    
    // Type in search
    await page.fill('input[placeholder="Search contacts..."]', 'john.doe');
    
    // Verify filtered results
    await expect(page.locator('text=john.doe')).toBeVisible();
  });

  test('Edit contact details', async ({ page }) => {
    await page.click('nav >> text=Contacts');
    
    // Click first contact
    await page.locator('tr').first().click();
    
    // Edit field
    await page.fill('input[name="jobTitle"]', 'CEO');
    await page.click('button:has-text("Update")');
    
    // Verify update
    await expect(page.locator('text=Contact updated')).toBeVisible();
  });

  test('Delete contact', async ({ page }) => {
    await page.click('nav >> text=Contacts');
    
    // Right-click context menu
    await page.locator('tr').first().click({ button: 'right' });
    await page.click('text=Delete');
    
    // Confirm deletion
    await page.click('button:has-text("Confirm")');
    
    await expect(page.locator('text=Contact deleted')).toBeVisible();
  });

  // ========== CSV Import/Export ==========

  test('Import contacts from CSV', async ({ page }) => {
    await page.click('nav >> text=Tools');
    await page.click('text=Import CSV');
    
    // Upload file
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles('test-data/contacts.csv');
    
    // Verify preview
    await expect(page.locator('text=10 contacts found')).toBeVisible();
    
    // Start import
    await page.click('button:has-text("Import")');
    
    // Wait for completion
    await expect(page.locator('text=Import complete')).toBeVisible({ timeout: 10000 });
  });

  test('Export contacts to CSV', async ({ page }) => {
    await page.click('nav >> text=Tools');
    await page.click('text=Export CSV');
    
    // Trigger download
    const downloadPromise = page.waitForEvent('download');
    await page.click('button:has-text("Download CSV")');
    const download = await downloadPromise;
    
    // Verify file
    expect(download.suggestedFilename()).toBe('contacts.csv');
  });

  // ========== Deduplication ==========

  test('Find duplicate contacts', async ({ page }) => {
    await page.click('nav >> text=Tools');
    await page.click('text=Find Duplicates');
    
    // Start analysis
    await page.click('button:has-text("Scan")');
    
    // Wait for results
    await expect(page.locator('text=Duplicates found:')).toBeVisible({ timeout: 10000 });
  });

  test('Merge duplicate contacts', async ({ page }) => {
    await page.click('nav >> text=Tools');
    await page.click('text=Find Duplicates');
    
    await page.click('button:has-text("Scan")');
    await expect(page.locator('text=Duplicates found:')).toBeVisible({ timeout: 10000 });
    
    // Click merge on first duplicate pair
    await page.click('button:has-text("Merge")').first();
    
    // Confirm merge
    await page.click('button:has-text("Confirm Merge")');
    
    await expect(page.locator('text=Contacts merged')).toBeVisible();
  });

  // ========== Email ==========

  test('Send email to contact', async ({ page }) => {
    await page.click('nav >> text=Contacts');
    await page.locator('tr').first().click();
    
    // Open email composer
    await page.click('button:has-text("Send Email")');
    
    // Fill email
    await page.fill('input[name="subject"]', 'Test Subject');
    await page.fill('[contenteditable]', 'Test message body');
    
    // Send
    await page.click('button:has-text("Send")');
    
    // Verify success (check MailHog)
    await expect(page.locator('text=Email sent')).toBeVisible();
  });

  test('Use email template', async ({ page }) => {
    await page.click('nav >> text=Contacts');
    await page.locator('tr').first().click();
    await page.click('button:has-text("Send Email")');
    
    // Select template
    await page.click('select[name="template"]');
    await page.click('option:has-text("Follow-up")');
    
    // Verify template variables replaced
    await expect(page.locator('[contenteditable]')).toContainText('John');
    
    await page.click('button:has-text("Send")');
    await expect(page.locator('text=Email sent')).toBeVisible();
  });

  // ========== Campaigns ==========

  test('Create campaign', async ({ page }) => {
    await page.click('nav >> text=Campaigns');
    await page.click('button:has-text("New Campaign")');
    
    // Fill form
    await page.fill('input[name="name"]', 'Q1 Sales Push');
    await page.click('select[name="type"]');
    await page.click('option:has-text("Email")');
    
    // Select contacts
    await page.click('button:has-text("Select Contacts")');
    await page.locator('input[type="checkbox"]').first().check();
    await page.click('button:has-text("Done")');
    
    // Create
    await page.click('button:has-text("Create Campaign")');
    
    await expect(page.locator('text=Campaign created')).toBeVisible();
  });

  test('View campaign metrics', async ({ page }) => {
    await page.click('nav >> text=Campaigns');
    
    // Click on campaign
    await page.locator('[role="button"]:has-text("Q1")').first().click();
    
    // Verify metrics displayed
    await expect(page.locator('text=Sent')).toBeVisible();
    await expect(page.locator('text=Opened')).toBeVisible();
    await expect(page.locator('text=Clicked')).toBeVisible();
  });

  // ========== Lead Scoring ==========

  test('Calculate lead scores', async ({ page }) => {
    await page.click('nav >> text=Analytics');
    await page.click('text=Lead Scores');
    
    // Trigger calculation
    await page.click('button:has-text("Calculate")');
    
    // Wait for results
    await expect(page.locator('text=Scores calculated')).toBeVisible({ timeout: 10000 });
    
    // Verify grades displayed
    await expect(page.locator('text=Grade')).toBeVisible();
  });

  test('Filter contacts by score grade', async ({ page }) => {
    await page.click('nav >> text=Contacts');
    
    // Filter by score
    await page.click('button:has-text("Filter")');
    await page.click('text=By Score Grade');
    
    // Select grade A
    await page.click('input[value="A"]').first();
    
    // Verify filtering
    await expect(page.locator('text=(Grade: A)')).toBeVisible();
  });

  // ========== Analytics & Reports ==========

  test('View pipeline analytics', async ({ page }) => {
    await page.click('nav >> text=Analytics');
    await page.click('text=Pipeline');
    
    // Verify charts loaded
    await expect(page.locator('canvas')).toBeVisible(); // Chart.js
    
    // Verify metrics
    await expect(page.locator('text=Total Pipeline Value')).toBeVisible();
    await expect(page.locator('text=Win Probability')).toBeVisible();
  });

  test('Export analytics report', async ({ page }) => {
    await page.click('nav >> text=Analytics');
    
    const downloadPromise = page.waitForEvent('download');
    await page.click('button:has-text("Export Report")');
    const download = await downloadPromise;
    
    expect(download.suggestedFilename()).toContain('.pdf');
  });
});

// ============================================
// TIER 7: Social Network Smoke Tests
// ============================================

test.describe('TIER 7 - Social Network', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:5173');
    await page.waitForLoadState('networkidle');
  });

  test('Connect WebSocket', async ({ page }) => {
    // Monitor WebSocket connection
    const wsPromise = page.waitForEvent('websocket', ws => {
      return ws.url().includes('localhost:9001');
    });
    
    await page.click('nav >> text=Social');
    const ws = await wsPromise;
    
    expect(ws.url()).toContain('localhost');
  });

  test('Send direct message', async ({ page }) => {
    await page.goto('http://localhost:5173/social/messages');
    
    // Click on conversation
    await page.locator('div[role="button"]:has-text("Alice")').first().click();
    
    // Send message
    await page.fill('input[placeholder="Type a message..."]', 'Hello Alice!');
    await page.click('button[aria-label="Send"]');
    
    // Verify message appears
    await expect(page.locator('text=Hello Alice!')).toBeVisible();
  });

  test('Post to social feed', async ({ page }) => {
    await page.click('nav >> text=Social Feed');
    
    // Click compose
    await page.click('button:has-text("What\'s on your mind")');
    
    // Type post
    await page.fill('textarea', 'This is my first post on X3 Social!');
    
    // Post
    await page.click('button:has-text("Post")');
    
    await expect(page.locator('text=This is my first post')).toBeVisible();
  });

  test('Upload media to IPFS', async ({ page }) => {
    await page.click('nav >> text=Social Feed');
    await page.click('button:has-text("What\'s on your mind")');
    
    // Upload file
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles('test-data/image.jpg');
    
    // Verify preview
    await expect(page.locator('img')).toBeVisible();
    
    // Post with media
    await page.fill('textarea', 'Check out this image!');
    await page.click('button:has-text("Post")');
    
    // Verify IPFS hash displayed
    await expect(page.locator(/Qm[A-Za-z0-9]{44}/)).toBeVisible();
  });

  test('Receive notifications', async ({ page }) => {
    // Trigger a notification (like, follow, etc.)
    // In real test, this would be from another user
    
    // Check notification bell
    const notificationBell = page.locator('button[aria-label="Notifications"]');
    await expect(notificationBell).toContainText('1');
    
    // Click to view
    await notificationBell.click();
    
    // Verify notification list
    await expect(page.locator('text=/liked your post|followed you/')).toBeVisible();
  });

  test('ActivityPub federation - profile is discoverable', async ({ page }) => {
    // Test that profile is accessible via ActivityPub
    const response = await page.request.get('http://localhost:5173/.well-known/webfinger?resource=acct:testuser@localhost');
    
    expect(response.status()).toBe(200);
    const data = await response.json();
    expect(data.subject).toContain('testuser');
  });

  test('Like and unlike post', async ({ page }) => {
    await page.click('nav >> text=Social Feed');
    
    // Find first post like button
    const likeButton = page.locator('button[aria-label="Like"]:first-of-type');
    
    // Like
    await likeButton.click();
    await expect(likeButton).toHaveClass(/liked/);
    
    // Unlike
    await likeButton.click();
    await expect(likeButton).not.toHaveClass(/liked/);
  });

  test('Follow user', async ({ page }) => {
    await page.click('nav >> text=Discover');
    
    // Find user
    await page.fill('input[placeholder="Search users..."]', 'Alice');
    
    // Click follow
    await page.locator('button:has-text("Follow")').first().click();
    
    // Verify following
    await expect(page.locator('button:has-text("Following")')).toBeVisible();
  });

  test('Comment on post', async ({ page }) => {
    await page.click('nav >> text=Social Feed');
    
    // Click comment button
    await page.locator('button[aria-label="Comment"]:first-of-type').click();
    
    // Type comment
    await page.fill('input[placeholder="Write a comment..."]', 'Great post!');
    
    // Submit
    await page.click('button:has-text("Comment")');
    
    await expect(page.locator('text=Great post!')).toBeVisible();
  });

  test('View user profile', async ({ page }) => {
    await page.click('nav >> text=Social Feed');
    
    // Click on user profile link
    await page.locator('[role="link"]:has-text("User Name"):first-of-type').click();
    
    // Verify profile page
    await expect(page.locator('heading:has-text("User Name")')).toBeVisible();
    await expect(page.locator('text=Posts')).toBeVisible();
    await expect(page.locator('text=Followers')).toBeVisible();
  });
});

// ============================================
// Integration Tests
// ============================================

test.describe('Integration - CRM + Social', () => {
  test('Link contact profile to social account', async ({ page }) => {
    await page.goto('http://localhost:5173');
    
    // Navigate to contact
    await page.click('nav >> text=Contacts');
    await page.locator('tr').first().click();
    
    // Open link social account modal
    await page.click('button:has-text("Link Social Account")');
    
    // Search and select social user
    await page.fill('input[placeholder="Search user..."]', 'Alice');
    await page.click('text=alice_social');
    
    // Link
    await page.click('button:has-text("Link Account")');
    
    await expect(page.locator('text=Account linked')).toBeVisible();
  });

  test('Send email from campaign to social followers', async ({ page }) => {
    await page.click('nav >> text=Campaigns');
    await page.click('button:has-text("New Campaign")');
    
    // Select "Social Followers" source
    await page.click('text=Contact Source');
    await page.click('option:has-text("Social Followers")');
    
    // Create campaign
    await page.fill('input[name="name"]', 'Social Outreach');
    await page.click('button:has-text("Create Campaign")');
    
    await expect(page.locator('text=Campaign created')).toBeVisible();
  });
});

// ============================================
// Performance Tests
// ============================================

test.describe('Performance', () => {
  test('Dashboard loads in under 2 seconds', async ({ page }) => {
    const start = Date.now();
    
    await page.goto('http://localhost:5173/dashboard');
    await page.waitForLoadState('networkidle');
    
    const loadTime = Date.now() - start;
    expect(loadTime).toBeLessThan(2000);
  });

  test('Contacts list with 1000 items loads and is searchable', async ({ page }) => {
    await page.goto('http://localhost:5173/contacts');
    
    // Verify virtualization working (not all rows rendered)
    const visibleRows = page.locator('tbody tr');
    const count = await visibleRows.count();
    
    expect(count).toBeLessThan(100); // Virtualized, not all 1000 rendered
    
    // Search should be responsive
    await page.fill('input[placeholder="Search"]', 'john');
    await expect(page.locator('text=john')).toBeVisible({ timeout: 500 });
  });
});

// ============================================
// Error Handling Tests
// ============================================

test.describe('Error Handling', () => {
  test('Invalid email format shows error', async ({ page }) => {
    await page.goto('http://localhost:5173/contacts/new');
    
    await page.fill('input[name="email"]', 'invalid-email');
    await page.click('button:has-text("Save")');
    
    await expect(page.locator('text=Invalid email format')).toBeVisible();
  });

  test('Duplicate email shows warning', async ({ page }) => {
    await page.goto('http://localhost:5173/contacts/new');
    
    await page.fill('input[name="email"]', 'john.doe@example.com'); // Existing
    await page.click('button:has-text("Save")');
    
    await expect(page.locator('text=Contact with this email already exists')).toBeVisible();
  });

  test('Network error shows offline message', async ({ page }) => {
    // Simulate offline
    await page.context().setOffline(true);
    
    await page.goto('http://localhost:5173/contacts');
    
    await expect(page.locator('text=You are offline')).toBeVisible();
  });
});
