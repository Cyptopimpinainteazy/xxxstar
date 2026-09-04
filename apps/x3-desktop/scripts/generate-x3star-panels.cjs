#!/usr/bin/env node

/**
 * Generate panel shim files for all 41 x3star-*.html pages
 * Each shim delegates to IframePanel which loads the page from localhost:8080
 */

const fs = require('fs');
const path = require('path');

// The 41 pages from x3-site-nav.js organized by category
const PAGES = [
  // Overview (5)
  { name: 'Landing', path: '/x3star-landing.html' },
  { name: 'Dashboard', path: '/x3star-dashboard.html' },
  { name: 'Spine', path: '/x3star-spine.html' },
  { name: 'EcosystemHeartbeat', path: '/x3star-ecosystem-heartbeat.html' },
  { name: 'MissionTerminal', path: '/x3star-mission-terminal.html' },

  // Operations (7)
  { name: 'NetworkPulse', path: '/x3star-network-pulse.html' },
  { name: 'NodeHealth', path: '/x3star-node-health.html' },
  { name: 'Governance', path: '/x3star-governance.html' },
  { name: 'Staking', path: '/x3star-staking.html' },
  { name: 'TransparencyLedger', path: '/x3star-transparency-ledger.html' },
  { name: 'ProofWall', path: '/x3star-proof-wall.html' },
  { name: 'OperatorWarRoom', path: '/x3star-operator-war-room.html' },

  // Capital (12)
  { name: 'TokenPresale', path: '/x3star-token-presale.html' },
  { name: 'ValidatorPresale', path: '/x3star-validator-presale.html' },
  { name: 'ScarecityClock', path: '/x3star-scarcity-clock.html' },
  { name: 'SlotTracker', path: '/x3star-slot-tracker.html' },
  { name: 'FundraiseThermometer', path: '/x3star-fundraise-thermometer.html' },
  { name: 'SocialProofWall', path: '/x3star-social-proof-wall.html' },
  { name: 'InvestorRelations', path: '/x3star-investor-relations.html' },
  { name: 'KycOnboarding', path: '/x3star-kyc-onboarding.html' },
  { name: 'RoiCalculator', path: '/x3star-roi-calculator.html' },
  { name: 'IfYouInvested', path: '/x3star-if-you-invested.html' },
  { name: 'IfYouHad', path: '/x3star-if-you-had.html' },
  { name: 'Portfolio', path: '/x3star-portfolio.html' },

  // Market (6)
  { name: 'WhaleTracker', path: '/x3star-whale-tracker.html' },
  { name: 'TokenomicsWarroom', path: '/x3star-tokenomics-warroom.html' },
  { name: 'ArbitrageEngine', path: '/x3star-arbitrage-engine.html' },
  { name: 'CompetitorGraveyard', path: '/x3star-competitor-graveyard.html' },
  { name: 'LeaderboardArena', path: '/x3star-leaderboard-arena.html' },
  { name: 'HallOfFame', path: '/x3star-hall-of-fame.html' },

  // Ecosystem (5)
  { name: 'Affiliate', path: '/x3star-affiliate.html' },
  { name: 'GrantHub', path: '/x3star-grant-hub.html' },
  { name: 'GrantMissionControl', path: '/x3star-grant-mission-control.html' },
  { name: 'BountyBoard', path: '/x3star-bounty-board.html' },
  { name: 'BarterExchange', path: '/x3star-barter-exchange.html' },

  // Benchmarks (5)
  { name: 'RpcReport', path: '/chainbench-pro.html' },
  { name: 'BenchmarkOverview', path: '/chainbench-ultimate.html' },
  { name: 'BenchmarkOverviewAlt', path: '/chainbench-ultimate(1).html' },
  { name: 'StressArtifact', path: '/blockchain-stress-test.html' },
  { name: 'StressArtifactAlt', path: '/blockchain-stress-test(1).html' },
];

const EMBEDS_DIR = '/home/lojak/Desktop/X3_ATOMIC_STAR/apps/x3-desktop/src/components/panels/embeds';
const BASE_URL = 'http://localhost:8080';

function generateShimFile(name, path) {
  const componentName = `${name}Panel`;
  const content = `import React from "react";
import IframePanel from "@/components/panels/IframePanel";

const URL = import.meta.env.DEV
  ? "${BASE_URL}${path}"
  : "https://x3star.net${path}";

const ${componentName}: React.FC = () => (
  <IframePanel url={URL} title="${name}" />
);

export default ${componentName};
`;
  return content;
}

function main() {
  console.log(`Generating ${PAGES.length} panel shim files...\n`);

  let created = 0;
  let skipped = 0;
  let errors = 0;

  PAGES.forEach(({ name, path }) => {
    const fileName = `${name}Panel.tsx`;
    const filePath = `${EMBEDS_DIR}/${fileName}`;

    try {
      if (fs.existsSync(filePath)) {
        console.log(`⊘ ${fileName} (already exists)`);
        skipped++;
      } else {
        const content = generateShimFile(name, path);
        fs.writeFileSync(filePath, content, 'utf-8');
        console.log(`✓ ${fileName}`);
        created++;
      }
    } catch (error) {
      console.log(`✗ ${fileName} (${error.message})`);
      errors++;
    }
  });

  console.log(`\n✓ Created: ${created}`);
  console.log(`⊘ Skipped: ${skipped}`);
  console.log(`✗ Errors: ${errors}`);
  console.log('Done!');
}

main();
