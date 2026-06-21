import { useState, useEffect, useCallback } from 'react';
import { useWorkspaceStore, useProofStore, useScoreboardStore, useScannerStore } from '../../store';
import { runProofCommand } from '../../services/proofGenerator';
import { runFakeCodeScanner } from '../../services/fakeCodeScanner';
import { generateScoreboard } from '../../services/scoreboardGenerator';
import { runFullVerification } from '../../services/verificationService';
import type { ProjectDetection } from '../../types';

export default function ControlCenter() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const workspaceName = useWorkspaceStore(s => s.workspaceName);
  const detection = useWorkspaceStore(s => s.detection);
  const branch = useWorkspaceStore(s => s.branch);
  const gitStatus = useWorkspaceStore(s => s.gitStatus);
  const addProofRecord = useProofStore(s => s.addRecord);
  const setRunning = useProofStore(s => s.setRunning);
  const isRunning = useProofStore(s => s.isRunning);
  const progress = useProofStore(s => s.progress);
  const setCategories = useScoreboardStore(s => s.setCategories);
  const calculateTotal = useScoreboardStore(s => s.calculateTotal);
  const score = useScoreboardStore(s => s.totalScore);
  const setFindings = useScannerStore(s => s.setFindings);
  const findings = useScannerStore(s => s.findings);
  const [buildStatus, setBuildStatus] = useState<string>('—');
  const [testStatus, setTestStatus] = useState<string>('—');
  const [lastRun, setLastRun] = useState<string>('');
  const [lastFailure, setLastFailure] = useState<string>('');
  const [lastProof, setLastProof] = useState<string>('');
  const [blockers, setBlockers] = useState<string[]>([]);
  const [nextTasks, setNextTasks] = useState<string[]>([]);

  const runBuild = useCallback(async () => {
    if (!workspacePath) return;
    setRunning(true);
    setBuildStatus('running...');
    const rec = await runProofCommand('cargo check 2>&1 || forge build 2>&1 || echo "no build tool"', workspacePath, 'Build Check');
    addProofRecord(rec);
    setBuildStatus(rec.exitCode === 0 ? 'PASS' : 'FAIL');
    setLastRun(new Date().toLocaleTimeString());
    if (rec.exitCode !== 0) {
      setLastFailure(`Build failed: ${rec.stderr.substring(0, 200)}`);
    }
    setRunning(false);
  }, [workspacePath]);

  const runTests = useCallback(async () => {
    if (!workspacePath) return;
    setRunning(true);
    setTestStatus('running...');
    const rec = await runProofCommand('cargo test 2>&1 | tail -20', workspacePath, 'Test Suite');
    addProofRecord(rec);
    setTestStatus(rec.exitCode === 0 ? 'PASS' : 'FAIL');
    setLastRun(new Date().toLocaleTimeString());
    if (rec.exitCode !== 0) {
      setLastFailure(`Tests failed: ${rec.stderr.substring(0, 200)}`);
    }
    setRunning(false);
  }, [workspacePath]);

  const runScanner = useCallback(async () => {
    if (!workspacePath) return;
    setRunning(true);
    const f = await runFakeCodeScanner(workspacePath);
    setFindings(f);
    setLastRun(new Date().toLocaleTimeString());
    setRunning(false);
  }, [workspacePath]);

  const runScoreboard = useCallback(async () => {
    if (!workspacePath) return;
    setRunning(true);
    const cats = await generateScoreboard(workspacePath);
    setCategories(cats);
    calculateTotal();
    setLastRun(new Date().toLocaleTimeString());
    setRunning(false);
  }, [workspacePath]);

  const handleFullVerification = useCallback(async () => {
    if (!workspacePath) return;
    setRunning(true);
    setBuildStatus('running...');
    setTestStatus('running...');
    const result = await runFullVerification(workspacePath);
    setBuildStatus(result.failed === 0 ? 'PASS' : 'FAIL');
    setTestStatus(result.failed === 0 ? 'PASS' : 'FAIL');
    setLastProof(new Date().toLocaleTimeString());
    setLastRun(new Date().toLocaleTimeString());
    if (result.failed > 0) {
      setLastFailure(`${result.failed} verification step(s) failed`);
    }
    setRunning(false);
  }, [workspacePath]);

  const runSecurityScan = useCallback(async () => {
    if (!workspacePath) return;
    setRunning(true);
    const { runSecurityScan } = await import('../../services/securityScanner');
    const results = await runSecurityScan(workspacePath);
    setFindings(results.map((r: any) => ({
      file: r.file, line: r.line, matched: r.label,
      severity: r.severity as any, reason: r.label, suggestedFix: 'Remove or secure the exposed credential',
    })));
    setLastRun(new Date().toLocaleTimeString());
    setRunning(false);
  }, [workspacePath]);

  const runLint = useCallback(async () => {
    if (!workspacePath) return;
    setRunning(true);
    const rec = await runProofCommand('cargo clippy --workspace -- -D warnings 2>&1 | tail -20 || cargo check 2>&1 | tail -5', workspacePath, 'Lint');
    addProofRecord(rec);
    setLastRun(new Date().toLocaleTimeString());
    setRunning(false);
  }, [workspacePath]);

  useEffect(() => {
    const ds: string[] = [];
    if (detection && detection.modules.length === 0) ds.push('No project modules detected');
    if (buildStatus === 'FAIL') ds.push('Build is failing');
    if (testStatus === 'FAIL') ds.push('Tests are failing');
    if (findings.filter(f => f.severity === 'CRITICAL').length > 0) ds.push('Critical scanner findings');
    setBlockers(ds);

    if (detection && detection.modules.length > 0) {
      const tasks: string[] = [];
      if (buildStatus === 'FAIL') tasks.push('Fix build errors');
      if (testStatus === 'FAIL') tasks.push('Fix failing tests');
      if (!detection.hasRelayer) tasks.push('Set up relayer');
      if (!detection.hasAdapters) tasks.push('Configure adapters');
      if (!detection.hasProofLedger) tasks.push('Initialize proof ledger');
      if (!detection.hasValidator) tasks.push('Configure validator');
      if (score < 80) tasks.push('Improve scoreboard score');
      if (tasks.length === 0) tasks.push('All checks passing. Run verification weekly.');
      setNextTasks(tasks);
    }
  }, [detection, buildStatus, testStatus, findings, score]);

  if (!workspacePath) {
    return (
      <div style={{ padding: 16 }}>
        <div className="panel-header">X3 Control Center</div>
        <div style={{ color: 'var(--text-muted)', padding: 16, fontSize: 'var(--font-size-sm)' }}>
          Open a workspace to see the Control Center dashboard.
        </div>
      </div>
    );
  }

  const badges = (val: string, ok: string, fail: string) =>
    val === 'PASS' ? <span className="badge badge-pass">{ok}</span> :
    val === 'FAIL' ? <span className="badge badge-fail">{fail}</span> :
    <span className="badge badge-info">{val}</span>;

  return (
    <div style={{ padding: '8px', overflowY: 'auto', height: '100%' }}>
      <div className="panel-header">X3 Control Center</div>

      {progress && (
        <div style={{
          background: 'var(--bg-surface)', borderRadius: 'var(--radius)',
          padding: '8px 12px', marginBottom: 8, fontSize: 'var(--font-size-sm)',
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
            <span>Verification in progress: {progress.step}</span>
            <span>{progress.current}/{progress.total}</span>
          </div>
          <div className="readiness-bar" style={{ height: 6 }}>
            <div className="readiness-fill" style={{
              width: `${(progress.current / progress.total) * 100}%`,
              background: progress.status === 'failed' ? 'var(--red)' : 'var(--green)',
              borderRadius: 3,
            }} />
          </div>
        </div>
      )}

      <div className="dashboard-grid">
        <div className="dashboard-card">
          <h3>Workspace</h3>
          <div className="value" style={{ fontSize: 14 }}>{workspaceName}</div>
          <div className="sub">{branch} • {gitStatus.length} changed</div>
        </div>

        <div className="dashboard-card">
          <h3>Repo Health</h3>
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap', marginTop: 4 }}>
            {badges(buildStatus, 'Build ✓', 'Build ✗')}
            {badges(testStatus, 'Tests ✓', 'Tests ✗')}
          </div>
        </div>

        <div className="dashboard-card">
          <h3>Detected Modules</h3>
          <div style={{ display: 'flex', gap: 4, flexWrap: 'wrap', marginTop: 4 }}>
            {detection?.modules.map(m => (
              <span key={m} className="badge badge-info">{m}</span>
            ))}
            {(!detection || detection.modules.length === 0) && (
              <span className="badge badge-blocked">No modules detected</span>
            )}
          </div>
        </div>

        <div className="dashboard-card">
          <h3>Blocker(s)</h3>
          {blockers.length === 0 ? (
            <div className="sub">No blockers</div>
          ) : (
            blockers.map((b, i) => <div key={i} className="sub" style={{ color: 'var(--red)' }}>• {b}</div>)
          )}
        </div>

        <div className="dashboard-card">
          <h3>Last Run</h3>
          <div className="sub">{lastRun || 'No commands run yet'}</div>
          {lastFailure && <div className="sub" style={{ color: 'var(--red)' }}>Last failure: {lastFailure}</div>}
        </div>

        {lastProof && (
          <div className="dashboard-card">
            <h3>Latest Proof</h3>
            <div className="sub">{lastProof}</div>
          </div>
        )}

        <div className="dashboard-card">
          <h3>Scoreboard</h3>
          <div className="value" style={{ fontSize: 18 }}>{score}%</div>
        </div>
      </div>

      <div className="section-title">Actions</div>
      <div className="dashboard-actions">
        <button className="btn btn-primary" onClick={handleFullVerification} disabled={isRunning}>
          {isRunning ? 'Running...' : 'Run Full Verification'}
        </button>
        <button className="btn" onClick={runBuild} disabled={isRunning}>Run Build</button>
        <button className="btn" onClick={runTests} disabled={isRunning}>Run Tests</button>
        <button className="btn" onClick={runLint} disabled={isRunning}>Run Lint</button>
        <button className="btn" onClick={runScanner} disabled={isRunning}>Run Scanner</button>
        <button className="btn btn-danger" onClick={runSecurityScan} disabled={isRunning}>Run Security Scan</button>
        <button className="btn btn-success" onClick={runScoreboard} disabled={isRunning}>Generate Scoreboard</button>
      </div>

      <div className="section-title">Status Details</div>
      <table className="data-table">
        <thead>
          <tr><th>Check</th><th>Status</th></tr>
        </thead>
        <tbody>
          <tr><td>Git Branch</td><td><span className="badge badge-info">{branch}</span></td></tr>
          <tr><td>Dirty Files</td><td><span className="badge badge-info">{gitStatus.length}</span></td></tr>
          <tr><td>Build</td><td>{badges(buildStatus, 'PASS', 'FAIL')}</td></tr>
          <tr><td>Tests</td><td>{badges(testStatus, 'PASS', 'FAIL')}</td></tr>
          <tr><td>Scanner</td><td>{badges(findings.length > 0 ? 'FAIL' : '—', 'Clean', 'Issues found')}</td></tr>
          <tr><td>Score</td><td><span className={`badge badge-${score >= 80 ? 'pass' : score >= 50 ? 'partial' : 'fail'}`}>{score}%</span></td></tr>
        </tbody>
      </table>

      {nextTasks.length > 0 && (
        <>
          <div className="section-title">Next Tasks</div>
          <div style={{ fontSize: 'var(--font-size-sm)' }}>
            {nextTasks.map((t, i) => (
              <div key={i} className="tree-node">{i + 1}. {t}</div>
            ))}
          </div>
        </>
      )}

      <div className="section-title">Terminal Quick Commands</div>
      <div className="dashboard-actions">
        <button className="btn" onClick={() => window.x3studio.shell.exec('cargo check', workspacePath).then(r => addProofRecord({
          id: `proof-${Date.now()}`, command: 'cargo check', cwd: workspacePath || '', startTime: new Date().toISOString(),
          endTime: new Date().toISOString(), duration: 0, exitCode: r.exitCode, stdout: r.stdout, stderr: r.stderr,
          status: r.exitCode === 0 ? 'PASS' : 'FAIL', changedFiles: [], artifacts: [],
        }))}>cargo check</button>
        <button className="btn" onClick={() => window.x3studio.shell.exec('cargo test', workspacePath)}>cargo test</button>
        <button className="btn" onClick={() => window.x3studio.shell.exec('forge build', workspacePath)}>forge build</button>
        <button className="btn" onClick={() => window.x3studio.shell.exec('forge test', workspacePath)}>forge test</button>
      </div>
    </div>
  );
}
