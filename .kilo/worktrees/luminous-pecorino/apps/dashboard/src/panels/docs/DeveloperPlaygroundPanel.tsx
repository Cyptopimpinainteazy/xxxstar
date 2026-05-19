import React, { useState } from 'react';
import { Code, Play, Terminal, Save, Copy, AlertCircle } from 'lucide-react';

interface CodeFile {
  name: string;
  language: 'x3-lang' | 'javascript' | 'python';
  code: string;
  lastModified: string;
}

interface CompileResult {
  status: 'success' | 'error';
  bytecode?: string;
  gas?: number;
  errors?: string[];
  warnings?: string[];
}

export const DeveloperPlaygroundPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'editor' | 'output' | 'docs'>('editor');
  const [code, setCode] = useState(`// X3-Lang Hello World Smart Contract
contract HelloWorld {
  pub state: String = "Hello, X3!"
  
  pub fn greet(name: String) -> String {
    return concat(state, " ", name)
  }
}
`);
  const [compileResult, setCompileResult] = useState<CompileResult>({
    status: 'success',
    bytecode: '0x6080604052...',
    gas: 45230,
  });

  const handleCompile = () => {
    setCompileResult({
      status: 'success',
      bytecode: '0x6080604052348015610010575f80fd5b50...',
      gas: 45230,
    });
  };

  const handleDeploy = () => {
    alert('Deploying to X3 testnet...');
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Developer Playground
            </h1>
            <p className="text-gray-400">X3-Lang IDE • Compile • Deploy • Test</p>
          </div>
          <Code className="w-12 h-12 text-cyan-400" />
        </div>

        <div className="grid grid-cols-3 gap-4 mb-6">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-gray-400 text-xs mb-1">Memory Used</div>
            <div className="text-xl font-bold text-blue-400">24 KB</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-gray-400 text-xs mb-1">Estimated Gas</div>
            <div className="text-xl font-bold text-teal-400">{compileResult.gas || 0}</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-gray-400 text-xs mb-1">Status</div>
            <div className="text-xl font-bold text-green-400">Ready</div>
          </div>
        </div>

        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['editor', 'output', 'docs'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'editor' && 'Editor'}
              {tab === 'output' && 'Output'}
              {tab === 'docs' && 'Docs'}
            </button>
          ))}
        </div>

        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden">
          {activeTab === 'editor' && (
            <div>
              <div className="bg-[#0a0a0f] border-b border-[#2a2a35] p-3 flex justify-between items-center">
                <span className="text-gray-400 font-mono text-sm">contract.x3</span>
                <div className="flex gap-2">
                  <button
                    onClick={handleCompile}
                    className="px-3 py-1 bg-cyan-600 hover:bg-cyan-700 text-white text-xs rounded font-semibold transition flex items-center gap-1"
                  >
                    <Terminal className="w-3 h-3" /> Compile
                  </button>
                  <button
                    onClick={handleDeploy}
                    className="px-3 py-1 bg-green-600 hover:bg-green-700 text-white text-xs rounded font-semibold transition flex items-center gap-1"
                  >
                    <Play className="w-3 h-3" /> Deploy
                  </button>
                </div>
              </div>
              <textarea
                value={code}
                onChange={(e) => setCode(e.target.value)}
                className="w-full h-96 bg-[#0a0a0f] text-gray-300 font-mono p-4 border-none focus:outline-none resize-none"
                spellCheck="false"
              />
            </div>
          )}

          {activeTab === 'output' && (
            <div className="p-6">
              <h3 className="text-white font-semibold mb-4">Compilation Output</h3>
              {compileResult.status === 'success' ? (
                <div className="space-y-4">
                  <div className="bg-green-500/10 border border-green-500/20 rounded-lg p-3">
                    <p className="text-green-400 font-semibold">✓ Compilation Successful</p>
                  </div>
                  <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                    <div className="text-gray-400 text-sm mb-2">Bytecode (ABI-encoded):</div>
                    <div className="bg-[#1a1a2e] p-2 rounded font-mono text-xs text-gray-300 break-all">
                      {compileResult.bytecode}
                    </div>
                  </div>
                  <div className="grid grid-cols-3 gap-3 text-sm">
                    <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                      <div className="text-gray-400 mb-1">Estimated Gas (Deploy)</div>
                      <div className="text-blue-400 font-bold">{compileResult.gas}</div>
                    </div>
                    <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                      <div className="text-gray-400 mb-1">Code Size</div>
                      <div className="text-blue-400 font-bold">1.24 KB</div>
                    </div>
                    <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                      <div className="text-gray-400 mb-1">Functions</div>
                      <div className="text-blue-400 font-bold">2</div>
                    </div>
                  </div>
                </div>
              ) : (
                <div className="bg-red-500/10 border border-red-500/20 rounded-lg p-4">
                  <p className="text-red-400 font-semibold mb-2">✗ Compilation Failed</p>
                  {compileResult.errors?.map((err, idx) => (
                    <p key={idx} className="text-red-300 text-sm">
                      {err}
                    </p>
                  ))}
                </div>
              )}
            </div>
          )}

          {activeTab === 'docs' && (
            <div className="p-6">
              <h3 className="text-white font-semibold mb-4">X3-Lang Reference</h3>
              <div className="space-y-3 text-sm">
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                  <h4 className="text-cyan-400 font-semibold mb-1">contract keyword</h4>
                  <p className="text-gray-400">Define a smart contract scope with state and functions.</p>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                  <h4 className="text-cyan-400 font-semibold mb-1">pub visibility modifier</h4>
                  <p className="text-gray-400">Mark functions and state as publicly callable from external transactions.</p>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                  <h4 className="text-cyan-400 font-semibold mb-1">fn function declaration</h4>
                  <p className="text-gray-400">Declare contract methods with typed parameters and return values.</p>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default DeveloperPlaygroundPanel;
