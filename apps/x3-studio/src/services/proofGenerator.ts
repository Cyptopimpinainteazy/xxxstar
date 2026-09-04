import type { ProofRecord } from '../types';

export async function runProofCommand(
  command: string,
  cwd: string,
  label: string
): Promise<ProofRecord> {
  const startTime = new Date().toISOString();
  const startMs = Date.now();

  const { stdout, stderr, exitCode } = await window.x3studio.shell.exec(command, cwd);
  const duration = Date.now() - startMs;

  const changedFiles: string[] = [];
  try {
    const diff = await window.x3studio.git.diff(cwd);
    if (diff) {
      const lines = diff.split('\n').filter(l => l.includes('|'));
      changedFiles.push(...lines.map(l => l.split('|')[0].trim()));
    }
  } catch {}

  const status: ProofRecord['status'] =
    exitCode === 0 ? 'PASS' :
    exitCode === null ? 'BLOCKED' :
    'FAIL';

  const record: ProofRecord = {
    id: `proof-${Date.now()}`,
    command,
    cwd,
    startTime,
    endTime: new Date().toISOString(),
    duration,
    exitCode,
    stdout,
    stderr,
    status,
    changedFiles,
    artifacts: [],
  };

  // Write proof report
  try {
    const proofDir = `${cwd}/x3-proof`;
    await window.x3studio.fs.createDirectory(proofDir);
    const report = generateProofReport(record);
    await window.x3studio.fs.writeFile(`${proofDir}/PROOF_REPORT.json`, JSON.stringify(record, null, 2));
    await window.x3studio.fs.writeFile(`${proofDir}/PROOF_REPORT.md`, report);
  } catch {}

  return record;
}

export function generateProofReport(record: ProofRecord): string {
  return `# Proof Report

## Command
\`\`\`
${record.command}
\`\`\`

## Status
**${record.status}** (exit code: ${record.exitCode})

## Duration
${record.duration}ms

## Working Directory
\`${record.cwd}\`

## Stdout
\`\`\`
${record.stdout.substring(0, 2000)}
\`\`\`

## Stderr
\`\`\`
${record.stderr.substring(0, 2000)}
\`\`\`

## Changed Files
${record.changedFiles.map(f => `- ${f}`).join('\n') || 'None'}

## Timestamps
- Start: ${record.startTime}
- End: ${record.endTime}
`;
}
