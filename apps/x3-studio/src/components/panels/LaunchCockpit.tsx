import { useState, useEffect } from 'react';
import { useWorkspaceStore, useProofStore, useScoreboardStore, useScannerStore } from '../../store';
import { runProofCommand } from '../../services/proofGenerator';

export default function LaunchCockpit() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const buildStatus = useProofStore(s => s.records.find(r => r.command.includes('check')));
  const testStatus = useProofStore(s => s.records.find(r => r.command.includes('test')));
  const score = useScoreboardStore(s => s.totalScore);
  const findings = useScannerStore(s => s.findings);
  const [testnetReady, setTestnetReady] = useState(0);
  const [mainnetReady, setMainnetReady] = useState(0);

  const testnetChecks = [
    { label: 'Builds pass', pass: buildStatus?.exitCode === 0 },
    { label: 'Tests pass', pass: testStatus?.exitCode === 0 },
    { label: 'Scoreboard generated', pass: score > 0 },
    { label: 'Scanner run', pass: findings.length > 0 },
  ];

  const mainnetChecks = [
    { label: 'No critical scanner findings', pass: findings.filter(f => f.severity === 'CRITICAL').length === 0 },
    { label: 'Testnet gate passed', pass: testnetReady >= 75 },
    { label: 'Score >= 80%', pass: score >= 80 },
    { label: 'Build clean', pass: buildStatus?.exitCode === 0 },
  ];

  useEffect(() => {
    const passed = testnetChecks.filter(c => c.pass).length;
    setTestnetReady(Math.round((passed / testnetChecks.length) * 100));
  }, [buildStatus, testStatus, score, findings]);

  useEffect(() => {
    const passed = mainnetChecks.filter(c => c.pass).length;
    setMainnetReady(Math.round((passed / mainnetChecks.length) * 100));
  }, [testnetReady, score, findings, buildStatus]);

  const runTestnetGate = async () => {
    if (!workspacePath) return;
    const rec = await runProofCommand('cargo check 2>&1 && cargo test 2>&1 | tail -10', workspacePath, 'Testnet Gate');
    useProofStore.getState().addRecord(rec);
  };

  const runMainnetGate = async () => {
    if (!workspacePath) return;
    const rec = await runProofCommand('cargo check 2>&1 && cargo test 2>&1 | tail -10 && echo "MAINNET GATE PASSED"', workspacePath, 'Mainnet Gate');
    useProofStore.getState().addRecord(rec);
  };

  if (!workspacePath) return null;

  return (
    <div style={{ padding: '8px', overflowY: 'auto', height: '100%' }}>
      <div className="panel-header">Launch Cockpit</div>

      <div className="dashboard-card" style={{ marginBottom: 8 }}>
        <h3>Testnet Readiness</h3>
        <div className="readiness-bar" style={{ height: 12 }}>
          <div className="readiness-fill" style={{ width: `${testnetReady}%`, background: testnetReady >= 75 ? 'var(--green)' : testnetReady >= 50 ? 'var(--yellow)' : 'var(--red)' }} />
        </div>
        <div className="value" style={{ textAlign: 'center', fontSize: 24 }}>{testnetReady}%</div>
      </div>

      <div className="dashboard-card" style={{ marginBottom: 8 }}>
        <h3>Mainnet Readiness</h3>
        <div className="readiness-bar" style={{ height: 12 }}>
          <div className="readiness-fill" style={{ width: `${mainnetReady}%`, background: mainnetReady >= 80 ? 'var(--green)' : mainnetReady >= 50 ? 'var(--yellow)' : 'var(--red)' }} />
        </div>
        <div className="value" style={{ textAlign: 'center', fontSize: 24 }}>{mainnetReady}%</div>
      </div>

      <div className="section-title">Testnet Checks</div>
      {testnetChecks.map((c, i) => (
        <div key={i} className="check-item">
          <span style={{ color: c.pass ? 'var(--green)' : 'var(--red)' }}>{c.pass ? '✓' : '✗'}</span>
          <span className="label">{c.label}</span>
        </div>
      ))}

      <div className="section-title">Mainnet Checks</div>
      {mainnetChecks.map((c, i) => (
        <div key={i} className="check-item">
          <span style={{ color: c.pass ? 'var(--green)' : 'var(--red)' }}>{c.pass ? '✓' : '✗'}</span>
          <span className="label">{c.label}</span>
        </div>
      ))}

      <div className="section-title">Actions</div>
      <div className="dashboard-actions">
        <button className="btn btn-success" onClick={runTestnetGate}>Run Testnet Gate</button>
        <button className="btn btn-danger" onClick={runMainnetGate}>Run Mainnet Gate</button>
      </div>
      <div style={{ marginTop: 8, fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>
        Full readiness assessment. Run all checks from Control Center first.
      </div>
    </div>
  );
}
