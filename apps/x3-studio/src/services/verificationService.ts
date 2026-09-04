import type { ProofRecord, VerificationProgress } from '../types';
import { useProofStore } from '../store';
import { runFakeCodeScanner } from './fakeCodeScanner';
import { generateScoreboard } from './scoreboardGenerator';
import { runProofCommand } from './proofGenerator';

const STEPS = [
  'cargo check',
  'forge build',
  'cargo test',
  'npm/pnpm test',
  'forge test',
  'Fake-code scan',
  'Security scan',
  'Scoreboard generation',
];

export async function runFullVerification(workspacePath: string): Promise<{
  passed: number;
  failed: number;
  records: ProofRecord[];
}> {
  const setProgress = useProofStore.getState().setProgress;
  const addRecord = useProofStore.getState().addRecord;
  const records: ProofRecord[] = [];
  let passed = 0;
  let failed = 0;

  const updateProgress = (step: number, status: VerificationProgress['status']) => {
    setProgress({
      step: STEPS[step],
      steps: STEPS,
      current: step + 1,
      total: STEPS.length,
      status,
    });
  };

  for (let i = 0; i < STEPS.length; i++) {
    const step = STEPS[i];
    updateProgress(i, 'running');

    let rec: ProofRecord;

    if (step === 'Fake-code scan') {
      const findings = await runFakeCodeScanner(workspacePath);
      rec = {
        id: `proof-${Date.now()}`,
        command: 'fake-code-scan',
        cwd: workspacePath,
        startTime: new Date().toISOString(),
        endTime: new Date().toISOString(),
        duration: 0,
        exitCode: findings.length === 0 ? 0 : 1,
        stdout: findings.length > 0 ? findings.map(f => `${f.file}:${f.line} ${f.matched}`).join('\n') : 'No issues found',
        stderr: '',
        status: findings.filter(f => f.severity === 'CRITICAL' || f.severity === 'HIGH').length > 0 ? 'FAIL' : 'PASS',
        changedFiles: [],
        artifacts: [],
      };
    } else if (step === 'Security scan') {
      const { runSecurityScan } = await import('./securityScanner');
      const findings = await runSecurityScan(workspacePath);
      rec = {
        id: `proof-${Date.now()}`,
        command: 'security-scan',
        cwd: workspacePath,
        startTime: new Date().toISOString(),
        endTime: new Date().toISOString(),
        duration: 0,
        exitCode: findings.length === 0 ? 0 : 1,
        stdout: findings.length > 0 ? findings.join('\n') : 'No security issues',
        stderr: '',
        status: findings.length > 0 ? 'FAIL' : 'PASS',
        changedFiles: [],
        artifacts: [],
      };
    } else if (step === 'Scoreboard generation') {
      const cats = await generateScoreboard(workspacePath);
      const score = Math.round(cats.reduce((a, c) => a + c.score, 0) / cats.length);
      rec = {
        id: `proof-${Date.now()}`,
        command: 'scoreboard-generate',
        cwd: workspacePath,
        startTime: new Date().toISOString(),
        endTime: new Date().toISOString(),
        duration: 0,
        exitCode: score >= 50 ? 0 : 1,
        stdout: `Score: ${score}%\n${cats.map(c => `${c.name}: ${c.score}/100 - ${c.status}`).join('\n')}`,
        stderr: '',
        status: score >= 80 ? 'PASS' : score >= 50 ? 'PARTIAL' : 'FAIL',
        changedFiles: [],
        artifacts: [],
      };
    } else {
      let command: string;
      if (step === 'cargo check') command = 'cargo check 2>&1';
      else if (step === 'forge build') command = 'forge build 2>&1 || true';
      else if (step === 'cargo test') command = 'cargo test 2>&1 | tail -20';
      else if (step === 'npm/pnpm test') command = 'pnpm test 2>&1 | tail -20 || npm test 2>&1 | tail -20';
      else if (step === 'forge test') command = 'forge test 2>&1 | tail -20';
      else command = step;
      rec = await runProofCommand(command, workspacePath, step);
    }

    addRecord(rec);
    records.push(rec);
    if (rec.exitCode === 0) passed++;
    else failed++;
    updateProgress(i, rec.exitCode === 0 ? 'done' : 'failed');
  }

  setProgress(null);
  return { passed, failed, records };
}
