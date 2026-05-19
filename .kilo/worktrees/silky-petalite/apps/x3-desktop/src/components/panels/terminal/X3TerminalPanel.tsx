import React, { useState } from 'react';
import { Terminal, Play, History, BookOpen, Code, Zap } from 'lucide-react';

interface Command {
  id: string;
  command: string;
  description: string;
  category: 'wallet' | 'stake' | 'deploy' | 'query' | 'contract';
  example: string;
  output: string;
}

export const X3TerminalPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'shell' | 'cli-ref' | 'repl' | 'history'>('shell');
  const [terminalInput, setTerminalInput] = useState('');
  const [terminalOutput, setTerminalOutput] = useState<string[]>(['x3-shell v1.0.0', 'Type "help" for commands', '']);
  const [commandHistory, setCommandHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);

  const commands: Command[] = [
    {
      id: '1',
      command: 'x3 send <recipient> <amount>',
      description: 'Send X3 tokens to an address',
      category: 'wallet',
      example: 'x3 send x3:alice 100.5',
      output: 'TX: 0xabcd... confirmed in 2 blocks',
    },
    {
      id: '2',
      command: 'x3 stake <validator> <amount>',
      description: 'Delegate stake to a validator',
      category: 'stake',
      example: 'x3 stake x3:validator-01 1000',
      output: 'Staking 1000 X3 to validator-01... Confirmed',
    },
    {
      id: '3',
      command: 'x3 deploy <file.x3>',
      description: 'Deploy an X3-Lang contract',
      category: 'deploy',
      example: 'x3 deploy contracts/token.x3',
      output: 'Compiling... Deployed to x3:contract:abc123',
    },
    {
      id: '4',
      command: 'x3 query <contract> <method> [args]',
      description: 'Query a contract method (read-only)',
      category: 'query',
      example: 'x3 query x3:token:abc balance x3:alice',
      output: '1500.25 tokens',
    },
    {
      id: '5',
      command: 'x3 balance <address>',
      description: 'Check wallet balance',
      category: 'wallet',
      example: 'x3 balance',
      output: '3,245.50 X3 (≈$4,056.88 USD)',
    },
    {
      id: '6',
      command: 'x3 call <contract> <method> [args]',
      description: 'Call a contract method (state-changing)',
      category: 'contract',
      example: 'x3 call x3:token:abc transfer alice 100',
      output: 'TX: 0xdef4... broadcasted',
    },
    {
      id: '7',
      command: 'x3 compile <file.x3>',
      description: 'Compile X3-Lang to WASM bytecode',
      category: 'deploy',
      example: 'x3 compile src/dex.x3',
      output: 'Bytecode: 128.5 KB, Gas cost estimate: 250M',
    },
    {
      id: '8',
      command: 'x3 nft mint <collection> <metadata.json>',
      description: 'Mint an NFT',
      category: 'contract',
      example: 'x3 nft mint art metadata.json',
      output: 'NFT minted: x3:nft:xyz789 (ID: 1)',
    },
  ];

  const cliHistory = [
    '$ x3 balance',
    '3,245.50 X3',
    '$ x3 stake x3:validator-05 500',
    'TX: 0x7f3e9... confirmed in block 12450',
    '$ x3 query x3:dex:token0 reserves',
    '{reserve_x3: 1250000, reserve_usdc: 562500}',
  ];

  const handleTerminalCommand = (input: string) => {
    const newOutput = [...terminalOutput, `$ ${input}`];
    
    // Simple command simulation
    if (input === 'help') {
      newOutput.push('x3 send, stake, deploy, query, balance, call, compile, nft mint');
    } else if (input.startsWith('x3 balance')) {
      newOutput.push('3,245.50 X3 (≈$4,056.88 USD)');
    } else if (input === 'clear') {
      setTerminalOutput(['x3-shell v1.0.0\n']);
      setTerminalInput('');
      return;
    } else {
      newOutput.push(`executing: ${input}`);
    }
    
    newOutput.push('');
    setTerminalOutput(newOutput);
    setCommandHistory([...commandHistory, input]);
    setTerminalInput('');
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      if (terminalInput.trim()) {
        handleTerminalCommand(terminalInput);
      }
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (historyIndex < commandHistory.length - 1) {
        const newIndex = historyIndex + 1;
        setHistoryIndex(newIndex);
        setTerminalInput(commandHistory[commandHistory.length - 1 - newIndex]);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex > 0) {
        const newIndex = historyIndex - 1;
        setHistoryIndex(newIndex);
        setTerminalInput(commandHistory[commandHistory.length - 1 - newIndex]);
      } else if (historyIndex === 0) {
        setHistoryIndex(-1);
        setTerminalInput('');
      }
    }
  };

  return (
    <div className="w-full h-full flex flex-col bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] border-l border-[#2a2a35]">
      {/* Header */}
      <div className="px-6 py-4 border-b border-[#2a2a35] bg-gradient-to-r from-cyan-500/20 to-blue-500/20">
        <div className="flex items-center gap-3 mb-2">
          <Terminal className="w-5 h-5 text-cyan-400" />
          <h1 className="text-lg font-bold text-white">X3 Terminal Shell</h1>
        </div>
        <p className="text-sm text-gray-400">PTY shell, X3 CLI, autocomplete, command history, X3-Lang REPL</p>
      </div>

      {/* Tab Navigation */}
      <div className="flex gap-6 px-6 py-3 border-b border-[#2a2a35] bg-[#0f0f15]/50">
        {(['shell', 'cli-ref', 'repl', 'history'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`flex items-center gap-2 px-3 py-2 text-sm font-medium transition ${
              activeTab === tab
                ? 'text-cyan-400 border-b-2 border-cyan-400'
                : 'text-gray-400 hover:text-gray-300'
            }`}
          >
            {tab === 'shell' && <Terminal className="w-4 h-4" />}
            {tab === 'cli-ref' && <BookOpen className="w-4 h-4" />}
            {tab === 'repl' && <Code className="w-4 h-4" />}
            {tab === 'history' && <History className="w-4 h-4" />}
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {activeTab === 'shell' && (
          <div className="p-6 space-y-4">
            <div className="font-mono text-sm bg-[#0f0f15] border border-[#2a2a35] rounded-lg p-4 h-96 overflow-y-auto flex flex-col">
              {terminalOutput.map((line, idx) => (
                <div key={idx} className="text-gray-300">
                  {line}
                </div>
              ))}
              <div className="flex-1" />
            </div>
            <div className="flex items-center gap-2 bg-[#0f0f15] border border-[#2a2a35] rounded-lg px-3 py-2">
              <span className="text-cyan-400">$</span>
              <input
                type="text"
                value={terminalInput}
                onChange={(e) => setTerminalInput(e.target.value)}
                onKeyDown={handleKeyDown}
                placeholder="Type command (x3 send, stake, deploy, query...)"
                className="flex-1 bg-transparent text-white outline-none text-sm font-mono"
                autoFocus
              />
            </div>
            <div className="text-xs text-gray-500">
              Use ↑/↓ for history • Type "help" for commands • "x3 balance" to check wallet
            </div>
          </div>
        )}

        {activeTab === 'cli-ref' && (
          <div className="p-6 space-y-4">
            <div className="grid grid-cols-1 gap-4">
              {commands.map((cmd) => (
                <div key={cmd.id} className="p-4 border border-[#2a2a35] rounded-lg hover:border-cyan-500/30 transition">
                  <div className="flex items-start justify-between mb-2">
                    <code className="text-cyan-400 font-mono text-sm">{cmd.command}</code>
                    <span className="px-2 py-1 text-xs bg-[#2a2a35] text-gray-400 rounded">{cmd.category}</span>
                  </div>
                  <p className="text-sm text-gray-400 mb-2">{cmd.description}</p>
                  <div className="space-y-1 text-xs font-mono">
                    <div className="text-gray-500">
                      <span className="text-emerald-400">Example:</span> {cmd.example}
                    </div>
                    <div className="text-cyan-300">
                      <span className="text-emerald-400">Output:</span> {cmd.output}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {activeTab === 'repl' && (
          <div className="p-6 space-y-4">
            <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
              <div className="text-sm text-gray-400 mb-3">X3-Lang REPL Environment</div>
              <div className="font-mono text-sm space-y-2">
                <div className="text-cyan-400">x3&gt; let a = 100u64;</div>
                <div className="text-gray-400">variable 'a' defined: u64</div>
                <div className="text-cyan-400">x3&gt; let b = a + 50;</div>
                <div className="text-gray-400">variable 'b' computed: 150</div>
                <div className="text-cyan-400">x3&gt; #[derive(Contract)] pub struct Token &#123; supply: u64 &#125;</div>
                <div className="text-gray-400">Contract interface registered</div>
                <div className="text-cyan-400">x3&gt; let token = Token &#123; supply: 1_000_000 &#125;;</div>
                <div className="text-gray-400">instance created, bytecode: 45.2 KB</div>
              </div>
            </div>
            <div className="text-xs text-gray-500">
              REPL auto-compiles X3-Lang code. Type any valid X3 expression to execute. Use #[derive...] to define contracts.
            </div>
            <div className="border-t border-[#2a2a35] pt-4">
              <h3 className="text-sm font-semibold text-white mb-3">Keyboard Shortcuts</h3>
              <div className="grid grid-cols-2 gap-2 text-xs text-gray-400">
                <div><code className="text-cyan-400">Ctrl+L</code> — Clear screen</div>
                <div><code className="text-cyan-400">Ctrl+C</code> — Cancel execution</div>
                <div><code className="text-cyan-400">Tab</code> — Autocomplete</div>
                <div><code className="text-cyan-400">↑/↓</code> — Command history</div>
              </div>
            </div>
          </div>
        )}

        {activeTab === 'history' && (
          <div className="p-6 space-y-3">
            <div className="text-sm text-gray-400 mb-4">Recent Commands (Session: {new Date().toLocaleString()})</div>
            {cliHistory.map((cmd, idx) => (
              <div key={idx} className="font-mono text-sm p-3 bg-[#0f0f15] border border-[#2a2a35] rounded hover:border-cyan-500/30 cursor-pointer transition">
                <span className="text-cyan-400">{cmd.includes('$') ? '$ ' : '  '}</span>
                <span className={cmd.includes('$') ? 'text-gray-300' : 'text-emerald-300'}>
                  {cmd.replace('$ ', '')}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default X3TerminalPanel;
