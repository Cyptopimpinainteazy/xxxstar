const SECURITY_PATTERNS = [
  { pattern: 'PRIVATE_KEY|private_key|SECRET|secret_key', severity: 'CRITICAL', label: 'Exposed Secret Key' },
  { pattern: 'seed.?phrase|mnemonic', severity: 'CRITICAL', label: 'Seed Phrase / Mnemonic' },
  { pattern: '0x[a-fA-F0-9]{64}', severity: 'HIGH', label: 'Possible Private Key Hex' },
  { pattern: 'api.?key|API_KEY', severity: 'HIGH', label: 'API Key / Token' },
  { pattern: 'password.?=|PASSWORD', severity: 'CRITICAL', label: 'Hardcoded Password' },
  { pattern: 'unsafe.*permit|permit.*all', severity: 'HIGH', label: 'Overly permissive config' },
];

export async function runSecurityScan(workspacePath: string): Promise<any[]> {
  const allFindings: any[] = [];
  for (const sp of SECURITY_PATTERNS) {
    const r = await window.x3studio.scanner.scanFiles(workspacePath, [sp.pattern]);
    r.forEach(f => {
      allFindings.push({ ...f, severity: sp.severity, label: sp.label });
    });
  }
  return allFindings;
}
