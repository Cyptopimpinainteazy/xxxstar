import React, { useState } from "react";
import { Code2, Zap, Copy, Download, Settings, CheckCircle, AlertCircle, Eye } from "lucide-react";
import clsx from "clsx";

interface GeneratedCode {
  language: "typescript" | "python" | "go";
  code: string;
  timestamp: string;
}

interface AbiFunction {
  name: string;
  inputs: Array<{ name: string; type: string }>;
  outputs: Array<{ name: string; type: string }>;
  stateMutability: "pure" | "view" | "nonpayable" | "payable";
}

interface AbiEvent {
  name: string;
  inputs: Array<{ name: string; type: string; indexed: boolean }>;
}

const MOCK_ABI: (AbiFunction | AbiEvent)[] = [
  {
    name: "mint",
    inputs: [{ name: "amount", type: "uint256" }],
    outputs: [],
    stateMutability: "nonpayable",
  } as AbiFunction,
  {
    name: "transfer",
    inputs: [
      { name: "to", type: "address" },
      { name: "amount", type: "uint256" },
    ],
    outputs: [{ name: "", type: "bool" }],
    stateMutability: "nonpayable",
  } as AbiFunction,
  {
    name: "balanceOf",
    inputs: [{ name: "account", type: "address" }],
    outputs: [{ name: "", type: "uint256" }],
    stateMutability: "view",
  } as AbiFunction,
  {
    name: "Transfer",
    inputs: [
      { name: "from", type: "address", indexed: true },
      { name: "to", type: "address", indexed: true },
      { name: "value", type: "uint256", indexed: false },
    ],
  } as AbiEvent,
];

const MOCK_TYPESCRIPT = `import { ethers } from "ethers";

// Contract ABI
const CONTRACT_ABI = [
  {
    name: "mint",
    type: "function",
    inputs: [{ name: "amount", type: "uint256" }],
    outputs: [],
    stateMutability: "nonpayable",
  },
  {
    name: "transfer",
    type: "function",
    inputs: [
      { name: "to", type: "address" },
      { name: "amount", type: "uint256" },
    ],
    outputs: [{ name: "", type: "bool" }],
    stateMutability: "nonpayable",
  },
];

// Contract class
export class SimpleTokenContract {
  private contract: ethers.Contract;

  constructor(contractAddress: string, signer: ethers.Signer) {
    this.contract = new ethers.Contract(contractAddress, CONTRACT_ABI, signer);
  }

  async mint(amount: ethers.BigNumberish): Promise<ethers.ContractTransaction> {
    return this.contract.mint(amount);
  }

  async transfer(to: string, amount: ethers.BigNumberish): Promise<boolean> {
    return this.contract.transfer(to, amount);
  }
}`;

const MOCK_PYTHON = `from web3 import Web3
from web3.contract import Contract

# Contract ABI
CONTRACT_ABI = [
    {
        "name": "mint",
        "type": "function",
        "inputs": [{"name": "amount", "type": "uint256"}],
        "outputs": [],
        "stateMutability": "nonpayable",
    },
    {
        "name": "transfer",
        "type": "function",
        "inputs": [
            {"name": "to", "type": "address"},
            {"name": "amount", "type": "uint256"},
        ],
        "outputs": [{"name": "", "type": "bool"}],
        "stateMutability": "nonpayable",
    },
]

class SimpleTokenContract:
    def __init__(self, contract_address: str, web3: Web3):
        self.contract = web3.eth.contract(
            address=contract_address,
            abi=CONTRACT_ABI
        )

    def mint(self, amount: int) -> dict:
        return self.contract.functions.mint(amount).transact()

    def transfer(self, to: str, amount: int) -> bool:
        return self.contract.functions.transfer(to, amount).call()`;

const MOCK_GO = `package main

import (
    "github.com/ethereum/go-ethereum/accounts/abi"
    "github.com/ethereum/go-ethereum/common"
    "github.com/ethereum/go-ethereum/ethclient"
)

type SimpleTokenContract struct {
    Client   *ethclient.Client
    Contract *abi.ABI
    Address  common.Address
}

func NewSimpleTokenContract(addr string, client *ethclient.Client) *SimpleTokenContract {
    contractABI := \`[
        {"name":"mint","type":"function","inputs":[{"name":"amount","type":"uint256"}]},
        {"name":"transfer","type":"function","inputs":[{"name":"to","type":"address"},{"name":"amount","type":"uint256"}]}
    ]\`
    
    parsed, _ := abi.JSON(strings.NewReader(contractABI))
    
    return &SimpleTokenContract{
        Client:   client,
        Contract: &parsed,
        Address:  common.HexToAddress(addr),
    }
}

func (c *SimpleTokenContract) Mint(amount *big.Int) error {
    // Implementation
    return nil
}

func (c *SimpleTokenContract) Transfer(to common.Address, amount *big.Int) (bool, error) {
    // Implementation
    return true, nil
}`;

export default function SdkCodeGeneratorPanel() {
  const [abiInput, setAbiInput] = useState(JSON.stringify(MOCK_ABI, null, 2));
  const [selectedLanguages, setSelectedLanguages] = useState<("typescript" | "python" | "go")[]>(["typescript"]);
  const [generatedCode, setGeneratedCode] = useState<GeneratedCode | null>(null);
  const [activeTab, setActiveTab] = useState<"abi" | "preview">("abi");
  const [isGenerating, setIsGenerating] = useState(false);

  const handleToggleLanguage = (lang: "typescript" | "python" | "go") => {
    setSelectedLanguages((prev) =>
      prev.includes(lang) ? prev.filter((l) => l !== lang) : [...prev, lang]
    );
  };

  const handleGenerate = () => {
    setIsGenerating(true);
    setTimeout(() => {
      let code = "";
      if (selectedLanguages.includes("typescript")) code = MOCK_TYPESCRIPT;
      else if (selectedLanguages.includes("python")) code = MOCK_PYTHON;
      else if (selectedLanguages.includes("go")) code = MOCK_GO;

      setGeneratedCode({
        language: selectedLanguages[0] || "typescript",
        code,
        timestamp: new Date().toISOString(),
      });
      setActiveTab("preview");
      setIsGenerating(false);
    }, 1200);
  };

  const handleCopyCode = () => {
    if (generatedCode) {
      navigator.clipboard.writeText(generatedCode.code);
    }
  };

  const handleDownloadCode = () => {
    if (generatedCode) {
      const ext = {
        typescript: "ts",
        python: "py",
        go: "go",
      }[generatedCode.language];

      const element = document.createElement("a");
      element.setAttribute("href", "data:text/plain;charset=utf-8," + encodeURIComponent(generatedCode.code));
      element.setAttribute("download", `contract-sdk.${ext}`);
      element.style.display = "none";
      document.body.appendChild(element);
      element.click();
      document.body.removeChild(element);
    }
  };

  const functionCount = (MOCK_ABI.filter((item) => "stateMutability" in item) as AbiFunction[]).length;
  const eventCount = (MOCK_ABI.filter((item) => !("stateMutability" in item)) as AbiEvent[]).length;

  return (
    <div className="w-full h-full bg-[#0a0a0f] text-white p-6 flex flex-col">
      <h2 className="text-xl font-bold mb-4 flex items-center gap-2">
        <Code2 size={20} className="text-cyan-400" /> SDK Code Generator
      </h2>

      <div className="flex-1 overflow-hidden space-y-4 mb-4">
        {/* Overview */}
        <div className="grid grid-cols-4 gap-2">
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Functions</div>
            <div className="text-lg font-bold text-cyan-400">{functionCount}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Events</div>
            <div className="text-lg font-bold text-purple-400">{eventCount}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Languages</div>
            <div className="text-lg font-bold text-green-400">{selectedLanguages.length}</div>
          </div>
          <div className="bg-[#15151b] border border-[#2a2a35] rounded-lg p-3">
            <div className="text-xs text-gray-400 mb-1">Status</div>
            <div className={clsx("text-lg font-bold", generatedCode ? "text-green-400" : "text-yellow-400")}>
              {generatedCode ? "✓ Ready" : "Pending"}
            </div>
          </div>
        </div>

        {/* Main Editor Area */}
        <div className="flex-1 flex gap-3 overflow-hidden">
          {/* Left: Language Selector & ABI Input */}
          <div className="w-48 flex flex-col border border-[#2a2a35] rounded-lg overflow-hidden">
            {/* Language Selection */}
            <div className="bg-[#15151b] border-b border-[#2a2a35] p-3">
              <div className="text-xs font-semibold uppercase text-gray-400 mb-2">Target Languages</div>
              <div className="space-y-1.5">
                {(["typescript", "python", "go"] as const).map((lang) => (
                  <label key={lang} className="flex items-center gap-2 cursor-pointer text-xs">
                    <input
                      type="checkbox"
                      checked={selectedLanguages.includes(lang)}
                      onChange={() => handleToggleLanguage(lang)}
                      className="w-3 h-3 accent-cyan-600"
                    />
                    <span className="capitalize">{lang}</span>
                  </label>
                ))}
              </div>
            </div>

            {/* ABI Preview */}
            <div className="flex-1 overflow-y-auto p-3 bg-[#0a0a0f] border-t border-[#2a2a35]">
              <div className="text-xs font-semibold uppercase text-gray-400 mb-2">ABI Functions</div>
              <div className="space-y-1.5 text-xs">
                {(MOCK_ABI.filter((item) => "stateMutability" in item) as AbiFunction[]).map((fn) => (
                  <div key={fn.name} className="p-2 bg-[#15151b] border border-[#2a2a35] rounded text-cyan-400 font-mono">
                    {fn.name}()
                  </div>
                ))}
                {(MOCK_ABI.filter((item) => !("stateMutability" in item)) as AbiEvent[]).map((evt) => (
                  <div key={evt.name} className="p-2 bg-[#15151b] border border-[#2a2a35] rounded text-purple-400 font-mono">
                    ↑ {evt.name}
                  </div>
                ))}
              </div>
            </div>

            {/* Generate Button */}
            <div className="border-t border-[#2a2a35] p-3 bg-[#15151b]">
              <button
                onClick={handleGenerate}
                disabled={isGenerating || selectedLanguages.length === 0}
                className={clsx(
                  "w-full flex items-center justify-center gap-2 px-3 py-2 rounded-lg border transition font-semibold text-xs",
                  isGenerating || selectedLanguages.length === 0
                    ? "bg-cyan-600/10 border-cyan-600/30 cursor-not-allowed opacity-50"
                    : "bg-[#2a2a35] border-[#2a2a35] hover:border-cyan-600"
                )}
              >
                <Zap size={12} /> {isGenerating ? "..." : "Generate"}
              </button>
            </div>
          </div>

          {/* Right: ABI Input & Code Preview */}
          <div className="flex-1 flex flex-col border border-[#2a2a35] rounded-lg overflow-hidden">
            {/* Tabs */}
            <div className="flex gap-2 bg-[#15151b] border-b border-[#2a2a35] px-3 py-2">
              {(["abi", "preview"] as const).map((tab) => (
                <button
                  key={tab}
                  onClick={() => setActiveTab(tab)}
                  className={clsx(
                    "px-3 py-1 text-xs font-semibold rounded transition",
                    activeTab === tab
                      ? "bg-cyan-600/20 text-cyan-400"
                      : "text-gray-400 hover:text-gray-300"
                  )}
                >
                  {tab === "abi" ? "ABI Input" : "Generated Code"}
                </button>
              ))}
            </div>

            {activeTab === "abi" && (
              <textarea
                value={abiInput}
                onChange={(e) => setAbiInput(e.target.value)}
                className="flex-1 bg-[#0a0a0f] text-gray-300 p-3 font-mono text-xs focus:outline-none resize-none"
                spellCheck={false}
                placeholder="Paste ABI JSON here..."
              />
            )}

            {activeTab === "preview" && generatedCode && (
              <div className="flex-1 overflow-y-auto">
                <textarea
                  readOnly
                  value={generatedCode.code}
                  className="w-full h-full bg-[#0a0a0f] text-gray-300 p-3 font-mono text-xs focus:outline-none resize-none"
                />
              </div>
            )}

            {activeTab === "preview" && !generatedCode && (
              <div className="flex-1 flex items-center justify-center text-gray-500 bg-[#0a0a0f]">
                <div className="text-center">
                  <AlertCircle size={32} className="mx-auto mb-2 opacity-50" />
                  <div className="text-xs">Generate code to preview</div>
                </div>
              </div>
            )}

            {/* Code Actions */}
            {generatedCode && (
              <div className="border-t border-[#2a2a35] bg-[#15151b] p-3 flex gap-2">
                <button
                  onClick={handleCopyCode}
                  className="flex-1 flex items-center justify-center gap-2 px-3 py-1.5 bg-[#2a2a35] hover:border-cyan-600 border border-[#2a2a35] rounded text-xs font-semibold transition"
                >
                  <Copy size={12} /> Copy
                </button>
                <button
                  onClick={handleDownloadCode}
                  className="flex-1 flex items-center justify-center gap-2 px-3 py-1.5 bg-[#2a2a35] hover:border-cyan-600 border border-[#2a2a35] rounded text-xs font-semibold transition"
                >
                  <Download size={12} /> Download
                </button>
                <div className="flex items-center text-xs text-gray-500">
                  {generatedCode.language.toUpperCase()}
                </div>
              </div>
            )}
          </div>
        </div>
      </div>

      <div className="text-xs text-gray-500 text-center pt-4 border-t border-[#2a2a35]">
        Automatically generate TypeScript, Python, or Go SDKs from contract ABI with full type safety.
      </div>
    </div>
  );
}
