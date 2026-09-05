# X3 Atomic Star — Grant & Sponsor Prospects Database

**Purpose:** Track every potential grant, sponsor, credits program, or in-kind provider for the costly items in `docs/current/MAINNET_GAMEPLAN.md`.
**Maintained by:** Background research agents (spawned by session).
**Owner:** User (lojak).
**Last updated:** 2026-09-05.

> **How to use this DB:** Each row = one prospect. `STATUS` is the pipeline stage: `RESEARCH` (just found), `QUALIFIED` (looks like a fit), `APPLIED` (application sent), `AWARDED` (won), `REJECTED` (passed). `CONFIDENCE` is the agent's qualitative 1-5 rating.

---

## Costly items we need to fund

From `MAINNET_GAMEPLAN.md`:

| Need | Estimated cost | Timeline |
|---|---|---|
| External audit (runtime + contracts + ops) | $80k–$250k one-time | W4–W13 |
| Bug bounty program (initial pool) | $50k–$150k one-time + $20k–$50k/yr | W14 |
| 3-host testnet (VPS) | $150–$600/mo recurring | W0+ |
| Public RPC hosting (3 regions) | $300–$1500/mo recurring | W14+ |
| Legal counsel | $10k–$30k one-time | W24 |
| CI/CD compute | $200–$1000/mo recurring | W0+ |
| Hardware/ISP/datacenter co-lo | varies | optional |

---

## Prospect categories

| ID | Category | Subcategory |
|---|---|---|
| C1 | Cloud & infrastructure credits | Hyperscaler startup programs |
| C2 | Cloud & infrastructure credits | Web3-native compute marketplaces |
| C3 | Cloud & infrastructure credits | Decentralized physical infra (DePIN) |
| C4 | Web3 ecosystem grants | Polkadot / Substrate ecosystem |
| C5 | Web3 ecosystem grants | Cross-chain foundation grants |
| C6 | Web3 ecosystem grants | EVM-aligned grants |
| C7 | Web3 ecosystem grants | SVM / Solana-aligned grants |
| C8 | Audit grants | Audit-specific sponsorship programs |
| C9 | Bug bounty bootstrapping | Immunefi / HackerOne / Code4rena |
| C10 | Legal support | Pro-bono crypto legal clinics |
| C11 | Hardware / ISP / datacenter | Direct sponsorships |
| C12 | Open-source foundations | Linux Foundation, Apache, etc. |

---

## Prospects

(Populated by research agents. Each row includes: program name, what it covers, eligibility, award size, URL, fit notes, contact, status, confidence.)

### C1 — Cloud & infrastructure credits: Hyperscaler startup programs

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C1.1 | AWS Activate | Compute, storage, egress | $1k–$100k credits (2 tiers: Founders $1k, Portfolio $25k–$100k) | Early-stage startups, <10yr old, <$10M raised, not subsidiary of larger co. Apply via AWS Partner Network (APN) or VC referral. | https://aws.amazon.com/activate/ | Easy apply; portfolio tier requires VC intro; covers all AWS regions | RESEARCH | 4 |
| C1.2 | Google Cloud for Startups | Compute (GKE), storage, BigQuery, AI | $100k Cloud Credits (2yr); up to $350k additional for AI startups | Pre-seed to Series A, <5yr old, <$10M raised. Apply via Google Cloud partner (YC, etc.) or direct. | https://cloud.google.com/startup | GKE great for nodes; BigQuery nice for analytics-service | RESEARCH | 4 |
| C1.3 | Microsoft for Startups Founders Hub | Azure compute, GitHub Enterprise, M365 | Up to $150k Azure credits + GitHub Enterprise free | Any founder, any stage. Just sign up with LinkedIn. | https://foundershub.microsoft.com/ | Easiest apply of the three; Azure for some nodes + GitHub Actions for CI | RESEARCH | 4 |
| C1.4 | DigitalOcean Hatch | Compute (Droplets), storage, bandwidth | $5k credits for 1yr | Startups <$10M raised; apply via DO website | https://www.digitalocean.com/hatch | Good for cheap 3-host testnet; smaller than AWS/GCP | RESEARCH | 3 |
| C1.5 | Hetzner Cloud | Dedicated vCPU servers | No free tier; very cheap dedicated servers | Open to anyone; ~€4–€40/mo per server | https://www.hetzner.com/cloud | NOT a grant, but very cheap hardware for testnet; $50–$200/mo for 3 hosts | NOT-A-GRANT | 5 |
| C1.6 | OVHcloud Startup Program | Compute, storage | €10k credits | Apply via website | https://www.ovhcloud.com/en/startup/ | European provider; good geographic diversity | RESEARCH | 3 |
| C1.7 | Linode (Akamai) | Compute, storage | No startup program; $100 trial credit | Open to anyone | https://www.linode.com/ | Cheap; good for one-off nodes | NOT-A-GRANT | 3 |
| C1.8 | Vultr | Compute, storage | $100 trial credit | Open to anyone | https://www.vultr.com/ | Cheap; good geographic spread | NOT-A-GRANT | 3 |

### C2 — Cloud & infrastructure credits: Web3-native compute marketplaces

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C2.1 | Akash Network | Decentralized compute marketplace | Pay-as-you-go with AKT; can earn AKT by being a provider | Open marketplace | https://akash.network/ | Good for validator nodes; cheaper than AWS for sustained workloads | NOT-A-GRANT | 3 |
| C2.2 | Filecoin / Storacha (Web3.Storage) | Decentralized storage | Pay-as-you-go; some free tier | Open | https://web3.storage/ | Useful for IPFS-pinned chain data / archives | NOT-A-GRANT | 2 |
| C2.3 | Aleph Cloud | Decentralized compute + storage | Pay-as-you-go with ALEPH | Open marketplace | https://aleph.im/ | Smaller ecosystem but credible | NOT-A-GRANT | 2 |
| C2.4 | Fleek | Decentralized hosting | Free tier for small sites | Open | https://fleek.xyz/ | Could host explorer/faucet | NOT-A-GRANT | 2 |

### C3 — Cloud & infrastructure credits: DePIN (Decentralized Physical Infrastructure)

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C3.1 | Render Network | GPU compute for rendering | Pay-as-you-go with RNDR | Open marketplace | https://renderfoundation.com/ | Useful if we need GPU compute for validator benchmarks | NOT-A-GRANT | 2 |
| C3.2 | io.net | Distributed GPU compute | Pay-as-you-go | Open | https://io.net/ | Same as Render; cheaper for batch GPU | NOT-A-GRANT | 2 |
| C3.3 | Spheron Network | Decentralized compute | Pay-as-you-go; grants for select projects | Apply for grants | https://www.spheron.network/ | Web3-native; some grants available | RESEARCH | 3 |

### C4 — Web3 ecosystem grants: Polkadot / Substrate ecosystem

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C4.1 | Web3 Foundation Grants Program | Substrate-based chain infrastructure | $5k–$100k per milestone; rolling application | Open source, Substrate/polkadot-sdk based | https://grants.web3.foundation/ | **Best fit** — we're Substrate-based. Multi-stage: Application → Proposal → Milestone-based disbursement | RESEARCH | 5 |
| C4.2 | Polkadot Treasury (OpenGov) | Ecosystem public goods | Variable (DAO votes on each spend) | Anyone can submit a proposal; must pass referendum | https://polkadot.js.org/apps/#/treasury | Requires passing Polkadot community vote; good for larger asks | RESEARCH | 3 |
| C4.3 | Substrate Builder Program (by Parity) | Engineering support, ecosystem intros | No cash; technical mentorship | Teams building on Substrate | https://www.substrate.io/builders-program/ | Engineering leverage; can complement grant ask | RESEARCH | 4 |
| C4.4 | Moonbeam / Moonriver Grants | EVM-Substrate bridge projects | $5k–$50k | Projects integrating with Moonbeam/Moonriver | https://moonbeam.foundation/grants/ | Possible if we add Moonbeam integration | RESEARCH | 3 |
| C4.5 | Astar Foundation Grants | Substrate + EVM + WASM dApps | $5k–$50k | Projects on Astar ecosystem | https://www.astar.network/foundation | Lower fit unless we deploy to Astar | RESEARCH | 2 |
| C4.6 | Polkadot Pioneers Prize | Innovation in Polkadot ecosystem | $10k–$50k | Open to builders | https://pioneers.polkadot.network/ | Award-based (not grant); research + demo required | RESEARCH | 3 |

### C5 — Web3 ecosystem grants: Cross-chain foundation grants

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C5.1 | Ethereum Foundation (ESP) Grants | L2 / scaling / infra / research | $5k–$500k depending on scope | Open source Ethereum ecosystem | https://esp.ethereum.foundation/ | Possible if we add Ethereum L2 bridge | RESEARCH | 3 |
| C5.2 | Arbitrum Foundation Grants | Arbitrum ecosystem | $5k–$250k | Projects building on Arbitrum | https://arbitrum.foundation/grants | Specific to Arbitrum deployment | RESEARCH | 2 |
| C5.3 | Optimism RetroPGF | Public goods funding | Variable (token-weighted votes) | Public goods serving Optimism ecosystem | https://app.optimism.io/quest/19 | Retroactive; apply after building | RESEARCH | 2 |
| C5.4 | Polygon Village / Grants | Polygon ecosystem | $5k–$50k | Projects deploying on Polygon | https://polygon.technology/grants | Possible if we add Polygon bridge | RESEARCH | 2 |
| C5.5 | Base / Coinbase Grants | Base L2 ecosystem | Variable; via direct application | Projects on Base | https://www.coinbase.com/cloud/products/base | Specific to Base deployment | RESEARCH | 2 |

### C6 — Web3 ecosystem grants: EVM-aligned grants

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C6.1 | Gitcoin Grants Stack | Quadratic-funding rounds | Variable (community-matched) | Open source, public goods | https://gitcoin.co/grants/ | Community-driven; good for ongoing development | RESEARCH | 4 |
| C6.2 | Octant | Community-governed public goods | Variable (GLM token allocations) | Public goods projects | https://octant.app/ | Token-based; apply each epoch | RESEARCH | 3 |
| C6.3 | Giveth | Altruistic public goods funding | Variable | Open source | https://giveth.io/ | Smaller; good for early traction | RESEARCH | 2 |

### C7 — Web3 ecosystem grants: SVM / Solana-aligned grants

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C7.1 | Solana Foundation Grants | Solana ecosystem | $5k–$250k | Projects building on Solana | https://solana.org/grants | Relevant for SVM bridge work | RESEARCH | 4 |
| C7.2 | Metaplex Foundation | Solana NFT ecosystem | Variable | NFT-related projects | https://www.metaplex.com/ | Lower fit unless we add NFT support | RESEARCH | 2 |

### C8 — Audit grants

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C8.1 | Web3 Foundation Audit Grants | Audit funding for Substrate projects | Up to ~$50k per audit | Substrate-based projects with W3F grant | https://github.com/w3f/Grants-Program/blob/master/README.md#auditing-grants | Need to have W3F grant first | RESEARCH | 5 |
| C8.2 | OpenZeppelin Defender / Audit Services | Audit services | Variable; discounted | Projects using OZ libraries (we may be using OZ v4 in EVM contracts) | https://www.openzeppelin.com/security-audits | Direct engagement | NOT-A-GRANT | 4 |
| C8.3 | Trail of Bits Build Credit | Audit credits for OSS | Variable | Open-source security tooling | https://www.trailofbits.com/ | Direct engagement | NOT-A-GRANT | 4 |
| C8.4 | Cantina (Cantina.xyz) | Audit marketplace | Variable | Projects seeking audit | https://cantina.xyz/ | Crowd-sourced audit option; cheaper than boutique firms | RESEARCH | 4 |
| C8.5 | Code4rena | Audit competitions | Variable | Projects with deployed code | https://code4rena.com/ | Public audit competitions; cheaper | RESEARCH | 3 |

### C9 — Bug bounty bootstrapping

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C9.1 | Immunefi (Project Bonus) | Bonus pool matching | Up to $50k matching | Active Immunefi project | https://immunefi.com/ | Apply when launching bug bounty | RESEARCH | 4 |
| C9.2 | HackerOne (HackerOne Plus) | Platform + triage | Subscription fee | Enterprise tier | https://www.hackerone.com/ | Alternative to Immunefi | NOT-A-GRANT | 3 |
| C9.3 | Cantina (Bounties) | Bug bounty hosting | Platform fee | Projects on Cantina | https://cantina.xyz/bounties | Combined audit + bounty | RESEARCH | 3 |

### C10 — Legal support

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C10.1 | LexDAO | Pro-bono smart contract legal | Volunteer hours | Open-source Web3 projects | https://www.lexdao.community/ | Pro-bono legal for token launches, governance | RESEARCH | 3 |
| C10.2 | Crypto Legal Defense Fund | Litigation defense | Variable | Web3 projects facing legal threats | https://www.theblockchainassociation.org/ | Defensive only | RESEARCH | 2 |
| C10.3 | a16z Crypto Startup School legal clinic | Pro-bono legal advice | Limited hours | Selected a16z portfolio + applicants | https://a16zcrypto.com/ | Apply when ready | RESEARCH | 3 |
| C10.4 | Local law firm pro-bono (varies) | Local regulatory advice | Variable | Per firm | — | Find local Web3-experienced firm | RESEARCH | 3 |

### C11 — Hardware / ISP / datacenter co-location

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C11.1 | Equinix / Digital Realty co-lo sponsorship | Rack space, power, cross-connects | Variable; rare | Mid-to-large Web3 projects with revenue | https://www.equinix.com/ | Direct sponsorship programs rare; usually revenue-share | RESEARCH | 2 |
| C11.2 | Hetzner dedicated server leases | Dedicated hardware | ~€40–€150/mo per server | Open to anyone | https://www.hetzner.com/dedicated-rootserver | NOT a grant; very cheap dedicated machines (not VPS) | NOT-A-GRANT | 5 |
| C11.3 | OVH bare metal | Bare metal servers | €50–€200/mo per server | Open to anyone | https://www.ovh.com/bare-metal/ | Cheap dedicated hardware | NOT-A-GRANT | 4 |
| C11.4 | Local datacenter co-lo | Rack space | Variable | Geographic; depends on local provider | — | Many cities have crypto-friendly datacenters; inquire directly | RESEARCH | 2 |
| C11.5 | Home ISP business plans (static IPs) | Residential/business ISP | ~$60–$200/mo | Open to anyone | — | Local providers (Comcast Business, Verizon Fios, etc.) | NOT-A-GRANT | 3 |
| C11.6 | Helium / World Mobile Token (DePIN) | Decentralized wireless ISP | Pay-as-you-go with tokens | Open marketplace | https://www.helium.com/, https://worldmobiletoken.com/ | Interesting alternative if validators can be geographically distributed via DePIN | RESEARCH | 3 |

### C12 — Open-source foundations

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C12.1 | Linux Foundation / Hyperledger | Membership + ecosystem support | Membership fee | Open-source blockchain projects | https://www.linuxfoundation.org/ | Could provide legal framework + ecosystem cred | RESEARCH | 3 |
| C12.2 | Apache Software Foundation | Mentorship, legal umbrella | Variable | Apache-licensed projects | https://www.apache.org/ | Our license is Apache-2.0; could incubate | RESEARCH | 2 |
| C12.3 | Open Source Collective (fiscal sponsor) | 501(c)(6) fiscal sponsorship | Donation receiving | Open-source projects | https://www.oscollective.org/ | Useful for receiving tax-deductible donations | RESEARCH | 4 |
| C12.4 | GitHub Sponsors | Recurring donations | Variable | Open-source projects with GitHub presence | https://github.com/sponsors | Easy to enable; community-driven | RESEARCH | 4 |

---

## Recommended next actions

| Priority | Action | Owner | Deadline |
|---|---|---|---|
| P0 | Apply to **Web3 Foundation Grants Program** (C4.1) — best fit, $5k–$100k | You | W0+1 |
| P0 | Sign up for **AWS Activate** (C1.1) via partner referral if possible | You | W0 |
| P0 | Sign up for **Google Cloud for Startups** (C1.2) and **Microsoft for Startups** (C1.3) | You | W0 |
| P1 | Apply to **Substrate Builder Program** (C4.3) — engineering leverage | You | W0 |
| P1 | Engage **OpenZeppelin / Trail of Bits / Runtime Verification** for audit quotes | You | W0 |
| P1 | Sign up for **Immunefi** project bonus program (C9.1) ahead of W14 | You | W13 |
| P2 | Apply to **Solana Foundation Grants** (C7.1) when SVM bridge work begins | You | W7 |
| P2 | Apply to **Gitcoin Grants Stack** (C6.1) for ongoing community funding | You | W2 |
| P3 | Enable **GitHub Sponsors** (C12.4) | Agent | W1 |
| P3 | Enable **Open Source Collective** fiscal sponsorship (C12.3) | You | W2 |

---

## How to read the columns

- **PROGRAM** — name of the grant/sponsor
- **COVERS** — what they pay for (compute, audit, legal, cash, etc.)
- **AWARD** — typical size range
- **ELIGIBILITY** — basic requirements
- **URL** — application link
- **FIT NOTES** — assessment of fit for X3 Atomic Star
- **STATUS** — pipeline stage: RESEARCH / QUALIFIED / APPLIED / AWARDED / REJECTED
- **CONFIDENCE** — 1-5 rating by research agent

---

## Notes for the research agents

1. Each row should be filled with verifiable facts (URL, eligibility, award size).
2. Do NOT fabricate program names. If unsure, mark the row RESEARCH with confidence 1.
3. For categories where no direct grant exists, mark NOT-A-GRANT with the cheapest commercial alternative.
4. The user is in America/Denver timezone and likely US-based (check `USER.md`); many grant programs are geography-neutral but some require specific jurisdictions.
5. The repo is Substrate/Polkadot-based (`polkadot-sdk stable2512`) — this is the strongest ecosystem fit and should be highlighted in P0 grants.
