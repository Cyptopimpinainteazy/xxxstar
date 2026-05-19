import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";
import { formatDistanceToNow } from "date-fns";

const BulletinsPage: React.FC = () => {
  const { bulletins, loadBulletins, postBulletin } = useSocialStore();
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  const [composing, setComposing] = useState(false);

  const [systemBulletin, setSystemBulletin] = useState<{ title: string; body: string; pinned?: boolean; timestamp?: number } | null>(null);

  useEffect(() => { (async () => { loadBulletins(); try { const r = await fetch('https://blockchain-tps-go.x3star.net/api/system_bulletin'); if (r.ok) setSystemBulletin(await r.json()); } catch { /* ignore */ } })(); }, []);

  const handlePost = async () => {
    if (!title.trim() || !body.trim()) return;
    await postBulletin(title.trim(), body.trim());
    setTitle(""); setBody(""); setComposing(false);
  };

  return (
    <div>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
        <h1 className="social-page-title" style={{ margin: 0, border: "none", paddingBottom: 0 }}>Bulletins</h1>
        <button className="social-btn social-btn-sm" onClick={() => setComposing(!composing)}>
          {composing ? "Cancel" : "Post Bulletin"}
        </button>
      </div>

      {composing && (
        <div className="social-card" style={{ marginTop: "1rem" }}>
          <div className="social-card-header">Post a Bulletin</div>
          <div className="social-form-group">
            <label className="social-label">Subject</label>
            <input className="social-input" value={title} onChange={e => setTitle(e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Body</label>
            <textarea className="social-textarea" rows={5} value={body} onChange={e => setBody(e.target.value)} />
          </div>
          <button className="social-btn" onClick={handlePost} disabled={!title.trim() || !body.trim()}>
            Post Bulletin
          </button>
        </div>
      )}

      {systemBulletin && (
        <div className="social-card" style={{ marginTop: "1rem", borderLeft: "4px solid #ffd700", background: 'linear-gradient(90deg, rgba(255,223,0,0.04), rgba(255,255,255,0.01))' }}>
          <div className="social-card-header">Pinned — {systemBulletin.title}</div>
          <div style={{ padding: '0.6rem 1rem', whiteSpace: 'pre-wrap', color: '#fdf6e3' }}>{systemBulletin.body}</div>
          <div style={{ padding: '0.6rem 1rem', fontSize: '0.85rem', color: '#a8a8a8' }}>Updated: {systemBulletin.timestamp ? new Date(systemBulletin.timestamp).toLocaleString() : '—'}</div>
        </div>
      )}

      {/* Team links quick reference (pinned) */}
      <div className="social-card" style={{ marginTop: "1rem", borderLeft: "4px solid #06b6d4" }}>
        <div className="social-card-header">Team Links — Blockchain TPS</div>
        <div style={{ padding: "0.5rem 1rem" }}>
          <ul style={{ listStyle: "none", padding: 0, margin: 0 }}>
            <li><a href="https://blockchain-tps-go.x3star.net/presale.html" target="_blank" rel="noreferrer">Presale & Demo Request</a></li>
            <li><a href="https://blockchain-tps-go.x3star.net/company.html" target="_blank" rel="noreferrer">Company Signup</a></li>
            <li><a href="https://blockchain-tps-go.x3star.net/leaderboard.html" target="_blank" rel="noreferrer">Leaderboard</a></li>
            <li><a href="https://blockchain-tps-go.x3star.net/whoisonline.html" target="_blank" rel="noreferrer">Who's Online</a></li>
            <li><a href="https://blockchain-tps-go.x3star.net/sessions.html" target="_blank" rel="noreferrer">Sessions (admin)</a> (requires token)</li>
            <li><a href="https://x3star.net" target="_blank" rel="noreferrer">X3 Desktop (root)</a></li>
            <li><a href="https://blockchain-tps-go.x3star.net/leaderboard.html" target="_blank" rel="noreferrer">Benchmark Leaderboard</a></li>
          </ul>
          <div style={{ marginTop: "0.8rem" }}>
            <button className="social-btn" onClick={async () => {
              // Post a bulletin with quick links to the team feed
              const t = "X3 TPS: Presale & Demo Links";
              const b = `Presale & Demo: https://blockchain-tps-go.x3star.net/presale.html\nCompany signup: https://blockchain-tps-go.x3star.net/company.html\nLeaderboard: https://blockchain-tps-go.x3star.net/leaderboard.html\nWho's online: https://blockchain-tps-go.x3star.net/whoisonline.html`;
              await postBulletin(t, b);
              await loadBulletins();
            }}>Post Links to Bulletin</button>
          </div>
        </div>
      </div>

      <div className="social-card" style={{ marginTop: "1rem" }}>
        {bulletins.length === 0 ? (
          <div style={{ color: "#666", textAlign: "center", padding: "1rem", fontSize: "0.85rem" }}>
            No bulletins yet. You and your friends' bulletins will appear here.
          </div>
        ) : (
          bulletins.map(b => (
            <div key={b.id} className="bulletin-card">
              <div style={{ display: "flex", gap: "0.75rem" }}>
                {b.avatarUrl ? (
                  <img src={b.avatarUrl} alt="" style={{ width: 36, height: 36, borderRadius: 6, objectFit: "cover" }} />
                ) : (
                  <div style={{ width: 36, height: 36, borderRadius: 6, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0 }}>
                    {b.displayName[0]?.toUpperCase()}
                  </div>
                )}
                <div style={{ flex: 1 }}>
                  <div className="bulletin-title">{b.title}</div>
                  <div className="bulletin-meta">
                    by <Link to={`/social/view/${b.userId}`} style={{ color: "#ff8c42", textDecoration: "none" }}>{b.displayName}</Link>
                    {" · "}{formatDistanceToNow(new Date(b.createdAt), { addSuffix: true })}
                  </div>
                  <div className="bulletin-body">{b.body}</div>
                </div>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
};

export default BulletinsPage;
