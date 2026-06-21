import { useState } from 'react';
import { useWorkspaceStore, useEditorStore, useDiagnosticsStore } from '../../store';

export default function TestRunnerPanel() {
  const wp = useWorkspaceStore(s => s.workspacePath);
  const openFile = useEditorStore(s => s.openFile);
  const addDiagnostics = useDiagnosticsStore(s => s.addEntries);
  const [results, setResults] = useState<{ name: string; status: string; duration: string; output: string }[]>([]);
  const [command, setCommand] = useState('cargo test');
  const [running, setRunning] = useState(false);
  const [testFilter, setTestFilter] = useState('');

  const runTests = async () => {
    if (!wp) return;
    setRunning(true);
    const fullCmd = testFilter ? `${command} ${testFilter}` : command;
    try {
      const start = Date.now();
      const res = await window.x3studio.shell.exec(fullCmd + ' 2>&1', wp);
      const duration = ((Date.now() - start) / 1000).toFixed(1);
      const lines = res.stdout.split('\n');

      const tests: { name: string; status: string; duration: string; output: string }[] = [];
      for (const line of lines) {
        if (line.includes('test ') && (line.includes('... ok') || line.includes('... FAILED') || line.includes('PASS') || line.includes('FAIL'))) {
          const status = line.includes('FAIL') || line.includes('FAILED') ? 'FAIL' : 'PASS';
          const match = line.match(/test\s+([^\s]+)/);
          const name = match ? match[1] : line.trim();
          tests.push({ name, status, duration, output: line });
        }
      }
      if (tests.length === 0) {
        const passCount = (res.stdout.match(/ok/g) || []).length;
        const failCount = (res.stdout.match(/FAILED/g) || []).length;
        tests.push({ name: `Test Suite (${passCount} passed, ${failCount} failed)`, status: failCount > 0 ? 'FAIL' : 'PASS', duration, output: res.stdout.substring(0, 500) });
      }

      setResults(prev => [...tests, ...prev].slice(0, 100));

      if (res.exitCode !== 0) {
        addDiagnostics([{ file: '', line: 1, column: 1, message: `Tests failed: ${res.stderr.substring(0, 200)}`, severity: 'error', source: 'cargo' }]);
      }
    } catch (e: any) {
      setResults(prev => [{ name: 'Error', status: 'FAIL', duration: '0', output: e.message }, ...prev].slice(0, 100));
    }
    setRunning(false);
  };

  const runSingleTest = async (testName: string) => {
    setTestFilter(testName);
    setCommand('cargo test');
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">Test Runner</div>
      <div style={{ padding: '4px 8px', display: 'flex', gap: 4, borderBottom: '1px solid var(--border-color)' }}>
        <select className="select-field" style={{ width: 'auto', fontSize: 10 }} value={command} onChange={e => setCommand(e.target.value)}>
          <option value="cargo test">cargo test</option>
          <option value="cargo test --workspace">cargo test --workspace</option>
          <option value="forge test">forge test</option>
          <option value="forge test -vvv">forge test -vvv</option>
          <option value="pnpm test">pnpm test</option>
          <option value="npm test">npm test</option>
          <option value="cargo clippy">cargo clippy</option>
        </select>
        <input className="input-field" style={{ flex: 1, fontSize: 10 }} value={testFilter} onChange={e => setTestFilter(e.target.value)} placeholder="Test filter (optional)" />
        <button className="btn btn-primary" onClick={runTests} disabled={running || !wp}>{running ? 'Running...' : '▶ Run'}</button>
      </div>

      <div style={{ overflow: 'auto', flex: 1 }}>
        <table className="data-table" style={{ fontSize: 10 }}>
          <thead><tr><th>Status</th><th>Test</th><th>Duration</th></tr></thead>
          <tbody>
            {results.map((r, i) => (
              <tr key={i}>
                <td><span className={`badge badge-${r.status === 'PASS' ? 'pass' : 'fail'}`} style={{ fontSize: 9 }}>{r.status}</span></td>
                <td style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>
                  <span style={{ cursor: 'pointer' }} onClick={() => runSingleTest(r.name)} title="Run this test only">{r.name}</span>
                </td>
                <td style={{ fontSize: 10, color: 'var(--text-muted)' }}>{r.duration}s</td>
              </tr>
            ))}
            {results.length === 0 && (
              <tr><td colSpan={3} style={{ textAlign: 'center', color: 'var(--text-muted)', padding: 16 }}>No tests run. Click Run to start.</td></tr>
            )}
          </tbody>
        </table>
      </div>

      {results.length > 0 && (
        <div style={{ maxHeight: 120, overflow: 'auto', borderTop: '1px solid var(--border-color)' }}>
          <div className="section-title" style={{ padding: '4px 8px', fontSize: 10 }}>Output</div>
          <pre style={{ padding: 8, fontSize: 10, whiteSpace: 'pre-wrap' }}>
            {results[0]?.output || ''}
          </pre>
        </div>
      )}
    </div>
  );
}
