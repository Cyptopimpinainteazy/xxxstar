import { describe, it, expect } from 'vitest';
import { generateProofReport } from '../src/services/proofGenerator';

describe('Proof Report Generator', () => {
  it('generates markdown report', () => {
    const report = generateProofReport({
      id: 'test',
      command: 'echo hello',
      cwd: '/tmp',
      startTime: new Date().toISOString(),
      endTime: new Date().toISOString(),
      duration: 100,
      exitCode: 0,
      stdout: 'hello',
      stderr: '',
      status: 'PASS',
      changedFiles: [],
      artifacts: [],
    });
    expect(report).toContain('# Proof Report');
    expect(report).toContain('**PASS**');
  });
});
