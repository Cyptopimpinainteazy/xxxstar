const fs = require("fs");
const path = require("path");

const root = path.join(__dirname, "..");
const files = [
  path.join(root, "js", "x3-data-api.js"),
  path.join(root, "js", "x3-page-adapters.js"),
  path.join(root, "server.js"),
  path.join(root, "server", "site-services.js"),
  path.join(root, "x3star-network-pulse.html"),
  path.join(root, "x3star-node-health.html"),
  path.join(root, "x3star-proof-wall.html"),
  path.join(root, "x3star-slot-tracker.html"),
  path.join(root, "x3star-whale-tracker.html"),
  path.join(root, "x3star-tokenomics-warroom.html"),
  path.join(root, "x3star-scarcity-clock.html"),
  path.join(root, "x3star-ecosystem-heartbeat.html"),
  path.join(root, "x3star-operator-war-room.html"),
  path.join(root, "x3star-mission-terminal.html"),
  path.join(root, "x3star-arbitrage-engine.html"),
  path.join(root, "chainbench-pro.html"),
  path.join(root, "chainbench-ultimate.html"),
  path.join(root, "chainbench-ultimate(1).html"),
  path.join(root, "blockchain-stress-test.html"),
  path.join(root, "blockchain-stress-test(1).html"),
];

const forbidden = [/Math\.random/, /Using fallback data/i, /simulated/i];
const failures = [];

for (const file of files) {
  if (!fs.existsSync(file)) continue;
  const content = fs.readFileSync(file, "utf8");
  for (const pattern of forbidden) {
    if (pattern.test(content)) {
      failures.push(`${path.relative(root, file)} matched ${pattern}`);
    }
  }
}

if (failures.length > 0) {
  process.stderr.write(`Synthetic data guard failed:\n${failures.join("\n")}\n`);
  process.exit(1);
}

process.stdout.write("Synthetic data guard passed.\n");
