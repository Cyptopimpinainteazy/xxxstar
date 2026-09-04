import { useWorkspaceStore } from '../../store';

export default function ProjectPanel() {
  const detection = useWorkspaceStore(s => s.detection);
  const workspaceName = useWorkspaceStore(s => s.workspaceName);

  if (!detection) {
    return (
      <div style={{ padding: 16 }}>
        <div className="panel-header">Project Detection</div>
        <div style={{ color: 'var(--text-muted)', padding: 16, fontSize: 'var(--font-size-sm)' }}>
          Open a workspace to detect project modules.
        </div>
      </div>
    );
  }

  return (
    <div style={{ padding: '8px', height: '100%', overflowY: 'auto' }}>
      <div className="panel-header">{workspaceName}</div>

      <div className="section-title">Detected Modules</div>
      {detection.modules.length === 0 && (
        <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 8 }}>
          No recognizable project modules found.
        </div>
      )}
      {detection.modules.map((m, i) => (
        <div key={i} className="check-item">
          <span style={{ color: 'var(--green)' }}>✓</span>
          <span className="label">{m}</span>
        </div>
      ))}

      <div className="section-title">Detection Details</div>
      <table className="data-table">
        <tbody>
          <tr><td>Cargo.toml</td><td>{detection.hasCargo ? '✓' : '—'}</td></tr>
          <tr><td>package.json</td><td>{detection.hasPackageJson ? '✓' : '—'}</td></tr>
          <tr><td>Hardhat</td><td>{detection.hasHardhat ? '✓' : '—'}</td></tr>
          <tr><td>Foundry</td><td>{detection.hasFoundry ? '✓' : '—'}</td></tr>
          <tr><td>Anchor/SVM</td><td>{detection.hasAnchor ? '✓' : '—'}</td></tr>
          <tr><td>Substrate</td><td>{detection.hasSubstrate ? '✓' : '—'}</td></tr>
          <tr><td>x3-lang files</td><td>{detection.hasX3Files ? '✓' : '—'}</td></tr>
          <tr><td>Pallets</td><td>{detection.hasPallets ? '✓' : '—'}</td></tr>
          <tr><td>Smart Contracts</td><td>{detection.hasContracts ? '✓' : '—'}</td></tr>
          <tr><td>Relayer</td><td>{detection.hasRelayer ? '✓' : '—'}</td></tr>
          <tr><td>Adapters</td><td>{detection.hasAdapters ? '✓' : '—'}</td></tr>
          <tr><td>Proof Ledger</td><td>{detection.hasProofLedger ? '✓' : '—'}</td></tr>
          <tr><td>Validator</td><td>{detection.hasValidator ? '✓' : '—'}</td></tr>
          <tr><td>Docker</td><td>{detection.hasDocker ? '✓' : '—'}</td></tr>
          <tr><td>Git</td><td>{detection.hasGit ? '✓' : '—'}</td></tr>
        </tbody>
      </table>
    </div>
  );
}
