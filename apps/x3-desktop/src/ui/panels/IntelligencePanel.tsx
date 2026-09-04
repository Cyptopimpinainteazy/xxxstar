import { useEffect, useState, useCallback, useRef } from 'react';
import { invoke } from '../../ipc/tauri';

type Tab = 'chat' | 'forge' | 'roster';

interface ChatMessage {
  role: 'user' | 'ai';
  text: string;
}

interface RosterAgent {
  id?: string;
  name?: string;
  role?: string;
  capabilities?: string[];
}

function IntelligencePanel() {
  const [activeTab, setActiveTab] = useState<Tab>('chat');
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [chatInput, setChatInput] = useState('');
  const [chatLoading, setChatLoading] = useState(false);
  const [forgeDesc, setForgeDesc] = useState('');
  const [forgeResult, setForgeResult] = useState<string | null>(null);
  const [forgeLoading, setForgeLoading] = useState(false);
  const [roster, setRoster] = useState<RosterAgent[]>([]);
  const [rosterLoading, setRosterLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const chatEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [chatMessages]);

  // Chat handlers
  const handleSend = useCallback(async () => {
    const msg = chatInput.trim();
    if (!msg || chatLoading) return;
    setChatInput('');
    setChatMessages((prev) => [...prev, { role: 'user', text: msg }]);
    setChatLoading(true);
    setError(null);
    try {
      const response = await invoke<string>('agents_chat', {
        userId: 'desktop',
        agentId: 'x3-agent',
        message: msg,
      });
      setChatMessages((prev) => [...prev, { role: 'ai', text: response || 'No response' }]);
    } catch (err) {
      console.error('Chat error:', err);
      setError('AI chat request failed');
      setChatMessages((prev) => [...prev, { role: 'ai', text: 'Error: AI chat unreachable' }]);
    } finally {
      setChatLoading(false);
    }
  }, [chatInput, chatLoading]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        handleSend();
      }
    },
    [handleSend]
  );

  // Forge handlers
  const handleForge = useCallback(async () => {
    const desc = forgeDesc.trim();
    if (!desc || forgeLoading) return;
    setForgeLoading(true);
    setForgeResult(null);
    setError(null);
    try {
      const result = await invoke<string>('foundry_generate', { description: desc });
      setForgeResult(result || 'No result returned');
    } catch (err) {
      console.error('Forge error:', err);
      setError('dApp generation failed');
      setForgeResult('Error: generation service unreachable');
    } finally {
      setForgeLoading(false);
    }
  }, [forgeDesc, forgeLoading]);

  // Roster fetch
  const fetchRoster = useCallback(async () => {
    try {
      const result = await invoke<RosterAgent[]>('agents_get_roster');
      if (Array.isArray(result)) {
        setRoster(result);
      }
      setError(null);
    } catch (err) {
      console.error('Roster error:', err);
      setError('Failed to fetch agent roster');
    } finally {
      setRosterLoading(false);
    }
  }, []);

  useEffect(() => {
    if (activeTab === 'roster') {
      fetchRoster();
    }
  }, [activeTab, fetchRoster]);

  const tabs: { id: Tab; label: string }[] = [
    { id: 'chat', label: 'AI Chat' },
    { id: 'forge', label: 'dApp Forge' },
    { id: 'roster', label: 'Agent Roster' },
  ];

  return (
    <div className="view p-6">
      <div className="mb-4">
        <h2 className="text-xl font-bold text-white">Intelligence</h2>
        <p className="text-gray-400 text-sm">AI chat, dApp forge, and agent management</p>
      </div>

      {error && (
        <div className="bg-red-900/30 border border-red-600/30 rounded-lg p-3 mb-4 text-red-300 text-sm">
          {error}
        </div>
      )}

      {/* Tab bar */}
      <div className="flex gap-1 mb-4 border-b border-white/10 pb-2">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveTab(tab.id)}
            className={`px-4 py-1.5 text-xs rounded transition-colors ${
              activeTab === tab.id
                ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/30'
                : 'text-gray-500 hover:text-gray-300 hover:bg-white/5'
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* AI Chat Tab */}
      {activeTab === 'chat' && (
        <div className="flex flex-col h-[500px]">
          <div className="flex-1 overflow-y-auto mb-3 space-y-3 pr-1">
            {chatMessages.length === 0 && (
              <div className="text-gray-500 text-sm text-center mt-20">
                Start a conversation with the X3 AI agent.
              </div>
            )}
            {chatMessages.map((msg, i) => (
              <div key={i} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                <div
                  className={`max-w-[80%] rounded-xl px-4 py-2 text-sm ${
                    msg.role === 'user'
                      ? 'bg-cyan-500/20 border border-cyan-500/30 text-cyan-200'
                      : 'bg-white/5 border border-white/10 text-gray-200'
                  }`}
                  style={{
                    backdropFilter: 'blur(12px)',
                    WebkitBackdropFilter: 'blur(12px)',
                  }}
                >
                  {msg.text}
                </div>
              </div>
            ))}
            {chatLoading && (
              <div className="flex justify-start">
                <div className="bg-white/5 border border-white/10 rounded-xl px-4 py-2 text-sm text-gray-400 animate-pulse">
                  AI is thinking...
                </div>
              </div>
            )}
            <div ref={chatEndRef} />
          </div>
          <div className="flex gap-2">
            <input
              type="text"
              value={chatInput}
              onChange={(e) => setChatInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Type a message..."
              className="flex-1 bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white placeholder-gray-500 focus:outline-none focus:border-cyan-500/50"
            />
            <button
              onClick={handleSend}
              disabled={chatLoading || !chatInput.trim()}
              className="px-4 py-2 bg-cyan-600/30 border border-cyan-500/40 rounded-lg text-cyan-300 text-sm hover:bg-cyan-600/50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              Send
            </button>
          </div>
        </div>
      )}

      {/* dApp Forge Tab */}
      {activeTab === 'forge' && (
        <div className="space-y-3">
          <textarea
            value={forgeDesc}
            onChange={(e) => setForgeDesc(e.target.value)}
            placeholder="Describe the dApp you want to generate..."
            rows={5}
            className="w-full bg-white/5 border border-white/10 rounded-lg px-3 py-2 text-sm text-white placeholder-gray-500 focus:outline-none focus:border-cyan-500/50 resize-none"
          />
          <button
            onClick={handleForge}
            disabled={forgeLoading || !forgeDesc.trim()}
            className="px-4 py-2 bg-cyan-600/30 border border-cyan-500/40 rounded-lg text-cyan-300 text-sm hover:bg-cyan-600/50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
          >
            {forgeLoading ? 'Generating...' : 'Generate'}
          </button>
          {forgeResult && (
            <div
              className="bg-white/5 border border-white/10 rounded-xl p-4 text-sm text-gray-200 font-mono whitespace-pre-wrap max-h-80 overflow-y-auto"
              style={{
                backdropFilter: 'blur(12px)',
                WebkitBackdropFilter: 'blur(12px)',
              }}
            >
              {forgeResult}
            </div>
          )}
        </div>
      )}

      {/* Agent Roster Tab */}
      {activeTab === 'roster' && (
        <div>
          {rosterLoading ? (
            <div className="text-gray-400 text-sm">Loading agent roster...</div>
          ) : roster.length === 0 ? (
            <div className="text-gray-500 text-sm">No agents found.</div>
          ) : (
            <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
              {roster.map((agent, i) => (
                <div
                  key={agent.id || i}
                  className="bg-white/5 border border-white/10 rounded-xl p-4"
                  style={{
                    backdropFilter: 'blur(12px)',
                    WebkitBackdropFilter: 'blur(12px)',
                  }}
                >
                  <div className="text-white font-semibold text-sm mb-1">
                    {agent.name || agent.id || `Agent #${i + 1}`}
                  </div>
                  <div className="text-gray-400 text-xs mb-2">{agent.role || 'Unknown role'}</div>
                  {agent.capabilities && agent.capabilities.length > 0 && (
                    <div className="flex flex-wrap gap-1">
                      {agent.capabilities.map((cap, j) => (
                        <span
                          key={j}
                          className="bg-cyan-500/10 border border-cyan-500/20 rounded px-2 py-0.5 text-[10px] text-cyan-300"
                        >
                          {cap}
                        </span>
                      ))}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default IntelligencePanel;
