import { useState } from 'react';
import type { DaoProposal } from '../../types';

export default function DaoProposalPanel() {
  const [proposals, setProposals] = useState<DaoProposal[]>([]);
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [votingPeriod, setVotingPeriod] = useState('604800');
  const [quorum, setQuorum] = useState('4');
  const [proposer, setProposer] = useState('0x...');
  const [actions, setActions] = useState('[{ "target": "0x...", "value": "0", "data": "0x..." }]');
  const [output, setOutput] = useState('');

  const buildProposal = () => {
    let parsedActions: any[];
    try { parsedActions = JSON.parse(actions); } catch { parsedActions = []; }
    const proposal: DaoProposal = { title, description, actions: parsedActions, votingPeriod: parseInt(votingPeriod), quorum: parseInt(quorum), proposer };
    setProposals(prev => [proposal, ...prev].slice(0, 20));
    setOutput(JSON.stringify(proposal, null, 2));
    setTitle(''); setDescription('');
  };

  const exportProposal = (p: DaoProposal) => {
    setOutput(JSON.stringify({ ...p, generatedAt: new Date().toISOString() }, null, 2));
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%', padding: 8 }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>DAO Proposal Builder</div>
      <p style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8 }}>
        Build and export governance proposals for X3 DAO.
      </p>

      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Title</label>
        <input className="input-field" value={title} onChange={e => setTitle(e.target.value)} /></div>
      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Description</label>
        <textarea className="input-field" rows={3} value={description} onChange={e => setDescription(e.target.value)} /></div>
      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Proposer Address</label>
        <input className="input-field" value={proposer} onChange={e => setProposer(e.target.value)} placeholder="0x..." /></div>
      <div style={{ display: 'flex', gap: 8, marginBottom: 8 }}>
        <div className="form-group" style={{ flex: 1 }}><label style={{ fontSize: 'var(--font-size-sm)' }}>Voting Period (s)</label>
          <input className="input-field" value={votingPeriod} onChange={e => setVotingPeriod(e.target.value)} /></div>
        <div className="form-group" style={{ flex: 1 }}><label style={{ fontSize: 'var(--font-size-sm)' }}>Quorum (ETH)</label>
          <input className="input-field" value={quorum} onChange={e => setQuorum(e.target.value)} /></div>
      </div>
      <div className="form-group"><label style={{ fontSize: 'var(--font-size-sm)' }}>Actions (JSON array)</label>
        <textarea className="input-field" rows={3} style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }} value={actions} onChange={e => setActions(e.target.value)} /></div>

      <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
        <button className="btn btn-primary" onClick={buildProposal} disabled={!title}>Build Proposal</button>
      </div>

      <div className="section-title">Recent Proposals ({proposals.length})</div>
      {proposals.map((p, i) => (
        <div key={i} style={{ background: 'var(--bg-surface)', borderRadius: 'var(--radius)', padding: 8, marginBottom: 6 }}>
          <div style={{ fontWeight: 600, fontSize: 'var(--font-size-sm)' }}>{p.title}</div>
          <div style={{ fontSize: 10, color: 'var(--text-muted)' }}>Actions: {p.actions.length} | Voting: {p.votingPeriod}s | Quorum: {p.quorum} ETH</div>
          <button className="btn" style={{ fontSize: 9, padding: '2px 6px', marginTop: 4 }} onClick={() => exportProposal(p)}>Export JSON</button>
        </div>
      ))}

      {output && (
        <>
          <div className="section-title">Output</div>
          <pre style={{ background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)', fontSize: 10, maxHeight: 200, overflow: 'auto', whiteSpace: 'pre-wrap' }}>{output}</pre>
        </>
      )}
    </div>
  );
}
