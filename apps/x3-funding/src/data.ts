// ─── REAL PROJECT DATA ────────────────────────────────────────────────────
// sourced from: FEATURE_REGISTRY.toml, Cargo.toml workspace, production crates

export interface Grant {
  id: string
  badge: string
  bc?: string
  codename: string
  hero: [string, string]
  sub: string
  warn?: string
  readiness: number
  mode: string
  funders: string[]
  problem: string
  solution: string
  modules: { name: string; desc: string }[]
  fundingUnlocks: string[]
  roadmap: { phase: string; title: string; desc: string; status: 'sh' | 'pr' | 'fn' }[]
  breakdown: { label: string; pct: number }[]
  status: { sh: string[]; pr: string[]; fn: string[] }
  cta: string[]
}

export interface Sponsor {
  id: string
  badge: string
  bc?: string
  codename: string
  hero: [string, string]
  sub: string
  problem: string
  solution: string
  modules: { name: string; desc: string }[]
  hardware?: string[]
  acceptance?: string[]
  security?: string
  note?: string
  tiers: { num: string; name: string; range: string; benefits: string }[]
  status: { sh: string[]; pr: string[]; fn: string[] }
  cta: string[]
}

export interface FundingTarget {
  id: string
  badge: string
  bc?: string
  codename: string
  hero: [string, string]
  sub: string
  pitch: string
  targets: { name: string; amount: string; desc: string }[]
  status: { sh: string[]; pr: string[]; fn: string[] }
  cta: string[]
}

// ─── GRANTS ────────────────────────────────────────────────────────────────

export const GD: Grant[] = [
  {
    id: 'atomic-kernel',
    badge: 'CORE INFRASTRUCTURE',
    codename: 'ATOMIC KERNEL',
    hero: ['Canonical Settlement','for Multi-VM Execution'],
    sub: 'X3 Atomic Kernel provides canonical supply accounting, 6-route cross-VM atomic transfers, and invariant-enforced settlement across X3Native, X3Evm, and X3Svm execution domains.',
    readiness: 88,
    mode: 'live_testnet',
    funders: [
      'Web3 Foundation Grants',
      'Polkadot Treasury',
      'Substrate Builders Program',
      'Ecosystem Infrastructure Grants',
    ],
    problem: 'Multi-chain execution is fragmented. Assets on different VMs have no canonical source of truth, leading to supply drift, settlement ambiguity, and cross-domain accounting errors.',
    solution: 'X3 Atomic Kernel enforces a universal asset ledger with supply invariants across all execution domains. The Cross-VM Router (6-route matrix) executes HTLC-guaranteed atomic transfers with economic halt on invariant violation.',
    modules: [
      { name: 'Atomic Kernel', desc: 'Canonical ledger for cross-domain asset accounting with invariant enforcement' },
      { name: 'Cross-VM Router', desc: '6-route atomic transfer matrix: X3Native↔X3Evm, X3Native↔X3Svm, X3Evm↔X3Svm, and reverse' },
      { name: 'Supply Ledger', desc: 'Real-time supply tracking with automatic invariant guard checks' },
      { name: 'Settlement Engine', desc: 'Atomic settlement state machine with commit-or-rollback semantics' },
      { name: 'Economic Halt', desc: 'Automatic chain halt when supply invariant drift exceeds configured threshold' },
      { name: 'Proof-of-Execution', desc: 'PoAE proof generation for every atomic bundle with validator verification' },
      { name: 'Invariant Dashboard', desc: 'Live monitoring of all system invariants with alerting on breach' },
    ],
    fundingUnlocks: [
      'Multi-validator invariant testing',
      'Economic halt simulation suite',
      'External security audit of cross-VM path',
      'Public invariant dashboard deployment',
      'Formal proof of atomicity properties',
      'CI fuzz testing infrastructure',
    ],
    roadmap: [
      { phase: 'Q3 24', title: 'Atomic Kernel design', desc: 'Core accounting architecture and invariant definitions', status: 'sh' },
      { phase: 'Q4 24', title: 'Cross-VM router v1', desc: 'Basic X3Native↔X3Evm transfer path', status: 'sh' },
      { phase: 'Q1 25', title: 'Supply ledger integration', desc: 'Supply tracking and invariant monitoring', status: 'sh' },
      { phase: 'Q2 25', title: '6-route matrix', desc: 'All cross-VM paths operational on testnet', status: 'sh' },
      { phase: 'Q3 25', title: 'Economic halt', desc: 'Automatic invariant breach response', status: 'pr' },
      { phase: 'Q4 25', title: 'PoAE proofs', desc: 'Proof-of-execution for atomic bundles', status: 'pr' },
      { phase: 'Q1 26', title: 'Production hardening', desc: 'Multi-validator invariant test suite', status: 'fn' },
    ],
    breakdown: [
      { label: 'Cross-VM router development', pct: 35 },
      { label: 'Invariant test infrastructure', pct: 25 },
      { label: 'Security audit & formal proof', pct: 20 },
      { label: 'Dashboard & monitoring', pct: 12 },
      { label: 'Documentation & spec', pct: 8 },
    ],
    status: {
      sh: ['Atomic Kernel architecture', '6-route cross-VM matrix', 'Supply ledger integration', 'Basic invariant dashboard'],
      pr: ['Economic halt simulation', 'PoAE proof generation', 'Multi-validator test suite'],
      fn: ['External security audit', 'Formal proof of atomicity', 'Public invariant dashboard'],
    },
    cta: ['Fund Kernel Hardening', 'Request Architecture Spec', 'Sponsor Security Audit'],
  },
  {
    id: 'cross-chain-gateway',
    badge: 'INTEROPERABILITY',
    codename: 'ATOMIC GATEWAY',
    hero: ['Cross-Chain Messages','with Verifiable Proofs'],
    sub: 'Atomic Gateway connects X3 Atomic Star to external blockchain ecosystems through a governance-gated bridge with deposit proof verification, validator attestation quorums, and circuit breaker protection.',
    readiness: 65,
    mode: 'guarded_testnet',
    funders: [
      'Polkadot Bridges Program',
      'Web3 Foundation',
      'Ecosystem Interoperability Grants',
      'Cross-Chain Infrastructure Funds',
    ],
    problem: 'Cross-chain bridges carry massive trust assumptions. Most use simplified validation that can be exploited through validator collusion, delayed finality, or proof forgery.',
    solution: 'X3 Atomic Gateway uses SPV proof verification, validator attestation quorums, a dispute window lifecycle, risk engine limits, and circuit breakers to harden every cross-chain message.',
    modules: [
      { name: 'Gateway State Machine', desc: 'Deposit → verify → attest → credit lifecycle with dispute window' },
      { name: 'SPV Proof Verifier', desc: 'Light-client proof verification for incoming deposits' },
      { name: 'Validator Attestation', desc: 'Quorum-based attestation with configurable threshold' },
      { name: 'Circuit Breaker', desc: 'Automatic bridge halt on anomalous conditions' },
      { name: 'Risk Engine', desc: 'Per-asset rate limits and exposure tracking' },
      { name: 'Dispute Resolution', desc: 'Challenge window for invalid attestations with slashing' },
      { name: 'Gateway Indexer', desc: 'Event indexing for cross-chain message tracking' },
    ],
    fundingUnlocks: [
      'External bridge path enabling on mainnet',
      'Multi-validator attestation test suite',
      'Security audit of gateway integration',
      'Public gateway dashboard',
      'Risk engine calibration with real data',
      'Integration documentation',
    ],
    roadmap: [
      { phase: 'Q4 24', title: 'Gateway design', desc: 'State machine and circuit breaker architecture', status: 'sh' },
      { phase: 'Q1 25', title: 'SPV proof verification', desc: 'Deposit proof verification implementation', status: 'sh' },
      { phase: 'Q2 25', title: 'Validator attestation', desc: 'Quorum-based attestation flow', status: 'sh' },
      { phase: 'Q3 25', title: 'Risk engine integration', desc: 'Rate limits and exposure tracking', status: 'pr' },
      { phase: 'Q4 25', title: 'Dispute resolution', desc: 'Challenge window with slashing', status: 'pr' },
      { phase: 'Q1 26', title: 'External bridge path', desc: 'Mainnet enablement of external bridges', status: 'fn' },
      { phase: 'Q2 26', title: 'Security audit', desc: 'External review of gateway integration', status: 'fn' },
    ],
    breakdown: [
      { label: 'SPV proof verification', pct: 30 },
      { label: 'Validator attestation system', pct: 25 },
      { label: 'Circuit breaker & risk engine', pct: 20 },
      { label: 'Test infrastructure', pct: 15 },
      { label: 'Documentation & integration', pct: 10 },
    ],
    status: {
      sh: ['Gateway state machine', 'SPV proof verification', 'Validator attestation flow'],
      pr: ['Risk engine integration', 'Dispute resolution lifecycle', 'Circuit breaker wiring'],
      fn: ['External bridge mainnet enablement', 'Security audit', 'Public gateway dashboard'],
    },
    cta: ['Fund Gateway Hardening', 'Request Bridge Spec', 'Sponsor Security Review'],
  },
  {
    id: 'btc-fortress',
    badge: 'BITCOIN INFRASTRUCTURE',
    codename: 'BTC FORTRESS',
    hero: ['Bitcoin-Backed','Liquidity Layer'],
    sub: 'BTC Fortress creates a verifiable Bitcoin bridge with SPV proof verification, threshold multisig, UTXO tracking, and canonical xBTC representation within the Atomic Kernel supply model.',
    readiness: 25,
    mode: 'sim_testnet',
    funders: [
      'Bitcoin Ecosystem Grants',
      'Starknet Bitcoin Grants',
      'Cross-Chain Infrastructure Funds',
      'BTC L2 Infrastructure Programs',
    ],
    problem: 'Bitcoin liquidity is trapped in custodial wraps and opaque bridges. Users cannot verify that wrapped BTC is fully backed by real BTC locks.',
    solution: 'BTC Fortress uses SPV proof verification of BTC lock transactions, threshold multisig custody, and canonical supply tracking through the Atomic Kernel — making every xBTC unit verifiably backed.',
    modules: [
      { name: 'BTC Vault', desc: 'Threshold multisig vault with PSBT-based withdrawal flow' },
      { name: 'UTXO Tracker', desc: 'Bitcoin UTXO set tracking with selection algorithm' },
      { name: 'SPV Proof Verifier', desc: 'Bitcoin block header + merkle proof verification' },
      { name: 'Deposit State Machine', desc: 'PendingConfirmations → PendingSpvVerification → PendingSignerApproval → Approved → Completed' },
      { name: 'Withdrawal State Machine', desc: 'PendingX3Proof → PendingSignerApproval → Approved → Broadcasted → Completed' },
      { name: 'xBTC Supply Ledger', desc: 'Canonical wrapped Bitcoin within Atomic Kernel accounting' },
      { name: 'Signer Quorum', desc: 'Multi-signer approval with configurable threshold' },
    ],
    fundingUnlocks: [
      'Real BTC testnet signer quorum',
      'Testnet BTC watcher infrastructure',
      'xBTC mint/burn integration tests',
      'Security review of Bitcoin vault',
      'STARDEX xBTC liquidity pool',
      'Public BTC proof dashboard',
    ],
    roadmap: [
      { phase: 'Q1 25', title: 'BTC vault prototype', desc: 'Multisig address generation and PSBT building', status: 'sh' },
      { phase: 'Q2 25', title: 'SPV proof verification', desc: 'Bitcoin header chain and merkle proof verification', status: 'sh' },
      { phase: 'Q3 25', title: 'Deposit / withdrawal flows', desc: 'Full state machine for lock and release', status: 'pr' },
      { phase: 'Q4 25', title: 'Testnet signer quorum', desc: 'Multi-signer BTC testnet operation', status: 'pr' },
      { phase: 'Q1 26', title: 'xBTC accounting integration', desc: 'Canonical xBTC supply in Atomic Kernel', status: 'fn' },
      { phase: 'Q2 26', title: 'Security audit', desc: 'External review of vault and bridge', status: 'fn' },
    ],
    breakdown: [
      { label: 'BTC vault implementation', pct: 30 },
      { label: 'SPV proof verification', pct: 25 },
      { label: 'Signer quorum infrastructure', pct: 20 },
      { label: 'xBTC accounting integration', pct: 15 },
      { label: 'Security review', pct: 10 },
    ],
    status: {
      sh: ['BTC vault prototype', 'SPV proof verification', 'PSBT building'],
      pr: ['Deposit/withdrawal state machines', 'Testnet signer quorum'],
      fn: ['xBTC supply integration', 'Security audit', 'Public proof dashboard'],
    },
    cta: ['Fund BTC Fortress Sprint', 'Sponsor Vault Security Review', 'Request Architecture Spec'],
  },
  {
    id: 'quantum-crypto',
    badge: 'POST-QUANTUM SECURITY',
    bc: 'pu',
    codename: 'QUANTUM READINESS',
    hero: ['Post-Quantum Migration','for Blockchain Infrastructure'],
    sub: 'X3 Atomic Star maintains quantum-resistant cryptography primitives including Kyber (KEM), Dilithium (signatures), and SPHINCS+ (stateless hashes) for future-proofing validator identity, bridge attestations, and wallet keys.',
    warn: 'PQC research track only — not currently quantum-safe in production. Uses precise terms: "research prototype" and "migration roadmap."',
    readiness: 35,
    mode: 'research',
    funders: [
      'NIST Cybersecurity Grants',
      'OpenSSF Security Tooling',
      'NSF SBIR / STTR',
      'Blockchain Security Research Programs',
    ],
    problem: 'Classical public-key cryptography creates long-term risk for wallets, validator identities, bridge attestations, governance signatures, and archived settlement proofs.',
    solution: 'X3 maintains quantum-crypto crate with ML-KEM (Kyber), ML-DSA (Dilithium), and SLH-DSA (SPHINCS+) implementations, targeting a phased hybrid cryptography migration roadmap.',
    modules: [
      { name: 'ML-KEM (Kyber)', desc: 'Module-Lattice Key Encapsulation Mechanism — NIST standardized KEM' },
      { name: 'ML-DSA (Dilithium)', desc: 'Module-Lattice Digital Signature Algorithm — NIST standardized signatures' },
      { name: 'SLH-DSA (SPHINCS+)', desc: 'Stateless Hash-Based Digital Signature — conservative fallback' },
      { name: 'Hybrid Signature Design', desc: 'Classical + PQ hybrid attestations for backward-compatible identity' },
      { name: 'Known-Answer Test Vectors', desc: 'KAT suite for independent verification of PQ implementations' },
      { name: 'Cryptographic SBOM', desc: 'Bill of materials covering all cryptographic primitives across X3' },
    ],
    fundingUnlocks: [
      'NIST-aligned PQ implementation audit',
      'Known-answer test vector generation',
      'Differential testing harness',
      'Fuzz testing infrastructure',
      'External cryptography review',
      'Wallet and validator integration prototype',
    ],
    roadmap: [
      { phase: 'Q1 25', title: 'PQC prototype crates', desc: 'Rust ML-KEM, ML-DSA, SLH-DSA implementations', status: 'sh' },
      { phase: 'Q2 25', title: 'Known-answer test vectors', desc: 'KAT generation and validation', status: 'sh' },
      { phase: 'Q3 25', title: 'Hybrid signature design', desc: 'Architecture for hybrid classical + PQ identity', status: 'pr' },
      { phase: 'Q4 25', title: 'Cryptographic SBOM', desc: 'Bill of materials for all crypto primitives', status: 'pr' },
      { phase: 'Q1 26', title: 'NIST compliance audit', desc: 'Standards compliance review of implementations', status: 'fn' },
      { phase: 'Q2 26', title: 'External review', desc: 'Independent cryptography implementation review', status: 'fn' },
    ],
    breakdown: [
      { label: 'PQC implementation audit', pct: 35 },
      { label: 'Test vectors & harness', pct: 20 },
      { label: 'External cryptography review', pct: 20 },
      { label: 'Hybrid architecture design', pct: 15 },
      { label: 'Documentation & migration guide', pct: 10 },
    ],
    status: {
      sh: ['ML-KEM prototype', 'ML-DSA prototype', 'SLH-DSA prototype', 'KAT test vectors'],
      pr: ['Hybrid signature architecture', 'Cryptographic SBOM draft'],
      fn: ['NIST compliance audit', 'External cryptography review', 'Wallet migration prototype'],
    },
    cta: ['Fund PQC Audit Sprint', 'Partner on Quantum Readiness', 'Request Cryptography SBOM'],
  },
  {
    id: 'proof-forge',
    badge: 'OPEN SOURCE / PUBLIC GOODS',
    codename: 'PROOFFORGE',
    hero: ['Mainnet Claims Should Be','Machine-Verifiable'],
    sub: 'ProofForge is an open-source readiness audit system that compares blockchain documentation, feature claims, tests, and launch gates against the actual codebase to generate machine-readable GO / NO-GO reports.',
    readiness: 55,
    mode: 'live_testnet',
    funders: [
      'Gitcoin',
      'OpenSSF',
      'Ethereum Public Goods',
      'Web3 Foundation',
      'Filecoin Public Goods',
    ],
    problem: 'Crypto projects routinely overclaim readiness. Docs drift from code. Tests rot. Security gaps hide behind marketing. Mainnet launches happen without verifiable evidence.',
    solution: 'ProofForge maps feature claims to implementation evidence, detects doc/code drift, enforces test coverage gates, audits unsafe Rust patterns, and generates signed readiness reports.',
    modules: [
      { name: 'Feature-Claim Matrix', desc: 'Maps documented feature claims to implementation evidence in the codebase' },
      { name: 'Docs Drift Detector', desc: 'Identifies divergence between documentation and code over time' },
      { name: 'Test Coverage Gate', desc: 'Enforces minimum test coverage thresholds for launch-critical modules' },
      { name: 'Panic / Unwrap Audit', desc: 'Identifies unsafe panic and unwrap patterns in Rust codebases' },
      { name: 'Genesis Config Lint', desc: 'Validates chain genesis configuration against security best practices' },
      { name: 'GO / NO-GO Report', desc: 'Final machine-readable launch readiness report with evidence links' },
    ],
    fundingUnlocks: [
      'Open-source CLI release',
      'Web dashboard deployment',
      'GitHub Action integration',
      'Multi-chain template library',
      'Standardized report schema',
      'Grant-ready evidence pack templates',
    ],
    roadmap: [
      { phase: 'Q4 24', title: 'Readiness gate concepts', desc: 'Initial specification of launch gates and checks', status: 'sh' },
      { phase: 'Q1 25', title: 'Internal readiness tooling', desc: 'Private ProofForge running on X3 codebase', status: 'sh' },
      { phase: 'Q2 25', title: 'Feature-claim matrix', desc: 'Claims-to-evidence mapping system', status: 'sh' },
      { phase: 'Q3 25', title: 'Test coverage gate', desc: 'Automated coverage enforcement for launch modules', status: 'pr' },
      { phase: 'Q4 25', title: 'Docs drift prototype', desc: 'Automated doc-to-code divergence detection', status: 'pr' },
      { phase: 'Q1 26', title: 'Open-source CLI v1', desc: 'Public CLI tool for blockchain readiness auditing', status: 'fn' },
      { phase: 'Q2 26', title: 'Web dashboard', desc: 'Browser-based readiness report viewer', status: 'fn' },
    ],
    breakdown: [
      { label: 'Open-source CLI development', pct: 35 },
      { label: 'Dashboard & report schema', pct: 25 },
      { label: 'GitHub Action & CI integration', pct: 15 },
      { label: 'Multi-chain template library', pct: 15 },
      { label: 'Documentation & community', pct: 10 },
    ],
    status: {
      sh: ['Feature-claim matrix', 'Internal ProofForge tooling', 'Readiness gate specification', '24 runner modules'],
      pr: ['Test coverage gate integration', 'Docs drift detection', 'Report schema draft'],
      fn: ['Open-source CLI v1', 'Web dashboard', 'GitHub Action', 'Multi-chain templates'],
    },
    cta: ['Fund ProofForge Release', 'Sponsor Open-Source Gates', 'Request Demo Report'],
  },
  {
    id: 'dex-defi',
    badge: 'DEFI INFRASTRUCTURE',
    codename: 'AXE DEX + FORGE',
    hero: ['Native AMM DEX','and Token Launchpad'],
    sub: 'AXE is X3 Atomic Star\'s native AMM DEX with concentrated liquidity, flash loans, and token launchpad integration. The Forge enables permissionless token creation within the Atomic Kernel supply model.',
    readiness: 75,
    mode: 'guarded_testnet',
    funders: [
      'DeFi Infrastructure Grants',
      'Uniswap / AMM Ecosystem Grants',
      'Ecosystem Development Programs',
      'DeFi Security Research',
    ],
    problem: 'New chains launch without native DeFi primitives, forcing users to bridged DEXs with wrapped assets and fragmented liquidity.',
    solution: 'AXE DEX provides native AMM trading, concentrated liquidity positions, flash loans, and token launchpad — all within the Atomic Kernel supply model with invariant enforcement.',
    modules: [
      { name: 'AXE AMM DEX', desc: 'Native automated market maker with concentrated liquidity support' },
      { name: 'Flash Loan Pallet', desc: 'Atomic flash loans for arbitrage and liquidation' },
      { name: 'Token Factory (Forge)', desc: 'Permissionless token creation within supply invariant model' },
      { name: 'Launchpad', desc: 'Token presale and launch platform for new projects' },
      { name: 'LP Token Locker', desc: 'Anti-rug liquidity locking for token launches' },
      { name: 'Auction Pallet', desc: 'English auction mechanism for NFT and token sales' },
    ],
    fundingUnlocks: [
      'Concentrated liquidity implementation',
      'Limit order support',
      'DeFi security audit',
      'Integration test suite',
      'Frontend deployment',
      'Liquidity incentive program',
    ],
    roadmap: [
      { phase: 'Q4 24', title: 'Basic AMM prototype', desc: 'Constant product AMM with swap and add/remove liquidity', status: 'sh' },
      { phase: 'Q1 25', title: 'Flash loan pallet', desc: 'Atomic flash loan implementation', status: 'sh' },
      { phase: 'Q2 25', title: 'Token factory', desc: 'Permissionless Forge integration', status: 'sh' },
      { phase: 'Q3 25', title: 'Launchpad integration', desc: 'Token presale platform with LP locking', status: 'pr' },
      { phase: 'Q4 25', title: 'Concentrated liquidity', desc: 'CLMM implementation for capital efficiency', status: 'pr' },
      { phase: 'Q1 26', title: 'Security audit', desc: 'External DeFi security review', status: 'fn' },
    ],
    breakdown: [
      { label: 'AXE AMM engine', pct: 30 },
      { label: 'Token factory & launchpad', pct: 25 },
      { label: 'Flash loan & auction', pct: 15 },
      { label: 'Security audit', pct: 20 },
      { label: 'Frontend & integration', pct: 10 },
    ],
    status: {
      sh: ['Constant product AMM', 'Flash loan pallet', 'Token factory', 'Basic launchpad'],
      pr: ['Concentrated liquidity', 'Limit order support', 'LP locker audit'],
      fn: ['Security audit', 'Frontend deployment', 'Liquidity incentives'],
    },
    cta: ['Fund DeFi Audit Sprint', 'Sponsor Liquidity Pool', 'Request Integration Spec'],
  },
  {
    id: 'gpu-reactor',
    badge: 'GPU INFRASTRUCTURE',
    bc: 'or',
    codename: 'X3 REACTOR',
    hero: ['GPU-Accelerated','Validator Benchmarking'],
    sub: 'X3 Reactor provides repeatable validator benchmarks, GPU-accelerated stress testing, telemetry pipelines, and signed performance reports for validators on X3 Atomic Star.',
    readiness: 40,
    mode: 'live_testnet',
    funders: [
      'NVIDIA Inception',
      'GPU Cloud Providers',
      'Akash / Decentralized Compute',
      'Cloud Credit Programs',
    ],
    problem: 'Validator performance claims are rarely reproducible. TPS numbers are quoted without methodology. Real stress test data is absent from most chain launches.',
    solution: 'X3 Reactor runs standardized benchmarks against fresh validator deployments, measuring TPS, latency (p50/p95/p99), failure recovery, and geographic distribution — all in signed reports.',
    modules: [
      { name: 'Validator Benchmark Runner', desc: 'Standardized workload execution with reproducible results' },
      { name: 'GPU Stress Test Engine', desc: 'GPU-accelerated transaction load generation at scale' },
      { name: 'RPC Latency Tester', desc: 'Multi-endpoint latency measurement across geographies' },
      { name: 'Stress Score Index', desc: 'Normalized performance score for validators under identical conditions' },
      { name: 'Telemetry Pipeline', desc: 'Real-time validator metrics ingestion and alerting' },
      { name: 'Signed Benchmark Report', desc: 'Cryptographically signed machine-readable performance reports' },
    ],
    fundingUnlocks: [
      'Multi-GPU benchmark infrastructure',
      'Multi-region cloud deployment',
      'Public benchmark dashboard',
      'Validator telemetry pipeline',
      'Signed report system',
      'GPU workload optimization',
    ],
    roadmap: [
      { phase: 'Q1 25', title: 'Basic benchmark scripts', desc: 'TPS and latency measurement tooling', status: 'sh' },
      { phase: 'Q2 25', title: 'Stress test framework', desc: 'Configurable load generation with failure scenarios', status: 'sh' },
      { phase: 'Q3 25', title: 'GPU acceleration', desc: 'GPU-accelerated load generation', status: 'pr' },
      { phase: 'Q4 25', title: 'Telemetry pipeline', desc: 'Validator metrics ingestion and Grafana dashboard', status: 'pr' },
      { phase: 'Q1 26', title: 'Multi-region infrastructure', desc: 'Geographically distributed benchmark execution', status: 'fn' },
      { phase: 'Q2 26', title: 'Public dashboard', desc: 'Open benchmark results with historical data', status: 'fn' },
    ],
    breakdown: [
      { label: 'GPU compute nodes', pct: 35 },
      { label: 'Multi-region cloud infrastructure', pct: 25 },
      { label: 'Telemetry pipeline & dashboard', pct: 20 },
      { label: 'Benchmark engine development', pct: 12 },
      { label: 'Documentation & onboarding', pct: 8 },
    ],
    status: {
      sh: ['TPS measurement tooling', 'Stress test framework v1', 'Basic benchmark scripts'],
      pr: ['GPU acceleration engine', 'Telemetry pipeline', 'Stress Score algorithm'],
      fn: ['Multi-GPU infrastructure', 'Multi-region deployment', 'Public dashboard'],
    },
    cta: ['Fund Reactor GPU Cluster', 'Sponsor Benchmark Node', 'Request Benchmark Results'],
  },
  {
    id: 'ai-agents',
    badge: 'AI / AGENT SYSTEMS',
    bc: 'pu',
    codename: 'X3 SWARM',
    hero: ['Multi-Agent AI Systems','for Blockchain Operations'],
    sub: 'X3 Swarm is a multi-agent AI framework for blockchain operations: automated testing, security monitoring, invariant verification, and maintenance — running on-chain through agent accounts with on-chain memory.',
    readiness: 20,
    mode: 'research',
    funders: [
      'AI Infrastructure Grants',
      'Decentralized AI Networks',
      'Open Source AI Funding',
      'Web3 AI Research Programs',
    ],
    problem: 'Blockchain maintenance is manual, reactive, and under-automated. Testing, invariant monitoring, security scanning, and upgrade rehearsal require significant human effort.',
    solution: 'X3 Swarm provides on-chain agent accounts with memory, policy enforcement, and swarm coordination — enabling automated testing agents, invariant monitors, security scanners, and maintenance bots.',
    modules: [
      { name: 'Agent Accounts', desc: 'On-chain accounts for AI agents with controlled permissions' },
      { name: 'Agent Memory', desc: 'On-chain memory pallet for agent state persistence' },
      { name: 'Agent Law (Policy)', desc: 'Policy enforcement framework for agent behavior constraints' },
      { name: 'Swarm Coordinator', desc: 'Multi-agent coordination and task distribution' },
      { name: 'Northern Swarm', desc: 'Off-chain swarm executor for agent task processing' },
      { name: 'Proof-Carrying Agent', desc: 'Agent framework with verifiable execution proofs' },
    ],
    fundingUnlocks: [
      'Agent development SDK',
      'Swarm coordination deployment',
      'Automated invariant monitoring agents',
      'Security scanning agent pipeline',
      'Agent memory optimization',
      'Proof-carrying agent framework',
    ],
    roadmap: [
      { phase: 'Q1 25', title: 'Agent accounts pallet', desc: 'On-chain agent identity and permissions', status: 'sh' },
      { phase: 'Q2 25', title: 'Agent memory pallet', desc: 'On-chain memory for agent state', status: 'sh' },
      { phase: 'Q3 25', title: 'Swarm coordination', desc: 'Multi-agent task distribution system', status: 'pr' },
      { phase: 'Q4 25', title: 'Policy enforcement', desc: 'Agent Law policy constraints', status: 'pr' },
      { phase: 'Q1 26', title: 'Northern Swarm executor', desc: 'Off-chain agent execution pipeline', status: 'fn' },
      { phase: 'Q2 26', title: 'Proof-carrying agents', desc: 'Verifiable execution proof framework', status: 'fn' },
    ],
    breakdown: [
      { label: 'Agent account & memory', pct: 25 },
      { label: 'Swarm coordination', pct: 25 },
      { label: 'Policy enforcement', pct: 20 },
      { label: 'Off-chain executor', pct: 20 },
      { label: 'Documentation & SDK', pct: 10 },
    ],
    status: {
      sh: ['Agent accounts pallet', 'Agent memory pallet', 'Basic agent identity model'],
      pr: ['Swarm coordination', 'Policy enforcement', 'Agent registry'],
      fn: ['Northern Swarm executor', 'Proof-carrying agent framework', 'SDK release'],
    },
    cta: ['Fund Swarm Development', 'Sponsor Agent Research', 'Request Technical Whitepaper'],
  },
  {
    id: 'infrastructure-cloud',
    badge: 'INFRASTRUCTURE',
    bc: 'or',
    codename: 'X3 CLOUD STACK',
    hero: ['Production Infrastructure','for Public Testnet'],
    sub: 'X3 Atomic Star operates testnet validators, RPC endpoints, indexers, monitoring, and edge infrastructure — with Kubernetes, Docker, Prometheus, Grafana, and Cloudflare.',
    readiness: 35,
    mode: 'live_testnet',
    funders: [
      'AWS Activate',
      'Google Cloud Startups',
      'Microsoft Founders Hub',
      'Cloudflare for Startups',
      'DigitalOcean Hatch',
    ],
    problem: 'Running a production-quality blockchain testnet requires reliable multi-region infrastructure, monitoring, alerting, and developer access — all before mainnet revenue exists.',
    solution: 'X3 deploys Kubernetes-managed validator nodes, load-balanced RPC endpoints, chain indexers, Prometheus/Grafana monitoring, and Cloudflare edge protection across multiple regions.',
    modules: [
      { name: 'Validator Nodes', desc: 'Kubernetes-managed validator deployments in multiple regions' },
      { name: 'RPC Endpoints', desc: 'Load-balanced public RPC with rate limiting and DDoS protection' },
      { name: 'Chain Indexer', desc: 'Full chain history indexing for explorers and developer tooling' },
      { name: 'Monitoring Stack', desc: 'Prometheus + Grafana with Loki logging and AlertManager' },
      { name: 'Edge / API Protection', desc: 'Cloudflare edge caching, WAF, and DDoS mitigation' },
      { name: 'Blockchain TPS Monitor', desc: 'Real-time TPS tracking with historical trend visualization' },
    ],
    fundingUnlocks: [
      'Multi-region testnet deployment',
      'Public RPC endpoint availability',
      'External developer access',
      'Monitoring dashboard',
      'Validator diversity (community operators)',
      'CI/CD pipeline infrastructure',
    ],
    roadmap: [
      { phase: 'Q1 25', title: 'Local testnet', desc: 'Single-region testnet with basic monitoring', status: 'sh' },
      { phase: 'Q2 25', title: 'Docker deployment', desc: 'Containerized infrastructure with docker-compose', status: 'sh' },
      { phase: 'Q3 25', title: 'Kubernetes migration', desc: 'K8s-managed validator and RPC nodes', status: 'pr' },
      { phase: 'Q4 25', title: 'Multi-region expansion', desc: 'Second and third geographic deployment regions', status: 'pr' },
      { phase: 'Q1 26', title: 'Public RPC', desc: 'External developer RPC access with rate limiting', status: 'fn' },
      { phase: 'Q2 26', title: 'Community validators', desc: 'Independent operator onboarding', status: 'fn' },
    ],
    breakdown: [
      { label: 'Validator & testnet nodes', pct: 30 },
      { label: 'RPC / indexer infrastructure', pct: 20 },
      { label: 'Monitoring & alerting', pct: 20 },
      { label: 'Edge / DDoS protection', pct: 15 },
      { label: 'CI/CD pipeline', pct: 10 },
      { label: 'Backup & redundancy', pct: 5 },
    ],
    status: {
      sh: ['Single-region testnet', 'Docker infrastructure', 'Basic monitoring', 'Grafana dashboards'],
      pr: ['Kubernetes migration', 'Multi-region planning', 'TPS monitor'],
      fn: ['Multi-region deployment', 'Public RPC endpoint', 'Community validators'],
    },
    cta: ['Provide Cloud Credits', 'Sponsor Testnet Infrastructure', 'Request Infrastructure Plan'],
  },
]

// ─── SPONSORS ──────────────────────────────────────────────────────────────

export const SD: Sponsor[] = [
  {
    id: 'gpu-donation',
    badge: 'GPU SPONSORSHIP',
    bc: 'or',
    codename: 'X3 REACTOR POWER',
    hero: ['Power the','X3 Reactor'],
    sub: 'X3 Reactor uses GPUs for benchmark execution, transaction load generation, and validator stress testing. Donated hardware or cloud credits produce signed public benchmark reports.',
    problem: 'Validator performance benchmarking, GPU-accelerated load testing, and AI-assisted agent execution require compute that early-stage infrastructure projects cannot always afford.',
    solution: 'GPU sponsorship directly powers benchmark execution, stress simulation, and telemetry analysis — producing cryptographically signed public reports.',
    modules: [
      { name: 'Validator Benchmark Runner', desc: 'Standardized benchmark execution on donated GPUs' },
      { name: 'GPU Stress Test Engine', desc: 'GPU-accelerated transaction load generation at scale' },
      { name: 'Telemetry Analysis', desc: 'Large-scale data processing for performance analytics' },
      { name: 'Reactor Benchmark Reports', desc: 'Signed public benchmark reports with hardware attribution' },
    ],
    hardware: [
      'RTX 3090 / 4090 / 5090 class GPUs',
      'RTX A-series (A4000 / A5000 / A6000)',
      'L40S / A100 / H100 cloud credits',
      'Older GTX/RTX cards for lab nodes',
      'GPU servers with multi-card support',
      'PCIe bifurcation and riser hardware',
      'Power supplies and cooling',
    ],
    tiers: [
      { num: '01', name: 'GPU Donor', range: 'One card or small credit allocation', benefits: 'Sponsor wall listing, hardware impact report, optional anonymous donation' },
      { num: '02', name: 'Reactor Sponsor', range: 'Multi-GPU rig or $1K+ cloud credits', benefits: 'Named contribution to benchmark reports, dashboard mention' },
      { num: '03', name: 'Cloud GPU Sponsor', range: 'Monthly credit allocation', benefits: 'Recurring infrastructure recognition, monthly usage report' },
      { num: '04', name: 'Founding Compute Partner', range: 'Strategic recurring support', benefits: 'Primary compute sponsor status, co-authored reports' },
    ],
    status: {
      sh: ['GPU workload architecture', 'Basic benchmark scripts', 'Reactor report format'],
      pr: ['GPU acceleration engine', 'Validator simulation framework'],
      fn: ['Multi-GPU infrastructure', 'Multi-region benchmark deployment', 'Signed report system'],
    },
    cta: ['Donate a GPU', 'Sponsor GPU Cloud Credits', 'Fund a Reactor Node'],
  },
  {
    id: 'recycled-servers',
    badge: 'HARDWARE DONATION',
    codename: 'VALIDATOR LAB',
    hero: ['Retired Enterprise Iron,','New Validator Purpose'],
    sub: 'X3 Atomic Star accepts suitable retired enterprise servers to build validator lab nodes, testnet validators, archive nodes, and developer workstations.',
    problem: 'Enterprise hardware is often retired with years of useful life remaining. IT departments discard servers that can still run blockchain nodes reliably.',
    solution: 'X3 repurposes retired servers into validator lab infrastructure — private testnets, RPC endpoints, archive/indexer nodes, and developer workstations.',
    modules: [
      { name: 'Validator Lab Nodes', desc: 'Private testnet validators running on donated hardware' },
      { name: 'Public Testnet Backup', desc: 'Secondary validators for testnet redundancy' },
      { name: 'Archive / Indexer Nodes', desc: 'Chain history storage for explorers and developer tooling' },
      { name: 'Developer Lab Machines', desc: 'Workstations for contributor onboarding and testing' },
    ],
    hardware: [
      'Dell PowerEdge servers (R720, R730, R740+)',
      'Lenovo ThinkSystem servers',
      'HPE ProLiant servers',
      'Supermicro servers',
      'NVMe / SATA SSDs (500GB+)',
      'ECC DDR4 / DDR5 RAM (64GB+ preferred)',
      '10GbE / 25GbE NICs',
      'Rackmount chassis, rails, PSUs',
    ],
    acceptance: [
      'Boots to BIOS successfully',
      '64GB+ RAM preferred',
      'SSD capable',
      'Working power supplies',
      'No BIOS password lock',
      'Drives removed or confirmed wiped',
    ],
    security: 'All drives wiped using NIST 800-88 procedures. Firmware inventoried. Hardware photographed and logged with chain-of-custody documentation. No donor data retained.',
    tiers: [
      { num: '01', name: 'Component Donor', range: 'SSDs, RAM, NICs, cables', benefits: 'Sponsor wall listing, hardware impact report, optional anonymous' },
      { num: '02', name: 'Server Donor', range: 'Full server (any accepted model)', benefits: 'Named node, monthly uptime report' },
      { num: '03', name: 'Infrastructure Donor', range: 'Multiple servers or full rack', benefits: 'Infrastructure sponsor placement, quarterly impact report' },
    ],
    status: {
      sh: ['Hardware intake process', 'Drive wipe procedures', 'Asset logging system'],
      pr: ['Rack infrastructure setup', 'Validator lab power provisioning'],
      fn: ['Additional server donations', 'Rack rails and hardware', 'Freight sponsorship'],
    },
    cta: ['Donate Retired Servers', 'Sponsor Freight', 'Request Hardware Intake Checklist'],
  },
  {
    id: 'hardware-wallets',
    badge: 'HARDWARE WALLET INTEGRATION',
    codename: 'SECURE SIGNING',
    hero: ['Secure Signing for','Multi-VM Settlement'],
    sub: 'X3 Atomic Star seeks hardware wallet sponsorships and integrations for validator keys, treasury operations, developer wallets, and multi-VM transaction signing.',
    problem: 'Multi-VM settlement creates signing complexity across different transaction formats. Users and validators need hardware-backed, human-readable transaction security.',
    solution: 'X3 integrates hardware wallet flows for validator operations, token transfers, DEX swaps, cross-chain deposits, and treasury management with clear signing display.',
    modules: [
      { name: 'X3STAR Transfers', desc: 'Native token transfer signing with clear signing display' },
      { name: 'STARDEX Swap Signing', desc: 'DEX swap authorization with amount and slippage confirmation' },
      { name: 'Atomic Gateway Deposits', desc: 'Cross-chain deposit signing with destination verification' },
      { name: 'Validator Staking', desc: 'Validator key signing with clear signing support' },
      { name: 'Treasury Multisig', desc: 'Multi-signature treasury operations with hardware signing' },
    ],
    hardware: [
      'Ledger Nano X / Flex / Stax',
      'Trezor Model T / Safe 3 / Safe 5',
      'GridPlus Lattice1',
      'Keystone Pro',
      'OneKey Pro',
      'Developer test devices (any brand)',
    ],
    note: 'All wallet partnerships are listed as integration targets until functional integration is verified. No endorsement implied without explicit partner agreement.',
    tiers: [
      { num: '01', name: 'Device Sponsor', range: '1–5 developer test devices', benefits: 'Integration test coverage, acknowledgment in documentation' },
      { num: '02', name: 'SDK Partner', range: 'Developer SDK access and support', benefits: 'Named integration partner, co-authored integration guide' },
      { num: '03', name: 'Security Partner', range: 'Devices + SDK + integration support', benefits: 'Security sponsor placement once integration is verified' },
    ],
    status: {
      sh: ['Signing specification document', 'Transaction format design', 'Clear signing parameter mapping'],
      pr: ['Wallet SDK research', 'Integration design'],
      fn: ['Developer test devices', 'SDK access', 'Integration development', 'Clear signing implementation'],
    },
    cta: ['Sponsor Hardware Wallets', 'Partner on Wallet Integration', 'Request X3 Signing Spec'],
  },
  {
    id: 'validator-node-kit',
    badge: 'VALIDATOR SPONSORSHIP',
    codename: 'NODE KIT PROGRAM',
    hero: ['Sponsor a Validator,','Power a Testnet'],
    sub: 'Validator Node Kits help independent operators run X3 Atomic Star testnet validators with standardized hardware, monitoring, security, and documentation.',
    problem: 'Decentralized testnets fail when only the core team runs infrastructure. Independent validators are critical for geographic diversity and realistic testnet conditions.',
    solution: 'X3 Validator Node Kits provide standardized hardware specs, monitoring stacks, security configurations, and documentation — making it easy for anyone to run a node.',
    modules: [
      { name: 'Mini Node Kit', desc: 'Observer / RPC node for testnet participation' },
      { name: 'Validator Kit', desc: 'Full validator with monitoring, alerting, and key management' },
      { name: 'Archive Kit', desc: 'Indexer and archive node for developer tooling' },
      { name: 'Reactor Kit', desc: 'Benchmark node for validator performance measurement' },
    ],
    hardware: [
      'Server or workstation matching kit spec',
      'NVMe SSD storage',
      'ECC RAM',
      '10GbE NIC',
      'UPS (recommended)',
      'Setup guide and security checklist',
    ],
    tiers: [
      { num: '01', name: 'Mini Kit Sponsor', range: '$500 – $2,500', benefits: 'Named RPC node, testnet dashboard mention, monthly uptime report' },
      { num: '02', name: 'Validator Kit Sponsor', range: '$2,500 – $10,000', benefits: 'Named validator node, performance report, operator recognition' },
      { num: '03', name: 'Archive Kit Sponsor', range: '$2,500 – $15,000', benefits: 'Named archive node, storage reports, developer tool attribution' },
      { num: '04', name: 'Full Lab Sponsor', range: '$25,000 – $100,000', benefits: 'Major infrastructure sponsor, quarterly impact report' },
    ],
    status: {
      sh: ['Validator setup documentation v1', 'Monitoring stack configuration', 'Security checklist'],
      pr: ['Hardware specification finalization', 'Node kit packaging'],
      fn: ['Hardware for sponsored kits', 'Independent operator recruitment', 'Multi-region deployment'],
    },
    cta: ['Sponsor a Validator Kit', 'Apply to Run a Sponsored Node', 'Fund Validator Decentralization'],
  },
]

// ─── FUNDING PROGRAMS ──────────────────────────────────────────────────────

export const FD: FundingTarget[] = [
  {
    id: 'sbir-sttr',
    badge: 'SBIR / STTR',
    bc: 'sb',
    codename: 'FEDERAL NON-DILUTIVE',
    hero: ['Non-Dilutive Federal R&D','Funding Programs'],
    sub: 'SBIR / STTR funds approximately 4,000 companies per year with equity-free R&D capital. Phase I: $50K–$275K. NSF America\'s Seed Fund takes zero equity.',
    pitch: 'X3 Atomic Star develops proof-driven distributed infrastructure: machine-verifiable blockchain readiness (ProofForge), cross-VM settlement, post-quantum migration tooling, and AI-assisted agent operations — all applicable to SBIR/STTR topic areas.',
    targets: [
      { name: 'NSF America\'s Seed Fund', amount: 'Up to $2M, zero equity', desc: 'Deep-tech startups. Project Pitch accepted any time. Strong fit for ProofForge, cross-VM, and distributed systems research.' },
      { name: 'DoD SBIR / STTR', amount: 'Phase I: up to $275K', desc: 'Resilient networks, secure compute, post-quantum cryptography, and cyber infrastructure topics.' },
      { name: 'NIST SBIR', amount: 'Program-specific', desc: 'Security measurement, post-quantum cryptography, and cybersecurity tooling.' },
      { name: 'DHS / CISA Programs', amount: 'Varies', desc: 'Critical infrastructure cyber-readiness and resilient distributed systems.' },
    ],
    status: {
      sh: ['Company formation (SBIR eligibility)', 'Technical concept documentation', 'Prototype infrastructure running'],
      pr: ['NSF Project Pitch preparation', 'Technical whitepaper draft', 'Agency topic monitoring'],
      fn: ['SBIR application support', 'Technical writing resources', 'Phase I prototype funding'],
    },
    cta: ['Request Technical Whitepaper', 'Discuss SBIR Alignment', 'Sponsor Phase I Preparation'],
  },
  {
    id: 'colorado',
    badge: 'COLORADO PROGRAMS',
    codename: 'LOCAL ROOTS',
    hero: ['Colorado-Based','Blockchain Infrastructure'],
    sub: 'X3 Atomic Star is Colorado-based with a physical validator lab, eligible for state advanced industries funding, workforce development grants, and local economic development programs.',
    pitch: 'X3 operates a Colorado validator lab with repurposed server hardware, provides workforce training in Linux administration and blockchain operations, and develops AI-assisted infrastructure tooling.',
    targets: [
      { name: 'Advanced Industries Early-Stage Grant', amount: 'Up to $250K', desc: 'Requires 2:1 company-to-state cash match. Technology and infrastructure engineering qualify.' },
      { name: 'Proof of Concept Grant', amount: 'Up to $150K', desc: 'Colorado commercialization partnerships. Best with university research partner.' },
      { name: 'Opportunity Now Colorado', amount: '$89.5M+ awarded', desc: 'Workforce training grants. Angle: validator operations, blockchain development, infrastructure.' },
      { name: 'Denver / Front Range Programs', amount: 'Varies', desc: 'Local economic development, tech incubators, and innovation programs.' },
    ],
    status: {
      sh: ['Colorado entity registered', 'Physical lab operational', 'Team based in Colorado'],
      pr: ['Advanced Industries application research', 'Workforce training curriculum outline'],
      fn: ['Cash match for AI grant', 'University partnership', 'Workforce program development'],
    },
    cta: ['Apply for Colorado Grant', 'Partner on Workforce Training', 'Discuss Colorado Options'],
  },
  {
    id: 'accelerators',
    badge: 'ACCELERATOR PROGRAMS',
    bc: 'pu',
    codename: 'AI + CLOUD ACCELERATORS',
    hero: ['Startup Accelerators','and Cloud Credit Programs'],
    sub: 'X3 Atomic Star targets AI startup accelerators and cloud credit programs for infrastructure, compute, and AI-assisted development. No AGI hype — real distributed systems work.',
    pitch: 'X3 applies AI/ML to blockchain operations: automated invariant monitoring, agent-driven testing, and GPU-accelerated benchmarking. Eligible for AI infrastructure credits and hardware accelerator programs.',
    targets: [
      { name: 'NVIDIA Inception', amount: 'Developer tools + credits', desc: 'AI startup program with GPU credits, preferred pricing, and investor exposure. Strong fit for X3 Reactor.' },
      { name: 'AWS Activate', amount: 'Up to $100K credits', desc: 'Cloud credits for eligible startups. Infrastructure, testing, and deployment coverage.' },
      { name: 'Google Cloud Startups', amount: 'Up to $350K credits', desc: 'Cloud credits for AI-first startups. StarSignal inference and training infrastructure fit.' },
      { name: 'Microsoft Founders Hub', amount: 'Up to $150K credits', desc: 'Azure credits + AI model access + OpenAI integration.' },
    ],
    status: {
      sh: ['Program eligibility documented', 'Company registered', 'Infrastructure requirements specified'],
      pr: ['Application in progress', 'Credit allocation plan'],
      fn: ['Cloud credits deployed', 'Multi-region infrastructure', 'Public testnet operational'],
    },
    cta: ['Apply for Cloud Credits', 'Sponsor Accelerator Application', 'Request Infrastructure Plan'],
  },
  {
    id: 'ecosystem-grants',
    badge: 'ECOSYSTEM GRANTS',
    codename: 'PROTOCOL FUNDING',
    hero: ['Blockchain Ecosystem','Grant Programs'],
    sub: 'X3 Atomic Star targets ecosystem grant programs from blockchain foundations and protocols that fund infrastructure, interoperability, and open-source development.',
    pitch: 'X3 provides open-source blockchain infrastructure, cross-chain interoperability components, and public goods tooling (ProofForge) — all within scope of major ecosystem grant programs.',
    targets: [
      { name: 'Web3 Foundation Grants', amount: 'Up to $100K+', desc: 'Open-source blockchain infrastructure, research, and tooling. Strong fit for cross-VM and proof systems.' },
      { name: 'Polkadot Treasury', amount: 'Community-determined', desc: 'Substrate-based chain development, pallet development, and ecosystem integration.' },
      { name: 'Solana Foundation Grants', amount: 'Varies', desc: 'SVM integration, cross-chain bridging, and ecosystem development.' },
      { name: 'Gitcoin / Public Goods', amount: 'Community-determined', desc: 'Open-source public goods funding rounds. ProofForge alignment.' },
    ],
    status: {
      sh: ['Grant research complete', 'Proposal templates prepared', 'Technical documentation ready'],
      pr: ['Grant applications in preparation', 'Community engagement'],
      fn: ['Grant funding deployed', 'Milestone delivery', 'Open-source release'],
    },
    cta: ['Fund Open-Source Development', 'Sponsor Grant Application', 'Request Proposal Packet'],
  },
]

// ─── READINESS SCORE DATA ──────────────────────────────────────────────────

export interface RealFeature {
  name: string
  readiness: number
  mode: string
  description: string
}

export const REAL_FEATURES: RealFeature[] = [
  { name: 'Atomic Router', readiness: 88, mode: 'live_testnet', description: '6-route cross-VM atomic transfer matrix' },
  { name: 'Atomic Kernel', readiness: 85, mode: 'live_testnet', description: 'Canonical asset accounting and invariant enforcement' },
  { name: 'AXE DEX', readiness: 75, mode: 'guarded_testnet', description: 'Native AMM with concentrated liquidity' },
  { name: 'Atomic Lock', readiness: 68, mode: 'live_testnet', description: 'HTLC atomic swap primitive' },
  { name: 'TriForge Runtime', readiness: 65, mode: 'guarded_testnet', description: 'Native + EVM + SVM execution domains' },
  { name: 'Atomic Gateway', readiness: 65, mode: 'guarded_testnet', description: 'Cross-chain bridge with SPV proofs' },
  { name: 'Launch Gate', readiness: 55, mode: 'live_testnet', description: 'Binary pass/fail readiness gates' },
  { name: 'X3 Wallet Pallet', readiness: 55, mode: 'live_testnet', description: 'On-chain wallet with multisig' },
  { name: 'X3 Sentinel', readiness: 50, mode: 'guarded_testnet', description: 'Security monitoring and alerting' },
  { name: 'X3 Reactor', readiness: 40, mode: 'live_testnet', description: 'GPU validator benchmark system' },
  { name: 'BTC Fortress', readiness: 25, mode: 'sim_testnet', description: 'Bitcoin vault with SPV and multisig' },
  { name: 'X3 Swarm Core', readiness: 25, mode: 'guarded_testnet', description: 'Multi-agent AI coordination system' },
  { name: 'Quantum Crypto', readiness: 35, mode: 'research', description: 'ML-KEM, ML-DSA, SLH-DSA prototypes' },
]

// ─── BOUNTIES ──────────────────────────────────────────────────────────────

export interface Bounty {
  prize: string
  title: string
  desc: string
  tag: 'open' | 'hot' | 'research'
  difficulty: string
}

export const BOUNTIES: Bounty[] = [
  { prize: '$2,000', title: 'ProofForge GitHub Action', desc: 'Build a GitHub Action that runs ProofForge readiness checks on any Rust blockchain project and posts status comments on PRs.', tag: 'open', difficulty: 'Medium' },
  { prize: '$1,000', title: 'Testnet Faucet', desc: 'Deploy a rate-limited testnet token faucet with frontend and anti-abuse protections for X3 testnet.', tag: 'open', difficulty: 'Easy' },
  { prize: '$5,000', title: 'Block Explorer Integration', desc: 'Integrate X3 testnet with Blockscout or similar open-source block explorer.', tag: 'open', difficulty: 'Hard' },
  { prize: '$3,000', title: 'Hardware Wallet Signing Prototype', desc: 'Build working prototype for X3STAR transfer signing on Ledger or Trezor with clear signing display.', tag: 'hot', difficulty: 'Hard' },
  { prize: '$1,500', title: 'Validator Setup Script', desc: 'Write reproducible bash setup script for deploying X3 testnet validator on Ubuntu 24.04.', tag: 'open', difficulty: 'Easy' },
  { prize: '$10,000', title: 'PQC Test Vector Harness', desc: 'Build comprehensive known-answer test harness for ML-DSA and SLH-DSA, compatible with NIST KAT vectors.', tag: 'research', difficulty: 'Expert' },
  { prize: '$1,500', title: 'STARDEX UI Polish', desc: 'Improve the DEX frontend: better swap UX, animated price display, mobile responsiveness.', tag: 'open', difficulty: 'Medium' },
  { prize: '$4,000', title: 'Route Proof Verifier CLI', desc: 'Build CLI tool that reads X3 StarPacket route receipt and verifies cross-domain supply invariant.', tag: 'research', difficulty: 'Hard' },
  { prize: '$750', title: 'Grafana Dashboard Template', desc: 'Create Grafana dashboard template for X3 validator monitoring with key health metrics.', tag: 'open', difficulty: 'Easy' },
  { prize: '$2,500', title: 'Agent SDK TypeScript Wrapper', desc: 'Build TypeScript SDK wrapping agent account and memory pallet APIs with typed responses.', tag: 'open', difficulty: 'Medium' },
]

// ─── SERVICES ──────────────────────────────────────────────────────────────

export interface Service {
  badge: string
  title: string
  desc: string
  price: string
  features: string[]
}

export const SERVICES: Service[] = [
  { badge: 'PROOFFORGE AUDIT', title: 'Launch Readiness Audit', desc: 'We run ProofForge on your blockchain project and deliver a machine-readable readiness report with green/yellow/red status for every launch gate.', price: 'Contact for pricing', features: ['Feature-claim-to-code matrix', 'Docs drift detection', 'Test coverage gate analysis', 'Panic/unwrap audit (Rust)', 'Genesis config lint', 'GO / NO-GO report', 'Machine-readable JSON output'] },
  { badge: 'REACTOR BENCHMARK', title: 'Validator Benchmark Report', desc: 'We run X3 Reactor against your validator and deliver a signed benchmark report with real TPS, latency, and failure mode data.', price: 'Contact for pricing', features: ['Fresh-machine validator setup', 'TPS and latency measurement', 'p50/p95/p99 RPC latency', 'Failure mode stress testing', 'Stress Score Index', 'Signed benchmark JSON report', 'Reproducible methodology'] },
  { badge: 'STARSIGNAL RISK', title: 'Route Risk Assessment', desc: 'StarSignal analyzes your DEX routes and liquidity paths, delivering a signed risk classification report with safety scores.', price: 'Contact for pricing', features: ['Route safety scoring', 'Liquidity depth analysis', 'Counterparty risk classification', 'Bridge audit status check', 'Slippage exposure report', 'Machine-readable risk receipt'] },
  { badge: 'PQC REVIEW', title: 'Cryptographic SBOM', desc: 'We generate a cryptographic software bill of materials for your blockchain project and map it against NIST PQC migration targets.', price: 'Contact for pricing', features: ['Full cryptographic primitive inventory', 'NIST PQC migration mapping', 'Hybrid signature recommendations', 'Known-answer test vectors', 'Migration priority matrix', 'Signed SBOM artifact'] },
]

// ─── FOUNDING MEMBERSHIPS ──────────────────────────────────────────────────

export interface FoundingTier {
  tier: string
  price: string
  period: string
  color: string
  features: string[]
  featured?: boolean
  cta: string
}

export const FOUNDING_TIERS: FoundingTier[] = [
  { tier: 'BUILDER', price: '$99', period: 'one-time', color: '#4a6a88', features: ['Early testnet access', 'Private builder newsletter', 'Node setup documentation', 'Readiness report access', 'Supporter wall listing'], cta: 'Join as Builder' },
  { tier: 'OPERATOR', price: '$500', period: 'one-time', color: '#00c8ff', featured: true, features: ['Everything in Builder', 'Private builder calls (quarterly)', 'Validator setup support', 'X3 Reactor benchmark access', 'StarSignal risk API (beta)', 'Early ecosystem partner status', 'Named in infrastructure reports'], cta: 'Join as Operator' },
  { tier: 'VALIDATOR CIRCLE', price: '$2,500', period: 'one-time', color: '#8b5cf6', features: ['Everything in Operator', 'Named validator node on testnet', 'Monthly validator performance report', 'Early integration access', 'Direct team contact', 'Co-authored case study opportunity'], cta: 'Join Validator Circle' },
  { tier: 'INFRASTRUCTURE PATRON', price: '$10,000', period: 'one-time', color: '#f97316', features: ['Everything in Validator Circle', 'Founding patron placement', 'Hardware lab naming opportunity', 'Quarterly strategy calls', 'SBIR / grant evidence collaboration', 'Priority audit and benchmark access'], cta: 'Become Patron' },
]
