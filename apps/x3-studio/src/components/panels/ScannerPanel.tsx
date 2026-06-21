import { useScannerStore, useWorkspaceStore } from '../../store';
import { runFakeCodeScanner } from '../../services/fakeCodeScanner';

export default function ScannerPanel() {
  const findings = useScannerStore(s => s.findings);
  const isScanning = useScannerStore(s => s.isScanning);
  const setFindings = useScannerStore(s => s.setFindings);
  const setScanning = useScannerStore(s => s.setScanning);
  const workspacePath = useWorkspaceStore(s => s.workspacePath);

  const runScan = async () => {
    if (!workspacePath) return;
    setScanning(true);
    const results = await runFakeCodeScanner(workspacePath);
    setFindings(results);
    setScanning(false);
  };

  const severityColor = (s: string) =>
    s === 'CRITICAL' ? 'var(--red)' : s === 'HIGH' ? 'var(--orange)' : s === 'WARNING' ? 'var(--yellow)' : 'var(--text-muted)';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Fake-Code Scanner</span>
        <button className="btn btn-primary" onClick={runScan} disabled={isScanning} style={{ fontSize: 10, padding: '2px 8px' }}>
          {isScanning ? 'Scanning...' : 'Scan'}
        </button>
      </div>
      <div className="panel-body">
        <div style={{ marginBottom: 8, fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>
          {findings.length > 0
            ? `Found ${findings.length} potential issues`
            : 'No issues found. Run a scan to check for fake/stub/placeholder code.'}
        </div>

        {findings.map((f, i) => (
          <div key={i} style={{
            background: 'var(--bg-surface)', border: `1px solid ${severityColor(f.severity)}40`,
            borderRadius: 'var(--radius)', padding: '6px 8px', marginBottom: 4,
            borderLeft: `3px solid ${severityColor(f.severity)}`,
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: 'var(--font-size-sm)' }}>
              <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--text-primary)' }}>
                {f.file}:{f.line}
              </span>
              <span className={`badge badge-${f.severity === 'CRITICAL' ? 'fail' : f.severity === 'HIGH' ? 'blocked' : f.severity === 'WARNING' ? 'partial' : 'info'}`}
                style={{ fontSize: 10 }}>{f.severity}</span>
            </div>
            <div style={{ fontSize: 'var(--font-size-sm)', marginTop: 2 }}>
              <span style={{ color: severityColor(f.severity) }}>{f.matched}</span>
              <span style={{ color: 'var(--text-muted)' }}> — {f.reason}</span>
            </div>
            <div style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-secondary)', marginTop: 2 }}>
              Fix: {f.suggestedFix}
            </div>
          </div>
        ))}

        {findings.length === 0 && !isScanning && (
          <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 16, textAlign: 'center' }}>
            Click "Scan" to scan workspace for fake/stub/placeholder patterns.
          </div>
        )}
      </div>
    </div>
  );
}
