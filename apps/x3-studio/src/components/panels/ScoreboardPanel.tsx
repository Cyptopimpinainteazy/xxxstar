import { useScoreboardStore } from '../../store';

export default function ScoreboardPanel() {
  const categories = useScoreboardStore(s => s.categories);
  const totalScore = useScoreboardStore(s => s.totalScore);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Scoreboard</span>
        <span className={`badge badge-${totalScore >= 80 ? 'pass' : totalScore >= 50 ? 'partial' : 'fail'}`}>
          {totalScore}%
        </span>
      </div>
      <div className="panel-body">
        <div style={{ marginBottom: 12 }}>
          <div className="readiness-bar">
            <div className="readiness-fill" style={{
              width: `${totalScore}%`,
              background: totalScore >= 80 ? 'var(--green)' : totalScore >= 50 ? 'var(--yellow)' : 'var(--red)',
            }} />
          </div>
          <div style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)', textAlign: 'center' }}>
            Overall Score: {totalScore}%
          </div>
        </div>

        {categories.length === 0 && (
          <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 16, textAlign: 'center' }}>
            No scoreboard data. Run "Generate Scoreboard" from Control Center.
          </div>
        )}

        <table className="data-table">
          <thead>
            <tr>
              <th>Category</th>
              <th>Score</th>
              <th>Status</th>
              <th>Next Action</th>
            </tr>
          </thead>
          <tbody>
            {categories.map(c => (
              <tr key={c.name}>
                <td style={{ fontWeight: 500 }}>{c.name}</td>
                <td><span style={{ fontFamily: 'var(--font-mono)' }}>{c.score}/100</span></td>
                <td>
                  <span className={`badge badge-${c.status === 'PASS' ? 'pass' : c.status === 'PARTIAL' ? 'partial' : c.status === 'FAIL' ? 'fail' : 'blocked'}`}>
                    {c.status}
                  </span>
                </td>
                <td style={{ fontSize: 'var(--font-size-sm)', color: 'var(--text-muted)' }}>{c.nextAction}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
