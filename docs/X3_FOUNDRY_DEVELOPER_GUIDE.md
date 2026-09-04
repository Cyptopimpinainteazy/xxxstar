# X3 Foundry Developer Guide

## Quick Start (5 Minutes)

### Prerequisites
- Node.js 18+
- npm or yarn
- A wallet with X3 tokens for deployment

### 1. Install the SDK

```bash
npm install @x3/foundry-sdk
# or
yarn add @x3/foundry-sdk
```

### 2. Initialize the Client

```typescript
import { FoundryClient } from '@x3/foundry-sdk';

const client = new FoundryClient({
  apiUrl: 'https://foundry.x3',
  chainId: 42, // Atlas Sphere
  privateKey: '0x...', // Optional: for deployment
});
```

### 3. Create Your First dApp

```typescript
// Create a project
const project = await client.createProject({
  name: 'My NFT Marketplace',
  description: 'A marketplace for digital art with auctions',
});

// Generate the dApp with AI
const generation = await client.generateDapp(
  project.id,
  'Build me an NFT marketplace with auctions, royalties, and X3 payments'
);

console.log('Generated:', generation);
```

### 4. Audit and Simulate

```typescript
// Run security audit
const audit = await client.auditDapp(project.id);
console.log('Audit score:', audit.risk_score);
console.log('Passed:', audit.passed);

// Simulate revenue
const simulation = await client.simulateDapp(project.id);
console.log('Expected monthly revenue:', simulation.fee_revenue);
console.log('Gas cost:', simulation.gas_cost);
```

### 5. Deploy

```typescript
// Deploy to Atlas Sphere
const receipt = await client.deployDapp(project.id, {
  chainId: 42,
  confirm: true, // Requires user confirmation
});

console.log('Deployed! Contract:', receipt.contract_address);
console.log('View at: https://apps.x3/' + receipt.app_slug);
```

## Creating a dApp from a Template

```typescript
// List available templates
const templates = await client.listTemplates();
console.log('Available templates:', templates.map(t => t.name));

// Create from template
const project = await client.createProject({
  name: 'My Staking Pool',
  templateId: 'staking_pool',
});

// Customize
const config = {
  platform_fee_bps: 200, // 2%
  staking_rewards_apy: 12, // 12% APY
  min_lock_period_days: 7,
};

const result = await client.generateDapp(project.id, JSON.stringify(config));
```

## Revenue Configuration

```typescript
// Update fee config before deployment
await client.updateFeeConfig(project.id, {
  platform_fee_bps: 200,      // 2% platform fee
  creator_fee_bps: 9700,      // 97% creator share
  referral_fee_bps: 50,       // 0.5% referral
  fee_mode: 'GrossRevenue',   // Fee mode
});

// Check projected revenue
const stats = await client.getRevenueStats(project.id);
console.log('Projected monthly:', stats.projected_monthly_revenue);
```

## Deployment Guide

### Supported Chains
- Atlas Sphere (chain_id: 42)
- Ethereum (chain_id: 1)
- Optimism (chain_id: 10)
- BSC (chain_id: 56)
- Polygon (chain_id: 137)
- Arbitrum (chain_id: 42161)
- Base (chain_id: 8453)
- And all 103 chains in the Universal Registry

### Deployment Options

```typescript
// Single chain deployment
await client.deployDapp(project.id, { chainId: 42 });

// Multi-chain deployment
await client.deployDapp(project.id, {
  chains: [42, 1, 137, 42161],
  crossChainRouting: true,
});

// With custom treasury
await client.deployDapp(project.id, {
  chainId: 42,
  treasuryAddress: '0x...',
  feeToken: '0x...',
});
```

### Deployment Receipt

```typescript
const receipt = await client.getDeploymentReceipt(project.id);
// {
//   project_id: 'uuid',
//   chain_id: 42,
//   contract_address: '0x...',
//   transaction_hash: '0x...',
//   block_number: 1234567,
//   timestamp: '2026-06-10T00:00:00Z',
//   app_slug: 'my-nft-marketplace',
//   marketplace_url: 'https://apps.x3/my-nft-marketplace',
// }
```

## Cross-Chain Deployment

X3 Foundry leverages the Universal Chain Registry and Cross-Chain Position Manager for multi-chain deployment.

```typescript
// Deploy across multiple chains
const receipt = await client.deployDapp(project.id, {
  chains: [
    { chainId: 42, atlas: true },     // Atlas Sphere native
    { chainId: 1, evm: true },        // Ethereum
    { chainId: 137, evm: true },      // Polygon
  ],
  crossChainRouting: true,
  feeToken: 'X3',                      // Use X3 for fees on all chains
});
```

## Fork / Remix Guide

```typescript
// Fork an existing app
const forkedProject = await client.forkProject(
  'original-app-id',
  { name: 'My Forked Marketplace' }
);

// Check lineage
const lineage = await client.getProjectLineage(forkedProject.id);
console.log('Original:', lineage.original_app_id);
console.log('Fork depth:', lineage.fork_depth);

// Set remix royalty
await client.updateFeeConfig(forkedProject.id, {
  remix_royalty_bps: 50, // 0.5% to original creator
});
```

## API Reference

### FoundryClient Methods

| Method | Description |
|--------|-------------|
| `createProject(config)` | Create a new dApp project |
| `generateDapp(projectId, prompt)` | Generate dApp from prompt |
| `simulateDapp(projectId)` | Simulate revenue and costs |
| `auditDapp(projectId)` | Run security audit |
| `deployDapp(projectId, options)` | Deploy dApp to chain(s) |
| `getRevenueStats(projectId)` | Get revenue statistics |
| `getProjectHealth(projectId)` | Get app health score |
| `updateFeeConfig(projectId, config)` | Update fee configuration |
| `claimCreatorRevenue(projectId)` | Claim creator earnings |
| `forkProject(projectId, config)` | Fork an existing app |
| `listTemplates(category?)` | List available templates |
| `searchMarketplace(query)` | Search marketplace listings |

## Best Practices

### Fee Configuration
- Start with default 2% platform fee
- Be transparent about all fees
- Never charge fees on user principal
- Show fee schedule to users before transactions

### Security
- Always run audit before deployment
- Review audit findings carefully
- Use timelocks for admin functions
- Implement emergency pause
- Test on testnet first

### Revenue Optimization
- Monitor revenue dashboard regularly
- Optimize fee structure based on volume
- Use referral system for growth
- Consider subscription model for recurring revenue

## Troubleshooting

### Common Issues

**Deployment fails with "Critical findings"**
→ Run audit again with `--verbose` flag
→ Check fee configuration for hidden fees
→ Verify principal safety checks pass

**Revenue not showing in dashboard**
→ Check that revenue router is configured
→ Verify fee token address is correct
→ Ensure dApp is in "Approved" status

**Cannot withdraw creator earnings**
→ Verify you are the registered creator
→ Check that earnings have been recorded
→ Ensure withdrawal amount is available

**Template not found**
→ Verify template ID is correct
→ Check template is not deprecated
→ List available templates with `listTemplates()`
