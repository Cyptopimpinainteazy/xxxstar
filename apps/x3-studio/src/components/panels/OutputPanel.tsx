import { useProofStore } from '../../store';

export default function OutputPanel() {
  const records = useProofStore(s => s.records);

  return (
    <div style={{ padding: 8, height: '100%', overflowY: 'auto', fontSize: 'var(--font-size-sm)' }}>
      {records.length === 0 && (
        <div style={{ color: 'var(--text-muted)' }}>No command output yet. Run commands from Control Center or Proof Mode.</div>
      )}
      {records.map(r => (
        <div key={r.id} style={{ marginBottom: 12 }}>
          <div style={{ color: 'var(--accent)', fontFamily: 'var(--font-mono)', marginBottom: 4 }}>
            $ {r.command}
            <span style={{ color: r.exitCode === 0 ? 'var(--green)' : 'var(--red)', marginLeft: 8 }}>
              [{r.exitCode === 0 ? 'PASS' : `FAIL: ${r.exitCode}`}]
            </span>
          </div>
          {r.stdout && <div className="code-block" style={{ maxHeight: 100 }}>{r.stdout}</div>}
          {r.stderr && r.exitCode !== 0 && (
            <div className="code-block" style={{ maxHeight: 100, color: 'var(--red)' }}>{r.stderr}</div>
          )}
        </div>
      ))}
    </div>
  );
}
