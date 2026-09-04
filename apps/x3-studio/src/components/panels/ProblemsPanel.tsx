import { useScannerStore, useDiagnosticsStore } from '../../store';

export default function ProblemsPanel() {
  const findings = useScannerStore(s => s.findings);
  const diagnostics = useDiagnosticsStore(s => s.entries);

  const allProblems = [
    ...diagnostics.map(d => ({
      file: d.file,
      line: d.line,
      message: d.message,
      severity: d.severity === 'error' ? 'ERROR' as const : d.severity === 'warning' ? 'WARNING' as const : 'INFO' as const,
      source: d.source,
    })),
    ...findings.map(f => ({
      file: f.file,
      line: f.line,
      message: `${f.matched} — ${f.reason}`,
      severity: f.severity as 'ERROR' | 'WARNING' | 'INFO' | 'CRITICAL',
      source: 'scanner' as const,
    })),
  ];

  const critical = allProblems.filter(p => p.severity === 'CRITICAL' || p.severity === 'ERROR').length;
  const warnings = allProblems.filter(p => p.severity === 'WARNING').length;
  const infos = allProblems.filter(p => p.severity === 'INFO').length;

  return (
    <div style={{ fontSize: 'var(--font-size-sm)', padding: 8, height: '100%', overflow: 'auto' }}>
      <div style={{ display: 'flex', gap: 12, marginBottom: 8, color: 'var(--text-secondary)' }}>
        <span>{allProblems.length} problems</span>
        {critical > 0 && <span style={{ color: 'var(--red)' }}>{critical} errors</span>}
        {warnings > 0 && <span style={{ color: 'var(--yellow)' }}>{warnings} warnings</span>}
        {infos > 0 && <span style={{ color: 'var(--text-muted)' }}>{infos} infos</span>}
      </div>
      {allProblems.length === 0 && (
        <div style={{ color: 'var(--text-muted)', padding: 16, textAlign: 'center' }}>
          No problems detected. Run build, tests, or scanner to check for issues.
        </div>
      )}
      {allProblems.slice(0, 100).map((p, i) => (
        <div key={i} className="tree-node" style={{ fontSize: 'var(--font-size-sm)' }}>
          <span style={{
            color: p.severity === 'CRITICAL' ? 'var(--red)' : p.severity === 'ERROR' ? 'var(--red)' : p.severity === 'WARNING' ? 'var(--orange)' : 'var(--yellow)',
            marginRight: 4,
          }}>
            {p.severity === 'CRITICAL' ? '●' : p.severity === 'ERROR' ? '●' : p.severity === 'WARNING' ? '◉' : '○'}
          </span>
          <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--text-secondary)' }}>{p.file}:{p.line}</span>
          <span style={{ color: 'var(--text-muted)', marginLeft: 4 }}>— {p.message}</span>
        </div>
      ))}
    </div>
  );
}
