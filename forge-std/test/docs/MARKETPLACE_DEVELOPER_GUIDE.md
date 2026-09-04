# X3 Marketplace Developer Guide

## Publishing Your Plugin

Stream revenue from your X3 extensions. Register plugins, manage ratings, track earnings.

### Prerequisites

- X3 developer account
- Plugin metadata (name, description, icon, docs URL)
- Plugin code (JavaScript, TypeScript, Rust, or Python)
- License (MIT, Apache 2.0, GPL 3.0 recommended)

### 10-Minute Setup

```typescript
import { MarketplaceClient, PluginInstaller } from '@x3/marketplace-sdk';

// Initialize marketplace client
const marketplace = new MarketplaceClient(
    'https://marketplace.x3.chain',
    'your-api-key'
);

// Register your first plugin
const plugin = {
    name: 'Analytics Dashboard Pro',
    category: 'Analytics',
    description: 'Real-time portfolio analytics with advanced metrics',
    author: 'Your Company',
    repository: 'github.com/yourcompany/analytics-pro',
    documentation: 'docs.yourcompany.com',
    license: 'MIT',
    icon_hash: 'QmXxxx...', // IPFS hash
    version: '1.0.0'
};

const pluginId = await marketplace.registerPlugin(plugin);
console.log('Published as:', pluginId);
// Wait 24 hours for approval
```

## Plugin Registration

### Complete Registration

```typescript
// Full plugin registration with metadata
const response = await marketplace.registerPlugin({
    // Required fields
    name: 'Trading Bot Assistant',
    category: 'Trading',
    version: '1.0.0',
    
    // Metadata
    description: 'AI-powered trading signal generator',
    author: 'AlgoTrading Inc',
    repository: 'github.com/algotrading/bot-assistant',
    documentation: 'docs.algotrading.io/bot-assistant',
    license: 'Apache-2.0',
    
    // Assets
    icon_ipfs_hash: 'QmAbcd...', // 512x512 PNG
    demo_url: 'demo.algotrading.io',
    
    // Legal
    terms_url: 'terms.algotrading.io',
    privacy_url: 'privacy.algotrading.io'
});

console.log('Plugin ID:', response.id);
console.log('Status:', response.status); // "Pending Approval"
```

### Plugin Categories

Choose the most relevant category:

| Category | Best For | Example |
|----------|----------|---------|
| **Authentication** | Wallet integrations, signin | Metamask Plugin |
| **Analytics** | Dashboards, metrics | Portfolio analyzer |
| **Wallet** | Treasury management | Hardware wallet bridge |
| **Trading** | Order execution, signals | Bot assistant |
| **Governance** | Voting, proposal | DAO toolkit |
| **Staking** | Delegation, rewards | Staking optimizer |
| **Bridge** | Cross-chain transfers | Multichain bridge |
| **Oracle** | Price feeds, data | Real-time oracle |
| **DeFi** | Lending, swaps, yields | DEX aggregator |
| **NFT** | Collections, marketplace | NFT gallery |
| **Social** | Community, messaging | Chat interface |
| **Other** | Miscellaneous | Custom tools |

### Approval Timeline

```
Day 0: submit plugin
Status: Pending Review
  ↓
Day 1-5: Security review
Status: Under Review
  ↓
Day 5: Decision
Status: Approved ✓ or Rejected ✗
```

**Approval Criteria:**
- No malware or unsafe code
- Functioning demo
- Clear documentation
- Unique value proposition

### Update Status

```typescript
// Check approval status
const status = await marketplace.getPluginStatus('plugin_id');
console.log('Status:', status);
// Output: "Approved", "Pending", "Rejected", "Suspended"

if (status === 'Rejected') {
    // Re-submit with corrections
    await marketplace.resubmitPlugin('plugin_id', updatedMetadata);
}
```

## Managing Your Plugin

### Release Updates

```typescript
// Release new version with changelog
const newVersion = await marketplace.releaseVersion('plugin_id', {
    version: '1.1.0',
    changelog: [
        'Added dark mode support',
        'Fixed memory leak in chart rendering',
        'Improved API response time by 40%'
    ],
    breaking_changes: false,
    code_hash: crypto.sha256(pluginCode)
});

console.log('Released:', newVersion.version);
console.log('Downloads:', newVersion.initial_download_count); // Usually ~0
```

### Breaking Changes

Clearly mark versions with breaking changes:

```typescript
// Version with breaking API changes
await marketplace.releaseVersion('plugin_id', {
    version: '2.0.0',
    changelog: [
        'Migrated to new SDK v2.0 API',
        'Changed config file format',
        'Dropped Node.js 14 support'
    ],
    breaking_changes: true  // <- Warn users!
});

// Users will see: "⚠️ Breaking Changes in v2.0.0"
```

### View Downloads and Metrics

```typescript
// Get plugin metrics
const metrics = await marketplace.getPluginMetrics('plugin_id');

console.log('Total Downloads:', metrics.total_downloads);
console.log('This Week:', metrics.weekly_downloads);
console.log('Active Users:', metrics.active_installs);
console.log('Uninstalls:', metrics.uninstalls_this_month);
console.log('Trend:', metrics.trending_rank); // 1=trending, N/A=not trending
```

### View All Versions

```typescript
// Get version history with stats
const versions = await marketplace.getVersionHistory('plugin_id');

versions.forEach(v => {
    console.log(`${v.version} (${v.downloads} downloads)`);
    console.log(`  Released: ${v.release_date}`);
    console.log(`  Hash: ${v.code_hash}`);
    if (v.breaking_changes) {
        console.log(`  ⚠️ Breaking Changes`);
    }
});
```

## Review System

### Understanding Reviews

Every user can rate your plugin 1-5 stars with a written review. Reviews drive discoverability.

```typescript
// Get your plugin's reviews
const reviews = await marketplace.getPluginReviews('plugin_id', limit: 20);

reviews.forEach(review => {
    console.log(`${review.rating} stars - ${review.reviewer}`);
    if (review.verified) {
        console.log('✓ Verified User');
    }
    console.log(review.content);
    console.log(`Helpful: ${review.helpful_count} vs ${review.unhelpful_count}`);
});
```

### Review Statistics

```typescript
// Get aggregated review stats
const stats = await marketplace.getRatingSummary('plugin_id');

console.log('Average Rating:', stats.average_rating.toFixed(1)); // 1.0 - 5.0
console.log('Total Reviews:', stats.total_reviews);
console.log('Quality Score:', stats.quality_score.toFixed(0) + '/100'); 
console.log('Recommended:', stats.percent_recommended + '%');

// Rating distribution
console.log('5-star:', stats.distribution[4]); // index 4 = 5 stars
console.log('4-star:', stats.distribution[3]);
console.log('3-star:', stats.distribution[2]);
console.log('2-star:', stats.distribution[1]);
console.log('1-star:', stats.distribution[0]);
```

**Quality Score Calculation:**
```
QS = (1×N1 + 2×N2 + 3×N3 + 4×N4 + 5×N5) / (Total×5) × 100

Example: 1×10 + 2×5 + 3×20 + 4×40 + 5×25 / (100×5) × 100 = 82%
```

### Top-Rated Plugins

Quality matters:
- **5.0 stars**: Exceptional (rare)
- **4.5+ stars**: Excellent (top 10%)
- **4.0+ stars**: Very good (top 25%)
- **3.5+ stars**: Good (top 50%)
- **3.0-3.5 stars**: Average
- **Below 3.0**: Consider improvements

```typescript
// Get top-rated plugins to benchmark
const topRated = await marketplace.getTopRatedPlugins(limit: 5);
topRated.forEach(plugin => {
    console.log(`${plugin.name}: ${plugin.average_rating.toFixed(1)} stars`);
});
```

## Earning Revenue

### Understanding Fee Distribution

X3 marketplace uses a **80/20 split**:
- **80%** → Plugin developer (you)
- **20%** → Platform (X3)

```typescript
// Example: User buys $100 license
Distribution:
  Total:             $100.00
  Your share (80%):  $80.00
  X3 share (20%):    $20.00
```

### Revenue Streams

1. **Download Fees** — Small one-time fee per install
2. **License Revenue** — Subscription or perpetual licenses
3. **Support Plans** — Premium support tiers
4. **Endorsements** — Featured placement (opt-in)

```typescript
// Set up license tiers for your plugin
const licenses = {
    free: {
        name: 'Community Edition',
        price: 0,
        features: ['basic analytics', 'community support']
    },
    pro: {
        name: 'Pro',
        price: 9.99,
        period: 'monthly',
        features: ['all features', 'email support', 'API access']
    },
    enterprise: {
        name: 'Enterprise',
        price: 99.99,
        period: 'monthly',
        features: ['everything', 'priority support', 'SLA']
    }
};
```

### Checking Your Earnings

```typescript
// Get current earnings breakdown
const earnings = await marketplace.getPublisherEarnings('your_publisher_id');

console.log('Total Earned:', earnings.total_earned);
console.log('Claimed:', earnings.total_claimed);
console.log('Pending:', earnings.pending); // Unclaimed in last 30 days

// Breakdown by plugin
earnings.by_plugin.forEach(plugin => {
    console.log(`${plugin.name}: $${plugin.total_earned}`);
});
```

### Payment History

```typescript
// View payment records (paginated)
const payments = await marketplace.getPaymentHistory(limit: 50);

payments.forEach(payment => {
    console.log(`${payment.date}: ${payment.amount}`);
    console.log(`  From: ${payment.source}`); // "downloads", "license", "endorsement"
    console.log(`  Your share: ${payment.your_share}`);
    console.log(`  Status: ${payment.status}`); // "pending", "paid", "failed"
});

// Filter by month
const novemberPayments = payments.filter(p => 
    p.date.getMonth() === 10 // November (0-indexed)
);
```

### Claiming Earnings

```typescript
// Withdraw your accumulated earnings
const result = await marketplace.claimEarnings();

console.log('Claimed:', result.amount);
console.log('Transaction ID:', result.transaction_id);
console.log('Status:', result.status); // "pending", "confirmed"

// Funds transferred to your wallet ~24 hours after pending
// Check transaction at: explorer.x3.chain/tx/{transaction_id}
```

## IPFS Metadata Management

Store plugin metadata on decentralized IPFS network.

### Upload Plugin Metadata

```typescript
// Create plugin metadata document
const metadata = {
    plugin_id: 'plugin_123',
    name: 'Trading Bot',
    description: 'AI trading signals',
    version: '1.2.0',
    author: 'Bot Co',
    documentation: {
        setup: 'https://docs.bot.co/setup',
        api: 'https://docs.bot.co/api',
        examples: 'https://github.com/bot-co/examples'
    },
    supported_platforms: ['desktop', 'mobile', 'web'],
    requirements: {
        min_memory: '256MB',
        min_storage: '500MB'
    },
    changelog: [
        { version: '1.2.0', date: '2024-01-15', features: ['Faster inference'] },
        { version: '1.1.0', date: '2024-01-01', features: ['Dark mode'] },
        { version: '1.0.0', date: '2023-12-01', features: ['Initial release'] }
    ]
};

// Upload to IPFS
const result = await marketplace.uploadMetadata(metadata);

console.log('IPFS Hash:', result.hash);
console.log('Pinned at:', result.url); // https://ipfs.x3.chain/ipfs/{hash}
console.log('Replication:', result.pins); // Number of nodes pinning

// Safe to reference in plugin version
```

### Retrieve Metadata

```typescript
// Get metadata for a plugin version
const metadata = await marketplace.getIPFSMetadata('QmXxxx...');

console.log(metadata.changelog);
console.log(metadata.requirements);
console.log(metadata.documentation);
```

### Backup & Replication

IPFS automatically pins to multiple nodes:

```typescript
// Check replication status
const status = await marketplace.getIPFSStatus('QmXxxx...');

console.log('Pinned by nodes:', status.pins);
if (status.pins >= 3) {
    console.log('✓ Well replicated');
} else {
    console.log('⚠️ Consider increasing replication');
    // Replication happens automatically over 7 days
}
```

## JavaScript SDK Integration

### Installing the SDK

```bash
npm install @x3/marketplace-sdk
```

### Basic Usage

```typescript
import { MarketplaceClient, PluginCategory } from '@x3/marketplace-sdk';

const marketplace = new MarketplaceClient(
    'https://marketplace.x3.chain',
    process.env.MARKETPLACE_API_KEY
);

// Search plugins
const results = await marketplace.searchPlugins('analytics');
console.log(`Found ${results.length} plugins`);

// Get trending plugins
const trending = await marketplace.getTrendingPlugins(10);

// Get top-rated in category
const topAnalytics = await marketplace.getTopRatedPlugins(
    PluginCategory.Analytics,
    5
);

// Install a plugin
const installer = new PluginInstaller(marketplace);
await installer.install('plugin_id');

// Check for updates
const updates = await installer.checkForUpdates('plugin_id');
if (updates.available) {
    console.log(`Update v${updates.version} available`);
    console.log('Changes:', updates.changelog);
}
```

### User Reviews Integration

```typescript
// Display plugin reviews
const reviews = await marketplace.getPluginReviews('plugin_id', 10);

reviews.forEach(review => {
    // In your UI:
    // <div class="review">
    //   <div class="rating">⭐⭐⭐⭐</div>
    //   <div class="title">{review.title}</div>
    //   <div class="content">{review.content}</div>
    //   <div class="helpful">👍 {review.helpful_count}</div>
    // </div>
});

// User submits review
await marketplace.submitReview('plugin_id', 5, 'Excellent!', 'Works perfectly...');

// Mark review as helpful
await marketplace.markHelpful('review_id');
```

## Publishing Best Practices

### Documentation Excellence

1. **README** (plugin repo)
   - What it does
   - Installation steps
   - Basic usage example
   - Troubleshooting

2. **API Documentation** (external)
   - Method signatures
   - Parameter descriptions
   - Return values and errors
   - Complete examples

3. **Video Tutorials**
   - 5-minute quick start
   - Advanced features walkthrough
   - Real-world scenario demo

### Rating Optimization

```
Rating factors (in order):
1. Does it work as promised? ✓ Most important
2. Is it fast and stable?
3. Is documentation clear?
4. Is support responsive?
5. Is the price fair?
```

**To improve ratings:**
- Add features users ask for
- Fix bugs promptly
- Respond to reviews
- Improve performance
- Reduce price for high quality

### Pricing Strategy

```typescript
// Conservative (safe for new plugins)
Free tier + $4.99/month Pro
Expected: 100-200 downloads/month, $50-100/month revenue

// Aggressive (for established plugins)
Free tier + $9.99/month + $49.99/year
Expected: Higher conversions, $200-500/month revenue

// Enterprise (for mature products)
Free tier + $19.99/month + $99/month enterprise
Expected: $500-2000+/month revenue
```

## Support and Resources

### Troubleshooting

**Plugin stuck in "Pending Approval"?**
```
Resolution time: 3-5 business days
If longer, check:
  1. Valid IPFS icon hash
  2. Working documentation URL
  3. Clear description & purpose
Contact: reviews@x3.chain
```

**Low download numbers?**
```
1. Improve rating (rate = downloads)
2. Update description with keywords
3. Add video demo
4. Announce on social media
5. Consider free tier
```

**Earnings not showing?**
```
1. Wait 24 hours after sale
2. Check "Pending" tab
3. Downloads require 7-day wait (fraud prevention)
4. Ensure payment info is complete
```

### Community

- **Slack**: marketplace.x3.chain/slack
- **Discord**: discord.x3.chain (channel: #marketplace)
- **GitHub**: github.com/x3-chain/marketplace-sdk
- **Email**: marketplace-support@x3.chain

### API Reference

Full reference: https://docs.x3.chain/marketplace/api

Key endpoints:
- `POST /plugins/register` — Register new plugin
- `GET /plugins/{id}` — Get plugin details
- `POST /plugins/{id}/review` — Submit review
- `GET /analytics/{id}` — Get metrics
- `POST /earnings/claim` — Claim earnings

---

**Version**: 1.0.0  
**Last Updated**: 2024  
**Support**: marketplace-support@x3.chain
