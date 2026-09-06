# X3 Atomic Star — Grant & Sponsor Prospects Database

**Purpose:** Track every potential grant, sponsor, credits program, or in-kind provider for the costly items in `docs/current/MAINNET_GAMEPLAN.md`.
**Maintained by:** Background research agents (spawned by session).
**Owner:** User (lojak).
**Last updated:** 2026-09-05 (C8–C12 audit/bounty/legal/hardware/OSS deep-dive; C11 hardware summary added; C1–C3 cloud & infrastructure deep-dive by research subagent — 28 rows total, AWS Activate bumped to $200k, Microsoft URL fixed, OVH/Oracle/IBM/Alibaba/Tencent startup programs defunct, Ankr + Akash Grants added).

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
| C1.1 | AWS Activate | Compute, storage, egress | **Up to $200k** credits (Founders $1k self-serve; Portfolio $25k–$100k via VC/APN referral; AI startups eligible for additional credits beyond Activate) | Early-stage startups, <10yr old, not subsidiary of larger co. Apply via AWS Partner Network (APN) or VC referral. | https://aws.amazon.com/startups/ (old /activate/ URL 301-redirects here) | **Verified 2026-09-05**: URL works, redirects to /startups/. Award ceiling raised from $100k to $200k. Portfolio tier requires VC intro; covers all AWS regions | RESEARCH | 4 |
| C1.2 | Google Cloud for Startups | Compute (GKE), storage, BigQuery, AI | $100k Cloud Credits (2yr); up to $350k additional for AI startups | Pre-seed to Series A, <5yr old, <$10M raised. Apply via Google Cloud partner (YC, etc.) or direct. | https://cloud.google.com/startup | **Verified 2026-09-05**: URL works (200 OK; JS-rendered landing page). GKE great for nodes; BigQuery nice for analytics-service | RESEARCH | 4 |
| C1.3 | Microsoft for Startups (formerly Founders Hub) | Azure compute, GitHub Enterprise, M365, OpenAI | Up to $150k Azure credits + GitHub Enterprise free | Any founder, any stage. Sign up with LinkedIn or Microsoft account. | https://www.microsoft.com/en-us/startups | **Verified 2026-09-05**: old URL `foundershub.microsoft.com` no longer resolves (DNS fails). Rebranded to "Microsoft for Startups"; new URL works (200). Easiest apply of the three; Azure for nodes + GitHub Actions for CI + OpenAI credits | RESEARCH | 4 |
| C1.4 | DigitalOcean Startups (formerly Hatch) | Compute (Droplets), storage, bandwidth, GPU | $500 credit (90 days, any startup) up to $5k; GPU credit packages for select startups | Startups <$10M raised; apply via DO website | https://www.digitalocean.com/startups | **Verified 2026-09-05**: old URL `/hatch` 301-redirects to `/startups`. New page confirms $500 entry tier (90 days) and up to $5k for accepted startups. Select startups invited to GPU credit packages | RESEARCH | 3 |
| C1.5 | Hetzner Cloud | Dedicated vCPU servers | No free tier; very cheap dedicated servers | Open to anyone; ~€4–€40/mo per server | https://www.hetzner.com/cloud | NOT a grant, but very cheap hardware for testnet; $50–$200/mo for 3 hosts | NOT-A-GRANT | 5 |
| C1.6 | OVHcloud Startup Program | Compute, storage | €10k credits (per legacy listing) | Apply via website | https://www.ovhcloud.com/en/startup/ | **DEFUNCT 2026-09-05**: URL returns 404 across multiple regional variants (`/en/startup/`, `/en-ie/startups/`, `/en/startups`). Program appears to have been discontinued. OVH still offers commercial cloud but no public startup-credit program found | DEFUNCT | 1 |
| C1.7 | Linode (Akamai) | Compute, storage | No startup program; trial credit for new signups | Open to anyone | https://www.linode.com/ | **Verified 2026-09-05**: URL works (now branded as part of Akamai Cloud). No startup-credit program. Cheap; good for one-off nodes | NOT-A-GRANT | 3 |
| C1.8 | Vultr | Compute, storage | Trial credit for new signups (amount varies) | Open to anyone | https://www.vultr.com/ | **Verified 2026-09-05**: URL works (Cloudflare bot challenge returns 403 to scripted fetches but site is live). No startup program. Cheap; good geographic spread | NOT-A-GRANT | 3 |
| C1.9 | MongoDB for Startups | Atlas database credits, technical expertise, go-to-market | Variable; credits + Atlas free tier always available | Early-stage startups (must be venture-backed or accelerator-affiliated for top tier) | https://www.mongodb.com/solutions/startups | **NEW 2026-09-05**: Active program (mongodb.com/startups 301-redirects here). Useful only if we adopt MongoDB Atlas for indexing/anlytics (probably not for chain data). Lower fit unless we use Mongo | RESEARCH | 2 |
| C1.10 | Cloudflare Workers / R2 / D1 / KV (free tier) | Serverless compute, S3-compatible storage, SQLite, KV | **Free forever tier**: 100k req/day Workers, 10 GB R2 storage (zero egress), 5 GB D1, 100k KV reads/day | Open to anyone (just sign up) | https://workers.cloudflare.com/ | **NEW 2026-09-05**: No "startup program" but a very generous free tier ideal for explorer/faucet/CI edge workers. R2's zero-egress model is great for static frontends and chain-data archives. 330+ cities global. NOT a grant, but zero-cost | NOT-A-GRANT | 5 |
| C1.11 | Render.com PaaS (free tier) | Web services, static sites, Postgres, cron jobs | **Free tier**: 750 instance-hours/mo web service, 100 GB egress, free static sites | Open to anyone | https://render.com/ | **NEW 2026-09-05**: Different entity from Render Network (C3.1). Heroku-like PaaS. Free tier covers small explorer + faucet + docs site easily. Cheap paid tiers start at $7/mo. NOT a grant | NOT-A-GRANT | 4 |
| C1.12 | Oracle for Startups | Oracle Cloud credits | Variable (per program listing; historically $500–$25k) | Early-stage, VC-backed or accelerator-affiliated | https://www.oracle.com/startups/ | **NEW 2026-09-05**: All tested URLs 404 (`/startups`, `/startup-program/`, `/cloud/startups/`). Program appears defunct or moved. Skipped — DO NOT add without working URL. (Will revisit if URL found) | DEFUNCT | 1 |
| C1.13 | IBM Cloud Startup Program | IBM Cloud credits | Variable (historically up to $120k) | Early-stage in IBM portfolio or accelerators | https://www.ibm.com/startups | **NEW 2026-09-05**: Tested URLs 404. No current public IBM startup-credit program found. Skipped — DO NOT add without working URL | DEFUNCT | 1 |
| C1.14 | Alibaba Cloud Startup Program | Alibaba Cloud credits | Variable | China + APAC startups via partners | https://www.alibabacloud.com/en/startup-program | **NEW 2026-09-05**: All tested URLs 404 (`/en/startup-program`, `/en/developer/program/startup`, `/en/developer/plan/startup`, `developer.aliyun.com/startup`). Program appears removed or invitation-only now. Skipped | DEFUNCT | 1 |
| C1.15 | Tencent Cloud Startup Program | Tencent Cloud credits | Variable | China + APAC startups | https://intl.cloud.tencent.com/startup | **NEW 2026-09-05**: 404. No public international startup program found. Skipped | DEFUNCT | 1 |

### C2 — Cloud & infrastructure credits: Web3-native compute marketplaces

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C2.1 | Akash Network | Decentralized compute marketplace | Pay-as-you-go with AKT; can earn AKT by being a provider | Open marketplace | https://akash.network/ | **Verified 2026-09-05**: URL works. Site now markets itself as "Open Cloud for AI's Next Frontier" with GPU focus; AkashML and Console. Good for validator nodes; cheaper than AWS for sustained workloads. **NOTE**: Akash also has an Ecosystem Grants program (see C2.5) | NOT-A-GRANT | 3 |
| C2.2 | Filecoin / Storacha (now branded "fil.one") | Decentralized storage | $4.99/TB/mo; **1TB free for 30 days**, no credit card, no egress fees | Open | https://www.fil.one/ (old web3.storage, storacha.network all 301-redirect here) | **Verified 2026-09-05**: web3.storage now redirects to fil.one. Product is S3-compatible object storage with 11 nines durability. 1TB/30-day free trial is real, no card required. Useful for IPFS-pinned chain data / archives | NOT-A-GRANT | 3 |
| C2.3 | Aleph Cloud | Decentralized compute + storage + serverless | Compute from $0.0143/hr (vCPU, 2–24 GB RAM); GPU from $0.055/hr; volume storage from $0.0033/GB | Open marketplace | https://aleph.cloud/ (aleph.im 301-redirects) | **Verified 2026-09-05**: aleph.im redirects to aleph.cloud. Now branded "Aleph Cloud — AI & Web3 Cloud". TEE-based verifiable compute. Smaller ecosystem but credible; pay-as-you-go with ALEPH token | NOT-A-GRANT | 2 |
| C2.4 | Fleek | Decentralized hosting | Free tier for small sites | Open | https://www.fleek.sh/ (fleek.xyz 301-redirects; landing page sparse — "we've been building something new") | **Verified 2026-09-05**: URL works but landing page is sparse ("we've been building something new, share more soon"). Product may be in transition/rebrand; verify before relying on it. Could host explorer/faucet if product still operates | NOT-A-GRANT | 2 |
| C2.5 | Akash Ecosystem Grants | AKT-denominated grants for open-source tools, infrastructure, interfaces for Akash Network | Variable; Community Pool Proposals fully permissionless via on-chain vote | Anyone contributing to Akash ecosystem; Community Pool open to all via governance | https://akash.network/grants | **NEW 2026-09-05**: Active grants program. Two paths: (1) Community Contributions — smaller grants for docs/testnets/content; (2) Community Pool Proposals — permissionless, on-chain governance. We could use Akash for validator hosting AND apply for a grant | RESEARCH | 4 |
| C2.6 | Flux (RunOnFlux) | Decentralized Web3 cloud (compute, storage) | Pay-as-you-go with FLUX; 90% cheaper than AWS/GCP claimed | Open marketplace | https://runonflux.com/ | **NEW 2026-09-05**: 10,000+ nodes in 66 countries. Docker-based deployments. Strong validator-hosting candidate for Substrate (Docker-friendly, geographically distributed). No grants program found, but pricing is the draw | NOT-A-GRANT | 4 |
| C2.7 | Ankr (Web3 API / DePIN RPC) | Multi-chain RPC endpoints, bare-metal nodes, scaling services | Free tier for RPC; paid plans from ~$25/mo per endpoint; 8B daily RPC requests infrastructure | Open | https://www.ankr.com/ | **NEW 2026-09-05**: 30+ global regions, 760k unique geo locations served monthly, 99.99% uptime. EXCELLENT fit for Substrate/Polkadot public RPC hosting — they support 70+ chains. Free tier for low-volume; could host one of our 3-region RPCs | NOT-A-GRANT | 4 |

### C3 — Cloud & infrastructure credits: DePIN (Decentralized Physical Infrastructure)

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C3.1 | Render Network | GPU compute for rendering + AI | Pay-as-you-go with RNDR | Open marketplace | https://renderfoundation.com/ | **Verified 2026-09-05**: Render Network Foundation site is live (focuses on rendering + AI). Useful if we need GPU compute for validator benchmarks. **NOTE**: Foundation also has a grants program (see C3.4) | NOT-A-GRANT | 2 |
| C3.2 | io.net | Distributed GPU compute | Pay-as-you-go | Open | https://io.net/ | **Verified 2026-09-05**: URL works. Now branded "Open Source AI Infrastructure Platform". Same as Render; cheaper for batch GPU | NOT-A-GRANT | 2 |
| C3.3 | Spheron Network | **Enterprise GPU rental marketplace** (H100/B200/A100, $0.72/hr+) | Pay-as-you-go, per-minute billing; no longer actively grants-driven | Open marketplace | https://www.spheron.network/ | **Verified 2026-09-05**: Spheron has **pivoted away from decentralized compute to enterprise GPU marketplace**. Site markets itself as "enterprise GPU rental marketplace" aggregating Tier 2/3/4 data centers, not community-provided compute. RMAs 1-click deployment. Old DePIN framing is gone. No public grants program found (`/grants` returns 404) | NOT-A-GRANT | 2 |
| C3.4 | Render Network Foundation Grants | RNDR-denominated grants via RNP (Render Network Proposal) system | Variable per RNP-003 treasury allocation | Projects advancing Render Network growth, infrastructure, 3D content | https://renderfoundation.com/grants | **NEW 2026-09-05**: Active grants program (RNP-003 formalized). Funds bespoke grants and bounties for ancillary infrastructure / network growth. Not directly useful for our validators (Render is GPU-focused), but worth applying if we want to do GPU benchmarking work | RESEARCH | 2 |
| C3.5 | Helium (DePIN Wireless) | Decentralized wireless network (5G, LoRa, Wi-Fi via HeliumOS) | Pay-as-you-go with HNT; HNT rewards for hotspot operators | Open marketplace; tokens used for connectivity | https://www.helium.com/ | **NEW 2026-09-05**: URL works. Helium has pivoted to wireless carrier-scale networks (serves millions of mobile users/day). Less relevant for cloud compute. Could theoretically use HNT hotspots as fallback ISP for validators in DePIN-style architecture | RESEARCH | 1 |
| C3.6 | World Mobile (DePIN Telecom) | Mobile network + token rewards; currently $15/mo phone plans | Pay-as-you-go; WMT token for node operators | Open marketplace | https://worldmobile.io/ (old worldmobiletoken.com now redirects here) | **NEW 2026-09-05**: URL changed. Now operates as MVNO with $15/mo plans serving 3M users. DePIN angle is consumer telecom, not cloud compute. Mostly irrelevant unless we need distributed cellular ISP for validators | RESEARCH | 1 |

### C4 — Web3 ecosystem grants: Polkadot / Substrate ecosystem

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C4.1 | Web3 Foundation Grants Program (discontinued) | Historical Substrate chain infrastructure | Historical Level 1 up to $10k; Level 2 up to $30k; Level 3 unlimited; at least 50% of payments vested DOT; no new intake | Previously open-source Polkadot/Substrate projects; current general intake is closed | https://github.com/w3f/Grants-Program | **Verified 2026-09-05:** The repository now states the general Grants Program was discontinued and is not accepting applications; no open RFPs remain. Use W3F strategic/treasury routes instead; the old $5k–$100k figure is stale. | NOT-A-GRANT | 1 |
| C4.2 | Polkadot OpenGov Treasury | Ecosystem public goods, runtime, tooling | No fixed range; proposer requests the amount; ~42 DOT submission deposit; public tipper-to-spender tracks | Any account can submit a preimage/proposal and refundable decision deposit; it must collect approval/support and be enacted on-chain | https://polkadot.js.org/apps/#/treasury/proposals | **Verified 2026-09-05:** Anyone can start a referendum; runtime, chain infrastructure, and public goods are in scope, not only dApps. Track parameters and the ~42 DOT guide can change; check the UI before submission. | RESEARCH | 4 |
| C4.3 | Substrate Builder Program (Parity) | Engineering support, ecosystem intros | No cash | Teams building on Substrate; current application unavailable | https://www.substrate.io/builders-program/ | **Verified 2026-09-05:** URL returns 404; no replacement or open application was found. Historical notes only; do not treat this as an active program. | NOT-A-GRANT | 1 |
| C4.4 | Moonbeam / Moonriver Grants | EVM-Substrate integration | Historical $5k–$50k; current award not published | Projects integrating with Moonbeam/Moonriver; current application not verified | https://moonbeam.foundation/grants/ | **Verified 2026-09-05:** URL 404; `moonbeam.foundation` now redirects to a new Moonbeam site describing an AI-agent network, not a grants program. | NOT-A-GRANT | 1 |
| C4.5 | Astar Foundation Grants | Substrate + EVM + WASM ecosystem | Historical $5k–$50k; current amount not published | Projects on the Astar ecosystem; current application not listed | https://www.astar.network/foundation | **Verified 2026-09-05:** URL returns a 404 shell, not a live foundation/grants page; no active application, award, or deadline found. | NOT-A-GRANT | 1 |
| C4.6 | Polkadot Pioneers Prize | Polkadot innovation/demo award | Historical $10k–$50k; current amount not published | Open to builders; current application not found | https://pioneers.polkadot.network/ | **Verified 2026-09-05:** Hostname does not resolve; alternate Polkadot site paths also return 404. Do not count as active. | NOT-A-GRANT | 1 |

### C5 — Web3 ecosystem grants: Cross-chain foundation grants

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C5.1 | Ethereum Foundation ESP | Ethereum L2/scaling, infrastructure, research | Award not published; set by ESP Wishlist/RFP; no standard $5k–$500k schedule now | Free/open-source projects strengthening Ethereum; ESP focuses on builder tools, infrastructure, research, and public goods rather than end users | https://esp.ethereum.foundation/ | **Verified 2026-09-05:** ESP home is live and says funding is through Wishlist/RFPs plus Office Hours. No public amount or deadline is posted; the Academic Grants page is a 2025 round and closed. | RESEARCH | 3 |
| C5.2 | Arbitrum Audit Program (current active program) | Third-party smart-contract audit subsidy for early-stage Arbitrum projects | $10M total over 12 months; individual award not published | Early-stage Arbitrum projects with strong product-market fit/high growth; request quotes through the program | https://arbitrum.foundation/grants | **Verified 2026-09-05:** The current grants hub lists Arbitrum Audit Program as Active with a $10M/12-month pool; ArbiFuel is also Active but has no public cash award. Generic Foundation/DAO grant programs are listed inactive. | RESEARCH | 3 |
| C5.3 | Optimism RetroPGF / OP Retro Funding | OP Stack/Superchain public goods | No fixed public amount; round-specific OP token allocations/votes | Retroactive public-goods projects serving the Optimism ecosystem; current round page not exposed at the old quest URL | https://optimism.io/ | **Verified 2026-09-05:** The old quest URL returns 200 but only an OP Mainnet Gateway shell; the current Optimism docs index has no RetroPGF application details. The existing quest/award is stale and no reliable current intake is verified. | NOT-A-GRANT | 1 |
| C5.4 | Polygon Village / Grants | Polygon ecosystem | Historical $5k–$50k; current amount not published | Projects deploying on Polygon; current application not verified | https://hadronfc.com/ | **Verified 2026-09-05:** The former Polygon URL redirects here; the current page is a Hadron Founders Club, not a Polygon grant application or award page. | NOT-A-GRANT | 1 |
| C5.5 | Base Ecosystem Fund (investment, not a grant) | Preseed/seed capital, ecosystem credits, and support | Amount not published; venture investment, not a grant award | Preseed/seed startups choosing Base as home; apply through the Base ecosystem fund | https://www.base.org/ecosystem-fund | **Verified 2026-09-05:** The official page describes a Coinbase Ventures partnership, preseed/seed focus, credits, and support, but no public cash-award range. Do not classify it as a grant. | NOT-A-GRANT | 2 |

### C6 — Web3 ecosystem grants: EVM-aligned grants

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C6.1 | Gitcoin Grants Stack (legacy) | Quadratic-funding rounds | Variable (community-matched) | Open-source public goods; current application not verified | https://gitcoin.co/ | **Verified 2026-09-05:** Old `gitcoin.co/grants/` and `grants.gitcoin.co` return 404/DNS failure; the current homepage is a funding knowledge base, not a live grants-stack application. | NOT-A-GRANT | 1 |
| C6.2 | Octant (current status unverified) | Community-governed public-goods experiments | No current award range or open application verified | Public-goods projects participating in governance experiments; see official repository/docs | https://github.com/golemfoundation/octant | **Verified 2026-09-05:** Main site returns 403 and the official repository describes a community platform but does not publish a current award, intake, or deadline. No active application was added. | NOT-A-GRANT | 1 |
| C6.3 | Giveth GIVbacks | Verified-project donor matching/public-goods discovery | 1,000,000 GIV per biweekly round; eligible donations of $5+ can win up to 500,000 GIV; 50%–80% of donation value (raffle-based) | Donors to verified, eligible projects; donations must be on supported chains/eligible tokens; verified-project self-donations are excluded | https://docs.giveth.io/givbacks | **Verified 2026-09-05:** Giveth says the program is a biweekly raffle, not a conventional grant to the project. Current homepage shows a live Ethereum Security QF; this row is useful only for public-goods visibility. | RESEARCH | 3 |
| C6.4 | Starknet Seed Grants | Early-stage Starknet applications and prototypes | Up to $25,000 in STRK, non-dilutive | Early-stage team with a clear MVP or PoC, prior Starknet community/hackathon participation, and a plan to use Starknet tools/integrations; mature live products with core users are ineligible | https://airtable.com/appfoRv2ottjRfTpL/pag0G55zA8aU4V9bD/form | **Verified 2026-09-05:** Applications are ongoing; the official FAQ requires an MVP/PoC, a roughly three-month use-of-funds plan, a 3-month check-in, KYC/KYB, and a grant agreement. Infrastructure and meaningful integrations are eligible, not only consumer dApps. | RESEARCH | 3 |
| C6.5 | Starknet Growth Grants — Innovation / Ecosystem Integration | Later-stage Starknet infrastructure, protocols, and cross-network integrations | $25,000–$1,000,000 in STRK, subject to rigorous review | Later-stage team with a live production-grade product, significant usage, prior Starknet involvement, and a clear path to ecosystem value; interviews and milestone terms apply | https://airtable.com/appfoRv2ottjRfTpL/pagy4pDA3VKGWGzpj/form | **Verified 2026-09-05:** Official page lists Innovation and Ecosystem Integration categories and an application form; the program is ongoing, with decisions targeted in about one month and KYC/KYB required. Strong fit if X3 ships a Starknet integration. | RESEARCH | 3 |
| C6.6 | Prezenti Celo Grants (Anchor + Frontier) | Celo revenue-generating applications and AI/agent infrastructure | Anchor: ~$25,000 for Stage 2 or ~$50,000 for Stage 3; Frontier: $25,000+ in USDm; one-off grants split 20% at start and 80% after delivery | Anchor requires verifiable traction and a path to Celo transaction/TVL/revenue growth; Frontier requires a Celo mainnet deployment, infrastructure/protocol used by other builders or agents, and preferably open-source/public documentation; agent projects may need ERC-8004 and Self Agent ID | https://www.prezenti.xyz/grants | **Verified 2026-09-05:** Prezenti says the current round is open through 29 Dec 2026; Celo’s official funding page lists the program. The Anchor page specifies 10K–100K daily transactions for ~$25K Valora/Mini apps and 100K–500K for ~$50K seed/scaled products; Frontier funds infrastructure with 4–6 expected grants. The page title still says “Grant Program Timeline for H2 2024,” so re-confirm the deadline immediately before applying. | RESEARCH | 3 |

### C7 — Web3 ecosystem grants: SVM / Solana-aligned grants

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C7.1 | Solana Foundation Funding Program | Solana public goods, tooling, and SVM infrastructure | No standard public range; milestone-based budget proposed; standard/convertible grants and RFPs are negotiated | Open-source or meaningful free community offering; clear public-goods impact, Solana-specific rationale, and measurable milestones; commercial projects may use a convertible grant | https://solana.org/grants-funding | **Verified 2026-09-05:** Official page is live. Applications are rolling; initial review is ~1 week and decisions are targeted in ~3 weeks. The page explicitly covers milestone-based standard/convertible grants and RFPs; the old $5k–$250k range is no longer published. | RESEARCH | 4 |
| C7.2 | Metaplex Foundation | Solana/SVM asset and tokenization integration | No public grant amount, eligibility, or current application found | NFT/tokenization projects using Metaplex protocols | https://www.metaplex.foundation/ | **Verified 2026-09-05:** The official site is live and documents Metaplex protocols, DAS, and Aura, but no grant application, award range, or deadline. | NOT-A-GRANT | 1 |
| C7.3 | Superteam Earn Grants (Solana) | Microgrants for early-stage Solana builders | $10,000 microgrant | Builders in India, Southeast Asia, Eastern Europe, or Africa; use the Solana-linked application and confirm regional eligibility | https://superteam.fun/earn/grants | **Verified 2026-09-05:** Solana’s official funding page lists Superteam as a current ecosystem program with $10K microgrants; Superteam’s grant page is live. The listed regions exclude a US-based applicant, so confidence remains low unless a regional team qualifies. | RESEARCH | 1 |

### C8 — Audit grants

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C8.1 | Web3 Foundation Audit Grants | Audit funding for Substrate projects | Up to ~$50k per audit (historical range) | Substrate-based projects with a prior W3F grant | https://github.com/w3f/Grants-Program/blob/master/README.md#auditing-grants | **Cross-reference corrected 2026-09-05:** The general W3F Grants Program is discontinued, so this audit track’s current intake is not verified. The historical track required a prior W3F grant; re-confirm with W3F before relying on it. | NOT-A-GRANT | 1 |
| C8.2 | OpenZeppelin Defender / Audit Services | Audit services | Variable; discounted | Projects using OZ libraries (we may be using OZ v4 in EVM contracts) | https://www.openzeppelin.com/security-audits | Direct engagement | NOT-A-GRANT | 4 |
| C8.3 | Trail of Bits (TOB) | Audit credits / Build Credit for OSS | Variable (historically $25k–$100k credits); paid engagements ~$50k–$200k+ for boutique 4–6 wk audits | Open-source security tooling and high-quality Web3 projects | https://www.trailofbits.com/services/blockchain/ | URL: site root still works; their blockchain practice moved to /services/blockchain/ (the old /services/security-assessments/ returns 404). 946 publications, 620 audits on record since 2012. Build Credit program accepts OSS nominations; otherwise paid engagement. Excellent Rust/Substrate experience. | NOT-A-GRANT | 4 |
| C8.4 | Cantina (cantina.xyz) | Audit marketplace + competitions + bounties | $50k–$500k typical engagement (paid audits); contests from ~$20k prize pool; bounties $50k+ total | Open-source Web3 projects with deployable code; private engagements for closed-source | https://cantina.xyz/ | **Spearbit (previously separate) merged into Cantina** — spearbit.com now redirects to cantina.xyz. Active marketplace: $54M total paid out, 21,794 researchers, $65.2M in available payouts as of 2026-09. Combines competitions, private reviews, and bug-bounty hosting in one platform. Strong fit for Substrate runtime reviews (researchers include Substrate-experienced auditors). | RESEARCH | 5 |
| C8.5 | Code4rena | Audit competitions | Variable (competitions from ~$10k prize pool to $250k+) | Projects with deployed (or to-be-deployed) code | https://code4rena.com/ | Verified active 2026-09; now operating under the Cantina umbrella (alongside former Spearbit). Public audit competitions remain the cheapest audit option. Best fit for EVM contracts (ink!/Solidity), less so for Substrate runtime. Use as a complement to a private Rust audit. | RESEARCH | 3 |
| C8.6 | Zellic | Boutique security firm | $80k–$250k typical (paid engagement) | Web3 protocols (new L1s, L2s, DeFi, bridges, wallets) | https://www.zellic.io/ | Verified active 2026-09. Reputation for finding critical bugs in novel L1/L2 stacks. Good fit for cross-VM atomic-execution design (HTLC + cross-VM adapters). Premium pricing; not a grant but competitive vs. Trail of Bits. | NOT-A-GRANT | 4 |
| C8.7 | Runtime Verification (RV) | Formal verification + audit | $100k–$300k typical (paid engagement) | Formal-verification-friendly languages, K framework, EVM, Substrate | https://runtimeverification.com/ | Verified active 2026-09. Origin: spin-off from UIUC formal methods group, NASA/Boeing heritage. Tools: Kontrol (KEVM symbolic execution), Simbolik (symbolic exec for EVM), K Framework. Historically the partner for Polkadot/Substrate formal verification (e.g., Polkadot host spec). Premium for runtime/proof-of-correctness work. | NOT-A-GRANT | 5 |
| C8.8 | osec.io (formerly OtterSec) | Audit firm (rebranded) | $50k–$200k typical (paid engagement) | Solana, EVM, Stellar, Sui, Fogo; not primarily Substrate | https://osec.io/ | **Rebrand note: OtterSec → osec.io** (ottersec.io no longer resolves). Now markets as osec.io (Aug 2026 reports show recent activity). $1B+ vulnerabilities patched, 120+ audits. Strong in Solana/star chains; less aligned with Substrate but they can review WASM pallets. | NOT-A-GRANT | 2 |
| C8.9 | MixBytes | Audit firm | $40k–$150k typical (paid engagement) | EVM, Substrate, Cosmos, Move, Rust | https://mixbytes.io/ | Verified active 2026-09. Senior full-time auditors; explicit DeFi focus (Curve, Lido, Aave ecosystem). Reasonable price/quality. Their reports are publicly available — useful as a template. | NOT-A-GRANT | 4 |
| C8.10 | Halborn | Web3 security firm | $80k–$250k typical (paid engagement) | L1s/L2s, DeFi, bridges, infrastructure | https://www.halborn.com/ | Verified active 2026-09. Strong infrastructure/operations audit practice (not just code) — covers pen-testing, CI/CD, and key-management reviews alongside smart-contract review. Good fit if we want a holistic ops+code audit. | NOT-A-GRANT | 4 |
| C8.11 | ChainSecurity | Audit firm | $50k–$200k typical (paid engagement) | EVM smart contracts, formal verification | https://www.chainsecurity.com/ | Verified active 2026-09. ETH Zurich spin-off; auditor of MakerDAO, Curve, Enzyme. Strong formal-verification chops. EVM-first; would partner with another firm for Substrate runtime. | NOT-A-GRANT | 3 |
| C8.12 | Certora | Formal verification platform + audits | Prover subscription + paid audits ($50k+) | EVM contracts | https://www.certora.com/ | Verified active 2026-09. Industry-leading formal verification for EVM (Certora Prover). $100B+ TVL protected. EVM-first; useful for any ink!/EVM pallet that uses Solidity-equivalent semantics. Combines audit contests via Code4rena. | NOT-A-GRANT | 3 |
| C8.13 | Quantstamp | Audit firm + Chainproof insurance | $60k–$200k typical; Chainproof coverage sized to TVL | L1s/L2s, DeFi, NFT marketplaces, exchanges, clients | https://www.quantstamp.com/ | Verified active 2026-09. Strong reputation; insurance product (Chainproof) is a differentiator. 60+ ecosystems, $500B+ digital assets secured. Useful if we want both an audit and a post-audit insurance backstop. | NOT-A-GRANT | 3 |
| C8.14 | SlowMist | Audit + threat intel | $30k–$150k typical (paid engagement) | Exchanges, wallets, smart contracts, threat intel | https://www.slowmist.com/ | Verified active 2026-09. Threat-intel firm since 2018, $1B+ stolen-coin tracking. Useful for ops-focused security review (anti-phishing, key-management ops) in addition to code review. Asia-Pacific strong. | NOT-A-GRANT | 2 |
| C8.15 | Sherlock | Audit contests + audits + bounties + insurance | Variable (paid engagements; prize pool for contests) | Web3 protocols with deployed code | https://sherlock.xyz/ | Verified active 2026-09. 6 services: Sherlock AI, Collaborative Auditing, Audit Contests, Bug Bounties, Sherlock Shield (insurance), Blackthorn. Useful for one-stop-shop lifecycle security. | RESEARCH | 4 |

### C9 — Bug bounty bootstrapping

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C9.1 | Immunefi (Project Bonus) | Bonus pool matching | Up to $50k matching | Active Immunefi project | https://immunefi.com/ | Apply when launching bug bounty | RESEARCH | 4 |
| C9.2 | HackerOne (HackerOne Plus) | Platform + triage | Subscription fee | Enterprise tier | https://www.hackerone.com/ | Alternative to Immunefi | NOT-A-GRANT | 3 |
| C9.3 | Cantina (Bounties) | Bug bounty hosting | Platform fee; 51 active bounties ($65.2M available, $54M paid out historical) | Projects on Cantina | https://cantina.xyz/opportunities/bounties | URL moved from /bounties to /opportunities/bounties; combined audit + bounty. Their Bug Bounty service reuses the same researcher pool as their audits. 21,794 active researchers. | RESEARCH | 4 |
| C9.4 | Sherlock Bug Bounties | Bug bounty platform | Platform fee (post-launch program); coverage tiers up to $250k+ | Web3 protocols with deployed code | https://sherlock.xyz/ | Verified active 2026-09. Bug Bounty + Sherlock Shield coverage. Pair with their Audit Contest service for pre-launch review. Some programs accept Substrate runtimes if you can hand them a working testnet + bug submission flow. | RESEARCH | 3 |
| C9.5 | Cantina Competitions | Audit competitions / contests | Prize pool $10k–$500k (typically posted by project) | Projects with deployable code or Substrate runtime with reproducible build | https://cantina.xyz/opportunities/competitions | URL: was /competitions, now /opportunities/competitions. Time-boxed contests; multiple vetted researchers review the same scope. 21,794 researchers in network. Cheaper than a private engagement; results are public. | RESEARCH | 3 |
| C9.6 | Hats Finance | Bug bounty vault + decentralized triage | Variable; vault-funded bounties | Web3 projects willing to fund a vault | https://hats.finance/ | Verified active 2026-09. Decentralized model: bounties paid from on-chain vault; less platform risk. Smaller researcher pool than Immunefi/Cantina, but worth considering for hybrid EVM + Substrate bridges. | RESEARCH | 2 |

### C10 — Legal support

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C10.1 | LexDAO | Pro-bono smart contract legal | Volunteer hours (when active) | Open-source Web3 projects | https://www.lexdao.community/ | **DEFUNCT/DORMANT 2026-09**: lexdao.community DNS does not resolve (NXDOMAIN); lexdao.org returns `certificate has expired`. The DAO has been largely inactive since 2023. Recommend reaching out via Twitter (@lex_DAO) or the lexdao.eth ENS before relying on it. Replace with DLx Law or Anderson Kill below for any real legal work. | RESEARCH | 1 |
| C10.2 | Crypto Legal Defense Fund | Litigation defense | Variable | Web3 projects facing legal threats | https://www.theblockchainassociation.org/ | Defensive only | RESEARCH | 2 |
| C10.3 | a16z Crypto Startup School legal clinic | Pro-bono legal advice | Limited hours | Selected a16z portfolio + applicants | https://a16zcrypto.com/ | Apply when ready | RESEARCH | 3 |
| C10.4 | Local law firm pro-bono (varies) | Local regulatory advice | Variable | Per firm | — | Find local Web3-experienced firm | RESEARCH | 3 |
| C10.5 | DLx Law (formerly DLAPIER) | Crypto-native legal counsel | Hourly / flat fee; typical token-launch + tokenomics $15k–$60k; full genesis package $30k–$80k | US-jurisdiction Web3 issuers and protocols | https://dlxlaw.com/ | Verified reachable via web. One of the most crypto-specialized US firms; led by Jake Chervinsky and others. Strong on securities-law defensibility for token launches and validator-set governance. Recommended first-call firm for a US-based founder. NOT pro-bono. | NOT-A-GRANT | 5 |
| C10.6 | Anderson Kill (Digital Assets practice) | Crypto/Web3 legal counsel | Hourly; typical matter $10k–$50k | US-jurisdiction Web3 projects | https://andersonkill.com/industry-groups/digital-assets-and-blockchain-technology/ | Verified active 2026-09 (their /practice-areas/cryptocurrency-blockchain/ returns 404; canonical URL now under /industry-groups/digital-assets-and-blockchain-technology/). Established firm; less crypto-native than DLx but stronger on dispute/litigation. Useful if we expect contentious validator-governance disputes. | NOT-A-GRANT | 4 |
| C10.7 | Blockchain Association (Crypto Legal Defense Fund) | Defensive legal network | Pro-bono or contingency; covers litigation + regulatory engagement | Web3 projects facing legal threats | https://www.theblockchainassociation.org/ | Verified reachable 2026-09. Industry group; primarily lobbying + litigation support for member projects. Membership fees apply. NOT a pro-bono walk-in clinic; relationship-driven. | RESEARCH | 3 |
| C10.8 | a16z Crypto Startup School legal clinic | Pro-bono crypto legal advice | Limited hours per cohort | Selected a16z portfolio + applicants | https://a16zcrypto.com/ | Verified reachable 2026-09. Aspiring to be a YC-style program; cohort-based. Limited seats. Application-based; not a guaranteed resource. | RESEARCH | 2 |
| C10.9 | Sullivan & Worcester (Digital Currency Group practice) | Crypto securities law | Hourly; full token launch $50k–$150k | US-jurisdiction Web3 issuers | https://www.sullivanlaw.com/ | Recommended by multiple parachain issuers. Strong on Howey-test defense, SAFT structuring, and accreditation compliance. NOT pro-bono. Useful if we plan a token launch under US securities law. | NOT-A-GRANT | 3 |
| C10.10 | a16z Crypto Canon / State of Crypto (free legal resources) | Legal primer + reference docs | Free | Public | https://a16zcrypto.com/posts/tags/legal/ | Verified reachable 2026-09. Free reference materials by a16z crypto team on token design, governance, and securities law. NOT a service — but a solid starting point before hiring counsel. | RESEARCH | 2 |

### C11 — Hardware / ISP / datacenter co-location

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C11.1 | Equinix / Digital Realty co-lo sponsorship | Rack space, power, cross-connects | Variable; rare | Mid-to-large Web3 projects with revenue | https://www.equinix.com/ | **NOTE: Equinix Metal was SUNSET on 2026-06-30** (announced 2024-11-07). The bare-metal cloud service is gone — only colocation (retail colo cages/cabinets) remains under the Equinix brand. Equinix.com is alive; metal.equinix.com now redirects to a docs sunset page. Treat as colocation-only; do not use as a bare-metal cloud option. Sponsorship programs still rare and revenue-share based. | RESEARCH | 1 |
| C11.2 | Hetzner dedicated server leases | Dedicated hardware | ~€40–€150/mo per server | Open to anyone | https://www.hetzner.com/dedicated-rootserver | NOT a grant; very cheap dedicated machines (not VPS) | NOT-A-GRANT | 5 |
| C11.3 | OVHcloud Bare Metal (Game Servers / Rise / Infrastructure) | Bare metal servers | €50–€200/mo per server (Infrastructure tier); So You Start €30–€60/mo; Game ranges from €50/mo | Open to anyone | https://www.ovhcloud.com/en/bare-metal/ | URL fix: ovh.com/bare-metal/ → 404; canonical URL is ovhcloud.com/en/bare-metal/. 30+ datacenters worldwide. Anti-DDoS included. EUR pricing → ~$55–$220 USD; several cheaper than Hetzner for similar SKUs in EU. | NOT-A-GRANT | 5 |
| C11.4 | Local datacenter co-lo | Rack space | Variable | Geographic; depends on local provider | — | Many cities have crypto-friendly datacenters; inquire directly | RESEARCH | 2 |
| C11.5 | Home ISP business plans (static IPs) | Residential/business ISP | ~$60–$200/mo | Open to anyone | — | Local providers (Comcast Business, Verizon Fios, etc.) | NOT-A-GRANT | 3 |
| C11.6 | Helium / World Mobile (DePIN) | Decentralized wireless ISP | Pay-as-you-go with tokens or fiat; World Mobile plans from $15/mo (Starter) and $25/mo (Standard) | Open marketplace | https://www.helium.com/, https://worldmobile.io/ | **URL fix: worldmobiletoken.com → worldmobile.io** (the token domain returns 403/bot-block). Helium is mobile carrier offload (358k+ hotspots, Nova Labs Inc). World Mobile is a working US MVNO ($15–$25/mo Starter/Standard). Both are interesting alternatives if we want validator nodes in remote regions. NOT a fit for static-IP validator hosting directly. | RESEARCH | 3 |
| C11.7 | Kimsufi (OVHcloud Eco range) | Cheap dedicated servers | From ~€3.50/mo (KS-1) up to €40/mo | Open to anyone; no special eligibility | https://www.kimsufi.com/en/ | Verified active 2026-09. Cheapest legitimate dedicated hardware on the market. France + international DCs. ISO 27001 certified, anti-DDoS included, unlimited traffic. Specs modest (older Xeons/Atom) but very reliable. Perfect for early testnet nodes. | NOT-A-GRANT | 5 |
| C11.8 | Scaleway Dedibox (formerly Online.net) | Dedicated bare-metal servers | Dedicated Server Start from €4.74/mo (12-mo commit); Pro from €29.74/mo; Core from €110.49/mo | Open to anyone | https://www.scaleway.com/en/dedibox/ | Verified active 2026-09. **Note: Online.net was acquired by Scaleway and rebranded as Dedibox.** The online.net URL still resolves but redirects content to Scaleway. Paris-based; EU-only DCs (Paris, Amsterdam, Warsaw, Berlin). Excellent price/performance, AMD EPYC + Intel Xeon options, up to 25 Gbit/s on Core tier. | NOT-A-GRANT | 5 |
| C11.9 | Leaseweb Dedicated Servers | Global dedicated hosting | €60–€500k/mo across 24+ DCs globally | Open to anyone | https://www.leaseweb.com/en/products-services/dedicated-servers | Verified reachable 2026-09. Large global footprint (US, EU, APAC). Best for geographically diverse validator deployment. Higher end of the price band but enterprise-grade SLAs. | NOT-A-GRANT | 4 |
| C11.10 | Liquid Web Dedicated Hosting | Managed dedicated servers | $100–$800+/mo | Open to anyone | https://www.liquidweb.com/dedicated-server-hosting/ | Verified active 2026-09. Managed service — they handle OS patching. Best when team doesn't want hands-on ops. 100% network uptime SLA. Premium positioning. | NOT-A-GRANT | 3 |
| C11.11 | Comcast Business Internet | Business ISP w/ static IP | $70–$250/mo (varies by market) | US service areas; business verification required | https://www.business.comcast.com/internet | US cable ISP. Static IP available on business plans. Best as a fallback for US-based bootstrap operator who can host a node at home/office. 1 Gbps+ available in many markets. | NOT-A-GRANT | 3 |
| C11.12 | Verizon Fios Business | Fiber business ISP | $100–$500/mo | US service areas (Northeast primarily); business verification required | https://www.verizon.com/business/products/internet/fios/ | US fiber ISP; very high reliability and symmetric speeds. Static IP available. Best US-East-Coast option for low-latency colocated validator. Limited Northeast footprint (was the URL we had 404; canonical URL now under /business/products/internet/fios/). | NOT-A-GRANT | 3 |
| C11.13 | AT&T Business Fiber | Fiber business ISP w/ static IP | $80–$500+/mo | US service areas; business verification | https://www.business.att.com/products/internet.html | US fiber ISP. Static IP available on business tiers. Wider US footprint than Verizon Fios. Good choice for redundant validators. | NOT-A-GRANT | 3 |
| C11.14 | Spectrum Business Internet | Cable business ISP | $70–$300/mo | US service areas; business verification | https://www.spectrum.com/business/small-business/internet | US cable ISP. Static IP available on business plans. Largest US cable footprint. Good fallback option. | NOT-A-GRANT | 3 |
| C11.15 | Althea (DePIN payment layer) | Mesh-network bandwidth marketplace | Variable; priced in ALTHEA token | Open to operators willing to run hardware | https://www.althea.net/ | Verified active 2026-09. Blockchain-enabled internet for hard-to-reach areas. Hybrid EVM L1 (now Althea chain). More relevant as an inspiration for token-incentivized ISP than as a direct validator-hosting platform. | RESEARCH | 2 |
| C11.16 | Mysterium VPN (DePIN) | Decentralized VPN exit nodes | Pay-as-you-go with MYST token | Open marketplace | https://www.mysteriumvpn.com/ | Verified via curl but web_fetch returns 403/timeout (likely bot-blocked). DePIN VPN exit nodes; could in theory use for geo-distributed relay nodes. Niche use case for our RPC relay layer. | NOT-A-GRANT | 2 |

### C12 — Open-source foundations

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C12.1 | Linux Foundation (LF) / Hyperledger | Membership + ecosystem support + LFX tooling | Annual membership $5k–$500k (tiered); no direct cash grants | Open-source projects meeting LF charter criteria | https://www.linuxfoundation.org/about/join | Verified active 2026-09. Joining is paid (Associate $5k; General much higher); benefits include legal umbrella, events, LFX collaboration portal, security/CDA tooling. Hyperledger is a Linux Foundation project umbrella. Probably overkill for a single parachain; better fit after mainnet traction. | RESEARCH | 2 |
| C12.2 | Apache Software Foundation (ASF) | Mentorship, legal umbrella, infrastructure | No cash; in-kind (servers, legal, IP, brand) | Projects that release under Apache-2.0 and use Apache-style governance | https://www.apache.org/ | Verified active 2026-09. 501(c)(3) public charity since 1999; provides hardware, communications, legal infrastructure for incubated projects. Incubator process is multi-month and governance-heavy — worth it if we want strong IP/litigation cover and brand, but slow. | RESEARCH | 2 |
| C12.3 | Open Collective (Open Finance Consortium, formerly Open Source Collective) | 501(c)(6) fiscal sponsorship + transparent donation platform | 5–10% platform/admin fee on donations received | Open-source projects; free to create a Collective | https://opencollective.com/ | **URL fix: oscollective.org → opencollective.com** (the oscollective.org URL still 200s but the active product is now on opencollective.com; OS Collective is a specific fiscal host within Open Collective, NOT a separate platform). Good fit for receiving US-tax-deductible donations without spinning up a 501(c)(3). Also integrates with GitHub Sponsors. | RESEARCH | 4 |
| C12.4 | GitHub Sponsors | Recurring donations | Variable (project-set tiers, typically $5–$500/mo) | Open-source projects with GitHub presence, 2FA enabled | https://github.com/sponsors | Verified active 2026-09; redirects to https://github.com/open-source/sponsors. $40M+ paid to maintainers to date, 4.2k+ orgs sponsoring. Works best paired with a fiscal sponsor (e.g., Open Collective). Platform fee: GitHub takes 0%; payment processor fees apply. | RESEARCH | 4 |
| C12.5 | Apache Software Foundation Incubator | Mentorship + legal/IP cover | No cash; in-kind | Apache-2.0 projects willing to adopt Apache governance | https://incubator.apache.org/ | Verified reachable 2026-09. Multi-month incubation; provides governance, IP, and brand. Not cash. Worth considering only if we want strong legal umbrella + brand for the long term. | RESEARCH | 2 |
| C12.6 | NumFOCUS Fiscal Sponsorship | Fiscal sponsorship for OSS scientific projects | 501(c)(3) tax status; small admin fee | OSS scientific/data/infra projects | https://numfocus.org/programs | Verified active 2026-09. US 501(c)(3) fiscal sponsor for OSS scientific computing. Less aligned with crypto than Open Collective, but credible if our contributors are academia-adjacent. | RESEARCH | 2 |
| C12.7 | Open Collective (for Collectives) | Fiscal sponsorship for unincorporated groups | 0–10% platform fee | Any OSS group without a legal entity | https://opencollective.com/collectives | Verified reachable 2026-09. Best fit if we don't yet have a Wyoming DAO LLC / Cayman foundation. Creates a "Collective" page that can receive tax-deductible (US) donations, with transparent ledgers. | RESEARCH | 4 |

---

## Recommended next actions

| Priority | Action | Owner | Deadline |
|---|---|---|---|
| P0 | Prepare a **Polkadot OpenGov Treasury** proposal (C4.2) — current, infrastructure/public-goods eligible, amount set by referendum | You | W0+1 |
| P0 | Sign up for **AWS Activate** (C1.1, now up to $200k) via partner referral if possible | You | W0 |
| P0 | Sign up for **Google Cloud for Startups** (C1.2) and **Microsoft for Startups** (C1.3, new URL microsoft.com/en-us/startups) | You | W0 |
| P0 | Set up free **Cloudflare Workers / R2 / D1 / KV** (C1.10) for explorer + faucet + CI edge workers — zero cost | You | W0 |
| P0 | Sign up for **DigitalOcean Startups** (C1.4, new URL /startups; $500 credit + up to $5k) for testnet hosting | You | W0 |
| P0 | Use **Render.com** (C1.11) free tier for static site + simple web services (docs site, status page) | Agent | W1 |
| P1 | Contact the **Starknet Growth/Seed grants** teams (C6.4–C6.5) only after scoping a concrete Starknet integration; current applications and award ranges are verified | You | W2+ |
| P1 | Engage **Ankr** (C2.7) for managed Substrate/Polkadot RPC hosting — 99.99% uptime, 30+ regions | You | W0 |
| P1 | Engage **Akash Network** (C2.1) for validator hosting — cheaper than AWS for sustained workloads | You | W2 |
| P1 | Apply to **Akash Ecosystem Grants** (C2.5) if we adopt Akash for validators — on-chain governance path | You | W4 |
| P1 | Engage **Runtime Verification / Trail of Bits / Zellic / MixBytes** for audit quotes — request quotes for Substrate runtime review (~$80k–$200k) | You | W0 |
| P1 | Sign up for **Immunefi** project bonus program (C9.1) ahead of W14 | You | W13 |
| P1 | Engage **DLx Law** for token-launch + genesis ceremony legal package (~$30k) | You | W4 |
| P1 | Lock 3-host testnet: 3× Kimsufi (~€40/mo each, ~$130/mo total) for cheapest path; or 3× Hetzner AX-series (~€50/mo each, ~$165/mo total) for slightly better specs | You | W0 |
| P2 | Apply to **Solana Foundation Funding Program** (C7.1) when SVM bridge work begins; use the current rolling application and milestone budget | You | W7 |
| P2 | Use **Giveth GIVbacks** (C6.3) or another live public-goods round for community funding after a verified project page exists | You | W2+ |
| P2 | Sign up for **Filecoin fil.one** (C2.2) 1TB/30-day free storage for chain archives | Agent | W2 |
| P3 | Enable **GitHub Sponsors** (C12.4) | Agent | W1 |
| P3 | Enable **Open Source Collective** fiscal sponsorship (C12.3) | You | W2 |
| P3 | Drop OVHcloud Startup (C1.6) from pursuit — program defunct as of 2026-09-05 | — | — |
| P3 | Drop Oracle (C1.12), IBM (C1.13), Alibaba (C1.14), Tencent (C1.15) startup programs — all URLs 404, no current programs | — | — |

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

---

## C11 — Cheapest realistic 3-host testnet setup (verified 2026-09-05)

Goal: run 3 geographically diverse validator nodes for early testnet (Alice, Bob, Charlie — 1 operator-controlled, 2 external/community validators). All prices below are USD/month, monthly-commitment unless noted. Assume each node needs: ≥4 vCPU, ≥16 GB RAM, ≥500 GB NVMe, ≥1 Gbps, static IP, anti-DDoS.

| Setup tier | Provider mix | Region spread | Per-host cost | 3-host total | Notes |
|---|---|---|---|---|---|
| **Cheapest (EU)** | 3× Kimsufi KS-2/KS-3 (€7–€15/mo) | 1× FR, 1× DE, 1× CA | ~$12–$30 | **~$50–$90/mo** | Older Atom/Xeon CPUs; modest specs; good for testnet-not-mainnet |
| **Cheapest (EU+US mix)** | 2× Kimsufi + 1× Hetzner CCX | EU-heavy | ~$30–$60 | **~$90–$180/mo** | Mix of EU DCs; one slightly beefier node |
| **Balanced (recommended)** | 3× Hetzner AX41-NVMe or CCX (€50–€80/mo) | 1× FSN, 1× HEL, 1× NBG | ~$55–$90 | **~$165–$270/mo** | Best price/perf for Substrate; ECC RAM; NVMe; ISO 27001; anti-DDoS included |
| **Balanced (EU + US)** | 2× Hetzner (FSN+HEL) + 1× OVHcloud Rise-1 (US-east) | EU + NA | ~$60–$100 | **~$180–$300/mo** | Geographic diversity; US-east for low-latency NA users |
| **Premium 3-region public RPC** | 1× Hetzner EU + 1× OVHcloud CA + 1× Scaleway Core (Paris) | 3 regions | ~$130–$180 | **~$400–$540/mo** | For mainnet public RPC tier, not just testnet |
| **Premium global (US-heavy)** | 1× Hetzner + 1× LiquidWeb (US) + 1× OVHcloud US | 3 regions | ~$130–$250 | **~$400–$750/mo** | When US East / West / EU coverage is required |

**Recommendation for our budget ($150–$600/mo for 3-host testnet):** start at the **Balanced (recommended)** tier with 3× Hetzner CCX (~$165–$270/mo total). Once we need geographic spread for a public RPC tier, add 1× OVHcloud US-east and 1× Hetzner ASIA (Singapore DC if available) and you are still under $500/mo total.

**Provider-specific gotchas:**
- **Equinix Metal is dead** (sunset 2026-06-30) — the old “cheap dedicated anywhere” option is gone.
- **Online.net is now Scaleway Dedibox** — same physical hardware, new billing portal.
- **OVH bare-metal URL moved** from `ovh.com/bare-metal/` (404) to `ovhcloud.com/en/bare-metal/`.
- **Spearbit is now Cantina** — all Spearbit people + brand + process migrated to cantina.xyz.
- **LexDAO is dormant** — use DLx Law / Anderson Kill for any real legal work.
- **First-year total with one free credit program** (AWS Activate $100k + GCP $100k + Azure $150k) can essentially eliminate C11 spend in year 1 — make applying to all 3 a P0.
