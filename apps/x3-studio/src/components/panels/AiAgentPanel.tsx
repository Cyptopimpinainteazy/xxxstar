import { useState, useEffect } from 'react';
import { useWorkspaceStore, useEditorStore, useAiConversationStore, useSettingsStore } from '../../store';

const AGENT_MODES = [
  'Architect', 'Builder', 'Auditor', 'Security Reviewer',
  'x3-lang Specialist', 'Cross-VM Adapter Specialist',
  'Relayer Specialist', 'Validator Specialist', 'Mainnet Gatekeeper',
];

interface ToolCall { tool: string; args: Record<string, string>; result?: string; }

function extractToolCalls(text: string): ToolCall[] {
  const calls: ToolCall[] = [];
  const regex = /```tool\s+(\w+)\s*\n([\s\S]*?)```/g;
  let match;
  while ((match = regex.exec(text)) !== null) {
    const tool = match[1];
    const args: Record<string, string> = {};
    for (const line of match[2].trim().split('\n')) {
      const ci = line.indexOf(':');
      if (ci > 0) { args[line.substring(0, ci).trim()] = line.substring(ci + 1).trim(); }
    }
    calls.push({ tool, args });
  }
  return calls;
}

export default function AiAgentPanel() {
  const aiEndpoint = useSettingsStore(s => s.aiEndpoint);
  const aiModel = useSettingsStore(s => s.aiModel);
  const saveConversations = useSettingsStore(s => s.saveConversations);
  const conversationDir = useSettingsStore(s => s.conversationDir);
  const workspacePath = useWorkspaceStore(s => s.workspacePath);
  const openFile = useEditorStore(s => s.openFile);
  const conversations = useAiConversationStore(s => s.conversations);
  const setConversations = useAiConversationStore(s => s.setConversations);
  const activeConvId = useAiConversationStore(s => s.activeConversationId);
  const setActiveConvId = useAiConversationStore(s => s.setActive);
  const addConversation = useAiConversationStore(s => s.addConversation);
  const [input, setInput] = useState('');
  const [mode, setMode] = useState('Builder');
  const [loading, setLoading] = useState(false);
  const [toolOutputs, setToolOutputs] = useState<string[]>([]);

  const activeConv = conversations.find(c => c.id === activeConvId);
  const messages = activeConv?.messages || [];

  useEffect(() => {
    if (messages.length === 0 && conversations.length === 0) {
      createNewConversation();
    }
  }, []);

  const createNewConversation = () => {
    const id = `conv-${Date.now()}`;
    addConversation({ id, mode, messages: [], created: new Date().toISOString(), updated: new Date().toISOString() });
    setActiveConvId(id);
  };

  const persistConversations = async () => {
    if (!saveConversations || !workspacePath) return;
    try {
      await window.x3studio.fs.writeFile(
        workspacePath + '/' + conversationDir + '/conversations.json',
        JSON.stringify(conversations, null, 2)
      );
    } catch {}
  };

  const loadConversations = async () => {
    if (!saveConversations || !workspacePath) return;
    try {
      const data = await window.x3studio.fs.readFile(workspacePath + '/' + conversationDir + '/conversations.json');
      const loaded = JSON.parse(data);
      if (Array.isArray(loaded) && loaded.length > 0) {
        setConversations(loaded);
        setActiveConvId(loaded[0].id);
      }
    } catch {}
  };

  const executeToolCalls = async (calls: ToolCall[]): Promise<string> => {
    const results: string[] = [];
    for (const call of calls) {
      try {
        switch (call.tool) {
          case 'read_file': {
            if (!workspacePath) { results.push('Error: No workspace open'); break; }
            const filePath = call.args.path || call.args.file;
            if (!filePath) { results.push('Error: No file path'); break; }
            const content = await window.x3studio.fs.readFile(workspacePath + '/' + filePath);
            results.push(`File ${filePath}:\n\`\`\`\n${content.substring(0, 5000)}\n\`\`\``);
            openFile(workspacePath + '/' + filePath, content, filePath.endsWith('.rs') ? 'rust' : filePath.endsWith('.sol') ? 'sol' : 'text');
            break;
          }
          case 'write_file': {
            if (!workspacePath) { results.push('Error: No workspace open'); break; }
            const wfPath = call.args.path || call.args.file;
            const content = call.args.content;
            if (!wfPath || !content) { results.push('Error: path and content required'); break; }
            await window.x3studio.fs.writeFile(workspacePath + '/' + wfPath, content);
            results.push(`✓ Written ${wfPath} (${content.length} bytes)`);
            break;
          }
          case 'list_dir': {
            if (!workspacePath) { results.push('Error: No workspace open'); break; }
            const dirPath = call.args.path || '.';
            const entries = await window.x3studio.fs.readDir(workspacePath + '/' + dirPath);
            results.push(`Directory ${dirPath}:\n${entries.map(e => e.isDirectory ? `📁 ${e.name}` : `📄 ${e.name}`).join('\n')}`);
            break;
          }
          case 'run_command': {
            if (!workspacePath) { results.push('Error: No workspace open'); break; }
            const cmd = call.args.command;
            if (!cmd) { results.push('Error: No command'); break; }
            const result = await window.x3studio.shell.exec(cmd, workspacePath);
            results.push(`$ ${cmd}\nExit: ${result.exitCode}\nStdout:\n${result.stdout.substring(0, 2000)}\nStderr:\n${result.stderr.substring(0, 1000)}`);
            break;
          }
          case 'create_file': {
            if (!workspacePath) { results.push('Error: No workspace open'); break; }
            const cfPath = call.args.path || call.args.file;
            if (!cfPath) { results.push('Error: No file path'); break; }
            await window.x3studio.fs.createFile(workspacePath + '/' + cfPath);
            results.push(`✓ Created: ${cfPath}`);
            break;
          }
          case 'edit_file': {
            if (!workspacePath) { results.push('Error: No workspace open'); break; }
            const efPath = call.args.path || call.args.file;
            const efSearch = call.args.search || call.args.find;
            const efReplace = call.args.replace || call.args.replacement;
            if (!efPath || !efSearch || efReplace === undefined) { results.push('Error: path, search, and replace required'); break; }
            const efContent = await window.x3studio.fs.readFile(workspacePath + '/' + efPath);
            const newContent = efContent.split(efSearch).join(efReplace);
            await window.x3studio.fs.writeFile(workspacePath + '/' + efPath, newContent);
            const changes = efContent !== newContent ? (efContent.split(efSearch).length - 1) : 0;
            results.push(`✓ Edited ${efPath}: ${changes} replacements of "${efSearch}" → "${efReplace}"`);
            break;
          }
          case 'delete_file': {
            if (!workspacePath) { results.push('Error: No workspace open'); break; }
            const dfPath = call.args.path || call.args.file;
            if (!dfPath) { results.push('Error: No file path'); break; }
            await window.x3studio.fs.deleteFile(workspacePath + '/' + dfPath);
            results.push(`✓ Deleted: ${dfPath}`);
            break;
          }
          case 'search_files': {
            if (!workspacePath) { results.push('Error: No workspace open'); break; }
            const pattern = call.args.pattern || call.args.query;
            if (!pattern) { results.push('Error: No pattern'); break; }
            const globPat = call.args.glob || '**/*.{rs,sol,ts,js,py,x3,toml}';
            const files = await window.x3studio.fs.glob(workspacePath, globPat);
            results.push(`Found ${files.filter(f => f.includes(pattern)).length} files matching "${pattern}"`);
            break;
          }
          default: results.push(`Unknown tool: ${call.tool}`);
        }
      } catch (e: any) { results.push(`Tool error (${call.tool}): ${e.message}`); }
    }
    setToolOutputs(prev => [...prev, ...results]);
    return results.join('\n\n');
  };

  const sendMessage = async () => {
    if (!input.trim() || !activeConvId) return;
    const userMsg = input.trim();
    setInput('');

    const updatedMessages = [...messages, { role: 'user', content: userMsg }];
    updateConversation(updatedMessages);
    setLoading(true);

    const systemPrompt = `You are X3 Studio AI Agent in "${mode}" mode.
You help build X3 blockchain infrastructure. Never claim success without running commands.
Wrap tool calls in: \`\`\`tool tool_name\nkey: value\n\`\`\`
Available tools: read_file, write_file, create_file, edit_file, delete_file, list_dir, run_command, search_files
Workspace: ${workspacePath || 'N/A'}`;

    try {
      const resp = await fetch(`${aiEndpoint}/api/chat`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: aiModel,
          messages: [
            { role: 'system', content: systemPrompt },
            ...updatedMessages.map(m => ({ role: m.role, content: m.content })),
            { role: 'user', content: userMsg },
          ],
          stream: false,
        }),
      });
      const data = await resp.json();
      let response = data.message?.content || data.response || JSON.stringify(data);
      const toolCalls = extractToolCalls(response);
      if (toolCalls.length > 0) {
        const toolResults = await executeToolCalls(toolCalls);
        response += `\n\n**Tool Results:**\n${toolResults}`;
      }
      updateConversation([...updatedMessages, { role: 'assistant', content: response }]);
    } catch (err: any) {
      updateConversation([...updatedMessages, { role: 'assistant', content: `**API Error**: ${err.message}\nCheck that ${aiEndpoint} is running.` }]);
    }
    setLoading(false);
    persistConversations();
  };

  const updateConversation = (msgs: { role: string; content: string }[]) => {
    const cs = [...conversations];
    const idx = cs.findIndex(c => c.id === activeConvId);
    if (idx >= 0) {
      cs[idx] = { ...cs[idx], messages: msgs, updated: new Date().toISOString() };
      setConversations(cs);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>AI Agent</span>
        <select className="select-field" value={mode} onChange={e => setMode(e.target.value)}
          style={{ width: 'auto', fontSize: 10, padding: '2px 6px' }}>
          {AGENT_MODES.map(m => <option key={m}>{m}</option>)}
        </select>
      </div>
      <div style={{ display: 'flex', gap: 4, padding: '4px 8px', borderBottom: '1px solid var(--border-color)' }}>
        <select className="select-field" style={{ flex: 1, fontSize: 10 }}
          value={activeConvId || ''} onChange={e => setActiveConvId(e.target.value)}>
          {conversations.map(c => <option key={c.id} value={c.id}>{c.mode} - {c.messages.length} msgs</option>)}
        </select>
        <button className="btn" style={{ fontSize: 10 }} onClick={createNewConversation}>+ New</button>
        <button className="btn" style={{ fontSize: 10 }} onClick={loadConversations}>Load</button>
      </div>
      <div className="ai-chat panel-body">
        <div className="ai-messages">
          {messages.length === 0 && (
            <div style={{ color: 'var(--text-muted)', fontSize: 'var(--font-size-sm)', padding: 16, textAlign: 'center' }}>
              {aiEndpoint === 'http://localhost:11434'
                ? 'Connect to Ollama, LM Studio, or OpenAI. Configure in Settings.'
                : `Connected to ${aiEndpoint}.`}
            </div>
          )}
          {messages.map((m, i) => (
            <div key={i} className={`ai-message ${m.role}`}>
              <div style={{ fontWeight: 600, marginBottom: 4, fontSize: 'var(--font-size-sm)' }}>{m.role === 'user' ? 'You' : mode}</div>
              <div style={{ whiteSpace: 'pre-wrap', fontSize: 'var(--font-size-sm)' }}>{m.content}</div>
            </div>
          ))}
          {loading && <div className="ai-message assistant"><div className="spinner" /></div>}
        </div>
        {toolOutputs.length > 0 && (
          <div style={{ padding: '4px 8px', borderTop: '1px solid var(--border-color)', maxHeight: 60, overflow: 'auto', fontSize: 10, color: 'var(--text-muted)' }}>
            {toolOutputs.slice(-3).map((o, i) => <div key={i} className="tree-node">{o.substring(0, 100)}</div>)}
          </div>
        )}
        <div className="ai-input-bar">
          <input className="input-field" value={input} onChange={e => setInput(e.target.value)}
            placeholder={`Ask ${mode} to read/write files, run commands...`}
            onKeyDown={e => e.key === 'Enter' && !e.shiftKey && sendMessage()} disabled={loading} />
          <button className="btn btn-primary" onClick={sendMessage} disabled={loading || !input.trim()}>Send</button>
        </div>
      </div>
    </div>
  );
}
