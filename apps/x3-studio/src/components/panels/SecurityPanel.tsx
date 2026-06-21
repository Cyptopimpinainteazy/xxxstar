import { useState } from 'react';
import { useWorkspaceStore } from '../../store';

const SECURITY_PATTERNS = [
  { pattern: 'PRIVATE_KEY|private_key|SECRET|secret_key', severity: 'CRITICAL', label: 'Exposed Secret Key' },
  { pattern: 'seed.?phrase|mnemonic', severity: 'CRITICAL', label: 'Seed Phrase / Mnemonic' },
  { pattern: '0x[a-fA-F0-9]{64}', severity: 'HIGH', label: 'Possible Private Key Hex' },
  { pattern: 'api.?key|API_KEY', severity: 'HIGH', label: 'API Key / Token' },
  { pattern: 'password.?=|PASSWORD', severity: 'CRITICAL', label: 'Hardcoded Password' },
  { pattern: '.env', severity: 'WARNING', label: '.env file tracked' },
  { pattern: 'unsafe.*permit|permit.*all', severity: 'HIGH', label: 'Overly permissive config' },
];

export default function SecurityPanel() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const [results, setResults] = useState<any[]>([]);
  const [isScanning, setIsScanning] = useState(false);

  const runSecurityScan = async () => {
    if (!workspacePath) return;
    setIsScanning(true);
    try {
      const allFindings: any[] = [];
      for (const sp of SECURITY_PATTERNS) {
        const r = await window.x3studio.scanner.scanFiles(workspacePath, [sp.pattern]);
        r.forEach(f => {
          allFindings.push({ ...f, severity: sp.severity, label: sp.label });
        });
      }
      setResults(allFindings);
    } catch {}
    setIsScanning(false);
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Security Scanner</span>
        <button className="btn btn-danger" onClick={runSecurityScan} disabled={isScanning}>
          {isScanning ? 'Scanning...' : 'Scan Secrets'}
        </button>
      </div>
      <div className="panel-body">
        <div style={{ marginBottom: 8, fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>
          Scans for exposed secrets, private keys, API tokens, and insecure patterns.
        </div>
        {results.map((r, i) => (
          <div key={i} style={{
            background: 'var(--bg-surface)', border: '1px solid var(--red)',
            borderRadius: 'var(--radius)', padding: '6px 8px', marginBottom: 4,
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 'var(--font-size-sm)' }}>
              <span style={{ fontFamily: 'var(--font-mono)' }}>{r.file}:{r.line}</span>
              <span className="badge badge-fail" style={{ fontSize: 10 }}>{r.severity}</span>
            </div>
            <div style={{ fontSize: 'var(--font-size-sm)', color: 'var(--red)', marginTop: 2 }}>
              {r.label}: "{r.content}"
            </div>
          </div>
        ))}
        {results.length === 0 && !isScanning && (
          <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 16, textAlign: 'center' }}>
            Click "Scan Secrets" to check for exposed credentials.
          </div>
        )}
      </div>
    </div>
  );
}
