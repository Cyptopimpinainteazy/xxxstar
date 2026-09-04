import type { ScannerFinding } from '../types';

const FAKE_PATTERNS = [
  { pattern: 'TODO', severity: 'INFO' as const, reason: 'Incomplete code marker', fix: 'Implement the TODO or remove it' },
  { pattern: 'FIXME', severity: 'INFO' as const, reason: 'Known issue marker', fix: 'Fix the issue or remove the marker' },
  { pattern: 'HACK', severity: 'WARNING' as const, reason: 'Non-ideal workaround', fix: 'Refactor to a proper solution' },
  { pattern: 'placeholder', severity: 'WARNING' as const, reason: 'Placeholder implementation', fix: 'Replace with real implementation' },
  { pattern: 'stub', severity: 'WARNING' as const, reason: 'Stub implementation', fix: 'Replace with real implementation' },
  { pattern: 'mock', severity: 'WARNING' as const, reason: 'Mock implementation', fix: 'Replace with real implementation' },
  { pattern: 'fake', severity: 'HIGH' as const, reason: 'Fake/simulated code', fix: 'Replace with real implementation' },
  { pattern: 'not implemented', severity: 'HIGH' as const, reason: 'Feature not implemented', fix: 'Implement the feature' },
  { pattern: 'coming soon', severity: 'WARNING' as const, reason: 'Unfinished feature', fix: 'Finish the implementation' },
  { pattern: 'return true', severity: 'HIGH' as const, reason: 'Hardcoded success return', fix: 'Implement real logic' },
  { pattern: 'return false', severity: 'HIGH' as const, reason: 'Hardcoded failure return', fix: 'Implement real logic' },
  { pattern: 'noop', severity: 'WARNING' as const, reason: 'No-operation code', fix: 'Implement the function' },
  { pattern: 'skip validation', severity: 'CRITICAL' as const, reason: 'Validation bypassed', fix: 'Restore validation checks' },
  { pattern: 'ignore error', severity: 'CRITICAL' as const, reason: 'Error being ignored', fix: 'Handle the error properly' },
  { pattern: 'console.log\("success"', severity: 'WARNING' as const, reason: 'Fake success logging', fix: 'Replace with real verification' },
  { pattern: 'todo!', severity: 'INFO' as const, reason: 'Rust unimplemented macro', fix: 'Implement the function' },
  { pattern: 'unimplemented!', severity: 'HIGH' as const, reason: 'Rust unimplemented macro', fix: 'Implement the function' },
  { pattern: 'unreachable!', severity: 'WARNING' as const, reason: 'Rust unreachable assertion', fix: 'Verify control flow' },
  { pattern: 'panic!', severity: 'WARNING' as const, reason: 'Rust panic in production path', fix: 'Use Result instead' },
  { pattern: 'hardcoded', severity: 'WARNING' as const, reason: 'Hardcoded value', fix: 'Use configuration instead' },
  { pattern: 'dummy', severity: 'INFO' as const, reason: 'Dummy/test data', fix: 'Replace with real data' },
  { pattern: 'tx\\.origin', severity: 'CRITICAL' as const, reason: 'Vulnerable to phishing', fix: 'Use msg.sender instead' },
];

const CRITICAL_IMPORTS = [
  { pattern: 'fs\\.readFileSync|fs\\.writeFileSync', severity: 'WARNING' as const, reason: 'Unrestricted file access', fix: 'Validate file paths' },
  { pattern: 'child_process\\.exec|child_process\\.spawn', severity: 'WARNING' as const, reason: 'Shell command execution', fix: 'Validate/sanitize input' },
  { pattern: 'eval\\(', severity: 'CRITICAL' as const, reason: 'Arbitrary code execution', fix: 'Avoid eval entirely' },
];

export async function runFakeCodeScanner(workspacePath: string): Promise<ScannerFinding[]> {
  const allPatterns = [...FAKE_PATTERNS, ...CRITICAL_IMPORTS];
  const findings: ScannerFinding[] = [];

  const results = await window.x3studio.scanner.scanFiles(workspacePath, allPatterns.map(p => p.pattern));

  for (const r of results) {
    const matched = allPatterns.find(p => p.pattern.toLowerCase() === r.matched.toLowerCase() ||
      r.matched.toLowerCase().includes(p.pattern.toLowerCase()));
    if (matched) {
      findings.push({
        file: r.file,
        line: r.line,
        matched: r.matched,
        severity: matched.severity,
        reason: matched.reason,
        suggestedFix: matched.fix,
      });
    }
  }

  return findings;
}
