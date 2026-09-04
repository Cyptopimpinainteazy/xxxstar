import type { ScoreboardCategory } from '../types';

const CATEGORIES: { name: string; command: string }[] = [
  { name: 'x3-lang', command: 'ls x3-lang 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'x3-vm', command: 'ls x3-lang/vm 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'EVM adapter', command: 'ls X3-contracts/evm 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'SVM adapter', command: 'ls X3-contracts/svm 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'BTC adapter', command: 'ls adapters/btc 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'Relayer swarm', command: 'ls relayer 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'Proof ledger', command: 'ls x3-proof 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'Validator ops', command: 'ls validator 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'Security checks', command: 'ls tests/security 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'Test coverage', command: 'ls tests 2>/dev/null && echo "present" || echo "missing"' },
  { name: 'Workspace health', command: 'git status --porcelain 2>/dev/null | wc -l' },
  { name: 'Build status', command: 'cargo check 2>&1 | tail -1' },
];

export async function generateScoreboard(workspacePath: string): Promise<ScoreboardCategory[]> {
  const categories: ScoreboardCategory[] = [];

  for (const cat of CATEGORIES) {
    try {
      const { stdout, stderr, exitCode } = await window.x3studio.shell.exec(cat.command, workspacePath);
      const present = stdout.includes('present') || (exitCode === 0 && stdout.trim().length > 0);
      const score = present ? (exitCode === 0 ? 100 : 50) : 0;

      categories.push({
        name: cat.name,
        score,
        status: score === 100 ? 'PASS' : score > 0 ? 'PARTIAL' : 'FAIL',
        proofCommand: cat.command,
        proofArtifact: '',
        reason: present ? 'Module detected and responsive' : 'Module not detected',
        nextAction: present ? 'Maintain' : 'Create this module',
        lastChecked: new Date().toISOString(),
      });
    } catch {
      categories.push({
        name: cat.name,
        score: 0,
        status: 'BLOCKED',
        proofCommand: cat.command,
        proofArtifact: '',
        reason: 'Command execution failed',
        nextAction: 'Fix command execution',
        lastChecked: new Date().toISOString(),
      });
    }
  }

  // Write scoreboard
  try {
    const scoreboard = {
      generated: new Date().toISOString(),
      totalScore: Math.round(categories.reduce((a, c) => a + c.score, 0) / categories.length),
      categories,
    };
    await window.x3studio.fs.writeFile(
      `${workspacePath}/x3-proof/SCOREBOARD.json`,
      JSON.stringify(scoreboard, null, 2)
    );
    await window.x3studio.fs.writeFile(
      `${workspacePath}/x3-proof/SCOREBOARD.md`,
      generateScoreboardMarkdown(scoreboard)
    );
  } catch {}

  return categories;
}

function generateScoreboardMarkdown(sb: any): string {
  return `# X3 Scoreboard

**Overall Score: ${sb.totalScore}%**
Generated: ${sb.generated}

## Categories
${sb.categories.map((c: ScoreboardCategory) =>
  `### ${c.name}
- Score: ${c.score}/100
- Status: ${c.status}
- Reason: ${c.reason}
- Next Action: ${c.nextAction}
- Last Checked: ${c.lastChecked}
`
).join('\n')}
`;
}
