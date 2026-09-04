import { useState } from 'react';
import { useForgeCoverageStore, useWorkspaceStore } from '../../store';

export default function ForgeCoveragePanel() {
  const wp = useWorkspaceStore(s => s.workspacePath);
  const result = useForgeCoverageStore(s => s.result);
  const setResult = useForgeCoverageStore(s => s.setResult);
  const [output, setOutput] = useState('');
  const [forgePath, setForgePath] = useState('forge');

  const runCoverage = async () => {
    if (!wp) return;
    setOutput('Running forge coverage...');
    setResult(null);
    try {
      const res = await window.x3studio.shell.exec(`${forgePath} coverage --report lcov 2>&1`, wp);
      setOutput(res.stdout + '\n' + res.stderr);
      if (res.exitCode === 0) {
        const lines = res.stdout.split('\n');
        const files: { file: string; pct: number }[] = [];
        let totalLines = 0, coveredLines = 0;
        for (const line of lines) {
          if (line.includes('|') && line.includes('%')) {
            const parts = line.split('|').map(s => s.trim());
            if (parts.length >= 4) {
              const pct = parseFloat(parts[3].replace('%', ''));
              if (!isNaN(pct) && parts[1] !== '-' && parts[1] !== 'Total') {
                files.push({ file: parts[1] || 'unknown', pct });
              }
              const l = parseInt(parts[1]);
              const c = parseInt(parts[2]);
              if (!isNaN(l)) totalLines += l;
              if (!isNaN(c)) coveredLines += c;
            }
          }
        }
        setResult({
          lines: { total: totalLines, covered: coveredLines, pct: totalLines > 0 ? Math.round(coveredLines / totalLines * 100) : 0 },
          branches: { total: 0, covered: 0, pct: 0 },
          functions: { total: 0, covered: 0, pct: 0 },
          files,
        });
      }
    } catch (e: any) { setOutput('Error: ' + e.message); }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: 8 }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>Forge Coverage</div>
      <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
        <input className="input-field" style={{ width: 80, fontSize: 10 }} value={forgePath} onChange={e => setForgePath(e.target.value)} placeholder="forge path" />
        <button className="btn btn-primary" onClick={runCoverage} disabled={!wp}>Run Coverage</button>
      </div>

      {result && (
        <>
          <div className="section-title">Summary</div>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 8, marginBottom: 8 }}>
            {[
              { label: 'Lines', value: `${result.lines.covered}/${result.lines.total}`, pct: result.lines.pct },
              { label: 'Branches', value: `${result.branches.covered}/${result.branches.total}`, pct: result.branches.pct },
              { label: 'Functions', value: `${result.functions.covered}/${result.functions.total}`, pct: result.functions.pct },
            ].map(s => (
              <div key={s.label} style={{ background: 'var(--bg-surface)', borderRadius: 'var(--radius)', padding: 12, textAlign: 'center' }}>
                <div style={{ fontSize: 24, fontWeight: 700, color: s.pct >= 80 ? 'var(--pass-color)' : s.pct >= 50 ? 'var(--warn-color)' : 'var(--fail-color)' }}>{s.pct}%</div>
                <div style={{ fontSize: 11, color: 'var(--text-muted)' }}>{s.label}</div>
                <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>{s.value}</div>
              </div>
            ))}
          </div>

          <div className="section-title">By File ({result.files.length})</div>
          <table className="data-table" style={{ fontSize: 10 }}>
            <thead><tr><th>File</th><th>Coverage</th></tr></thead>
            <tbody>
              {result.files.slice(0, 50).map(f => (
                <tr key={f.file}>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{f.file.substring(0, 50)}</td>
                  <td>
                    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
                      <div style={{ flex: 1, height: 12, background: 'var(--border-color)', borderRadius: 6, overflow: 'hidden' }}>
                        <div style={{ width: `${f.pct}%`, height: '100%', background: f.pct >= 80 ? 'var(--pass-color)' : f.pct >= 50 ? 'var(--warn-color)' : 'var(--fail-color)' }} />
                      </div>
                      <span style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{f.pct}%</span>
                    </div>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}

      {output && (
        <>
          <div className="section-title">Output</div>
          <pre style={{ background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)', fontSize: 10, maxHeight: 150, overflow: 'auto', whiteSpace: 'pre-wrap' }}>{output}</pre>
        </>
      )}
    </div>
  );
}
