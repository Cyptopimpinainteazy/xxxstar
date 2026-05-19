import React, { useState } from "react";
import { Code2, Play, Copy, Download, Share2, AlertCircle, CheckCircle, Zap, FileJson } from "lucide-react";
import clsx from "clsx";

interface CodeSnippet {
  id: string;
  name: string;
  code: string;
  language: "x3lang" | "solidity" | "rust";
}

interface CompilationResult {
  success: boolean;
  message: string;
  bytecode?: string;
  abi?: object;
  gas?: number;
  timestamp: string;
}

interface DeploymentResult {
  success: boolean;
  contractAddress?: string;
  txHash?: string;
  blockNumber?: number;
  gasUsed?: number;
  timestamp: string;
  error?: string;
}

const MOCK_SNIPPETS: CodeSnippet[] = [
  {
    id: "1",
    name: "Simple Token",
    language: "x3lang",
    code: `contract SimpleToken {
  storage {
    balances: Map<Address, U256>;
    total_supply: U256;
  }

  public fn mint(amount: U256) {
    balances[tx.origin] += amount;
    total_supply += amount;
  }

  public fn transfer(to: Address, amount: U256) -> bool {
    require(balances[tx.origin] >= amount);
    balances[tx.origin] -= amount;
    balances[to] += amount;
    true
  }
}`,
  },
  {
    id: "2",
    name: "Voting Contract",
    language: "x3lang",
    code: `contract Voting {
  storage {
    proposals: Vec<Proposal>;
    votes: Map<(U256, Address), bool>;
  }

  struct Proposal {
    description: String;
    vote_count: U256;
    executed: bool;
  }

  public fn create_proposal(desc: String) {
    proposals.push(Proposal {
      description: desc,
      vote_count: 0,
      executed: false
    });
  }

  public fn vote(proposal_id: U256) {
    require(!votes[(proposal_id, tx.origin)]);
    votes[(proposal_id, tx.origin)] = true;
    proposals[proposal_id].vote_count += 1;
  }
}`,
  },
];

const MOCK_COMPILATION: CompilationResult = {
  success: true,
  message: "Compilation successful",
  bytecode: "0x608060405234801561001057600080fd5b5061012f806100206000396000f3fe",
  abi: { contract: "SimpleToken", functions: ["mint", "transfer"] },
  gas: 45678,
  timestamp: new Date().toISOString(),
};

export default function InteractiveCodePlaygroundPanel() {
  const [selectedSnippet, setSelectedSnippet] = useState<CodeSnippet>(MOCK_SNIPPETS[0]);
  const [code, setCode] = useState(MOCK_SNIPPETS[0].code);
  const [compilationResult, setCompilationResult] = useState<CompilationResult | null>(null);
  const [deploymentResult, setDeploymentResult] = useState<DeploymentResult | null>(null);
  const [activeTab, setActiveTab] = useState<"editor" | "output" | "deployed">("editor");
  const [isCompiling, setIsCompiling] = useState(false);
  const [isDeploying, setIsDeploying] = useState(false);

  const handleCompile = () => {
    setIsCompiling(true);
    setTimeout(() => {
      setCompilationResult(MOCK_COMPILATION);
      setIsCompiling(false);
    }, 1500);
  };

  const handleDeploy = () => {
    if (!compilationResult || !compilationResult.success) {
      alert("Please compile successfully first");
      return;
    }

    setIsDeploying(true);
    setTimeout(() => {
      setDeploymentResult({
        success: true,
        contractAddress: "0x742d35Cc6634C0532925a3b844Bc9e7595f3bEb0",
        txHash: "0x8c7d6dfe1c0a6bc20f8a8c9b3c9c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",
        blockNumber: 18527842,
        gasUsed: 2456789,
        timestamp: new Date().toISOString(),
      });
      setIsDeploying(false);
    }, 2000);
  };

  const handleCopyCode = () => {
    navigator.clipboard.writeText(code);
  };

  const handleDownloadCode = () => {
    const element = document.createElement("a");
    element.setAttribute("href", "data:text/plain;charset=utf-8," + encodeURIComponent(code));
    element.setAttribute("download", `${selectedSnippet.name}.x3`);
    element.style.display = "none";
    document.body.appendChild(element);
    element.click();
    document.body.removeChild(element);
  };

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Code2 size={20} className="text-green-400" /> Interactive Code Playground
      </h2>

      <div className="flex-1 overflow-hidden space-y-4 mb-4">
        {/* Snippet Selection */}
        <div className="flex gap-2 overflow-x-auto">
          {MOCK_SNIPPETS.map((snippet) => (
            <button
              key={snippet.id}
              onClick={() => {
                setSelectedSnippet(snippet);
                setCode(snippet.code);
                setCompilationResult(null);
                setDeploymentResult(null);
              }}
              className={clsx(
                "px-3 py-2 rounded-lg border transition text-sm font-semibold whitespace-nowrap",
                selectedSnippet.id === snippet.id
                  ? "border-green-600 bg-green-600/10 text-green-400"
                  : "border-[#2a2a35] bg-[#15151b] hover:border-[#3a3a45]"
              )}
            >
              {snippet.name}
            </button>
          ))}
        </div>

        {/* Main Editor Area */}
        <div className="flex-1 flex gap-3 overflow-hidden">
          {/* Code Editor */}
          <div className="flex-1 flex flex-col border border-[#2a2a35] rounded-lg overflow-hidden">
            {/* Editor Tabs */}
            <div className="flex gap-2 bg-[#15151b] border-b border-[#2a2a35] px-3 py-2">
              {(["editor", "output", "deployed"] as const).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab)}
                  className={clsx(
                    "px-3 py-1 text-xs font-semibold rounded transition",
                    activeTab === tab
                      ? "bg-green-600/20 text-green-400"
                      : "text-gray-400 hover:text-gray-300"
                  )}
                >
                  {tab === "editor" && "Editor"}
                  {tab === "output" && "Compilation"}
                  {tab === "deployed" && "Deployment"}
                </button>
              ))}
            </div>

            {activeTab === "editor" && (
              <div className="flex-1 flex flex-col overflow-hidden">
                <textarea
                  value={code}
                  onChange={(e) => setCode(e.target.value)}
                  className="flex-1 bg-[#0a0a0f] text-gray-300 p-3 font-mono text-xs focus:outline-none resize-none"
                  spellCheck={false}
                />
              </div>
            )}

            {activeTab === "output" && compilationResult && (
              <div className="flex-1 overflow-y-auto p-3 space-y-2 bg-[#0a0a0f]">
                <div
                  className={clsx("p-2 rounded flex items-center gap-2", compilationResult.success ? "bg-green-600/10 text-green-400" : "bg-red-600/10 text-red-400")}
                >
                  {compilationResult.success ? (
                    <CheckCircle size={14} />
                  ) : (
                    <AlertCircle size={14} />
                  )}
                  <span className="text-xs font-semibold">{compilationResult.message}</span>
                </div>

                {compilationResult.gas && (
                  <div className="bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs">
                    <div className="text-gray-400">Gas Estimate</div>
                    <div className="font-bold text-yellow-400 mt-0.5">{compilationResult.gas.toLocaleString()} units</div>
                  </div>
                )}

                {compilationResult.bytecode && (
                  <div className="bg-[#15151b] border border-[#2a2a35] rounded p-2">
                    <div className="text-xs text-gray-400 mb-1">Bytecode</div>
                    <div className="bg-[#0a0a0f] rounded p-2 text-xs font-mono text-gray-400 break-all max-h-24 overflow-y-auto">
                      {compilationResult.bytecode}
                    </div>
                  </div>
                )}

                {compilationResult.abi && (
                  <div className="bg-[#15151b] border border-[#2a2a35] rounded p-2">
                    <div className="text-xs text-gray-400 mb-1">ABI</div>
                    <div className="bg-[#0a0a0f] rounded p-2 text-xs font-mono text-gray-400">
                      {JSON.stringify(compilationResult.abi, null, 2)}
                    </div>
                  </div>
                )}
              </div>
            )}

            {activeTab === "deployed" && deploymentResult && (
              <div className="flex-1 overflow-y-auto p-3 space-y-2 bg-[#0a0a0f]">
                <div
                  className={clsx("p-2 rounded flex items-center gap-2", deploymentResult.success ? "bg-green-600/10 text-green-400" : "bg-red-600/10 text-red-400")}
                >
                  {deploymentResult.success ? (
                    <CheckCircle size={14} />
                  ) : (
                    <AlertCircle size={14} />
                  )}
                  <span className="text-xs font-semibold">
                    {deploymentResult.success ? "Deployment successful" : deploymentResult.error}
                  </span>
                </div>

                {deploymentResult.contractAddress && (
                  <div className="bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs">
                    <div className="text-gray-400">Contract Address</div>
                    <div className="font-mono text-cyan-400 mt-0.5 break-all">{deploymentResult.contractAddress}</div>
                  </div>
                )}

                {deploymentResult.txHash && (
                  <div className="bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs">
                    <div className="text-gray-400">Transaction Hash</div>
                    <div className="font-mono text-cyan-400 mt-0.5 break-all">{deploymentResult.txHash}</div>
                  </div>
                )}

                {deploymentResult.blockNumber && (
                  <div className="grid grid-cols-2 gap-2">
                    <div className="bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs">
                      <div className="text-gray-400">Block Number</div>
                      <div className="font-bold text-yellow-400 mt-0.5">{deploymentResult.blockNumber}</div>
                    </div>
                    <div className="bg-[#15151b] border border-[#2a2a35] rounded p-2 text-xs">
                      <div className="text-gray-400">Gas Used</div>
                      <div className="font-bold text-yellow-400 mt-0.5">{((deploymentResult.gasUsed ?? 0) / 1000000).toFixed(2)}M</div>
                    </div>
                  </div>
                )}
              </div>
            )}

            {!compilationResult && activeTab === "output" && (
              <div className="flex-1 flex items-center justify-center text-gray-500 bg-[#0a0a0f]">
                <div className="text-center">
                  <AlertCircle size={32} className="mx-auto mb-2 opacity-50" />
                  <div className="text-xs">Compile code to see output</div>
                </div>
              </div>
            )}

            {!deploymentResult && activeTab === "deployed" && (
              <div className="flex-1 flex items-center justify-center text-gray-500 bg-[#0a0a0f]">
                <div className="text-center">
                  <AlertCircle size={32} className="mx-auto mb-2 opacity-50" />
                  <div className="text-xs">Deploy to see deployment details</div>
                </div>
              </div>
            )}
          </div>

          {/* Toolbar */}
          <div className="flex flex-col gap-2 w-32">
            <button
              onClick={handleCompile}
              disabled={isCompiling}
              className={clsx(
                "flex-1 flex flex-col items-center justify-center gap-1 rounded-lg border transition font-semibold text-xs py-2",
                isCompiling
                  ? "bg-yellow-600/10 border-yellow-600/50 cursor-not-allowed"
                  : "bg-[#15151b] border-[#2a2a35] hover:border-yellow-600"
              )}
            >
              <Zap size={14} /> {isCompiling ? "..." : "Compile"}
            </button>

            <button
              onClick={handleDeploy}
              disabled={isDeploying || !compilationResult || !compilationResult.success}
              className={clsx(
                "flex-1 flex flex-col items-center justify-center gap-1 rounded-lg border transition font-semibold text-xs py-2",
                isDeploying || !compilationResult
                  ? "bg-green-600/10 border-green-600/30 cursor-not-allowed opacity-50"
                  : "bg-[#15151b] border-[#2a2a35] hover:border-green-600"
              )}
            >
              <Play size={14} /> {isDeploying ? "..." : "Deploy"}
            </button>

            <button
              onClick={handleCopyCode}
              className="flex-1 flex flex-col items-center justify-center gap-1 bg-[#15151b] border border-[#2a2a35] hover:border-cyan-600 rounded-lg transition font-semibold text-xs py-2"
            >
              <Copy size={14} /> Copy
            </button>

            <button
              onClick={handleDownloadCode}
              className="flex-1 flex flex-col items-center justify-center gap-1 bg-[#15151b] border border-[#2a2a35] hover:border-cyan-600 rounded-lg transition font-semibold text-xs py-2"
            >
              <Download size={14} /> Download
            </button>

            <button className="flex-1 flex flex-col items-center justify-center gap-1 bg-[#15151b] border border-[#2a2a35] hover:border-cyan-600 rounded-lg transition font-semibold text-xs py-2">
              <Share2 size={14} /> Share
            </button>
          </div>
        </div>
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Browser-based X3-Lang IDE with compile and testnet deployment capabilities.
      </div>
    </div>
  );
}
