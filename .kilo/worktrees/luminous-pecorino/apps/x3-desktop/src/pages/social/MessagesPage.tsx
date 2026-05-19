import React, { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { useSocialStore } from "@/stores/socialStore";
import { formatDistanceToNow } from "date-fns";

const MessagesPage: React.FC = () => {
  const { inbox, sentMessages, loadInbox, loadSentMessages, markRead, deleteMessage, sendMessage, friends } = useSocialStore();
  const [tab, setTab] = useState<"inbox" | "sent" | "compose">("inbox");
  const [selectedMsg, setSelectedMsg] = useState<string | null>(null);
  const [compTo, setCompTo] = useState("");
  const [compSubject, setCompSubject] = useState("");
  const [compBody, setCompBody] = useState("");

  useEffect(() => { loadInbox(); loadSentMessages(); }, []);

  const handleSend = async () => {
    if (!compTo || !compBody.trim()) return;
    await sendMessage(compTo, compSubject, compBody.trim());
    setCompTo(""); setCompSubject(""); setCompBody("");
    setTab("sent");
    loadSentMessages();
  };

  const openMsg = async (msgId: string, isRead: boolean) => {
    setSelectedMsg(msgId);
    if (!isRead) await markRead(msgId);
  };

  const selectedMessage = [...inbox, ...sentMessages].find(m => m.id === selectedMsg);

  return (
    <div>
      <h1 className="social-page-title">Messages</h1>

      <div style={{ display: "flex", gap: 0, marginBottom: "1rem" }}>
        <button className={`auth-tab ${tab === "inbox" ? "active" : ""}`} onClick={() => { setTab("inbox"); setSelectedMsg(null); }}>
          Inbox ({inbox.length})
        </button>
        <button className={`auth-tab ${tab === "sent" ? "active" : ""}`} onClick={() => { setTab("sent"); setSelectedMsg(null); }}>
          Sent ({sentMessages.length})
        </button>
        <button className={`auth-tab ${tab === "compose" ? "active" : ""}`} onClick={() => { setTab("compose"); setSelectedMsg(null); }}>
          Compose
        </button>
      </div>

      {/* Message Reader */}
      {selectedMessage && (
        <div className="social-card" style={{ marginBottom: "1rem" }}>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start" }}>
            <div>
              <div style={{ color: "#ff8c42", fontWeight: "bold", fontSize: "0.9rem" }}>{selectedMessage.subject || "(no subject)"}</div>
              <div style={{ color: "#888", fontSize: "0.75rem", marginTop: "0.25rem" }}>
                From: <Link to={`/social/view/${selectedMessage.fromUserId}`} style={{ color: "#ff8c42", textDecoration: "none" }}>{selectedMessage.fromDisplayName}</Link>
                {" · "}{formatDistanceToNow(new Date(selectedMessage.createdAt), { addSuffix: true })}
              </div>
            </div>
            <div style={{ display: "flex", gap: "0.5rem" }}>
              <button className="social-btn social-btn-sm" onClick={() => { setCompTo(selectedMessage.fromUserId); setCompSubject(`Re: ${selectedMessage.subject}`); setTab("compose"); setSelectedMsg(null); }}>Reply</button>
              <button className="social-btn social-btn-sm social-btn-danger" onClick={async () => { await deleteMessage(selectedMessage.id); setSelectedMsg(null); }}>Delete</button>
              <button className="social-btn social-btn-sm social-btn-outline" onClick={() => setSelectedMsg(null)}>Close</button>
            </div>
          </div>
          <div style={{ marginTop: "1rem", fontSize: "0.85rem", color: "#ccc", whiteSpace: "pre-wrap", lineHeight: 1.6 }}>
            {selectedMessage.body}
          </div>
        </div>
      )}

      {/* Inbox */}
      {tab === "inbox" && !selectedMsg && (
        <div className="social-card">
          {inbox.length === 0 ? (
            <div style={{ color: "#666", textAlign: "center", padding: "1rem", fontSize: "0.85rem" }}>Your inbox is empty.</div>
          ) : inbox.map(m => (
            <div key={m.id} className={`message-row ${!m.isRead ? "unread" : ""}`} onClick={() => openMsg(m.id, m.isRead)}>
              {m.fromAvatar ? <img src={m.fromAvatar} alt="" /> : (
                <div style={{ width: 36, height: 36, borderRadius: 6, background: "#223", display: "flex", alignItems: "center", justifyContent: "center", flexShrink: 0, fontSize: "0.8rem" }}>
                  {m.fromDisplayName[0]?.toUpperCase()}
                </div>
              )}
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: "flex", justifyContent: "space-between" }}>
                  <span className="message-subject">{m.subject || "(no subject)"}</span>
                  <span className="message-time">{formatDistanceToNow(new Date(m.createdAt), { addSuffix: true })}</span>
                </div>
                <div style={{ color: "#888", fontSize: "0.75rem" }}>from {m.fromDisplayName}</div>
                <div className="message-preview">{m.body}</div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Sent */}
      {tab === "sent" && !selectedMsg && (
        <div className="social-card">
          {sentMessages.length === 0 ? (
            <div style={{ color: "#666", textAlign: "center", padding: "1rem", fontSize: "0.85rem" }}>No sent messages.</div>
          ) : sentMessages.map(m => (
            <div key={m.id} className="message-row" onClick={() => openMsg(m.id, true)}>
              <div style={{ flex: 1, minWidth: 0 }}>
                <div style={{ display: "flex", justifyContent: "space-between" }}>
                  <span className="message-subject">{m.subject || "(no subject)"}</span>
                  <span className="message-time">{formatDistanceToNow(new Date(m.createdAt), { addSuffix: true })}</span>
                </div>
                <div style={{ color: "#888", fontSize: "0.75rem" }}>to {m.fromDisplayName}</div>
                <div className="message-preview">{m.body}</div>
              </div>
            </div>
          ))}
        </div>
      )}

      {/* Compose */}
      {tab === "compose" && (
        <div className="social-card">
          <div className="social-card-header">Compose New Message</div>
          <div className="social-form-group">
            <label className="social-label">To (select a friend)</label>
            <select className="social-select" style={{ width: "100%" }} value={compTo} onChange={e => setCompTo(e.target.value)}>
              <option value="">Select recipient...</option>
              {friends.map(f => (
                <option key={f.userId} value={f.userId}>{f.displayName} (@{f.username})</option>
              ))}
            </select>
          </div>
          <div className="social-form-group">
            <label className="social-label">Subject</label>
            <input className="social-input" value={compSubject} onChange={e => setCompSubject(e.target.value)} />
          </div>
          <div className="social-form-group">
            <label className="social-label">Message</label>
            <textarea className="social-textarea" rows={6} value={compBody} onChange={e => setCompBody(e.target.value)} />
          </div>
          <button className="social-btn" onClick={handleSend} disabled={!compTo || !compBody.trim()}>Send Message</button>
        </div>
      )}
    </div>
  );
};

export default MessagesPage;
