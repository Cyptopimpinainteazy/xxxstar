# TIER 6 & 7 Frontend Integration Guide
## How to Connect CRM & Social Features to Your UI

---

## QUICK START: Calling New Backend Functions

All new features are exposed as **Tauri commands** that can be called from TypeScript/React frontend.

### Basic Pattern

```typescript
// Import Tauri invoke
import { invoke } from '@tauri-apps/api/tauri';

// Call a CRM command
const result = await invoke('crm_create_contact', {
  userId: 'user123',
  input: {
    firstName: 'Alice',
    lastName: 'Wonder',
    email: 'alice@example.com',
    phone: '+1234567890',
    company: 'Tech Corp',
    jobTitle: 'CEO',
  }
});
```

---

## TIER 6: CRM INTEGRATION

### 1. Email Sending Feature

```typescript
// Send email to contact
const result = await invoke('crm_send_email', {
  userId: 'user123',
  input: {
    toEmail: 'contact@example.com',
    subject: 'Follow-up on our conversation',
    body: '<p>Hi, wanted to check in...</p>',
    contactId: 'contact123',
    templateId: null, // or use template ID
  }
});
```

### 2. Email Templates

```typescript
// Create template
const template = await invoke('crm_create_email_template', {
  userId: 'user123',
  input: {
    name: 'New Lead Follow-up',
    subject: 'Hello {{firstName}}!',
    body: '<p>Hi {{firstName}},</p><p>Thanks for your interest...</p>',
  }
});

// Use in campaign
await invoke('crm_send_email', {
  userId: 'user123',
  input: {
    toEmail: 'prospect@example.com',
    subject: template.subject,
    body: template.body.replace('{{firstName}}', 'John'),
    templateId: template.id,
  }
});
```

### 3. CSV Import

```typescript
// Import contacts from CSV
const csvContent = `first_name,last_name,email,phone,company
Alice,Wonder,alice@example.com,555-0001,Tech Corp
Bob,Builder,bob@example.com,555-0002,Build Inc`;

const result = await invoke('crm_import_csv', {
  userId: 'user123',
  input: {
    csvContent,
    columnMapping: {
      'first_name': 'firstName',
      'last_name': 'lastName',
      'email': 'email',
      'phone': 'phone',
      'company': 'company',
    },
    skipDuplicates: true,
    updateExisting: false,
  }
});

// Handle result
console.log(`Imported: ${result.importedCount}, Duplicates: ${result.duplicateCount}`);
```

### 4. CSV Export

```typescript
// Export all contacts to CSV
const csv = await invoke('crm_export_csv', {
  userId: 'user123',
});

// Download file
const blob = new Blob([csv], { type: 'text/csv' });
const url = URL.createObjectURL(blob);
const a = document.createElement('a');
a.href = url;
a.download = 'contacts.csv';
a.click();
```

### 5. Find & Merge Duplicates

```typescript
// Find all potential duplicates
const duplicates = await invoke('crm_find_duplicates', {
  userId: 'user123',
});

// Display duplicates to user, then merge
const merged = await invoke('crm_merge_contacts', {
  contactId: 'contact_primary', // unused but required
  userId: 'user123',
  input: {
    primaryId: 'contact_123', // Keep this ID
    secondaryId: 'contact_456', // Merge into primary
    keepFields: {
      'email': 'contact_123',  // Use email from contact_123
      'phone': 'contact_456',  // Use phone from contact_456
    },
  }
});
```

### 6. Campaign Management

```typescript
// Create campaign
const campaign = await invoke('crm_create_campaign', {
  userId: 'user123',
  input: {
    name: 'Q1 New Business Outreach',
    description: 'Reaching out to enterprise prospects',
    campaignType: 'email',
    scheduledAt: '2024-03-15T09:00:00Z',
  }
});

// Later: send campaign
for (const contactId of selectedContactIds) {
  const contact = await invoke('crm_get_contact', {
    contactId,
    userId: 'user123',
  });
  
  await invoke('crm_send_email', {
    userId: 'user123',
    input: {
      toEmail: contact.email,
      subject: 'Q1 Opportunity',
      body: 'Dear {{firstName}}...',
      contactId,
    }
  });
}
```

### 7. Lead Scoring

```typescript
// Calculate scores for all contacts
const scoredContacts = await invoke('crm_calculate_lead_scores', {
  userId: 'user123',
});

// Display in list with color coding
const getGradeColor = (grade: string) => {
  return {
    'A': '#10b981', // green
    'B': '#3b82f6', // blue
    'C': '#f59e0b', // amber
    'D': '#ef4444', // red
    'F': '#6b7280', // gray
  }[grade];
};

scoredContacts.forEach(item => {
  console.log(`${item.contact.firstName}: ${item.leadScore.grade} (${item.leadScore.score}/100)`);
});
```

### 8. Bulk Actions

```typescript
// Update multiple contacts at once
const result = await invoke('crm_bulk_update', {
  userId: 'user123',
  input: {
    contactIds: ['contact_1', 'contact_2', 'contact_3'],
    updates: {
      'stage': 'qualified',
      'priority': 'high',
    }
  }
});

console.log(`Updated ${result.successCount}, failed ${result.failureCount}`);
```

### 9. Pipeline Analytics

```typescript
// Get full pipeline analytics
const analytics = await invoke('crm_get_pipeline_analytics', {
  userId: 'user123',
});

// Display dashboard
console.log(`Total Pipeline Value: $${analytics.totalValue.toFixed(2)}`);
console.log(`Average Deal Size: $${analytics.averageDealValue.toFixed(2)}`);
console.log(`6-Month Forecast: $${analytics.weightedForecast.toFixed(2)}`);

// Show stage breakdown
Object.entries(analytics.stageBreakdown).forEach(([stage, stats]) => {
  console.log(`${stage}: ${stats.count} deals, $${stats.totalValue}, ${stats.winProbability}% win prob`);
});

// Display forecast chart
analytics.monthsForecast.forEach(forecast => {
  console.log(`${forecast.month}: $${forecast.confidenceMid.toFixed(0)} (${forecast.historicalAccuracy}% accuracy)`);
});
```

---

## TIER 7: SOCIAL INTEGRATION

### 1. Real-time Messaging

```typescript
// In Node.js/Rust backend, initialize WebSocket server
import { AppState } from './social/server';

const state = AppState::new();

// Subscribe to messages
const rx = state.subscribe();
while let Ok(msg) = rx.recv().await {
  // Route message to connected clients
}

// Send message from frontend
const ws = new WebSocket('ws://localhost:9001/ws/user123');

ws.onopen = () => {
  // Send direct message
  ws.send(JSON.stringify({
    from_user_id: 'user123',
    from_username: 'alice',
    to_user_id: 'user456',  // Direct to one user
    message: 'Hi!',
    message_type: 'chat',
    timestamp: new Date().toISOString(),
  }));

  // Broadcast announcement
  ws.send(JSON.stringify({
    from_user_id: 'user123',
    from_username: 'alice',
    to_user_id: null,  // Broadcast to all
    message: 'New product launch!',
    message_type: 'system',
    timestamp: new Date().toISOString(),
  }));
};

ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  console.log(`${msg.from_username}: ${msg.message}`);
};
```

### 2. Real-time Notifications

```typescript
// Create notification when action happens
import { NotificationManager, NotificationType } from './social/notifications';

// When user A likes user B's post
const notification = NotificationManager.post_liked(
  'user_B_id',
  'post_123',
  'user_A_id',
  'alice',
  'Alice Wonder',
  'https://example.com/alice.jpg'
);

// Send via WebSocket
ws.send(JSON.stringify({
  from_user_id: notification.from_user_id,
  from_username: notification.from_username,
  to_user_id: notification.user_id,  // Direct to User B
  message: JSON.stringify(notification),
  message_type: 'notification',
  timestamp: new Date().toISOString(),
}));

// Client receives and displays
ws.onmessage = (event) => {
  const msg = JSON.parse(event.data);
  if (msg.message_type === 'notification') {
    const notif = JSON.parse(msg.message);
    showNotification(`${notif.from_display_name}: ${notif.subject}`);
  }
};
```

### 3. ActivityPub Federation

```typescript
// When local user creates post
import { ActivityPubHandler } from './social/activitypub';

const handler = new ActivityPubHandler(
  'x3.network',
  'X3 Social',
  'admin@x3.network'
);

// Generate Activity
const activity = handler.post_to_activity(
  'post_123',
  'user_123',
  'Check out this amazing feature!',
  [['https://ipfs.io/ipfs/QmXxxx', 'image/jpeg']]
);

// Send to followers
const activity_json = JSON.stringify(activity);

// POST to each follower's inbox URL
for (const follower of user.followers) {
  const response = await fetch(follower.inbox, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/ld+json',
      'Date': new Date().toUTCString(),
      'Signature': generateSignature(activity_json),
    },
    body: activity_json,
  });
}
```

### 4. IPFS Media Upload

```typescript
// Initialize IPFS client
const config = {
  apiUrl: 'http://127.0.0.1:5001',
  gatewayUrl: 'https://ipfs.io',
  timeoutSecs: 30,
  maxFileSize: 100_000_000, // 100MB
};

const ipfs = new IpfsClient(config);

// Upload file
const fileInput = document.getElementById('image-upload');
const file = fileInput.files[0];

const result = await ipfs.add_file(file.path);

// Get URL
const ipfsUrl = result.url;  // https://ipfs.io/ipfs/bafyrei...
const gatewayUrls = result.gateway_urls;  // Fallback URLs

// Pin file permanently
await ipfs.pin_file(result.hash);

// Use in post
const post = {
  id: 'post_123',
  content: 'Check out my photo!',
  media: [{
    ipfs_hash: result.hash,
    gateway_url: ipfsUrl,
    file_type: 'image/jpeg',
    file_size: result.file_size,
  }],
};
```

---

## EXAMPLE: Full Email Campaign Flow

```typescript
async function runEmailCampaign() {
  const userId = 'user123';
  
  // 1. Import contacts
  const importResult = await invoke('crm_import_csv', {
    userId,
    input: {
      csvContent: csvData,
      columnMapping: fieldMap,
      skipDuplicates: true,
    }
  });
  console.log(`✅ Imported ${importResult.importedCount} contacts`);

  // 2. Score contacts
  const scoredContacts = await invoke('crm_calculate_lead_scores', { userId });
  const topLeads = scoredContacts
    .filter(item => item.leadScore.grade === 'A' || item.leadScore.grade === 'B')
    .map(item => item.contact);
  console.log(`✅ Found ${topLeads.length} A/B grade leads`);

  // 3. Create campaign
  const campaign = await invoke('crm_create_campaign', {
    userId,
    input: {
      name: 'Q1 Enterprise Outreach',
      campaignType: 'email',
      scheduledAt: new Date(Date.now() + 3600000).toISOString(),
    }
  });
  console.log(`✅ Created campaign: ${campaign.name}`);

  // 4. Send emails
  let successCount = 0;
  for (const contact of topLeads) {
    try {
      await invoke('crm_send_email', {
        userId,
        input: {
          toEmail: contact.email,
          subject: `${contact.firstName}, exclusive opportunity`,
          body: `Hi ${contact.firstName},\n\nWe have an exclusive offer for ${contact.company}..`,
          contactId: contact.id,
        }
      });
      successCount++;
    } catch (err) {
      console.error(`Failed to send to ${contact.email}: ${err}`);
    }
  }
  console.log(`✅ Sent ${successCount}/${topLeads.length} emails`);

  // 5. Get analytics
  const analytics = await invoke('crm_get_pipeline_analytics', { userId });
  console.log(`✅ Pipeline: $${analytics.totalValue} across ${analytics.totalDeals} deals`);
  console.log(`📊 6-month forecast: $${analytics.weightedForecast}`);
}

// Run it!
runEmailCampaign().catch(console.error);
```

---

## EXAMPLE: Real-time Social Flow

```typescript
// Setup
const socialState = new AppState();
const notificationManager = new NotificationManager();
const apHandler = new ActivityPubHandler('x3.network', 'X3', 'admin');

// When Alice follows Bob
async function handleFollowRequest(fromUserId: string, toUserId: string) {
  // 1. Save friendship
  await invoke('social_send_friend_request', { fromUserId, toUserId });

  // 2. Send notification
  const notif = NotificationManager::new_follower(
    toUserId,
    fromUserId,
    'alice',
    'Alice Wonder',
    'https://example.com/alice.jpg'
  );
  await socialState.send_notification(toUserId, notif);

  // 3. Federation
  const actor = apHandler.user_to_actor(fromUserId, ...);
  const acceptActivity = apHandler.accept_follow({ ... });
  // POST to Alice's inbox
}

// When Alice posts
async function handleNewPost(userId: string, content: string, mediaHashes: string[]) {
  // 1. Save locally
  const post = await invoke('social_create_blog_post', { userId, input: { body: content } });

  // 2. Broadcast notification
  const notif: NotificationType;
  // ... create notification for followers

  // 3. Federation - Send to Fediverse
  const activity = apHandler.post_to_activity(
    post.id,
    userId,
    content,
    mediaHashes.map(h => [`https://ipfs.io/ipfs/${h}`, 'image/jpeg'])
  );
  // POST to followers' inboxes
}
```

---

## TESTING YOUR INTEGRATION

### Frontend Tests

```typescript
describe('CRM Email', () => {
  it('should send email to contact', async () => {
    const result = await invoke('crm_send_email', {
      userId: 'test_user',
      input: {
        toEmail: 'test@example.com',
        subject: 'Test',
        body: 'Test email',
      }
    });
    expect(result.status).toBe('sent');
  });
});

describe('Social Messaging', () => {
  it('should receive messages via WebSocket', (done) => {
    const ws = new WebSocket('ws://localhost:9001/ws/user123');
    ws.onmessage = (event) => {
      expect(event.data).toBeDefined();
      done();
    };
  });
});
```

---

## COMMON ERRORS & SOLUTIONS

| Error | Cause | Solution |
|-------|-------|----------|
| `SMTP not configured` | SMTP credentials not saved | Run `crm_save_smtp_config` first |
| `Invalid to address` | Bad email format | Validate with regex: `/.+@.+\..+/` |
| `Duplicate detected` | Contact with same email exists | Set `skipDuplicates: false` to update |
| `WebSocket connection failed` | Server not running | Start server: `cargo build --release` |
| `IPFS timeout` | Node not running | Start IPFS: `ipfs daemon` |

---

## NEXT STEPS

1. **Wire UI Components** → Connect form submissions to Tauri commands
2. **Add Error Handling** → Handle all Result types properly
3. **Implement Loading States** → Show spinners during async operations
4. **Add Validation** → Validate email, phone before sending
5. **Setup WebSocket** → Connect frontend WebSocket client
6. **Test Locally** → Use dummy data before production
7. **Deploy** → Build release bundle and test on target platform

---

**Generated:** 2024-03-XX  
**Status:** Ready for Frontend Integration
