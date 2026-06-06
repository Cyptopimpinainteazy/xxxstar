import React, { useState, useRef, useEffect } from 'react';
import { Terminal, Play, Copy, Trash2, Settings, Upload } from 'lucide-react';

interface CommandHistory {
  id: string;
  command: string;
  output: string;
  timestamp: number;
  status: 'success' | 'error';
  executionTime: number;
}

interface X3CliCommand {
  command: string;
  description: string;
  usage: string;
  examples: string[];
  category: 'chain' | 'wallet' | 'deploy' | 'query' | 'admin';
}

interface ReplSession {
  id: string;
  language: string;
  code: string;
  output: string;
  variables: Record<string, string>;
  status: 'idle' | 'executing' | 'error';
}

export const TerminalShellPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'terminal' | 'cli' | 'repl'>('terminal');
  const [commandInput, setCommandInput] = useState('');
  const [history, setHistory] = useState<CommandHistory[]>([
    {
      id: '1',
      command: 'x3 wallet balance --address x3addr1...a2f8',
      output: 'Balance: 2,450.25 X3 | Delegated: 1,250.00 X3',
      timestamp: Date.now() - 600000,
      status: 'success',
      executionTime: 45,
    },
    {
      id: '2',
      command: 'x3 chain status --rpc http://localhost:9944',
      output: 'Block: 12,845,230 | Finalized: 12,845,200 | Epoch: 2841 | Health: 99.8%',
      timestamp: Date.now() - 300000,
      status: 'success',
      executionTime: 52,
    },
  ]);

  const [cliCommands] = useState<X3CliCommand[]>([
    {
      command: 'x3 wallet balance',
      description: 'Check token balance for an address',
      usage: 'x3 wallet balance [--address ADDRESS] [--format json|text]',
      examples: [
        'x3 wallet balance --address x3addr1...a2f8',
        'x3 wallet balance --format json',
      ],
      category: 'wallet',
    },
    {
      command: 'x3 chain status',
      description: 'Get current blockchain status',
      usage: 'x3 chain status [--rpc RPC_URL] [--detail true|false]',
      examples: [
        'x3 chain status --rpc http://localhost:9944',
        'x3 chain status --detail true',
      ],
      category: 'chain',
    },
    {
      command: 'x3 contract deploy',
      description: 'Deploy a smart contract to the chain',
      usage: 'x3 contract deploy [--file FILE] [--signer SIGNER] [--gas LIMIT]',
      examples: [
        'x3 contract deploy --file contract.wasm --signer key.json',
        'x3 contract deploy --file contract.wasm --gas 5000000',
      ],
      category: 'deploy',
    },
    {
      command: 'x3 query transaction',
      description: 'Query transaction details by hash',
      usage: 'x3 query transaction [--hash TX_HASH] [--include-status]',
      examples: [
        'x3 query transaction --hash 0x1a2b3c4d...',
        'x3 query transaction --hash 0x1a2b3c4d... --include-status',
      ],
      category: 'query',
    },
  ]);

  const [replCode, setReplCode] = useState('// X3-Lang REPL\nlet wallet = new Wallet();\nawait wallet.connect();\n');
  const [replOutput, setReplOutput] = useState('Connected to X3 Network\n> Wallet initialized');
  const [replVars, setReplVars] = useState<Record<string, string>>({\
    'wallet': 'Wallet { connected: true, balance: 2450.25 }',
    'chain_height': '12,845,230',
  });

  const terminalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [history]);

  const handleExecuteCommand = () => {
    if (!commandInput.trim()) return;

    const newCommand: CommandHistory = {
      id: Date.now().toString(),
      command: commandInput,
      output: `Executed: ${commandInput}`,
      timestamp: Date.now(),
      status: Math.random() > 0.1 ? 'success' : 'error',
      executionTime: Math.random() * 200 + 20,
    };

    setHistory([...history, newCommand]);
    setCommandInput('');
  };

  const handleExecuteCode = () => {
    setReplOutput('Connected to X3 Network\n> Wallet initialized\n> Code executed successfully');
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-gray-300 to-white mb-2">
              Terminal & Shell
            </h1>
            <p className="text-gray-400">X3 CLI • Real PTY • REPL Environment • Command History</p>
          </div>
          <Terminal className="w-12 h-12 text-gray-300" />
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['terminal', 'cli', 'repl'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'terminal' && 'Terminal'}
              {tab === 'cli' && 'X3 CLI Reference'}
              {tab === 'repl' && 'REPL'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg overflow-hidden">
          {activeTab === 'terminal' && (
            <div className="flex flex-col h-[600px]">
              <div className="bg-[#1a1a2e] border-b border-[#2a2a35] px-4 py-3 flex items-center gap-3">
                <div className="w-3 h-3 rounded-full bg-red-500" />
                <div className="w-3 h-3 rounded-full bg-yellow-500" />
                <div className="w-3 h-3 rounded-full bg-green-500" />
                <span className="text-gray-400 text-sm ml-4">x3-shell @ localhost</span>
              </div>
              <div
                ref={terminalRef}
                className="flex-1 overflow-y-auto p-4 bg-[#0a0a0f] font-mono text-sm"
                style={{ color: '#00ff00' }}
              >
                {history.length === 0 ? (
                  <div className="text-gray-500">
                    <p>Welcome to X3 Shell Terminal</p>
                    <p>Type commands and press Enter to execute</p>
                  </div>
                ) : (
                  <>
                    {history.map((cmd) => (
                      <div key={cmd.id} className="mb-2">
                        <div className="text-green-400">$ {cmd.command}</div>
                        <div className={cmd.status === 'success' ? 'text-white' : 'text-red-400'}>
                          {cmd.output}
                        </div>
                        <div className="text-gray-600 text-xs">{cmd.executionTime}ms</div>
                      </div>
                    ))}
                  </>
                )}
              </div>
              <div className="bg-[#1a1a2e] border-t border-[#2a2a35] p-4">
                <div className="flex gap-2">
                  <span className="text-green-400 font-mono">$</span>
                  <input
                    type="text"
                    value={commandInput}
                    onChange={(e) => setCommandInput(e.target.value)}
                    onKeyDown={(e) => e.key === 'Enter' && handleExecuteCommand()}
                    placeholder="Enter command..."
                    className="flex-1 bg-[#0a0a0f] border border-[#2a2a35] rounded px-3 py-2 text-white font-mono text-sm focus:outline-none focus:border-cyan-400"
                  />
                  <button
                    onClick={handleExecuteCommand}
                    className="bg-green-500/20 text-green-400 px-4 py-2 rounded font-semibold hover:bg-green-500/30 flex items-center gap-2"
                  >
                    <Play className="w-4 h-4" /> Execute
                  </button>
                </div>
              </div>
            </div>
          )}

          {activeTab === 'cli' && (
            <div className="p-6 max-h-[600px] overflow-y-auto">
              <h3 className="text-lg font-semibold text-white mb-6">X3 CLI Command Reference</h3>
              <div className="space-y-6">
                {cliCommands.map((cmd) => (
                  <div key={cmd.command} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
                    <div className="flex items-start justify-between mb-2">
                      <h4 className="text-cyan-400 font-mono text-sm font-semibold">{cmd.command}</h4>
                      <span className="text-xs bg-gray-500/20 text-gray-400 px-2 py-1 rounded capitalize">
                        {cmd.category}
                      </span>
                    </div>
                    <p className="text-gray-400 text-sm mb-3">{cmd.description}</p>
                    <div>
                      <p className="text-gray-500 text-xs mb-2">Usage:</p>
                      <code className="block bg-[#0a0a0f] border border-[#2a2a35] rounded p-2 text-green-400 font-mono text-xs mb-3">
                        {cmd.usage}
                      </code>
                    </div>
                    <div>
                      <p className="text-gray-500 text-xs mb-2">Examples:</p>
                      <div className="space-y-1">
                        {cmd.examples.map((ex) => (
                          <code
                            key={ex}
                            className="block bg-[#0a0a0f] border border-[#2a2a35] rounded p-2 text-green-400 font-mono text-xs"
                          >
                            $ {ex}
                          </code>
                        ))}
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {activeTab === 'repl' && (
            <div className="grid grid-cols-2 gap-0 h-[600px]">
              <div className="border-r border-[#2a2a35] flex flex-col">
                <div className="bg-[#1a1a2e] border-b border-[#2a2a35] px-4 py-3">
                  <p className="text-white font-semibold text-sm">X3-Lang Code</p>
                </div>
                <textarea
                  value={replCode}
                  onChange={(e) => setReplCode(e.target.value)}
                  className="flex-1 bg-[#0a0a0f] font-mono text-sm text-green-400 p-4 resize-none focus:outline-none border-0"
                  spellCheck="false"
                />
                <div className="bg-[#1a1a2e] border-t border-[#2a2a35] p-4 flex gap-2">
                  <button
                    onClick={handleExecuteCode}
                    className="flex-1 bg-blue-500/20 text-blue-400 px-4 py-2 rounded font-semibold hover:bg-blue-500/30 flex items-center justify-center gap-2"
                  >
                    <Play className="w-4 h-4" /> Run
                  </button>
                  <button className="flex-1 bg-gray-500/20 text-gray-400 px-4 py-2 rounded font-semibold hover:bg-gray-500/30 flex items-center justify-center gap-2">
                    <Trash2 className="w-4 h-4" /> Clear
                  </button>
                </div>
              </div>
              <div className="flex flex-col">
                <div className="bg-[#1a1a2e] border-b border-[#2a2a35] px-4 py-3 flex items-center justify-between">
                  <p className="text-white font-semibold text-sm">Output & Variables</p>
                  <Settings className="w-4 h-4 text-gray-400 cursor-pointer" />
                </div>
                <div className="flex-1 overflow-y-auto bg-[#0a0a0f] p-4">
                  <div className="mb-4">
                    <p className="text-gray-400 text-xs mb-2">Output:</p>
                    <pre className="text-green-400 font-mono text-xs bg-[#1a1a2e] border border-[#2a2a35] rounded p-2">
                      {replOutput}
                    </pre>
                  </div>
                  <div>
                    <p className="text-gray-400 text-xs mb-2">Variables:</p>
                    <div className="space-y-2">
                      {Object.entries(replVars).map(([key, value]) => (
                        <div key={key} className="bg-[#1a1a2e] border border-[#2a2a35] rounded p-2">
                          <p className="text-cyan-400 font-mono text-xs">
                            <span className="text-white">{key}</span>: {value}
                          </p>
                        </div>
                      ))}
                    </div>
                  </div>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Quick Stats */}
        <div className="grid grid-cols-4 gap-4 mt-6">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Commands Executed</div>
            <div className="text-2xl font-bold text-cyan-400">{history.length}</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Success Rate</div>
            <div className="text-2xl font-bold text-green-400">
              {history.length > 0
                ? (
                    ((history.filter((h) => h.status === 'success').length / history.length) * 100).toFixed(0) +
                    '%'
                  )
                : '—'}
            </div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Avg Execution</div>
            <div className="text-2xl font-bold text-blue-400">
              {history.length > 0
                ? (history.reduce((sum, h) => sum + h.executionTime, 0) / history.length).toFixed(0) + 'ms'
                : '—'}
            </div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">CLI Version</div>
            <div className="text-2xl font-bold text-purple-400">v2.1.0</div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default TerminalShellPanel;
