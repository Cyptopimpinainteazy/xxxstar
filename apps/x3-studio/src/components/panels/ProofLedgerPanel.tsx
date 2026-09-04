import { useState, useEffect } from 'react';
import { useWorkspaceStore } from '../../store';

export default function ProofLedgerPanel() {
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const [proofFiles, setProofFiles] = useState<string[]>([]);

  useEffect(() => {
    if (!workspacePath) return;
    (async () => {
      const exists = await window.x3studio.fs.exists(workspacePath + '/x3-proof');
      if (exists) {
        const entries = await window.x3studio.fs.readDir(workspacePath + '/x3-proof');
        setProofFiles(entries.filter(e => e.isFile).map(e => e.name));
      }
    })();
  }, [workspacePath]);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">Proof Ledger</div>
      <div className="panel-body">
        {proofFiles.length === 0 && (
          <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 16 }}>
            No proof artifacts found. Run commands in Proof Mode to generate proofs.
          </div>
        )}
        {proofFiles.map(f => (
          <div key={f} className="tree-node" onClick={async () => {
            if (!workspacePath) return;
            const content = await window.x3studio.fs.readFile(`${workspacePath}/x3-proof/${f}`);
            alert(content.substring(0, 500));
          }}>
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--font-size-sm)' }}>{f}</span>
          </div>
        ))}
        <div style={{ marginTop: 8, fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>
          Proof artifacts are stored in x3-proof/ directory. Each command generates a PROOF_REPORT.
        </div>
      </div>
    </div>
  );
}
