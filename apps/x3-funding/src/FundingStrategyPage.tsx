import { useState, useMemo } from 'react'

interface Strategy {
  id: number
  category: string
  catIcon: string
  catColor: string
  title: string
  desc: string
  focus: string[]
  effort: string
  timeline: string
}

const ALL: Strategy[] = [
  // 1. ECOSYSTEM GRANT PROGRAMS (8)
  ...([
    ['Ethereum ESP', '#627eea', 'Ethereum Ecosystem Support Program funds public goods, developer tooling, and security infrastructure. X3\'s proof ledger, RPC quorum, and finality oracle fit their scope.', ['public goods', 'security', 'interop', 'tooling']],
    ['Solana Foundation', '#9945ff', 'Solana grants fund SVM integration, cross-chain tooling, and ecosystem expansion. X3\'s SVM adapter and Solana cross-chain route work are direct matches.', ['SVM adapter', 'cross-chain', 'performance']],
    ['Web3 Foundation', '#00c8ff', 'Polkadot/Kusama grants for Substrate development, cross-chain research, and interoperability. X3\'s Substrate adapter and cross-VM research align directly.', ['Substrate', 'interop', 'research']],
    ['Arbitrum Grants', '#28a0f0', 'Stylus and EVM tooling grants, cross-chain infrastructure, and developer ecosystem support. X3\'s EVM adapter and cross-chain routing fit multiple tracks.', ['EVM', 'L2 tooling', 'cross-chain']],
    ['Optimism RetroPGF', '#ff0420', 'Retroactive public goods funding for projects that have delivered measurable impact. X3 should publish tools first and claim retro rewards.', ['retro', 'public goods', 'impact']],
    ['Cosmos IBC Grants', '#2e3148', 'Cosmos ecosystem funding for IBC-compatible bridges and CosmWasm integration. X3\'s Cosmos adapter and settlement research fit here.', ['IBC', 'CosmWasm', 'bridges']],
    ['Bitcoin Ecosystem', '#f7931a', 'Bitcoin-focused grants for Layer 2 infrastructure, BTC bridging, and HTLC atomicity research. X3\'s BTC Fortress vault is a direct match.', ['BTC vault', 'HTLC', 'SPV proofs']],
    ['ZK Ecosystem', '#a78bfa', 'Zero-knowledge proof ecosystem grants for proof compression, zk finality, and bridge safety research.', ['ZK proofs', 'compression', 'finality']],
  ] as const).map(([title, color, desc, focus], i) => ({
    id: i + 1, category: 'Ecosystem Grant Programs', catIcon: '⌘', catColor: '#627eea',
    title, desc, focus: focus as unknown as string[],
    effort: i < 2 ? 'Medium' : 'Medium', timeline: i < 2 ? 'Q3 2026' : 'Q4 2026',
  })),

  // 2. PUBLIC GOODS FUNDING (7)
  ...[
    { id: 9, title: 'Gitcoin Grants', desc: 'Seasonal matching rounds for open-source public goods. X3 should apply with ProofForge, RPC quorum tooling, and cross-chain test suites as reusable infrastructure.', focus: ['matching rounds', 'open source', 'community'], effort: 'Low', timeline: 'Ongoing' },
    { id: 10, title: 'Open Source Collective', desc: 'Fiscal sponsorship and funding management for open-source projects. Provides donation infrastructure, legal backing, and grant administration.', focus: ['fiscal host', 'donations', 'legal'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 11, title: 'Protocol Guild', desc: 'Funding stream for Ethereum protocol maintenance contributors. Relevant for X3\'s EVM compatibility and Frontier pallet maintenance work.', focus: ['protocol', 'EVM', 'maintenance'], effort: 'Medium', timeline: 'Q4 2026' },
    { id: 12, title: 'clr.fund', desc: 'Quadratic funding for public goods on Ethereum. X3 can participate in rounds with cross-chain infrastructure as a public good category.', focus: ['quadratic funding', 'ETH', 'public goods'], effort: 'Low', timeline: 'Ongoing' },
    { id: 13, title: 'Giveth Donation Matching', desc: 'Donation platform with matching pools for verified public goods projects. X3\'s open-source tooling qualifies for matching rounds.', focus: ['donations', 'matching', 'impact'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 14, title: 'Hypercerts Impact Certificates', desc: 'Claim impact certificates for X3\'s open-source contributions and redeem them against retro funding pools.', focus: ['impact certs', 'retro', 'attestation'], effort: 'Medium', timeline: 'Q4 2026' },
    { id: 15, title: 'Public Nouns DAO', desc: 'Nounish DAO that funds public goods and infrastructure through treasury proposals. Aligned with X3\'s infrastructure-first approach.', focus: ['DAO', 'public goods', 'infrastructure'], effort: 'Medium', timeline: 'Q1 2027' },
  ],

  // 3. RETROACTIVE FUNDING (6)
  ...[
    { id: 16, title: 'Optimism RetroPGF Round 5+', desc: 'Apply with proof of X3\'s impact on the Superchain ecosystem: shared sequencer research, cross-chain safety, and developer tooling usage.', focus: ['retro', 'superchain', 'impact proofs'], effort: 'High', timeline: 'Q4 2026' },
    { id: 17, title: 'Arbitrum RCP', desc: 'Arbitrum Retroactive Contribution Program. X3 can claim for EVM adapter work, Stylus tooling, and cross-chain research that benefits Arbitrum.', focus: ['retro', 'Arbitrum', 'Stylus'], effort: 'High', timeline: 'Q1 2027' },
    { id: 18, title: 'Base Retro Grants', desc: 'Coinbase L2 retro funding for on-chain builders. X3\'s cross-chain tooling and developer SDKs that saw Base usage qualify.', focus: ['retro', 'Base', 'on-chain'], effort: 'Medium', timeline: 'H1 2027' },
    { id: 19, title: 'zkSync Retro Airdrops', desc: 'Developers who built on or contributed to zkSync ecosystem qualify for retroactive rewards. X3\'s zk proof research qualifies.', focus: ['retro', 'zkSync', 'developers'], effort: 'Medium', timeline: '2027' },
    { id: 20, title: 'Polygon Retro Funding', desc: 'Polygon ecosystem retro rewards for infrastructure and tooling builders. X3\'s cross-chain work that touches Polygon qualifies.', focus: ['retro', 'Polygon', 'infra'], effort: 'Medium', timeline: '2027' },
    { id: 21, title: 'Celestia Modular Retro', desc: 'Retro funding for modular blockchain infrastructure contributors. X3\'s DA layer research and data availability work fits here.', focus: ['retro', 'modular', 'DA'], effort: 'Medium', timeline: '2027' },
  ],

  // 4. HACKATHONS & BOUNTIES (7)
  ...[
    { id: 22, title: 'ETHGlobal Hackathons', desc: 'Major hackathon circuit with prize pools and ecosystem visibility. Submit cross-VM atomic swap demo, x3-lang intents, or RPC quorum prototype.', focus: ['ETHGlobal', 'demo', 'cross-VM'], effort: 'High', timeline: 'Upcoming hackathons' },
    { id: 23, title: 'Solana Hackathons', desc: 'Solana ecosystem hackathons for SVM integration, cross-chain bridges, and high-throughput applications.', focus: ['Solana', 'SVM', 'bridge'], effort: 'High', timeline: 'Quarterly' },
    { id: 24, title: 'Polkadot Hackathons', desc: 'Polkadot-focused hackathons for Substrate pallets, parachain development, and cross-chain messaging.', focus: ['Polkadot', 'Substrate', 'XCM'], effort: 'High', timeline: 'Quarterly' },
    { id: 25, title: 'DoraHacks Multi-Chain', desc: 'Continuous hackathon platform with grants, bounties, and funding for multi-chain infrastructure projects.', focus: ['multi-chain', 'bounties', 'continuous'], effort: 'Medium', timeline: 'Ongoing' },
    { id: 26, title: 'Encode Club', desc: 'University-focused hackathons and educational programs. X3 can participate in blockchain tracks and recruit developer talent.', focus: ['university', 'education', 'recruiting'], effort: 'Medium', timeline: 'Semester-based' },
    { id: 27, title: 'Gitcoin Bounties', desc: 'Open bounty platform for small-to-medium development tasks. X3 can post bounties for SDK work, documentation, and adapter development.', focus: ['bounties', 'open tasks', 'community'], effort: 'Low', timeline: 'Ongoing' },
    { id: 28, title: 'HackerOne Bug Bounties', desc: 'Security bug bounty platform. X3 should seed a bug bounty pool and run a responsible disclosure program on HackerOne.', focus: ['security', 'bounties', 'disclosure'], effort: 'Medium', timeline: 'Q1 2027' },
  ],

  // 5. AUDIT & SECURITY FUNDING (6)
  ...[
    { id: 29, title: 'ImmuneFi Audit Partnerships', desc: 'Audit competition platform connecting projects with top security researchers. X3 can run audit competitions for bridge, HTLC, and adapter code.', focus: ['audit', 'competition', 'bridge'], effort: 'Medium', timeline: 'Q1 2027' },
    { id: 30, title: 'Code4rena Audit Contests', desc: 'Competitive audit platform with fixed-price contests. X3 should run C4 contests for cross-VM router, supply ledger, and gateway code.', focus: ['audit', 'competition', 'router'], effort: 'Medium', timeline: 'Q1 2027' },
    { id: 31, title: 'Sherlock Audit Contests', desc: 'Curated audit marketplace with top firms. X3 can sponsor contests for critical modules: settlement engine, BTC vault, and gateway.', focus: ['audit', 'curated', 'critical'], effort: 'High', timeline: 'Q2 2027' },
    { id: 32, title: 'OpenZeppelin Defender', desc: 'Security operations platform for monitoring, access control, and incident response. Defender grants available for qualifying projects.', focus: ['security ops', 'monitoring', 'access control'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 33, title: 'Trail of Bits Research', desc: 'Top-tier security firm with research partnership programs. X3 should pursue a joint research grant for cross-VM security formalization.', focus: ['research', 'formal', 'partnership'], effort: 'High', timeline: '2027' },
    { id: 34, title: 'Hats Finance Audit Competition', desc: 'Decentralized audit competition protocol. Projects post prizes, auditors compete, and only valid findings get paid.', focus: ['audit', 'decentralized', 'findings'], effort: 'Medium', timeline: 'Q1 2027' },
  ],

  // 6. CLOUD / GPU / INFRASTRUCTURE CREDITS (7)
  ...[
    { id: 35, title: 'AWS Activate', desc: 'Startup cloud credits program. X3 qualifies for up to $100K in AWS credits for testnet infrastructure, CI/CD, and monitoring.', focus: ['AWS', 'credits', 'infrastructure'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 36, title: 'Google Cloud Startups', desc: 'Google Cloud startup program with up to $350K in credits. X3\'s AI-agent and GPU validation work strengthens the application.', focus: ['GCP', 'credits', 'AI/ML'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 37, title: 'Microsoft Founders Hub', desc: 'Azure credits plus OpenAI access and AI model integration. Up to $150K for qualifying startups with an AI angle.', focus: ['Azure', 'credits', 'OpenAI'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 38, title: 'NVIDIA Inception', desc: 'GPU credits, developer tools, and go-to-market support for AI startups. X3 Reactor\'s GPU validator benchmarks fit perfectly.', focus: ['NVIDIA', 'GPU', 'benchmarks'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 39, title: 'DigitalOcean Hatch', desc: 'Simple cloud credit program for early-stage startups. Up to $100K in credits for testnet and development infrastructure.', focus: ['DigitalOcean', 'credits', 'simple'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 40, title: 'Oracle for Startups', desc: 'Oracle cloud credits for startups building on Oracle Cloud Infrastructure. Up to $30K in credits for qualifying projects.', focus: ['Oracle', 'credits', 'OCI'], effort: 'Low', timeline: 'Q4 2026' },
    { id: 41, title: 'Akash Network Grants', desc: 'Decentralized compute marketplace with grant programs for projects using Akash for GPU compute. X3 Reactor can run on Akash.', focus: ['decentralized compute', 'GPU', 'Akash'], effort: 'Medium', timeline: 'Q4 2026' },
  ],

  // 7. UNIVERSITY & RESEARCH GRANTS (6)
  ...[
    { id: 42, title: 'NSF SBIR / STTR', desc: 'Federal research grants for deep-tech startups. Phase I up to $275K, zero equity. X3\'s cross-VM settlement and proof systems fit multiple topic areas.', focus: ['NSF', 'SBIR', 'research'], effort: 'High', timeline: 'Rolling' },
    { id: 43, title: 'NIST Cybersecurity Grants', desc: 'Federal cybersecurity research funding. X3\'s post-quantum cryptography, bridge security, and formal verification work align here.', focus: ['NIST', 'cybersecurity', 'PQC'], effort: 'High', timeline: 'Annual cycle' },
    { id: 44, title: 'University Blockchain Labs', desc: 'Academic blockchain research labs seeking industry collaborations. Topics: cross-VM formal models, atomic swap verification, distributed consensus.', focus: ['academic', 'formal models', 'collaboration'], effort: 'Medium', timeline: 'Semester-based' },
    { id: 45, title: 'MIT DCI Grants', desc: 'MIT Digital Currency Initiative funds open-source blockchain research. X3\'s proof-driven infrastructure and settlement research fit.', focus: ['MIT', 'digital currency', 'research'], effort: 'High', timeline: 'Annual' },
    { id: 46, title: 'Stanford Blockchain Research', desc: 'Stanford Center for Blockchain Research partnerships. Topics: cross-chain protocols, validator coordination, and formal verification.', focus: ['Stanford', 'cross-chain', 'verification'], effort: 'High', timeline: 'Annual' },
    { id: 47, title: 'Colorado Advanced Industries', desc: 'Colorado state grant for advanced technology companies. Up to $250K with 2:1 cash match. X3\'s Denver-based lab and AI/blockchain work qualifies.', focus: ['Colorado', 'state grant', 'matching'], effort: 'Medium', timeline: 'Annual cycle' },
  ],

  // 8. STRATEGIC ECOSYSTEM PARTNERSHIPS (7)
  ...[
    { id: 48, title: 'RPC Provider Partnerships', desc: 'Partner with RPC providers (Alchemy, QuickNode, Infura, Blast) for sponsored endpoints, credits, and integration into their multi-chain offerings.', focus: ['RPC', 'providers', 'credits'], effort: 'Low', timeline: 'Q4 2026' },
    { id: 49, title: 'Cloud Provider Alliances', desc: 'AWS, GCP, Azure, and Oracle startup programs. Also pursue cloud credits from smaller providers like Hetzner, OVHcloud, and Linode.', focus: ['cloud', 'credits', 'alliances'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 50, title: 'Wallet Integration Partners', desc: 'Partner with wallet providers (Ledger, Trezor, MetaMask, WalletConnect, Phantom) for X3 chain support and sponsored integration development.', focus: ['wallets', 'integration', 'partners'], effort: 'Medium', timeline: '2027' },
    { id: 51, title: 'DEX / AMM Routing Partners', desc: 'Aggregation partnerships with DEXs and aggregators for cross-chain routing volume. Revenue-share model for liquidity provision.', focus: ['DEX', 'routing', 'revenue share'], effort: 'Medium', timeline: '2027' },
    { id: 52, title: 'Bridge Security Consortium', desc: 'Join or form a bridge security consortium with other bridge projects for shared security research, incident response, and best practices.', focus: ['bridges', 'security', 'consortium'], effort: 'Medium', timeline: 'Q4 2026' },
    { id: 53, title: 'Validator Network Partnerships', desc: 'Partner with validator infrastructure providers (Figment, Kiln, Chorus One, Blockscape) for staking support and co-marketing.', focus: ['validators', 'staking', 'providers'], effort: 'Medium', timeline: '2027' },
    { id: 54, title: 'Indexer / Explorer Partnerships', desc: 'Partner with The Graph, SubQuery, Blockscout, and Covalent for indexed data services and explorer integration.', focus: ['indexing', 'explorer', 'data'], effort: 'Low', timeline: '2027' },
  ],

  // 9. DONATIONS & COMMUNITY FUNDING (6)
  ...[
    { id: 55, title: 'Open Collective', desc: 'Fiscal host and donation platform for transparent community funding. X3 can accept tax-deductible donations through Open Collective Europe or Open Source Collective.', focus: ['donations', 'transparent', 'fiscal host'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 56, title: 'Sponsor a Validator', desc: 'Targeted giving program where donors fund a specific validator node. Monthly uptime reports and validator naming rights.', focus: ['validator', 'targeted giving', 'reporting'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 57, title: 'Fund an Audit', desc: 'Earmarked donation program for specific security audits. Donors fund an audit and receive the published report with acknowledgment.', focus: ['audit', 'targeted', 'transparency'], effort: 'Low', timeline: 'Q3 2026' },
    { id: 58, title: 'Hardware Donation Program', desc: 'Accept physical hardware donations: servers, GPUs, networking gear, storage. NIST 800-88 wipe procedures, tax receipt provided.', focus: ['hardware', 'servers', 'donation'], effort: 'Low', timeline: 'Ongoing' },
    { id: 59, title: 'Testnet Operations Fund', desc: 'Micro-donation pool for testnet operational costs: RPC endpoints, faucet, monitoring. $25/month supporter tier.', focus: ['testnet', 'operations', 'micro-donations'], effort: 'Low', timeline: 'Q4 2026' },
    { id: 60, title: 'Developer Documentation Fund', desc: 'Sponsored developer content: API docs, tutorials, video guides, and example repos. Donors receive attribution on the docs site.', focus: ['docs', 'content', 'developer'], effort: 'Low', timeline: 'Q4 2026' },
  ],

  // 10. INVESTOR & ANGEL FUNDING (7)
  ...[
    { id: 61, title: 'Strategic Angel Investors', desc: 'Angel investors with blockchain, infrastructure, and deep-tech thesis. Target: angels who understand the value of proof-driven infrastructure over hype.', focus: ['angels', 'strategic', 'infrastructure'], effort: 'High', timeline: '2027' },
    { id: 62, title: 'Blockchain VC Funds', desc: 'Crypto-native venture funds investing in L1/L2 infrastructure, cross-chain, and developer tooling. Target: a16z Crypto, Paradigm, Polychain, Multicoin.', focus: ['VC', 'crypto', 'infrastructure'], effort: 'High', timeline: '2027' },
    { id: 63, title: 'Infrastructure-Specific Funds', desc: 'Funds focused on blockchain infrastructure, staking, and validator ecosystems. Target: CoinFund, Blockchain Capital, Digital Currency Group.', focus: ['infrastructure', 'staking', 'validator'], effort: 'High', timeline: '2027' },
    { id: 64, title: 'AI / Crypto Crossover Funds', desc: 'Investors funding the intersection of AI and crypto. X3\'s AI agent swarm, GPU validation, and automated testing fit these theses.', focus: ['AI', 'crossover', 'agents'], effort: 'High', timeline: '2027' },
    { id: 65, title: 'Ecosystem Investment DAOs', desc: 'DAOs that invest in ecosystem infrastructure (e.g., Uniswap Grants, Compound Grants, Aave Grants). X3\'s DeFi components are eligible.', focus: ['DAO', 'ecosystem', 'DeFi'], effort: 'Medium', timeline: 'Ongoing' },
    { id: 66, title: 'Future Token / SAFT Structure', desc: 'Prepare compliant token structure for future strategic investment. SAFT or SAFE with token warrants for lead investors.', focus: ['token', 'SAFT', 'legal'], effort: 'High', timeline: '2027' },
    { id: 67, title: 'Revenue-Based Financing', desc: 'Non-dilutive financing against future service revenue (ProofForge audits, Reactor benchmarks, consulting fees).', focus: ['revenue', 'non-dilutive', 'services'], effort: 'Medium', timeline: '2027' },
  ],

  // 11. GOVERNMENT & ECONOMIC DEVELOPMENT (6)
  ...[
    { id: 68, title: 'Colorado OEDIT Grants', desc: 'Colorado Office of Economic Development and International Trade. Advanced Industries, job growth, and technology commercialization programs.', focus: ['Colorado', 'economic dev', 'commercialization'], effort: 'Medium', timeline: 'Annual' },
    { id: 69, title: 'Denver Economic Development', desc: 'Denver-area economic development programs, tech incubators, and innovation district partnerships. X3\'s Denver validator lab is an asset.', focus: ['Denver', 'local', 'incubator'], effort: 'Medium', timeline: 'Ongoing' },
    { id: 70, title: 'SBA Small Business Programs', desc: 'US Small Business Administration programs: 8(a) certification, HUBZone, and small business innovation research eligibility.', focus: ['SBA', 'small business', 'certification'], effort: 'Medium', timeline: 'Q1 2027' },
    { id: 71, title: 'DOE Advanced Computing Grants', desc: 'Department of Energy grants for energy-efficient computing, GPU optimization, and green data center research.', focus: ['DOE', 'energy', 'computing'], effort: 'High', timeline: 'Annual' },
    { id: 72, title: 'DOC Innovation Grants', desc: 'Department of Commerce grants for technology innovation, cybersecurity workforce development, and infrastructure resilience.', focus: ['DOC', 'innovation', 'cybersecurity'], effort: 'High', timeline: 'Annual' },
    { id: 73, title: 'USDA Rural Broadband / Tech', desc: 'USDA programs for rural technology infrastructure, broadband, and technology workforce development in rural areas.', focus: ['USDA', 'rural', 'infrastructure'], effort: 'Medium', timeline: 'Annual' },
  ],

  // 12. GRANT-FINDER SYSTEM (5)
  ...[
    { id: 74, title: 'Grant Opportunity Database', desc: 'Build and maintain a searchable database of 200+ grant programs across ecosystem, government, corporate, and foundation categories. Updated weekly.', focus: ['database', 'research',  'search'], effort: 'Medium', timeline: 'Ongoing' },
    { id: 75, title: 'Eligibility Scoring Engine', desc: 'AI system that scores each grant opportunity against X3\'s current readiness, module fit, and team capacity. Outputs a go/no-go recommendation.', focus: ['AI scoring', 'eligibility', 'automation'], effort: 'High', timeline: 'Q1 2027' },
    { id: 76, title: 'Deadline Calendar & Alerts', desc: 'Automated calendar with grant deadlines, application windows, and early reminders. Integrates with Slack/Telegram for team alerts.', focus: ['calendar', 'deadlines', 'alerts'], effort: 'Medium', timeline: 'Q4 2026' },
    { id: 77, title: 'Auto-Generate Application Pack', desc: 'System that assembles grant application materials from templates: executive summary, technical spec, budget, milestones, and evidence links.', focus: ['automation', 'templates', 'assembly'], effort: 'High', timeline: '2027' },
    { id: 78, title: 'Submission Tracker CRM', desc: 'CRM for tracking submitted applications, reviewer contacts, follow-ups, and resubmission opportunities. Pipeline view of all active applications.', focus: ['CRM', 'tracking', 'pipeline'], effort: 'Medium', timeline: 'Q1 2027' },
  ],

  // 13. PROPOSAL FACTORY (5)
  ...[
    { id: 79, title: 'Reusable Proposal Templates', desc: 'Standardized proposal templates for each funding category: ecosystem grants, government, audit, infrastructure credits, and donations. Fill-in-the-blank customization.', focus: ['templates', 'standardization', 'efficiency'], effort: 'Medium', timeline: 'Q3 2026' },
    { id: 80, title: 'Technical Whitepaper Pack', desc: 'Modular technical whitepaper sections that can be assembled per-funder: architecture overview, module specs, security model, and roadmap. Updated quarterly.', focus: ['whitepaper', 'technical', 'modular'], effort: 'Medium', timeline: 'Q3 2026' },
    { id: 81, title: 'Milestone Budget Generator', desc: 'Tool that generates per-milestone budgets with a standard template: engineering hours, infrastructure costs, audit costs, and contingency.', focus: ['budget', 'milestones', 'tooling'], effort: 'Medium', timeline: 'Q4 2026' },
    { id: 82, title: 'Evidence Link Assembler', desc: 'Automated system that pulls latest evidence from GitHub, testnet dashboard, ProofForge reports, and benchmark data into a structured evidence pack.', focus: ['evidence', 'automation', 'GitHub'], effort: 'High', timeline: '2027' },
    { id: 83, title: 'Funder-Specific Customizer', desc: 'AI-assisted tool that adapts the base proposal to a specific funder\'s language, priorities, and evaluation criteria. 80% template, 20% customization.', focus: ['customization', 'AI', 'adaptation'], effort: 'High', timeline: '2027' },
  ],

  // 14. PROOF-BASED FUNDER UPDATES (5)
  ...[
    { id: 84, title: 'Monthly Milestone Reports', desc: 'Automated monthly report for every active funder: milestone progress, GitHub commits, testnet metrics, budget spent, and next steps.', focus: ['reporting', 'monthly', 'automated'], effort: 'Low', timeline: 'Q4 2026' },
    { id: 85, title: 'Public Dashboard Integration', desc: 'Funder-specific view into the public dashboard showing their funded milestones, deliverables, and transparency metrics in real time.', focus: ['dashboard', 'transparency', 'real-time'], effort: 'Medium', timeline: 'Q1 2027' },
    { id: 86, title: 'Budget Transparency Ledger', desc: 'Public ledger showing grant fund allocation and spending by category: engineering, infrastructure, audits, operations, and contingencies.', focus: ['budget', 'transparency', 'ledger'], effort: 'Medium', timeline: 'Q1 2027' },
    { id: 87, title: 'Funder Portal', desc: 'Password-protected portal for active funders with detailed reports, milestone evidence, budget breakdowns, blocker tracking, and direct team contact.', focus: ['portal', 'funders', 'detailed'], effort: 'High', timeline: '2027' },
    { id: 88, title: 'Impact Report Generator', desc: 'Annual impact report template: public-good statistics, code written, validators onboarded, testnet reliability, security findings resolved, and community growth.', focus: ['impact', 'annual', 'public'], effort: 'Medium', timeline: 'Annual' },
  ],

  // 15. FUNDING FLYWHEEL STRATEGY (5)
  ...[
    { id: 89, title: 'Grant → Build → Prove → Report Cycle', desc: 'Core flywheel: apply for grant, build milestone, publish proof on dashboard, report results to funder, use evidence for next application. Each cycle compounds.', focus: ['cycle', 'compounding', 'evidence'], effort: 'Ongoing', timeline: 'Continuous' },
    { id: 90, title: 'Cross-Funder Referral Network', desc: 'Build relationships with grant officers and program managers. Successful delivery of one grant leads to referrals to other programs within their network.', focus: ['referrals', 'network', 'relationships'], effort: 'Medium', timeline: 'Ongoing' },
    { id: 91, title: 'Retro Funding Loop', desc: 'Publish open-source tools → collect usage data → apply for retro funding → use retro funding to publish more tools. Compounding public goods.', focus: ['retro', 'loop', 'public goods'], effort: 'Medium', timeline: '2027' },
    { id: 92, title: 'Partner → Revenue → Grant Loop', desc: 'Partner with RPC/wallet/provider → generate service revenue → use revenue as grant matching fund → apply for larger grants with matching. Self-reinforcing.', focus: ['partners', 'revenue', 'matching'], effort: 'High', timeline: '2027' },
    { id: 93, title: 'Community → Donations → Grant Loop', desc: 'Build community → micro-donations → demo grant readiness → qualify for larger grants → share results with community → more donations.', focus: ['community', 'donations', 'growth'], effort: 'Medium', timeline: 'Ongoing' },
  ],

  // 16. MEDIA & CONTENT STRATEGY (7)
  ...[
    { id: 94, title: 'Technical Blog Sponsorships', desc: 'Sponsored technical content on leading blockchain blogs and newsletters. Topics: cross-VM engineering deep-dives, post-quantum migration guides, proof-system design.', focus: ['blog', 'content', 'technical'], effort: 'Low', timeline: 'Q4 2026' },
    { id: 95, title: 'Podcast Guest Circuit', desc: 'Guest appearances on blockchain infrastructure, security, and AI podcasts. Builds credibility, network, and inbound grant/investor interest.', focus: ['podcast', 'credibility', 'network'], effort: 'Low', timeline: 'Ongoing' },
    { id: 96, title: 'YouTube / Twitch Dev Streams', desc: 'Live development streams of X3 engineering: building cross-VM adapters, running ProofForge audits, debugging settlement flows. Monetize via sponsorships.', focus: ['video', 'dev', 'streaming'], effort: 'Medium', timeline: 'Q4 2026' },
    { id: 97, title: 'Newsletter Sponsorships', desc: 'Sponsor blockchain infrastructure, security, and DeFi newsletters targeting grant reviewers, ecosystem teams, and infrastructure buyers.', focus: ['newsletter', 'sponsorship', 'outreach'], effort: 'Low', timeline: 'Q4 2026' },
    { id: 98, title: 'Grant-Finder Content Series', desc: 'Publish a recurring series documenting X3\'s grant-finding journey: which grants applied, results, lessons, and process. Builds audience and funder trust.', focus: ['content series', 'transparency', 'journey'], effort: 'Medium', timeline: 'Q1 2027' },
    { id: 99, title: 'Conference Speaking Engagements', desc: 'Submit talks to blockchain conferences (ETHGlobal, Polkadot Decoded, Solana Breakpoint, Cosmoverse) to present X3\'s technical work and attract funder attention.', focus: ['conferences', 'speaking', 'visibility'], effort: 'Medium', timeline: 'Ongoing' },
    { id: 100, title: 'Benchmark Report Publishing', desc: 'Publish quarterly benchmark reports with TPS, latency, validator performance, and cross-chain settlement data. Builds reputation as rigorous infrastructure project.', focus: ['benchmarks', 'reports', 'credibility'], effort: 'Medium', timeline: 'Quarterly' },
  ],
]

const EFFORT_COLORS: Record<string, string> = { Low: '#22c55e', Medium: '#eab308', High: '#ef4444', Ongoing: '#00c8ff' }
const CAT_ICONS: Record<string, string> = {
  'Ecosystem Grant Programs': '⌘',
  'Public Goods Funding': '⎔',
  'Retroactive Funding': '↩',
  'Hackathons & Bounties': '⚡',
  'Audit & Security Funding': '⊟',
  'Cloud / GPU / Infrastructure Credits': '◈',
  'University & Research Grants': '⌨',
  'Strategic Ecosystem Partnerships': '⇌',
  'Donations & Community Funding': '≡',
  'Investor & Angel Funding': '◆',
  'Government & Economic Development': '⌂',
  'Grant-Finder System': '⊡',
  'Proposal Factory': '⟨⟩',
  'Proof-Based Funder Updates': '⊞',
  'Funding Flywheel Strategy': '⟁',
  'Media & Content Strategy': '★',
}
const CAT_COLORS: Record<string, string> = {
  'Ecosystem Grant Programs': '#627eea',
  'Public Goods Funding': '#14b8a6',
  'Retroactive Funding': '#f59e0b',
  'Hackathons & Bounties': '#3b82f6',
  'Audit & Security Funding': '#ef4444',
  'Cloud / GPU / Infrastructure Credits': '#06b6d4',
  'University & Research Grants': '#8b5cf6',
  'Strategic Ecosystem Partnerships': '#10b981',
  'Donations & Community Funding': '#78716c',
  'Investor & Angel Funding': '#f43f5e',
  'Government & Economic Development': '#64748b',
  'Grant-Finder System': '#0ea5e9',
  'Proposal Factory': '#a855f7',
  'Proof-Based Funder Updates': '#84cc16',
  'Funding Flywheel Strategy': '#f97316',
  'Media & Content Strategy': '#ec4899',
}

export default function FundingStrategyPage({ nav }: { nav: (p: string) => void }) {
  const [filter, setFilter] = useState('all')
  const [search, setSearch] = useState('')

  const cats = useMemo(() => {
    const m = new Map<string, Strategy[]>()
    for (const s of ALL) {
      if (!m.has(s.category)) m.set(s.category, [])
      m.get(s.category)!.push(s)
    }
    return Array.from(m.entries())
  }, [])

  const filtered = useMemo(() => {
    if (filter === 'all' && !search) return cats
    return cats.map(([cat, items]) => {
      const fi = items.filter(s =>
        (filter === 'all' || s.effort === filter) &&
        (!search || s.title.toLowerCase().includes(search.toLowerCase()) || s.desc.toLowerCase().includes(search.toLowerCase()) || s.focus.some(f => f.toLowerCase().includes(search.toLowerCase())))
      )
      return [cat, fi] as [string, Strategy[]]
    }).filter(([_, items]) => items.length > 0)
  }, [filter, search, cats])

  return (
    <div className="pg">
      <div className="ph">
        <button className="back" onClick={() => nav('/')}>← HOME</button>
        <div className="badge" style={{ background: 'rgba(243,115,22,.15)', border: '1px solid rgba(243,115,22,.4)', color: '#f97316' }}>FUNDING STRATEGY</div>
        <div className="ptitle">100 Funding<br /><span className="ac">Strategies for X3</span></div>
        <div className="psub">
          X3 will use a multi-channel funding strategy instead of relying on one grant program, one investor, or one ecosystem. The project is designed to pursue ecosystem grants, public-goods funding, infrastructure credits, audit funding, research partnerships, hackathons, validator sponsorships, donations, and strategic investment. The core idea: X3 will fund itself the same way it builds itself — with proof, milestones, public dashboards, and repeatable systems.
        </div>
        <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', marginTop: 16 }}>
          <button className="btnp" onClick={() => nav('/angles')}>View All 190+ Angles</button>
          <button className="btnpu" onClick={() => nav('/grants')}>Grant Portal</button>
          <button className="btns" onClick={() => nav('/founding')}>Founding Builders</button>
        </div>
      </div>

      <div className="xc">
        <div className="shd"><span className="stag">THE PLAN</span><div className="sline" /><span className="stitle">{ALL.length} Distinct Strategies Across 16 Categories</span></div>

        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', margin: '24px 0 16px', alignItems: 'center' }}>
          <span style={{ fontFamily: 'var(--fm)', fontSize: 9, color: 'var(--txm)', letterSpacing: 2 }}>FILTER</span>
          {[{ k: 'all', l: 'All' }, { k: 'Low', l: 'Low Effort' }, { k: 'Medium', l: 'Medium Effort' }, { k: 'High', l: 'High Effort' }, { k: 'Ongoing', l: 'Ongoing' }].map(f => (
            <button key={f.k} className={`tag${filter === f.k ? ' active' : ''}`} onClick={() => setFilter(f.k)} style={filter === f.k ? { background: 'rgba(0,200,255,.2)', border: '1px solid var(--ac)' } : {}}>{f.l}</button>
          ))}
          <input className="strategy-search" placeholder="Search strategies..." value={search} onChange={e => setSearch(e.target.value)} style={{ marginLeft: 'auto' }} />
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 28 }}>
          {filtered.map(([cat, items]) => {
            const color = CAT_COLORS[cat] || '#00c8ff'
            const icon = CAT_ICONS[cat] || '○'
            return (
              <div key={cat} className="strategy-cat" style={{ borderLeftColor: color }}>
                <div className="strategy-cat-header">
                  <span className="strategy-cat-icon" style={{ color }}>{icon}</span>
                  <span className="strategy-cat-name" style={{ color }}>{cat}</span>
                  <span className="angle-count">{items.length} strategies</span>
                </div>
                <div className="strategy-items">
                  {items.map((s) => (
                    <div key={s.id} className="strategy-item">
                      <div className="strategy-num">#{s.id}</div>
                      <div className="strategy-body">
                        <div className="strategy-top">
                          <span className="strategy-title">{s.title}</span>
                          <span className="strategy-effort" style={{ color: EFFORT_COLORS[s.effort] || 'var(--txm)' }}>
                            {s.effort.toUpperCase()}
                          </span>
                        </div>
                        <div className="strategy-desc">{s.desc}</div>
                        <div className="strategy-bottom">
                          <div className="strategy-focus">
                            {s.focus.map((f, j) => <span key={j} className="tag">{f}</span>)}
                          </div>
                          <span className="strategy-timeline">→ {s.timeline}</span>
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            )
          })}
        </div>

        <div style={{ marginTop: 48, background: 'var(--s1)', border: '1px solid var(--b1)', borderRadius: 10, padding: '28px', textAlign: 'center' }}>
          <div style={{ fontFamily: 'var(--fh)', fontSize: 20, fontWeight: 700, color: 'var(--txb)', marginBottom: 12 }}>
            The Funding Flywheel
          </div>
          <div style={{ fontSize: 14, color: 'var(--txm)', lineHeight: 1.8, maxWidth: 600, margin: '0 auto' }}>
            Grant → Build milestone → Publish public proof → Report to funders → Attract users → Apply for retro funding → Attract partners → Win bigger grants → Launch stronger testnet → Repeat.
          </div>
          <div style={{ marginTop: 20, display: 'flex', justifyContent: 'center', gap: 6, flexWrap: 'wrap' }}>
            {['FIND', 'SCORE', 'MATCH', 'GENERATE', 'SUBMIT', 'SHIP', 'PROVE', 'REPORT', 'COMPOUND'].map((s, i) => (
              <div key={i} style={{ background: 'var(--void)', border: '1px solid var(--b1)', borderRadius: 4, padding: '4px 10px', fontFamily: 'var(--fm)', fontSize: 9, color: 'var(--ac)', letterSpacing: 2 }}>{s}</div>
            ))}
          </div>
        </div>

        <div style={{ marginTop: 32, display: 'flex', gap: 12, justifyContent: 'center', flexWrap: 'wrap' }}>
          <button className="btnp" onClick={() => nav('/match')}>⚡ Find My Funding Lane</button>
          <button className="btnpu" onClick={() => nav('/angles')}>View All Grant Angles</button>
          <button className="btns" onClick={() => nav('/founding')}>Become a Founding Builder</button>
        </div>
      </div>
    </div>
  )
}
