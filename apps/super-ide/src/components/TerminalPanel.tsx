import { useState, useRef, useEffect, type KeyboardEvent } from 'react';
import { Terminal } from 'lucide-react';

export function TerminalPanel() {
  const [lines, setLines] = useState<string[]>([
    'X3 Super IDE Terminal v0.2.0',
    'X3 Chain Developer Console',
    'Type "help" for commands.',
    '---',
  ]);
  const [input, setInput] = useState('');
  const [cmdHistory, setCmdHistory] = useState<string[]>([]);
  const [historyIdx, setHistoryIdx] = useState(-1);
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => { endRef.current?.scrollIntoView() }, [lines]);

  const execute = (cmd: string) => {
    const trimmed = cmd.trim();
    if (!trimmed) return;
    setCmdHistory(prev => [...prev, trimmed]);
    setHistoryIdx(-1);

    const newLines = [...lines, `$ ${trimmed}`];
    const args = trimmed.split(/\s+/);
    const command = args[0].toLowerCase();

    switch (command) {
      case 'help':
        newLines.push(
          '  help                          Show this help',
          '  clear                         Clear terminal',
          '  status                        System status',
          '  blocks <n>                    Show recent n blocks',
          '  peers                         Show peer count',
          '  rpc <method> [params...]      Call RPC method',
          '  compile <lang>                Compile code (x3|solidity)',
          '  accounts                      List accounts',
          '  balance <addr>                Check balance',
          '  deploy <name> <bytecode>      Deploy contract',
          '  events <addr>                 Get contract events',
          '  echo <text>                   Echo text',
          '  history                       Command history',
        );
        break;
      case 'clear':
        setLines([]);
        return;
      case 'status':
        newLines.push('  System:  X3 Chain Super IDE');
        newLines.push('  API:     http://127.0.0.1:8765');
        newLines.push('  RPC:     http://127.0.0.1:9933');
        newLines.push('  Repo:    xxxstar-main');
        newLines.push('  Status:  Running');
        break;
      case 'blocks':
        fetch('http://127.0.0.1:8765/api/explorer/blocks?limit=5')
          .then(r => r.json())
          .then(data => setLines(prev => [...prev, ...data.map((b: { number: number; hash: string; txCount: number }) => `  #${b.number}  ${b.hash.slice(0, 16)}...  ${b.txCount} txns`)]))
          .catch(() => setLines(prev => [...prev, '  Error: API unavailable']));
        break;
      case 'peers':
        fetch('http://127.0.0.1:8765/api/network/status')
          .then(r => r.json())
          .then(data => { newLines.push(`  Peers: ${data.peers}`, `  Syncing: ${data.syncing}`); setLines(newLines); })
          .catch(() => { newLines.push('  Error: API unavailable'); setLines(newLines); });
        return;
      case 'rpc':
        if (args.length < 2) { newLines.push('  Usage: rpc <method> [params...]'); break; }
        (async () => {
          try {
            const params = args.slice(2).length ? args.slice(2).map(p => { try { return JSON.parse(p) } catch { return p } }) : [];
            const res = await fetch('http://127.0.0.1:8765/api/rpc', {
              method: 'POST', headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ jsonrpc: '2.0', method: args[1], params, id: 1 }),
            });
            const data = await res.json();
            setLines(prev => [...prev, `  ${JSON.stringify(data, null, 2)}`]);
          } catch (e) { setLines(prev => [...prev, `  Error: ${e}`]); }
        })();
        return;
      case 'accounts':
        fetch('http://127.0.0.1:8765/api/accounts')
          .then(r => r.json())
          .then(data => setLines(prev => [...prev, ...data.map((a: { label: string; address: string; balance: string }) => `  ${a.label || '?'}: ${a.address.slice(0, 12)}...  ${a.balance} X3`)]))
          .catch(() => setLines(prev => [...prev, '  Error: API unavailable']));
        break;
      case 'balance':
        if (args.length < 2) { newLines.push('  Usage: balance <address>'); break; }
        fetch(`http://127.0.0.1:8765/api/inspect/balance?address=${args[1]}`)
          .then(r => r.json())
          .then(d => newLines.push(`  Balance: ${d.balance} X3`))
          .catch(() => newLines.push('  Error fetching balance'));
        break;
      case 'compile':
        newLines.push('  Use the Compiler panel in the sidebar for compilation.');
        break;
      case 'echo':
        newLines.push(`  ${args.slice(1).join(' ')}`);
        break;
      case 'history':
        if (cmdHistory.length === 0) newLines.push('  No commands.');
        else cmdHistory.forEach((c, i) => newLines.push(`  ${i + 1}. ${c}`));
        break;
      default:
        newLines.push(`  Unknown: ${command}. Try "help".`);
    }
    setLines(newLines);
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') { execute(input); setInput(''); }
    else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (cmdHistory.length > 0) {
        const newIdx = historyIdx < 0 ? cmdHistory.length - 1 : Math.max(0, historyIdx - 1);
        setHistoryIdx(newIdx);
        setInput(cmdHistory[newIdx]);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIdx >= 0) {
        const newIdx = historyIdx + 1;
        if (newIdx >= cmdHistory.length) { setHistoryIdx(-1); setInput(''); }
        else { setHistoryIdx(newIdx); setInput(cmdHistory[newIdx]); }
      }
    }
  };

  return (
    <div style={{ height: '100%', display: 'flex', flexDirection: 'column', color: '#d4d4d4' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 16px', borderBottom: '1px solid #333', background: '#252526', fontSize: 12 }}>
        <Terminal size={14} /> <span>Terminal — X3 Console</span>
      </div>
      <div style={{ flex: 1, overflow: 'auto', padding: 8, fontFamily: "'Fira Code', 'Consolas', monospace", fontSize: 13, lineHeight: 1.5, background: '#1e1e1e' }}>
        {lines.map((line, i) => (
          <div key={i} style={{ color: line.startsWith('$') ? '#4ec9b0' : line.startsWith('  Error') ? '#f48771' : line.startsWith('  ') ? '#d4d4d4' : '#888' }}>{line}</div>
        ))}
        <div style={{ display: 'flex', alignItems: 'center' }}>
          <span style={{ color: '#4ec9b0' }}>$</span>
          <input value={input} onChange={e => setInput(e.target.value)} onKeyDown={handleKeyDown}
            spellCheck={false} autoFocus
            style={{ flex: 1, background: 'transparent', border: 'none', outline: 'none', color: '#d4d4d4', fontFamily: 'inherit', fontSize: 13, marginLeft: 8 }}
          />
        </div>
        <div ref={endRef} />
      </div>
    </div>
  );
}
