import React, { useState, useEffect, useRef, useCallback } from "react";
import { useSocialStore } from "@/stores/socialStore";
import * as agentSvc from "@/services/agentService";
import type {
  AgentDef, AgentTask, AgentConversation, LeadFunnel, FunnelStats,
  OllamaStatus, UserEmailAssignment, UserProxy, SearchResult,
} from "@/services/agentService";

/* ════════════════════════════════════════════════════════
   CRM AGENTS PAGE — Full AI Marketing Command Center
   5 Agents • Web Search • RAG • Contact Import • Media
   ════════════════════════════════════════════════════════ */

type Tab = "agents" | "tasks" | "funnel" | "chat" | "search" | "contacts" | "rag" | "settings";

const STAGE_COLORS: Record<string, string> = {
  discovered: "#11a0dc", contacted: "#ff6b35", pitched: "#a855f7",
  negotiating: "#ffd740", converted: "#4caf50", lost: "#ef5350",
};
const STAGE_ORDER = ["discovered", "contacted", "pitched", "negotiating", "converted", "lost"];

/* ── Reusable styles ── */
const btn = (bg: string, small = false): React.CSSProperties => ({
  padding: small ? "4px 12px" : "8px 20px",
  background: bg, color: "#fff", border: "none",
  borderRadius: 8, fontWeight: 600, cursor: "pointer",
  fontSize: small ? 11 : 13,
});
const card: React.CSSProperties = { background: "#1a1a1a", borderRadius: 12, padding: 20, marginBottom: 16 };
const inputStyle: React.CSSProperties = {
  width: "100%", padding: "10px 14px", background: "#111", color: "#e0e0e0",
  border: "1px solid #333", borderRadius: 8, fontSize: 13,
};

const AgentsPage: React.FC = () => {
  const { session, currentUser } = useSocialStore();
  const userId = session?.userId ?? "";
  const username = currentUser?.username ?? session?.username ?? "";
  const isKing = currentUser?.username === "King" && currentUser?.role === "admin";

  /* ── Core state ── */
  const [tab, setTab] = useState<Tab>("agents");
  const [roster, setRoster] = useState<AgentDef[]>([]);
  const [ollamaStatus, setOllamaStatus] = useState<OllamaStatus | null>(null);
  const [tasks, setTasks] = useState<AgentTask[]>([]);
  const [leads, setLeads] = useState<LeadFunnel[]>([]);
  const [funnelStats, setFunnelStats] = useState<FunnelStats | null>(null);
  const [selectedAgent, setSelectedAgent] = useState<AgentDef | null>(null);
  const [chatHistory, setChatHistory] = useState<AgentConversation[]>([]);
  const [chatInput, setChatInput] = useState("");
  const [chatLoading, setChatLoading] = useState(false);
  const [taskPrompt, setTaskPrompt] = useState("");
  const [taskLoading, setTaskLoading] = useState(false);
  const [taskResult, setTaskResult] = useState<AgentTask | null>(null);
  const [userEmail, setUserEmail] = useState<UserEmailAssignment | null>(null);
  const [userProxy, setUserProxy] = useState<UserProxy | null>(null);
  const [loading, setLoading] = useState(true);
  const chatEndRef = useRef<HTMLDivElement>(null);

  /* ── Search state ── */
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [searchAnalysis, setSearchAnalysis] = useState<string | null>(null);
  const [searchLoading, setSearchLoading] = useState(false);
  const [websiteUrl, setWebsiteUrl] = useState("");
  const [websiteAnalysis, setWebsiteAnalysis] = useState<string | null>(null);

  /* ── Contact import state ── */
  const [contacts, setContacts] = useState<any[]>([]);
  const [pasteText, setPasteText] = useState("");
  const [importLoading, setImportLoading] = useState(false);
  const [importResult, setImportResult] = useState<any>(null);
  const [sortBy, setSortBy] = useState("ranking");
  const [filterNetwork, setFilterNetwork] = useState("");
  const [filterCountry, setFilterCountry] = useState("");
  const [contactFilters, setContactFilters] = useState<{ networks: string[]; countries: string[] }>({ networks: [], countries: [] });

  /* ── RAG state ── */
  const [ragFolder, setRagFolder] = useState("");
  const [ragQueryText, setRagQueryText] = useState("");
  const [ragAnswer, setRagAnswer] = useState<any>(null);
  const [ragStat, setRagStat] = useState<any>(null);
  const [ragLoading, setRagLoading] = useState(false);

  /* ── Proxy state ── */
  const [proxyEnabled, setProxyEnabled] = useState(false);
  const [proxyHost, setProxyHost] = useState("");
  const [proxyPort, setProxyPort] = useState("1080");
  const [proxyType, setProxyType] = useState("socks5");

  /* ── Media state ── */
  const [mediaFolder, setMediaFolder] = useState("");
  const [mediaFiles, setMediaFiles] = useState<any[]>([]);

  /* ── Load data ── */
  const load = useCallback(async () => {
    setLoading(true);
    try {
      const [r, s] = await Promise.all([agentSvc.getAgentRoster(), agentSvc.checkAgentStatus()]);
      setRoster(r); setOllamaStatus(s);
      if (userId) {
        const [t, l, e, p] = await Promise.all([
          agentSvc.getAgentTasks(userId, isKing),
          agentSvc.getLeads(userId, isKing),
          agentSvc.getUserEmail(userId),
          agentSvc.getProxy(userId),
        ]);
        setTasks(t); setLeads(l); setUserEmail(e); setUserProxy(p);
        if (p) setProxyEnabled(p.active);
        if (isKing) setFunnelStats(await agentSvc.getFunnelStats());
      }
    } catch (err) { console.error("Agent load error:", err); }
    setLoading(false);
  }, [userId, isKing]);

  useEffect(() => { load(); }, [load]);
  useEffect(() => { chatEndRef.current?.scrollIntoView({ behavior: "smooth" }); }, [chatHistory]);

  /* ── Actions ── */
  const openChat = async (agent: AgentDef) => {
    setSelectedAgent(agent); setTab("chat");
    try { setChatHistory(await agentSvc.getAgentHistory(userId, agent.id)); } catch { setChatHistory([]); }
  };

  const sendChat = async () => {
    if (!chatInput.trim() || !selectedAgent || chatLoading) return;
    const msg = chatInput.trim(); setChatInput(""); setChatLoading(true);
    setChatHistory(prev => [...prev, { id: "temp", agent_id: selectedAgent.id, user_id: userId, role: "user", content: msg, created_at: new Date().toISOString() }]);
    try {
      const resp = await agentSvc.chatWithAgent(userId, selectedAgent.id, msg);
      setChatHistory(prev => [...prev.filter(m => m.id !== "temp" || m.role !== "user"),
        { id: "sent", agent_id: selectedAgent.id, user_id: userId, role: "user", content: msg, created_at: new Date().toISOString() }, resp]);
    } catch (err: any) {
      setChatHistory(prev => [...prev, { id: "err", agent_id: selectedAgent.id, user_id: userId, role: "assistant", content: `Error: ${err?.message ?? err}`, created_at: new Date().toISOString() }]);
    }
    setChatLoading(false);
  };

  const runTask = async (agentId: string) => {
    if (!taskPrompt.trim() || taskLoading) return;
    setTaskLoading(true); setTaskResult(null);
    try {
      const result = await agentSvc.runAgentTask(userId, agentId, taskPrompt.trim());
      setTaskResult(result); setTasks(prev => [result, ...prev]);
    } catch (err: any) {
      setTaskResult({ id: "", agent_id: agentId, owner_user_id: userId, assigned_to_user_id: userId, task_type: "", prompt: taskPrompt, result: `Error: ${err?.message ?? err}`, status: "failed", leads_generated: 0, created_at: "", completed_at: "" });
    }
    setTaskLoading(false);
  };

  const assignMyEmail = async () => {
    if (!userId || !username) return;
    try { setUserEmail(await agentSvc.assignEmail(userId, username)); } catch (err) { console.error(err); }
  };

  const doSearch = async () => {
    if (!searchQuery.trim() || searchLoading) return;
    setSearchLoading(true); setSearchResults([]); setSearchAnalysis(null);
    try {
      const r = await agentSvc.webSearch(searchQuery.trim(), selectedAgent?.id);
      setSearchResults(r.results); setSearchAnalysis(r.analysis);
    } catch (err: any) { setSearchAnalysis(`Search error: ${err?.message ?? err}`); }
    setSearchLoading(false);
  };

  const doFetchWebsite = async () => {
    if (!websiteUrl.trim() || !selectedAgent || searchLoading) return;
    setSearchLoading(true); setWebsiteAnalysis(null);
    try {
      const r = await agentSvc.fetchWebsite(websiteUrl.trim(), selectedAgent.id);
      setWebsiteAnalysis(r.analysis);
    } catch (err: any) { setWebsiteAnalysis(`Error: ${err?.message ?? err}`); }
    setSearchLoading(false);
  };

  const doImportContacts = async () => {
    if (!pasteText.trim() || importLoading) return;
    setImportLoading(true); setImportResult(null);
    try {
      const r = await agentSvc.importContacts(userId, pasteText.trim());
      setImportResult(r); setPasteText("");
      await loadContacts();
    } catch (err: any) { setImportResult({ error: err?.message ?? err }); }
    setImportLoading(false);
  };

  const loadContacts = async () => {
    if (!userId) return;
    try {
      const [c, f] = await Promise.all([
        agentSvc.getContactsSorted(userId, sortBy, filterNetwork || undefined, filterCountry || undefined),
        agentSvc.getContactFilters(userId),
      ]);
      setContacts(c); setContactFilters(f);
    } catch (err) { console.error("Load contacts error:", err); }
  };

  useEffect(() => { if (tab === "contacts" && userId) loadContacts(); }, [tab, sortBy, filterNetwork, filterCountry]);

  const doRagIndex = async () => {
    if (!ragFolder.trim() || ragLoading) return;
    setRagLoading(true);
    try {
      await agentSvc.ragIndex(ragFolder.trim());
      setRagStat(await agentSvc.ragStats());
    } catch (err: any) { console.error(err); }
    setRagLoading(false);
  };

  const doRagQueryFn = async () => {
    if (!ragQueryText.trim() || !selectedAgent || ragLoading) return;
    setRagLoading(true); setRagAnswer(null);
    try {
      setRagAnswer(await agentSvc.ragQuery(ragQueryText.trim(), selectedAgent.id));
    } catch (err: any) { setRagAnswer({ answer: `Error: ${err?.message ?? err}`, sources: [] }); }
    setRagLoading(false);
  };

  const toggleProxyState = async () => {
    const newState = !proxyEnabled;
    setProxyEnabled(newState);
    try { await agentSvc.toggleProxy(userId, newState); } catch (err) { console.error(err); setProxyEnabled(!newState); }
  };

  const doAssignProxy = async () => {
    if (!proxyHost.trim()) return;
    try {
      const p = await agentSvc.assignProxy(userId, { proxy_host: proxyHost, proxy_port: parseInt(proxyPort) || 1080, proxy_type: proxyType });
      setUserProxy(p); setProxyEnabled(true);
    } catch (err) { console.error(err); }
  };

  const doScanMedia = async () => {
    if (!mediaFolder.trim()) return;
    try {
      const r = await agentSvc.scanMedia(mediaFolder.trim());
      setMediaFiles(r.files);
    } catch (err) { console.error(err); }
  };

  if (loading) return <div style={{ padding: 40, textAlign: "center", color: "#888" }}>Loading agents...</div>;

  /* ════════════════════════════════════════════════════════
     RENDER
     ════════════════════════════════════════════════════════ */
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", background: "#0d0d0d", color: "#e0e0e0" }}>
      {/* ── Header ── */}
      <div style={{ padding: "12px 24px", borderBottom: "1px solid #222", display: "flex", alignItems: "center", gap: 12, flexWrap: "wrap" }}>
        <h1 style={{ margin: 0, fontSize: 20, fontWeight: 700 }}>🤖 AI Agent Command Center</h1>
        <span style={{ fontSize: 11, color: ollamaStatus?.online ? "#4caf50" : "#ef5350", fontWeight: 600 }}>
          {ollamaStatus?.online ? "● Ollama Online" : "● Ollama Offline"}
        </span>
        {userEmail && <span style={{ fontSize: 11, color: "#11a0dc" }}>📧 {userEmail.email_address}</span>}
        {!userEmail && <button onClick={assignMyEmail} style={btn("#11a0dc", true)}>Get x3star.net Email</button>}
        {/* Proxy/VPN toggle in header */}
        <div style={{ display: "flex", alignItems: "center", gap: 6, marginLeft: "auto" }}>
          <span style={{ fontSize: 10, color: proxyEnabled ? "#4caf50" : "#666" }}>
            {proxyEnabled ? "🔒 VPN ON" : "🔓 VPN OFF"}
          </span>
          <button onClick={toggleProxyState} style={{
            width: 40, height: 20, borderRadius: 10, border: "none", cursor: "pointer",
            background: proxyEnabled ? "#4caf50" : "#333", position: "relative", transition: "background 0.2s",
          }}>
            <div style={{
              width: 16, height: 16, borderRadius: 8, background: "#fff", position: "absolute",
              top: 2, left: proxyEnabled ? 22 : 2, transition: "left 0.2s",
            }} />
          </button>
        </div>
        {isKing && <span style={{ background: "#ffd740", color: "#000", padding: "2px 10px", borderRadius: 8, fontSize: 10, fontWeight: 800 }}>👑 KING</span>}
      </div>

      {/* ── Tabs ── */}
      <div style={{ display: "flex", gap: 0, borderBottom: "1px solid #222", overflowX: "auto" }}>
        {(["agents", "tasks", "funnel", "chat", "search", "contacts", "rag", "settings"] as Tab[]).map(t => (
          <button key={t} onClick={() => setTab(t)} style={{
            padding: "10px 16px", background: tab === t ? "#1a1a1a" : "transparent",
            color: tab === t ? "#ff6b35" : "#888", border: "none", whiteSpace: "nowrap",
            borderBottom: tab === t ? "2px solid #ff6b35" : "2px solid transparent",
            fontWeight: tab === t ? 700 : 400, cursor: "pointer", fontSize: 12, textTransform: "capitalize",
          }}>{t === "rag" ? "📚 RAG" : t === "search" ? "🔍 Search" : t === "contacts" ? "👥 Contacts" : t}</button>
        ))}
      </div>

      {/* ── Content ── */}
      <div style={{ flex: 1, overflow: "auto", padding: 20 }}>

        {/* ══ AGENTS TAB ══ */}
        {tab === "agents" && (
          <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, minmax(300px, 1fr))", gap: 16 }}>
            {roster.map(agent => (
              <div key={agent.id} style={{ ...card, border: `1px solid ${agent.color}33` }}>
                <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
                  <span style={{ fontSize: 32 }}>{agent.avatar}</span>
                  <div>
                    <div style={{ fontWeight: 700, color: agent.color, fontSize: 16 }}>{agent.name}</div>
                    <div style={{ fontSize: 11, color: "#888" }}>{agent.role.replace(/_/g, " ")}</div>
                  </div>
                </div>
                <div style={{ fontSize: 12, color: "#aaa", marginBottom: 12, lineHeight: 1.5 }}>
                  {agent.system_prompt.slice(0, 150)}...
                </div>
                <div style={{ display: "flex", flexWrap: "wrap", gap: 4, marginBottom: 12 }}>
                  {agent.capabilities.map(c => (
                    <span key={c} style={{ fontSize: 9, background: `${agent.color}22`, color: agent.color, padding: "2px 8px", borderRadius: 8 }}>
                      {c.replace(/_/g, " ")}
                    </span>
                  ))}
                </div>
                <div style={{ display: "flex", gap: 8 }}>
                  <button onClick={() => openChat(agent)} style={{ ...btn(agent.color), flex: 1, padding: "8px 0" }}>💬 Chat</button>
                  <button onClick={() => { setSelectedAgent(agent); setTab("tasks"); }} style={{ ...btn("#333"), flex: 1, padding: "8px 0" }}>⚡ Task</button>
                  <button onClick={() => { setSelectedAgent(agent); setTab("search"); }} style={{ ...btn("#1e3a5f"), flex: 1, padding: "8px 0" }}>🔍 Search</button>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* ══ TASKS TAB ══ */}
        {tab === "tasks" && (
          <div>
            {selectedAgent && (
              <div style={{ ...card, border: `1px solid ${selectedAgent.color}33` }}>
                <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
                  <span style={{ fontSize: 24 }}>{selectedAgent.avatar}</span>
                  <span style={{ fontWeight: 700, color: selectedAgent.color }}>{selectedAgent.name}</span>
                  <span style={{ fontSize: 11, color: "#888" }}>— {selectedAgent.role.replace(/_/g, " ")}</span>
                </div>
                <textarea value={taskPrompt} onChange={e => setTaskPrompt(e.target.value)}
                  placeholder={`Give ${selectedAgent.name} a task...`}
                  style={{ ...inputStyle, minHeight: 80, resize: "vertical" }} />
                <button onClick={() => runTask(selectedAgent.id)} disabled={taskLoading || !taskPrompt.trim()}
                  style={{ ...btn(taskLoading ? "#555" : selectedAgent.color), marginTop: 8, cursor: taskLoading ? "wait" : "pointer" }}>
                  {taskLoading ? "⏳ Agent working..." : "⚡ Run Task"}
                </button>
                {taskResult && (
                  <div style={{ marginTop: 16, background: "#111", borderRadius: 8, padding: 16, maxHeight: 400, overflow: "auto" }}>
                    <div style={{ fontSize: 11, color: taskResult.status === "completed" ? "#4caf50" : "#ef5350", marginBottom: 8 }}>
                      {taskResult.status === "completed" ? "✅ Completed" : "❌ Failed"}
                    </div>
                    <pre style={{ whiteSpace: "pre-wrap", fontSize: 12, color: "#ccc", lineHeight: 1.6, margin: 0 }}>{taskResult.result}</pre>
                  </div>
                )}
              </div>
            )}
            {!selectedAgent && <p style={{ color: "#888" }}>Select an agent from the Agents tab to run tasks.</p>}
            <h3 style={{ color: "#ff6b35", marginBottom: 12 }}>{isKing ? "All Team Tasks" : "My Tasks"} ({tasks.length})</h3>
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {tasks.map(t => {
                const agent = roster.find(a => a.id === t.agent_id);
                return (
                  <div key={t.id} style={{ background: "#1a1a1a", borderRadius: 8, padding: 12, borderLeft: `3px solid ${agent?.color ?? "#555"}` }}>
                    <div style={{ display: "flex", justifyContent: "space-between", marginBottom: 4 }}>
                      <span style={{ fontWeight: 600, color: agent?.color ?? "#888" }}>{agent?.avatar} {agent?.name ?? t.agent_id}</span>
                      <span style={{ fontSize: 10, color: t.status === "completed" ? "#4caf50" : t.status === "failed" ? "#ef5350" : "#ffd740" }}>{t.status}</span>
                    </div>
                    <div style={{ fontSize: 12, color: "#aaa", marginBottom: 4 }}>{t.prompt.slice(0, 100)}{t.prompt.length > 100 ? "..." : ""}</div>
                    {t.result && <pre style={{ whiteSpace: "pre-wrap", fontSize: 11, color: "#777", maxHeight: 100, overflow: "hidden", margin: "4px 0" }}>{t.result.slice(0, 200)}</pre>}
                    <div style={{ fontSize: 10, color: "#555" }}>{new Date(t.created_at).toLocaleString()}</div>
                  </div>
                );
              })}
              {tasks.length === 0 && <p style={{ color: "#555", fontSize: 13 }}>No tasks yet. Select an agent and give it something to do.</p>}
            </div>
          </div>
        )}

        {/* ══ FUNNEL TAB ══ */}
        {tab === "funnel" && (
          <div>
            {isKing && funnelStats && (
              <div style={{ display: "grid", gridTemplateColumns: "repeat(6, 1fr)", gap: 8, marginBottom: 20 }}>
                {STAGE_ORDER.map(stage => (
                  <div key={stage} style={{ background: "#1a1a1a", borderRadius: 8, padding: 12, textAlign: "center", borderTop: `3px solid ${STAGE_COLORS[stage]}` }}>
                    <div style={{ fontSize: 24, fontWeight: 800, color: STAGE_COLORS[stage] }}>{(funnelStats.funnel as any)[stage] ?? 0}</div>
                    <div style={{ fontSize: 10, color: "#888", textTransform: "capitalize" }}>{stage}</div>
                  </div>
                ))}
              </div>
            )}
            <div style={{ display: "flex", gap: 12, minHeight: 400 }}>
              {STAGE_ORDER.map(stage => {
                const stageLeads = leads.filter(l => l.funnel_stage === stage);
                return (
                  <div key={stage} style={{ flex: 1, background: "#111", borderRadius: 8, padding: 8, minWidth: 0 }}>
                    <div style={{ textAlign: "center", padding: "8px 0", borderBottom: `2px solid ${STAGE_COLORS[stage]}`, marginBottom: 8 }}>
                      <span style={{ fontSize: 12, fontWeight: 700, color: STAGE_COLORS[stage], textTransform: "capitalize" }}>{stage}</span>
                      <span style={{ fontSize: 10, color: "#666", marginLeft: 4 }}>({stageLeads.length})</span>
                    </div>
                    {stageLeads.map(lead => (
                      <div key={lead.id} style={{ background: "#1a1a1a", borderRadius: 6, padding: 8, marginBottom: 6, fontSize: 11 }}>
                        <div style={{ fontWeight: 600, color: "#e0e0e0" }}>Score: {lead.score}</div>
                        {lead.notes && <div style={{ color: "#888", marginTop: 2 }}>{lead.notes.slice(0, 60)}</div>}
                      </div>
                    ))}
                    {stageLeads.length === 0 && <div style={{ color: "#333", textAlign: "center", fontSize: 10, padding: 20 }}>Empty</div>}
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* ══ CHAT TAB ══ */}
        {tab === "chat" && (
          <div style={{ display: "flex", flexDirection: "column", height: "calc(100vh - 200px)" }}>
            <div style={{ display: "flex", gap: 8, marginBottom: 12, flexWrap: "wrap" }}>
              {roster.map(a => (
                <button key={a.id} onClick={() => openChat(a)} style={{
                  padding: "6px 14px", borderRadius: 8, border: selectedAgent?.id === a.id ? `2px solid ${a.color}` : "1px solid #333",
                  background: selectedAgent?.id === a.id ? `${a.color}22` : "#1a1a1a", color: a.color, fontSize: 12, fontWeight: 600, cursor: "pointer",
                }}>{a.avatar} {a.name}</button>
              ))}
            </div>
            {selectedAgent ? (
              <>
                <div style={{ flex: 1, overflow: "auto", background: "#111", borderRadius: 8, padding: 12 }}>
                  {chatHistory.map((msg, i) => (
                    <div key={i} style={{ marginBottom: 12, display: "flex", flexDirection: "column", alignItems: msg.role === "user" ? "flex-end" : "flex-start" }}>
                      <div style={{
                        maxWidth: "80%", padding: "10px 14px", borderRadius: 12,
                        background: msg.role === "user" ? "#1e3a5f" : "#1a1a1a",
                        border: msg.role === "user" ? "1px solid #2d5a8e" : `1px solid ${selectedAgent.color}33`,
                      }}>
                        {msg.role === "assistant" && <div style={{ fontSize: 10, color: selectedAgent.color, marginBottom: 4, fontWeight: 700 }}>{selectedAgent.avatar} {selectedAgent.name}</div>}
                        <pre style={{ whiteSpace: "pre-wrap", fontSize: 12, color: "#ddd", margin: 0, lineHeight: 1.5, fontFamily: "inherit" }}>{msg.content}</pre>
                      </div>
                    </div>
                  ))}
                  {chatLoading && <div style={{ color: selectedAgent.color, fontSize: 12, padding: 8 }}>{selectedAgent.avatar} {selectedAgent.name} is thinking...</div>}
                  <div ref={chatEndRef} />
                </div>
                <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
                  <input value={chatInput} onChange={e => setChatInput(e.target.value)}
                    onKeyDown={e => { if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); sendChat(); } }}
                    placeholder={`Ask ${selectedAgent.name}...`} style={{ ...inputStyle, flex: 1 }} />
                  <button onClick={sendChat} disabled={chatLoading || !chatInput.trim()} style={btn(chatLoading ? "#555" : selectedAgent.color)}>Send</button>
                </div>
              </>
            ) : (
              <div style={{ textAlign: "center", padding: 40, color: "#555" }}>Select an agent above to start chatting.</div>
            )}
          </div>
        )}

        {/* ══ SEARCH TAB ══ — Web search + website analysis */}
        {tab === "search" && (
          <div>
            <div style={card}>
              <h3 style={{ color: "#11a0dc", margin: "0 0 12px" }}>🔍 Web Search</h3>
              <p style={{ fontSize: 12, color: "#888", marginBottom: 12 }}>Search the internet. {selectedAgent ? `${selectedAgent.avatar} ${selectedAgent.name} will analyze results.` : "Select an agent for AI analysis."}</p>
              <div style={{ display: "flex", gap: 8 }}>
                <input value={searchQuery} onChange={e => setSearchQuery(e.target.value)}
                  onKeyDown={e => { if (e.key === "Enter") doSearch(); }}
                  placeholder="Search for Web3 founders, blockchain projects, grants..." style={{ ...inputStyle, flex: 1 }} />
                <button onClick={doSearch} disabled={searchLoading || !searchQuery.trim()} style={btn(searchLoading ? "#555" : "#11a0dc")}>
                  {searchLoading ? "⏳ Searching..." : "🔍 Search"}
                </button>
              </div>
              {/* Agent selector for search */}
              <div style={{ display: "flex", gap: 6, marginTop: 8, flexWrap: "wrap" }}>
                {roster.map(a => (
                  <button key={a.id} onClick={() => setSelectedAgent(a)} style={{
                    padding: "4px 10px", borderRadius: 6, fontSize: 10, fontWeight: 600, cursor: "pointer",
                    border: selectedAgent?.id === a.id ? `2px solid ${a.color}` : "1px solid #333",
                    background: selectedAgent?.id === a.id ? `${a.color}22` : "transparent", color: a.color,
                  }}>{a.avatar} {a.name}</button>
                ))}
              </div>
            </div>

            {/* Search Results */}
            {searchResults.length > 0 && (
              <div style={card}>
                <h4 style={{ color: "#ff6b35", margin: "0 0 12px" }}>Results ({searchResults.length})</h4>
                {searchResults.map((r, i) => (
                  <div key={i} style={{ padding: "10px 0", borderBottom: "1px solid #222" }}>
                    <div style={{ fontWeight: 600, color: "#11a0dc", fontSize: 13 }}>{r.title}</div>
                    <div style={{ fontSize: 10, color: "#4caf50", marginBottom: 4 }}>{r.url}</div>
                    <div style={{ fontSize: 12, color: "#aaa" }}>{r.snippet}</div>
                  </div>
                ))}
              </div>
            )}

            {/* AI Analysis */}
            {searchAnalysis && (
              <div style={card}>
                <h4 style={{ color: selectedAgent?.color ?? "#ff6b35", margin: "0 0 8px" }}>
                  {selectedAgent?.avatar} {selectedAgent?.name ?? "Agent"} Analysis
                </h4>
                <pre style={{ whiteSpace: "pre-wrap", fontSize: 12, color: "#ccc", lineHeight: 1.6, margin: 0 }}>{searchAnalysis}</pre>
              </div>
            )}

            {/* Website analyzer */}
            <div style={{ ...card, marginTop: 16 }}>
              <h3 style={{ color: "#a855f7", margin: "0 0 12px" }}>🌐 Website Analyzer</h3>
              <p style={{ fontSize: 12, color: "#888", marginBottom: 12 }}>Paste a URL — agent reads the site and generates personalized outreach angles.</p>
              <div style={{ display: "flex", gap: 8 }}>
                <input value={websiteUrl} onChange={e => setWebsiteUrl(e.target.value)}
                  placeholder="https://example.com" style={{ ...inputStyle, flex: 1 }} />
                <button onClick={doFetchWebsite} disabled={searchLoading || !websiteUrl.trim() || !selectedAgent}
                  style={btn(searchLoading ? "#555" : "#a855f7")}>
                  {searchLoading ? "⏳ Reading..." : "🌐 Analyze"}
                </button>
              </div>
              {websiteAnalysis && (
                <div style={{ marginTop: 12, background: "#111", borderRadius: 8, padding: 16, maxHeight: 400, overflow: "auto" }}>
                  <pre style={{ whiteSpace: "pre-wrap", fontSize: 12, color: "#ccc", lineHeight: 1.6, margin: 0 }}>{websiteAnalysis}</pre>
                </div>
              )}
            </div>
          </div>
        )}

        {/* ══ CONTACTS TAB ══ — Sort by NETWORK/COUNTRY/Ranking + Paste Import */}
        {tab === "contacts" && (
          <div>
            {/* Paste Import */}
            <div style={card}>
              <h3 style={{ color: "#ff6b35", margin: "0 0 12px" }}>📋 Paste & Import Contacts</h3>
              <p style={{ fontSize: 12, color: "#888", marginBottom: 8 }}>Paste contact lists, LinkedIn profiles, business cards, emails — AI auto-parses and categorizes by network, country, and ranking.</p>
              <textarea value={pasteText} onChange={e => setPasteText(e.target.value)}
                placeholder={"Paste contact info here...\n\nExamples:\n- Vitalik Buterin, Ethereum Foundation, vitalik@ethereum.org, Switzerland\n- Anatoly Yakovenko, CEO Solana Labs, San Francisco\n- Or paste a CSV, list from LinkedIn, email signatures, etc."}
                style={{ ...inputStyle, minHeight: 120, resize: "vertical", fontFamily: "monospace" }} />
              <div style={{ display: "flex", gap: 8, marginTop: 8, alignItems: "center" }}>
                <button onClick={doImportContacts} disabled={importLoading || !pasteText.trim()}
                  style={btn(importLoading ? "#555" : "#ff6b35")}>
                  {importLoading ? "⏳ AI parsing..." : "🤖 Import & Auto-Sort"}
                </button>
                {importResult && !importResult.error && (
                  <span style={{ fontSize: 12, color: "#4caf50" }}>
                    ✅ Imported {importResult.contacts_imported} contacts!
                  </span>
                )}
                {importResult?.error && <span style={{ fontSize: 12, color: "#ef5350" }}>❌ {importResult.error}</span>}
              </div>
            </div>

            {/* Sort & Filter controls */}
            <div style={{ display: "flex", gap: 12, marginBottom: 16, flexWrap: "wrap", alignItems: "center" }}>
              <label style={{ fontSize: 12, color: "#888" }}>Sort by:</label>
              {["ranking", "network", "country", "name"].map(s => (
                <button key={s} onClick={() => setSortBy(s)} style={{
                  ...btn(sortBy === s ? "#ff6b35" : "#333", true), textTransform: "capitalize",
                }}>{s}</button>
              ))}
              {contactFilters.networks.length > 0 && (
                <select value={filterNetwork} onChange={e => setFilterNetwork(e.target.value)}
                  style={{ ...inputStyle, width: "auto", padding: "4px 8px", fontSize: 11 }}>
                  <option value="">All Networks</option>
                  {contactFilters.networks.map(n => <option key={n} value={n}>{n}</option>)}
                </select>
              )}
              {contactFilters.countries.length > 0 && (
                <select value={filterCountry} onChange={e => setFilterCountry(e.target.value)}
                  style={{ ...inputStyle, width: "auto", padding: "4px 8px", fontSize: 11 }}>
                  <option value="">All Countries</option>
                  {contactFilters.countries.map(c => <option key={c} value={c}>{c}</option>)}
                </select>
              )}
              <span style={{ fontSize: 11, color: "#666", marginLeft: "auto" }}>{contacts.length} contacts</span>
            </div>

            {/* Contact list */}
            <div style={{ display: "flex", flexDirection: "column", gap: 4 }}>
              {/* Header */}
              <div style={{ display: "grid", gridTemplateColumns: "2fr 2fr 1fr 1fr 80px", gap: 8, padding: "8px 12px", background: "#1a1a1a", borderRadius: 8, fontSize: 10, fontWeight: 700, color: "#888" }}>
                <span>NAME</span><span>COMPANY / TITLE</span><span>NETWORK</span><span>COUNTRY</span><span>RANK</span>
              </div>
              {contacts.map((c: any) => (
                <div key={c.id} style={{ display: "grid", gridTemplateColumns: "2fr 2fr 1fr 1fr 80px", gap: 8, padding: "8px 12px", background: "#111", borderRadius: 6, fontSize: 12, alignItems: "center" }}>
                  <div>
                    <span style={{ fontWeight: 600, color: "#e0e0e0" }}>{c.first_name} {c.last_name}</span>
                    {c.email && <div style={{ fontSize: 10, color: "#666" }}>{c.email}</div>}
                  </div>
                  <div>
                    <span style={{ color: "#aaa" }}>{c.company}</span>
                    {c.job_title && <div style={{ fontSize: 10, color: "#555" }}>{c.job_title}</div>}
                  </div>
                  <span style={{ color: "#11a0dc", fontSize: 11, fontWeight: 600 }}>{c.network || "—"}</span>
                  <span style={{ color: "#888", fontSize: 11 }}>{c.country || "—"}</span>
                  <div style={{ display: "flex", alignItems: "center", gap: 4 }}>
                    <div style={{
                      width: 24, height: 24, borderRadius: "50%", display: "flex", alignItems: "center", justifyContent: "center",
                      fontSize: 11, fontWeight: 800,
                      background: c.ranking >= 8 ? "#4caf5033" : c.ranking >= 5 ? "#ffd74033" : "#33333366",
                      color: c.ranking >= 8 ? "#4caf50" : c.ranking >= 5 ? "#ffd740" : "#666",
                    }}>{c.ranking}</div>
                  </div>
                </div>
              ))}
              {contacts.length === 0 && (
                <div style={{ textAlign: "center", padding: 40, color: "#555" }}>No contacts yet. Paste some above to get started!</div>
              )}
            </div>
          </div>
        )}

        {/* ══ RAG TAB ══ — Index .md docs + query with agents */}
        {tab === "rag" && (
          <div>
            <div style={card}>
              <h3 style={{ color: "#a855f7", margin: "0 0 12px" }}>📚 RAG Knowledge Base</h3>
              <p style={{ fontSize: 12, color: "#888", marginBottom: 12 }}>Index .md files from any folder. Agents use this context when answering questions — like OpenNotebook for your project docs.</p>

              {/* Index folder */}
              <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
                <input value={ragFolder} onChange={e => setRagFolder(e.target.value)}
                  placeholder="/path/to/folder/with/docs" style={{ ...inputStyle, flex: 1 }} />
                <button onClick={doRagIndex} disabled={ragLoading || !ragFolder.trim()} style={btn(ragLoading ? "#555" : "#a855f7")}>
                  {ragLoading ? "⏳ Indexing..." : "📁 Index Folder"}
                </button>
              </div>

              {/* Stats */}
              {ragStat && (
                <div style={{ background: "#111", borderRadius: 8, padding: 12, fontSize: 12, marginBottom: 12 }}>
                  <span style={{ color: "#4caf50", fontWeight: 700 }}>{ragStat.total_docs} docs indexed</span>
                  <span style={{ color: "#888", marginLeft: 12 }}>{ragStat.total_tokens?.toLocaleString()} tokens</span>
                  {ragStat.files?.slice(0, 5).map((f: any, i: number) => (
                    <div key={i} style={{ fontSize: 10, color: "#555", marginTop: 2 }}>📄 {f.path?.split("/").pop()} ({f.tokens} tokens)</div>
                  ))}
                </div>
              )}

              {/* Query */}
              <div style={{ display: "flex", gap: 6, marginBottom: 8, flexWrap: "wrap" }}>
                {roster.map(a => (
                  <button key={a.id} onClick={() => setSelectedAgent(a)} style={{
                    padding: "4px 10px", borderRadius: 6, fontSize: 10, fontWeight: 600, cursor: "pointer",
                    border: selectedAgent?.id === a.id ? `2px solid ${a.color}` : "1px solid #333",
                    background: selectedAgent?.id === a.id ? `${a.color}22` : "transparent", color: a.color,
                  }}>{a.avatar} {a.name}</button>
                ))}
              </div>
              <div style={{ display: "flex", gap: 8 }}>
                <input value={ragQueryText} onChange={e => setRagQueryText(e.target.value)}
                  onKeyDown={e => { if (e.key === "Enter") doRagQueryFn(); }}
                  placeholder="Ask a question using your indexed docs..." style={{ ...inputStyle, flex: 1 }} />
                <button onClick={doRagQueryFn} disabled={ragLoading || !ragQueryText.trim() || !selectedAgent}
                  style={btn(ragLoading ? "#555" : selectedAgent?.color ?? "#a855f7")}>
                  {ragLoading ? "⏳ Querying..." : "🧠 Ask"}
                </button>
              </div>
            </div>

            {/* RAG Answer */}
            {ragAnswer && (
              <div style={card}>
                <h4 style={{ color: selectedAgent?.color ?? "#a855f7", margin: "0 0 8px" }}>
                  {selectedAgent?.avatar} {selectedAgent?.name ?? "Agent"} Answer
                </h4>
                <pre style={{ whiteSpace: "pre-wrap", fontSize: 12, color: "#ccc", lineHeight: 1.6, margin: 0 }}>{ragAnswer.answer}</pre>
                {ragAnswer.sources?.length > 0 && (
                  <div style={{ marginTop: 12, fontSize: 10, color: "#888" }}>
                    <strong>Sources:</strong>
                    {ragAnswer.sources.map((s: string, i: number) => (
                      <div key={i}>📄 {s.split("/").pop()}</div>
                    ))}
                  </div>
                )}
              </div>
            )}
          </div>
        )}

        {/* ══ SETTINGS TAB ══ — Email, Proxy/VPN, Media, Ollama */}
        {tab === "settings" && (
          <div style={{ maxWidth: 700 }}>
            <h3 style={{ color: "#ff6b35", marginBottom: 16 }}>Agent Settings</h3>

            {/* Email assignment */}
            <div style={card}>
              <h4 style={{ color: "#11a0dc", margin: "0 0 8px" }}>📧 x3star.net Email</h4>
              {userEmail ? (
                <div>
                  <div style={{ fontSize: 14, fontWeight: 600, color: "#4caf50" }}>{userEmail.email_address}</div>
                  <div style={{ fontSize: 11, color: "#888", marginTop: 4 }}>SMTP user: {userEmail.smtp_username}</div>
                </div>
              ) : (
                <div>
                  <p style={{ color: "#888", fontSize: 13 }}>Get your personal @x3star.net email address for agent outreach.</p>
                  <button onClick={assignMyEmail} style={btn("#11a0dc")}>Assign Email</button>
                </div>
              )}
            </div>

            {/* Proxy / VPN */}
            <div style={card}>
              <h4 style={{ color: "#a855f7", margin: "0 0 12px" }}>🔒 Proxy / VPN</h4>
              <div style={{ display: "flex", alignItems: "center", gap: 12, marginBottom: 12 }}>
                <span style={{ fontSize: 13, color: proxyEnabled ? "#4caf50" : "#888" }}>
                  {proxyEnabled ? "🟢 VPN Active" : "⚫ VPN Off"}
                </span>
                <button onClick={toggleProxyState} style={{
                  width: 48, height: 24, borderRadius: 12, border: "none", cursor: "pointer",
                  background: proxyEnabled ? "#4caf50" : "#333", position: "relative",
                }}>
                  <div style={{
                    width: 20, height: 20, borderRadius: 10, background: "#fff", position: "absolute",
                    top: 2, left: proxyEnabled ? 26 : 2, transition: "left 0.2s",
                  }} />
                </button>
              </div>
              {userProxy ? (
                <div style={{ fontSize: 13, color: "#e0e0e0" }}>
                  {userProxy.proxy_type}://{userProxy.proxy_host}:{userProxy.proxy_port}
                  <div style={{ fontSize: 11, color: "#888", marginTop: 4 }}>Active: {proxyEnabled ? "Yes" : "No"}</div>
                </div>
              ) : (
                <div>
                  <p style={{ color: "#888", fontSize: 12, marginBottom: 8 }}>Configure a proxy for agent web requests (SOCKS5/HTTP).</p>
                  <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
                    <select value={proxyType} onChange={e => setProxyType(e.target.value)}
                      style={{ ...inputStyle, width: 100, padding: "6px 8px", fontSize: 11 }}>
                      <option value="socks5">SOCKS5</option>
                      <option value="http">HTTP</option>
                      <option value="https">HTTPS</option>
                    </select>
                    <input value={proxyHost} onChange={e => setProxyHost(e.target.value)} placeholder="host" style={{ ...inputStyle, flex: 1 }} />
                    <input value={proxyPort} onChange={e => setProxyPort(e.target.value)} placeholder="port" style={{ ...inputStyle, width: 80 }} />
                    <button onClick={doAssignProxy} disabled={!proxyHost.trim()} style={btn("#a855f7")}>Set Proxy</button>
                  </div>
                </div>
              )}
            </div>

            {/* Media folder */}
            <div style={card}>
              <h4 style={{ color: "#ff2d55", margin: "0 0 12px" }}>🖼️ Media Folder</h4>
              <p style={{ fontSize: 12, color: "#888", marginBottom: 8 }}>Scan a folder for images, videos, PDFs that agents can reference in personalized messages.</p>
              <div style={{ display: "flex", gap: 8 }}>
                <input value={mediaFolder} onChange={e => setMediaFolder(e.target.value)} placeholder="/path/to/media" style={{ ...inputStyle, flex: 1 }} />
                <button onClick={doScanMedia} disabled={!mediaFolder.trim()} style={btn("#ff2d55")}>📁 Scan</button>
              </div>
              {mediaFiles.length > 0 && (
                <div style={{ marginTop: 12, maxHeight: 200, overflow: "auto" }}>
                  <div style={{ fontSize: 11, color: "#4caf50", marginBottom: 8 }}>{mediaFiles.length} files found</div>
                  {mediaFiles.map((f: any, i: number) => (
                    <div key={i} style={{ display: "flex", alignItems: "center", gap: 8, padding: "4px 0", borderBottom: "1px solid #1a1a1a", fontSize: 11 }}>
                      <span>{["png","jpg","jpeg","gif","webp","svg"].includes(f.file_type) ? "🖼️" : ["mp4","webm","mov"].includes(f.file_type) ? "🎬" : "📄"}</span>
                      <span style={{ color: "#e0e0e0", flex: 1 }}>{f.file_name}</span>
                      <span style={{ color: "#555" }}>{(f.file_size / 1024).toFixed(0)}KB</span>
                    </div>
                  ))}
                </div>
              )}
            </div>

            {/* Ollama status */}
            <div style={card}>
              <h4 style={{ color: "#ff6b35", margin: "0 0 8px" }}>🤖 Ollama Status</h4>
              <div style={{ fontSize: 13 }}>
                <div>Status: <span style={{ color: ollamaStatus?.online ? "#4caf50" : "#ef5350" }}>{ollamaStatus?.online ? "Online" : "Offline"}</span></div>
                <div>URL: {ollamaStatus?.url ?? "N/A"}</div>
                <div style={{ marginTop: 8 }}>Models loaded:</div>
                {ollamaStatus?.models?.map((m: any, i: number) => (
                  <div key={i} style={{ fontSize: 11, color: "#888", marginLeft: 12 }}>• {m.name}</div>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default AgentsPage;
