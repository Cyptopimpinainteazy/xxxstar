# X3 Foundry — AI-Powered dApp Factory

> **Describe it. Simulate it. Audit it. Deploy it. Earn from it.**

X3 Foundry is a self-service, AI-powered dApp creation economy built on Atlas Sphere. Users describe a dApp in plain language, and X3 Foundry generates the complete application — frontend, smart contracts, X3 modules, treasury hooks, fee routing, deployment scripts, tests, docs, and marketplace listing — then deploys it across Atlas Sphere and supported EVM/SVM chains.

## Core Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         X3 FOUNDRY                                   │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────┐  │
│  │   AI Builder      │    │   Security       │    │  Simulator   │  │
│  │   Agents          │───▶│   Auditor        │───▶│  Engine      │  │
│  │  • ProductArch    │    │  • Static Anal.  │    │  • Volume    │  │
│  │  • ContractEng    │    │  • Reentrancy    │    │  • Fees      │  │
│  │  • FrontendEng    │    │  • Fee Sanity    │    │  • Gas       │  │
│  │  • Tokenomics     │    │  • Scam Detect   │    │  • BreakEven │  │
│  │  • Compliance     │    │  • License Check │    │  • Treasury  │  │
│  └──────────────────┘    └──────────────────┘    └──────────────┘  │
│           │                       │                       │          │
│           ▼                       ▼                       ▼          │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Deployment Pipeline                        │   │
│  │  • Contracts  • Frontend  • Treasury  • Marketplace  • Analytics│   │
│  └──────────────────────────────────────────────────────────────┘   │
│           │                                                         │
│           ▼                                                         │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │                    Revenue Engine                              │   │
│  │  • Platform Fee (2%)  • Creator Share (97%)  • Treasury Split │   │
│  │  • Referral Rewards   • Maintenance Vault   • GPU Swarm       │   │
│  └──────────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────────┘
```

## Key Components

| Component | Description | Location |
|-----------|-------------|----------|
| **foundry-core** | Rust backend — AI agents, security, simulation, deployment | `crates/x3-foundry-core/` |
| **foundry-indexer** | On-chain event indexer (PostgreSQL) | `crates/x3-foundry-indexer/` |
| **foundry-revenue** | Revenue calculation and distribution | `crates/x3-foundry-revenue/` |
| **foundry-auditor** | Automated security auditing | `crates/x3-foundry-auditor/` |
| **foundry-sdk** | TypeScript SDK for developers | `packages/x3-foundry-sdk/` |
| **foundry-contracts** | Solidity smart contracts (13 contracts) | `X3-contracts/evm/contracts/foundry/` |
| **x3-templates** | X3-lang dApp templates (10 templates) | `x3-templates/` |

## User Flow

1. **Describe** — User enters a prompt: "Build me an NFT marketplace with auctions"
2. **Generate** — AI Builder Agents generate frontend, contracts, config, tests
3. **Audit** — Security pipeline runs static analysis, fuzz tests, scam detection
4. **Simulate** — Revenue simulator projects volume, fees, gas costs, break-even
5. **Review** — User reviews code diff, fee schedule, deployment cost
6. **Deploy** — One-click deployment across Atlas Sphere + EVM chains
7. **Earn** — Revenue routes automatically via the Revenue Engine

## Revenue Model

| Revenue Source | Platform Cut | Notes |
|---------------|-------------|-------|
| dApp protocol fees | 2% | Fair and scalable default |
| NFT sales fee | 2% of marketplace fee | Never on full sale price |
| Token launchpad raise | 1–3% | Depends on support level |
| Subscription apps | 2% | Clean SaaS model |
| Trading bot vaults | 5–10% of performance fee | Not principal |
| AI SaaS apps | 5% of credit sales | GPU swarm rewards |
| Domain registry apps | 2–5% | Recurring revenue |
| Referral/affiliate flows | 0.25–1% | Optional |

**Golden Rule:** Take from revenue, not principal.

## Security Philosophy

Every generated dApp must pass:
- Template integrity check
- Compiler verification
- Reentrancy scan
- Privilege scan
- Treasury fee sanity check
- Principal safety check
- Rug-pattern detection
- License compliance check
- Fuzz tests
- Dry-run deployment

**Deployment is blocked** if critical findings > 0 or platform fee is hidden.

## Quick Start

```bash
# Install the SDK
npm install @x3/foundry-sdk

# Initialize the client
import { FoundryClient } from '@x3/foundry-sdk';
const client = new FoundryClient({ apiUrl: 'https://foundry.x3', chainId: 42 });

# Create a dApp
const project = await client.createProject({ name: 'My NFT Marketplace' });
const result = await client.generateDapp(project.id, 'Build me an NFT marketplace with auctions');
const audit = await client.auditDapp(project.id);
const simulation = await client.simulateDapp(project.id);
const receipt = await client.deployDapp(project.id);
```

## Further Reading

- [Revenue Model](X3_FOUNDRY_REVENUE_MODEL.md)
- [Security Rules](X3_FOUNDRY_SECURITY_RULES.md)
- [Governance](X3_FOUNDRY_GOVERNANCE.md)
- [Developer Guide](X3_FOUNDRY_DEVELOPER_GUIDE.md)
- [User Guide](X3_FOUNDRY_USER_GUIDE.md)
