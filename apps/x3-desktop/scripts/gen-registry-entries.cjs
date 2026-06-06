#!/usr/bin/env node

const PAGES = [
  { name: 'Landing', id: 'x3star-landing' },
  { name: 'Dashboard', id: 'x3star-dashboard' },
  { name: 'Spine', id: 'x3star-spine' },
  { name: 'EcosystemHeartbeat', id: 'x3star-ecosystem-heartbeat' },
  { name: 'MissionTerminal', id: 'x3star-mission-terminal' },
  { name: 'NetworkPulse', id: 'x3star-network-pulse' },
  { name: 'NodeHealth', id: 'x3star-node-health' },
  { name: 'Governance', id: 'x3star-governance' },
  { name: 'Staking', id: 'x3star-staking' },
  { name: 'TransparencyLedger', id: 'x3star-transparency-ledger' },
  { name: 'ProofWall', id: 'x3star-proof-wall' },
  { name: 'OperatorWarRoom', id: 'x3star-operator-war-room' },
  { name: 'TokenPresale', id: 'x3star-token-presale' },
  { name: 'ValidatorPresale', id: 'x3star-validator-presale' },
  { name: 'ScarecityClock', id: 'x3star-scarcity-clock' },
  { name: 'SlotTracker', id: 'x3star-slot-tracker' },
  { name: 'FundraiseThermometer', id: 'x3star-fundraise-thermometer' },
  { name: 'SocialProofWall', id: 'x3star-social-proof-wall' },
  { name: 'InvestorRelations', id: 'x3star-investor-relations' },
  { name: 'KycOnboarding', id: 'x3star-kyc-onboarding' },
  { name: 'RoiCalculator', id: 'x3star-roi-calculator' },
  { name: 'IfYouInvested', id: 'x3star-if-you-invested' },
  { name: 'IfYouHad', id: 'x3star-if-you-had' },
  { name: 'Portfolio', id: 'x3star-portfolio' },
  { name: 'WhaleTracker', id: 'x3star-whale-tracker' },
  { name: 'TokenomicsWarroom', id: 'x3star-tokenomics-warroom' },
  { name: 'ArbitrageEngine', id: 'x3star-arbitrage-engine' },
  { name: 'CompetitorGraveyard', id: 'x3star-competitor-graveyard' },
  { name: 'LeaderboardArena', id: 'x3star-leaderboard-arena' },
  { name: 'HallOfFame', id: 'x3star-hall-of-fame' },
  { name: 'Affiliate', id: 'x3star-affiliate' },
  { name: 'GrantHub', id: 'x3star-grant-hub' },
  { name: 'GrantMissionControl', id: 'x3star-grant-mission-control' },
  { name: 'BountyBoard', id: 'x3star-bounty-board' },
  { name: 'BarterExchange', id: 'x3star-barter-exchange' },
  { name: 'RpcReport', id: 'chainbench-pro' },
  { name: 'BenchmarkOverview', id: 'chainbench-ultimate' },
  { name: 'BenchmarkOverviewAlt', id: 'chainbench-ultimate-alt' },
  { name: 'StressArtifact', id: 'blockchain-stress-test' },
  { name: 'StressArtifactAlt', id: 'blockchain-stress-test-alt' },
];

console.log('/* ── X3STAR Pages (41 HTML pages) ──── */');
PAGES.forEach(({ name }) => {
  console.log(`const ${name}Panel = lazy(() => import("@/components/panels/embeds/${name}Panel"));`);
});

console.log('\n/* X3STAR Pages - Registry Entries */');
PAGES.forEach(({ name, id }) => {
  console.log(`  "${id}": ${name}Panel,`);
});
