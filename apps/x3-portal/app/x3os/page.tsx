"use client";

import { useEffect, useState, useCallback, useRef } from "react";
import { motion, AnimatePresence } from "framer-motion";

/* ===============================
   SYSTEM CONSTANTS
   =============================== */

const ACCENT_COLORS = {
  evm: "#22d3ee",   // cyan
  svm: "#a78bfa",   // violet
  x3vm: "#fbbf24",  // amber
  btc: "#f97316",   // orange
  default: "#22d3ee"
};

type VMType = "evm" | "svm" | "x3vm" | "btc";
type AppId = "terminal" | "execution" | "vmmanager" | "atomic" | "ecosystem" | "overview";

/* ===============================
   SYSTEM COPY (AUTHORITATIVE)
   =============================== */

const SYSTEM_COPY = {
  boot: [
    "x3Star OS v1.0",
    "Initializing execution kernel…",
    "Loading VM abstraction layer…",
    "Synchronizing atomic state…",
    "Connecting to consensus network…",
    "System ready."
  ],
  terminal: {
    commands: {
      status: `STATUS: ONLINE
NETWORK: TESTNET
BLOCK HEIGHT: 1,847,293
EXECUTION: STABLE
CONSENSUS: GRANDPA FINALIZING
PEERS: 12 CONNECTED`,
      vms: `AVAILABLE EXECUTION ENVIRONMENTS:

EVM  │ Deterministic. Ubiquitous. Battle-tested.
SVM  │ Parallel execution. 65,000 TPS capacity.
x3VM │ Native. AI-aware. Constraint-optimized.
BTC  │ Atomic settlement. No bridges. No trust.

All VMs share canonical state.
Cross-VM atomicity guaranteed.`,
      atomic: `ATOMIC EXECUTION MODE: ENABLED

FAILURE DOMAIN: GLOBAL
PARTIAL EXECUTION: DISALLOWED
ROLLBACK GUARANTEE: AUTOMATIC

When constraints fail, state reverts.
There is no middle ground.`,
      why: `WHY x3STAR:

Other chains execute transactions.
This executes intent.

Other chains bridge assets.
This atomically settles them.

Other chains promise security.
This enforces determinism.

The difference is not philosophical.
The difference is architectural.`,
      help: `AVAILABLE COMMANDS:

status    │ Network and consensus state
vms       │ Virtual machine overview
atomic    │ Atomic execution details
why       │ Protocol philosophy
clear     │ Clear terminal output
help      │ This message`
    }
  },
  execution: {
    tagline: "Determinism is not optional.",
    stages: [
      { id: "ingest", label: "TX INGEST", status: "active" },
      { id: "order", label: "CONSTRAINT ORDER", status: "processing" },
      { id: "group", label: "ATOMIC GROUP", status: "pending" },
      { id: "execute", label: "VM DISPATCH", status: "pending" },
      { id: "finalize", label: "FINALIZE", status: "pending" }
    ]
  },
  vms: {
    evm: {
      name: "EVM",
      fullName: "Ethereum Virtual Machine",
      status: "ACTIVE",
      description: "Battle-tested execution. Maximum compatibility.",
      specs: [
        "Solidity / Vyper compatible",
        "Gas metering: Standard",
        "State: Merkle Patricia Trie",
        "Execution: Sequential"
      ],
      tps: "~30",
      latency: "12s blocks"
    },
    svm: {
      name: "SVM",
      fullName: "Solana Virtual Machine",
      status: "ACTIVE",
      description: "Parallelized execution. Speed without chaos.",
      specs: [
        "Rust / Anchor compatible",
        "Execution: Parallel (Sealevel)",
        "State: Account model",
        "Transaction: Versioned"
      ],
      tps: "~65,000",
      latency: "400ms"
    },
    x3vm: {
      name: "x3VM",
      fullName: "Native Execution Engine",
      status: "ACTIVE",
      description: "Native execution. AI-first. Constraint-aware.",
      specs: [
        "Intent-based execution",
        "AI constraint solver",
        "MEV-resistant ordering",
        "Cross-VM orchestration"
      ],
      tps: "Adaptive",
      latency: "Sub-second"
    },
    btc: {
      name: "BTC",
      fullName: "Bitcoin Atomic Layer",
      status: "ACTIVE",
      description: "Atomic Bitcoin settlement. No bridges.",
      specs: [
        "HTLC-based settlement",
        "Non-custodial always",
        "Atomic swap native",
        "Taproot compatible"
      ],
      tps: "~7",
      latency: "10min confirm"
    }
  },
  atomic: {
    tagline: "Either everything happens — or nothing does."
  },
  ecosystem: {
    note: "These are not apps. These are workloads.",
    processes: [
      { name: "arb-agent-001", type: "Arbitrage", status: "RUNNING", cpu: 12, mem: 256 },
      { name: "quant-executor", type: "Quant Infra", status: "RUNNING", cpu: 34, mem: 512 },
      { name: "ai-intent-solver", type: "AI Executor", status: "RUNNING", cpu: 67, mem: 1024 },
      { name: "mev-shield", type: "MEV Protection", status: "RUNNING", cpu: 8, mem: 128 },
      { name: "cross-vm-bridge", type: "Bridge Daemon", status: "RUNNING", cpu: 5, mem: 64 },
      { name: "consensus-monitor", type: "Validator", status: "RUNNING", cpu: 23, mem: 384 }
    ]
  },
  overview: {
    title: "x3Star OS",
    subtitle: "Execution Layer for Everything",
    description: `x3Star is a unified execution environment 
spanning EVM, SVM, and native Bitcoin settlement.

This is not another L1.
This is not another bridge.
This is canonical cross-chain state.

Atomic execution across virtual machines.
Deterministic outcomes enforced by architecture.
No partial states. No trust assumptions.`
  }
};

/* ===============================
   BOOT SEQUENCE COMPONENT
   =============================== */

function BootSequence({ onComplete }: { onComplete: () => void }) {
  const [lines, setLines] = useState<string[]>([]);
  const [cursorVisible, setCursorVisible] = useState(true);

  useEffect(() => {
    let lineIndex = 0;
    const interval = setInterval(() => {
      if (lineIndex < SYSTEM_COPY.boot.length) {
        setLines(prev => [...prev, SYSTEM_COPY.boot[lineIndex]]);
        lineIndex++;
      } else {
        clearInterval(interval);
        setCursorVisible(false);
        setTimeout(onComplete, 600);
      }
    }, 350);

    return () => clearInterval(interval);
  }, [onComplete]);

  useEffect(() => {
    const blink = setInterval(() => {
      setCursorVisible(v => !v);
    }, 530);
    return () => clearInterval(blink);
  }, []);

  return (
    <motion.div 
      className="fixed inset-0 bg-black z-[200] flex items-start justify-start p-8"
      exit={{ opacity: 0 }}
      transition={{ duration: 0 }}
    >
      <div className="font-mono text-sm text-zinc-300 leading-relaxed">
        {lines.map((line, i) => (
          <div key={i} className={line === "System ready." ? "text-emerald-400" : ""}>
            {line}
          </div>
        ))}
        <span className={`${cursorVisible ? "opacity-100" : "opacity-0"} text-zinc-300`}>▮</span>
      </div>
    </motion.div>
  );
}

/* ===============================
   SYSTEM BAR COMPONENT
   =============================== */

function SystemBar({ 
  activeVM, 
  blockHeight,
  onLogoClick 
}: { 
  activeVM: VMType; 
  blockHeight: number;
  onLogoClick: () => void;
}) {
  const [time, setTime] = useState("");

  useEffect(() => {
    const updateTime = () => {
      const now = new Date();
      setTime(now.toISOString().slice(11, 19) + " UTC");
    };
    updateTime();
    const interval = setInterval(updateTime, 1000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="fixed top-0 left-0 right-0 h-12 bg-zinc-950 border-b border-zinc-800 flex items-center px-4 font-mono text-xs z-[100]">
      {/* Left: Logo */}
      <button 
        onClick={onLogoClick}
        className="text-zinc-200 hover:text-white transition-colors flex items-center gap-2"
      >
        <div className="w-2 h-2 bg-emerald-400 rounded-full animate-pulse" />
        <span className="font-bold tracking-wider">x3Star OS</span>
      </button>

      {/* Center: Network status */}
      <div className="flex-1 flex items-center justify-center gap-6 text-zinc-500">
        <div className="flex items-center gap-2">
          <span className="text-amber-400">TESTNET</span>
          <span className="text-zinc-700">│</span>
          <span>BLOCK #{blockHeight.toLocaleString()}</span>
        </div>
      </div>

      {/* Right: VM indicators + clock */}
      <div className="flex items-center gap-4 text-zinc-500">
        <div className="flex items-center gap-1">
          {(["evm", "svm", "x3vm", "btc"] as VMType[]).map((vm) => (
            <span 
              key={vm} 
              className={`px-2 py-0.5 rounded text-[10px] font-bold uppercase transition-colors ${
                activeVM === vm 
                  ? "text-black" 
                  : "text-zinc-600"
              }`}
              style={{ 
                backgroundColor: activeVM === vm ? ACCENT_COLORS[vm] : "transparent"
              }}
            >
              {vm}
            </span>
          ))}
        </div>
        <span className="text-zinc-700">│</span>
        <span className="text-zinc-400 tabular-nums">{time}</span>
      </div>
    </div>
  );
}

/* ===============================
   DOCK COMPONENT
   =============================== */

function Dock({ 
  activeApp, 
  onAppClick 
}: { 
  activeApp: AppId | null;
  onAppClick: (app: AppId) => void;
}) {
  const apps: { id: AppId; label: string; icon: string }[] = [
    { id: "terminal", label: "Terminal", icon: ">" },
    { id: "execution", label: "Execution", icon: "◈" },
    { id: "vmmanager", label: "VMs", icon: "◧" },
    { id: "atomic", label: "Atomic", icon: "⊕" },
    { id: "ecosystem", label: "Processes", icon: "◉" },
  ];

  return (
    <div className="fixed left-0 top-12 bottom-0 w-16 bg-zinc-950 border-r border-zinc-800 flex flex-col items-center py-4 z-[90]">
      {apps.map((app) => (
        <button
          key={app.id}
          onClick={() => onAppClick(app.id)}
          className="group relative w-12 h-12 mb-2 flex items-center justify-center"
        >
          <div 
            className={`w-10 h-10 rounded-lg flex items-center justify-center text-lg font-mono transition-all ${
              activeApp === app.id 
                ? "bg-zinc-800 text-cyan-400" 
                : "text-zinc-600 hover:text-zinc-300 hover:bg-zinc-900"
            }`}
          >
            {app.icon}
          </div>
          {activeApp === app.id && (
            <div className="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-6 bg-cyan-400 rounded-r" />
          )}
          <div className="absolute left-14 px-2 py-1 bg-zinc-800 text-zinc-300 text-xs rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap pointer-events-none">
            {app.label}
          </div>
        </button>
      ))}
    </div>
  );
}

/* ===============================
   WINDOW COMPONENT
   =============================== */

interface WindowProps {
  title: string;
  children: React.ReactNode;
  isActive: boolean;
  onFocus: () => void;
  onClose: () => void;
  position: { x: number; y: number };
  size: { width: number; height: number };
  statusText?: string;
  accentColor?: string;
}

function Window({ 
  title, 
  children, 
  isActive, 
  onFocus, 
  onClose,
  position,
  size,
  statusText,
  accentColor = ACCENT_COLORS.default
}: WindowProps) {
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95, y: 10 }}
      animate={{ 
        opacity: 1, 
        scale: 1, 
        y: 0,
        filter: isActive ? "blur(0px)" : "blur(2px)"
      }}
      exit={{ opacity: 0, scale: 0.95 }}
      transition={{ duration: 0.15, ease: "easeOut" }}
      onClick={onFocus}
      className={`absolute bg-zinc-950 border rounded-lg overflow-hidden shadow-2xl transition-all duration-150 ${
        isActive ? "border-zinc-700 z-[60]" : "border-zinc-800/50 z-[50] opacity-70"
      }`}
      style={{
        left: position.x,
        top: position.y,
        width: size.width,
        height: size.height
      }}
    >
      {/* Title bar */}
      <div 
        className="h-8 flex items-center justify-between px-3 border-b border-zinc-800 bg-zinc-900"
        style={{ borderTopColor: isActive ? accentColor : "transparent", borderTopWidth: 2 }}
      >
        <span className="text-xs font-mono text-zinc-400 uppercase tracking-wider">{title}</span>
        <button 
          onClick={(e) => { e.stopPropagation(); onClose(); }}
          className="w-4 h-4 rounded bg-zinc-800 hover:bg-red-500 transition-colors text-[10px] text-zinc-600 hover:text-white flex items-center justify-center"
        >
          ×
        </button>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-auto" style={{ height: `calc(100% - ${statusText ? "56px" : "32px"})` }}>
        {children}
      </div>

      {/* Status bar */}
      {statusText && (
        <div className="h-6 px-3 bg-zinc-900 border-t border-zinc-800 flex items-center">
          <span className="text-[10px] font-mono text-zinc-600">{statusText}</span>
        </div>
      )}
    </motion.div>
  );
}

/* ===============================
   TERMINAL APP
   =============================== */

function TerminalApp({ accentColor }: { accentColor: string }) {
  const [history, setHistory] = useState<{ input: string; output: string }[]>([]);
  const [input, setInput] = useState("");
  const outputRef = useRef<HTMLDivElement>(null);

  const executeCommand = (cmd: string) => {
    const trimmed = cmd.trim().toLowerCase();
    let output = "";
    
    if (trimmed === "clear") {
      setHistory([]);
      setInput("");
      return;
    }
    
    if (trimmed in SYSTEM_COPY.terminal.commands) {
      output = SYSTEM_COPY.terminal.commands[trimmed as keyof typeof SYSTEM_COPY.terminal.commands];
    } else if (trimmed === "") {
      return;
    } else {
      output = `Command not found: ${trimmed}\nType 'help' for available commands.`;
    }
    
    setHistory(prev => [...prev, { input: cmd, output }]);
    setInput("");
  };

  useEffect(() => {
    if (outputRef.current) {
      outputRef.current.scrollTop = outputRef.current.scrollHeight;
    }
  }, [history]);

  return (
    <div className="h-full flex flex-col bg-black font-mono text-sm">
      <div ref={outputRef} className="flex-1 overflow-auto p-4 space-y-4">
        {/* Welcome message */}
        <div className="text-zinc-500">
          x3Star Terminal v1.0
          <br />
          Type &apos;help&apos; for available commands.
        </div>

        {/* Command history */}
        {history.map((item, i) => (
          <div key={i}>
            <div className="flex items-center gap-2">
              <span style={{ color: accentColor }}>❯</span>
              <span className="text-zinc-300">{item.input}</span>
            </div>
            <pre className="text-zinc-400 mt-1 whitespace-pre-wrap text-xs leading-relaxed">
              {item.output}
            </pre>
          </div>
        ))}

        {/* Current input */}
        <div className="flex items-center gap-2">
          <span style={{ color: accentColor }}>❯</span>
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") executeCommand(input);
            }}
            className="flex-1 bg-transparent text-zinc-300 outline-none"
            autoFocus
          />
        </div>
      </div>

      {/* Preset command buttons */}
      <div className="p-3 border-t border-zinc-800 flex gap-2 flex-wrap">
        {["status", "vms", "atomic", "why", "help"].map(cmd => (
          <button
            key={cmd}
            onClick={() => executeCommand(cmd)}
            className="px-3 py-1 text-xs border border-zinc-700 text-zinc-500 hover:text-zinc-300 hover:border-zinc-600 transition-colors rounded"
          >
            {cmd}
          </button>
        ))}
      </div>
    </div>
  );
}

/* ===============================
   EXECUTION ENGINE APP
   =============================== */

function ExecutionEngineApp({ accentColor }: { accentColor: string }) {
  const [activeStage, setActiveStage] = useState(0);

  useEffect(() => {
    const interval = setInterval(() => {
      setActiveStage(prev => (prev + 1) % 5);
    }, 2000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="h-full p-6 font-mono text-xs bg-zinc-950">
      {/* Tagline */}
      <div className="text-center mb-8">
        <div className="text-2xl font-bold text-zinc-300 tracking-tight">
          {SYSTEM_COPY.execution.tagline}
        </div>
      </div>

      {/* Execution pipeline */}
      <div className="flex items-center justify-between mb-8">
        {SYSTEM_COPY.execution.stages.map((stage, i) => (
          <div key={stage.id} className="flex items-center">
            <div className={`w-24 text-center ${i <= activeStage ? "text-zinc-300" : "text-zinc-700"}`}>
              <div 
                className={`w-12 h-12 mx-auto mb-2 rounded-lg flex items-center justify-center border-2 transition-all ${
                  i === activeStage 
                    ? "border-current animate-pulse" 
                    : i < activeStage 
                      ? "border-emerald-500 bg-emerald-500/10" 
                      : "border-zinc-800"
                }`}
                style={{ borderColor: i === activeStage ? accentColor : undefined }}
              >
                <span className="text-lg">{i < activeStage ? "✓" : i + 1}</span>
              </div>
              <div className="text-[10px] uppercase tracking-wider">{stage.label}</div>
            </div>
            {i < 4 && (
              <div className={`w-16 h-0.5 ${i < activeStage ? "bg-emerald-500" : "bg-zinc-800"}`} />
            )}
          </div>
        ))}
      </div>

      {/* Live execution visualization */}
      <div className="border border-zinc-800 rounded-lg p-4 bg-black">
        <div className="text-zinc-500 mb-3 uppercase text-[10px] tracking-wider">Live Execution Queue</div>
        <div className="space-y-2">
          {[...Array(5)].map((_, i) => (
            <motion.div
              key={i}
              initial={{ opacity: 0, x: -10 }}
              animate={{ opacity: 1, x: 0 }}
              transition={{ delay: i * 0.1 }}
              className="flex items-center gap-3 text-zinc-600"
            >
              <span className="w-20 text-zinc-700">0x{Math.random().toString(16).slice(2, 10)}</span>
              <div className="flex-1 h-2 bg-zinc-900 rounded overflow-hidden">
                <motion.div 
                  className="h-full rounded"
                  style={{ backgroundColor: accentColor }}
                  initial={{ width: 0 }}
                  animate={{ width: `${Math.random() * 100}%` }}
                  transition={{ duration: 0.5, delay: i * 0.1 }}
                />
              </div>
              <span className="text-[10px] text-zinc-700">{["EVM", "SVM", "x3VM"][i % 3]}</span>
            </motion.div>
          ))}
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-4 mt-6">
        {[
          { label: "Pending", value: "247" },
          { label: "Executed", value: "1.2M" },
          { label: "Reverted", value: "0.01%" },
          { label: "Avg Gas", value: "42,847" }
        ].map(stat => (
          <div key={stat.label} className="text-center p-3 border border-zinc-800 rounded-lg bg-zinc-900/50">
            <div className="text-xl font-bold text-zinc-300">{stat.value}</div>
            <div className="text-[10px] text-zinc-600 uppercase tracking-wider">{stat.label}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

/* ===============================
   VM MANAGER APP
   =============================== */

function VMManagerApp({ 
  activeVM, 
  onVMChange,
  accentColor 
}: { 
  activeVM: VMType;
  onVMChange: (vm: VMType) => void;
  accentColor: string;
}) {
  const vms: VMType[] = ["evm", "svm", "x3vm", "btc"];
  const vmData = SYSTEM_COPY.vms[activeVM];

  return (
    <div className="h-full flex font-mono text-xs">
      {/* VM Selector */}
      <div className="w-24 border-r border-zinc-800 bg-zinc-900/50">
        {vms.map(vm => (
          <button
            key={vm}
            onClick={() => onVMChange(vm)}
            className={`w-full h-16 flex flex-col items-center justify-center transition-all ${
              activeVM === vm 
                ? "bg-zinc-800 text-zinc-200" 
                : "text-zinc-600 hover:bg-zinc-900 hover:text-zinc-400"
            }`}
            style={{ 
              borderLeft: activeVM === vm ? `3px solid ${ACCENT_COLORS[vm]}` : "3px solid transparent"
            }}
          >
            <span className="text-lg font-bold uppercase">{vm}</span>
            <span className="text-[8px] text-emerald-500 mt-1">● ACTIVE</span>
          </button>
        ))}
      </div>

      {/* VM Details */}
      <div className="flex-1 p-6 overflow-auto">
        <div className="mb-6">
          <div 
            className="text-3xl font-bold tracking-tight mb-1"
            style={{ color: ACCENT_COLORS[activeVM] }}
          >
            {vmData.name}
          </div>
          <div className="text-zinc-500 text-sm">{vmData.fullName}</div>
        </div>

        <div className="text-zinc-400 mb-6 leading-relaxed">
          {vmData.description}
        </div>

        {/* Specs */}
        <div className="border border-zinc-800 rounded-lg overflow-hidden mb-6">
          <div className="bg-zinc-900 px-4 py-2 text-[10px] text-zinc-500 uppercase tracking-wider border-b border-zinc-800">
            Specifications
          </div>
          <div className="p-4 space-y-2">
            {vmData.specs.map((spec, i) => (
              <div key={i} className="flex items-center gap-2 text-zinc-400">
                <span style={{ color: accentColor }}>▪</span>
                {spec}
              </div>
            ))}
          </div>
        </div>

        {/* Performance */}
        <div className="grid grid-cols-2 gap-4">
          <div className="border border-zinc-800 rounded-lg p-4 text-center">
            <div className="text-2xl font-bold text-zinc-300">{vmData.tps}</div>
            <div className="text-[10px] text-zinc-600 uppercase tracking-wider">TPS</div>
          </div>
          <div className="border border-zinc-800 rounded-lg p-4 text-center">
            <div className="text-2xl font-bold text-zinc-300">{vmData.latency}</div>
            <div className="text-[10px] text-zinc-600 uppercase tracking-wider">Latency</div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ===============================
   ATOMIC LAYER APP
   =============================== */

function AtomicLayerApp({ accentColor }: { accentColor: string }) {
  const [phase, setPhase] = useState(0);
  const [success, setSuccess] = useState(true);

  useEffect(() => {
    const interval = setInterval(() => {
      if (phase < 4) {
        setPhase(prev => prev + 1);
      } else {
        setPhase(0);
        setSuccess(Math.random() > 0.1);
      }
    }, 800);
    return () => clearInterval(interval);
  }, [phase]);

  const lanes = ["EVM", "SVM", "BTC"];

  return (
    <div className="h-full p-6 font-mono text-xs bg-zinc-950">
      {/* Tagline */}
      <div className="text-center mb-8">
        <div className="text-xl font-bold text-zinc-300 tracking-tight">
          {SYSTEM_COPY.atomic.tagline}
        </div>
      </div>

      {/* Timeline lanes */}
      <div className="space-y-4 mb-8">
        {lanes.map((lane, laneIndex) => (
          <div key={lane} className="flex items-center gap-4">
            <div className="w-12 text-zinc-500 text-right">{lane}</div>
            <div className="flex-1 h-8 bg-zinc-900 rounded relative overflow-hidden border border-zinc-800">
              {/* Progress */}
              <motion.div
                className="absolute inset-y-0 left-0"
                style={{ 
                  backgroundColor: success ? accentColor : "#ef4444",
                  opacity: 0.3
                }}
                animate={{ 
                  width: phase >= 4 
                    ? (success ? "100%" : "0%") 
                    : `${(phase / 4) * 100}%` 
                }}
                transition={{ duration: 0.3 }}
              />
              {/* Blocks */}
              <div className="absolute inset-0 flex items-center justify-around px-4">
                {[...Array(4)].map((_, i) => (
                  <motion.div
                    key={i}
                    className="w-4 h-4 rounded"
                    style={{ 
                      backgroundColor: i <= phase && phase < 4 
                        ? accentColor 
                        : phase >= 4 && success 
                          ? "#22c55e"
                          : phase >= 4 && !success
                            ? "#ef4444"
                            : "#27272a"
                    }}
                    animate={{ 
                      scale: i === phase && phase < 4 ? 1.3 : 1,
                      opacity: phase >= 4 && !success ? 0.3 : 1
                    }}
                  />
                ))}
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Status */}
      <div className={`text-center p-4 rounded-lg border ${
        phase >= 4 
          ? success 
            ? "border-emerald-500/30 bg-emerald-500/5 text-emerald-400"
            : "border-red-500/30 bg-red-500/5 text-red-400"
          : "border-zinc-800 text-zinc-500"
      }`}>
        {phase >= 4 
          ? success 
            ? "ATOMIC COMMIT SUCCESSFUL — All VMs in sync"
            : "ATOMIC ROLLBACK — Constraint violation detected"
          : "EXECUTING ATOMIC BATCH..."
        }
      </div>

      {/* Stats */}
      <div className="grid grid-cols-3 gap-4 mt-6">
        {[
          { label: "Atomic Batches", value: "847,293" },
          { label: "Success Rate", value: "99.97%" },
          { label: "Avg Batch Size", value: "12 TXs" }
        ].map(stat => (
          <div key={stat.label} className="text-center p-3 border border-zinc-800 rounded-lg">
            <div className="text-lg font-bold text-zinc-300">{stat.value}</div>
            <div className="text-[10px] text-zinc-600 uppercase tracking-wider">{stat.label}</div>
          </div>
        ))}
      </div>
    </div>
  );
}

/* ===============================
   ECOSYSTEM / PROCESS MONITOR APP
   =============================== */

function EcosystemApp({ accentColor }: { accentColor: string }) {
  const [processes, setProcesses] = useState(SYSTEM_COPY.ecosystem.processes);

  useEffect(() => {
    const interval = setInterval(() => {
      setProcesses(prev => prev.map(p => ({
        ...p,
        cpu: Math.max(1, Math.min(99, p.cpu + (Math.random() - 0.5) * 10)),
        mem: Math.max(32, p.mem + Math.floor((Math.random() - 0.5) * 32))
      })));
    }, 1500);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="h-full font-mono text-xs bg-zinc-950">
      {/* Header */}
      <div className="p-4 border-b border-zinc-800">
        <div className="text-zinc-500 uppercase text-[10px] tracking-wider">
          {SYSTEM_COPY.ecosystem.note}
        </div>
      </div>

      {/* Process table */}
      <div className="overflow-auto">
        <table className="w-full">
          <thead className="bg-zinc-900 sticky top-0">
            <tr className="text-[10px] text-zinc-600 uppercase tracking-wider">
              <th className="text-left p-3">Process</th>
              <th className="text-left p-3">Type</th>
              <th className="text-left p-3">Status</th>
              <th className="text-right p-3">CPU %</th>
              <th className="text-right p-3">MEM MB</th>
            </tr>
          </thead>
          <tbody>
            {processes.map((proc, i) => (
              <tr 
                key={proc.name} 
                className="border-b border-zinc-900 hover:bg-zinc-900/50 transition-colors"
              >
                <td className="p-3 text-zinc-300">{proc.name}</td>
                <td className="p-3 text-zinc-500">{proc.type}</td>
                <td className="p-3">
                  <span className="flex items-center gap-2">
                    <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse" />
                    <span className="text-emerald-400">{proc.status}</span>
                  </span>
                </td>
                <td className="p-3 text-right">
                  <span style={{ color: proc.cpu > 50 ? accentColor : "#71717a" }}>
                    {proc.cpu.toFixed(1)}%
                  </span>
                </td>
                <td className="p-3 text-right text-zinc-500">{proc.mem}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Summary */}
      <div className="p-4 border-t border-zinc-800 bg-zinc-900/50 flex gap-6">
        <div>
          <span className="text-zinc-600">Processes: </span>
          <span className="text-zinc-300">{processes.length}</span>
        </div>
        <div>
          <span className="text-zinc-600">Total CPU: </span>
          <span style={{ color: accentColor }}>
            {processes.reduce((a, p) => a + p.cpu, 0).toFixed(1)}%
          </span>
        </div>
        <div>
          <span className="text-zinc-600">Total MEM: </span>
          <span className="text-zinc-300">
            {processes.reduce((a, p) => a + p.mem, 0)} MB
          </span>
        </div>
      </div>
    </div>
  );
}

/* ===============================
   SYSTEM OVERVIEW APP
   =============================== */

function SystemOverviewApp() {
  return (
    <div className="h-full p-8 font-mono flex flex-col items-center justify-center text-center bg-zinc-950">
      <div className="text-4xl font-bold text-zinc-200 mb-2 tracking-tight">
        {SYSTEM_COPY.overview.title}
      </div>
      <div className="text-lg text-zinc-500 mb-8">
        {SYSTEM_COPY.overview.subtitle}
      </div>
      <div className="max-w-lg text-sm text-zinc-400 leading-relaxed whitespace-pre-line">
        {SYSTEM_COPY.overview.description}
      </div>
    </div>
  );
}

/* ===============================
   ROOT OS SHELL
   =============================== */

interface WindowState {
  id: AppId;
  isOpen: boolean;
  position: { x: number; y: number };
  size: { width: number; height: number };
}

const DEFAULT_WINDOWS: Record<AppId, WindowState> = {
  terminal: { id: "terminal", isOpen: false, position: { x: 100, y: 80 }, size: { width: 600, height: 400 } },
  execution: { id: "execution", isOpen: false, position: { x: 180, y: 100 }, size: { width: 700, height: 500 } },
  vmmanager: { id: "vmmanager", isOpen: false, position: { x: 260, y: 120 }, size: { width: 600, height: 450 } },
  atomic: { id: "atomic", isOpen: false, position: { x: 340, y: 140 }, size: { width: 650, height: 420 } },
  ecosystem: { id: "ecosystem", isOpen: false, position: { x: 420, y: 160 }, size: { width: 700, height: 400 } },
  overview: { id: "overview", isOpen: false, position: { x: 200, y: 150 }, size: { width: 500, height: 350 } }
};

export default function X3StarOS() {
  const [booted, setBooted] = useState(false);
  const [activeVM, setActiveVM] = useState<VMType>("evm");
  const [blockHeight, setBlockHeight] = useState(1847293);
  const [windows, setWindows] = useState<Record<AppId, WindowState>>(DEFAULT_WINDOWS);
  const [focusedWindow, setFocusedWindow] = useState<AppId | null>(null);

  const accentColor = ACCENT_COLORS[activeVM];

  // Simulate block height updates
  useEffect(() => {
    if (!booted) return;
    const interval = setInterval(() => {
      setBlockHeight(prev => prev + 1);
    }, 6000);
    return () => clearInterval(interval);
  }, [booted]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (!booted) return;
      
      if (e.key === "Escape" && focusedWindow) {
        closeWindow(focusedWindow);
        return;
      }

      if ((e.metaKey || e.ctrlKey)) {
        switch (e.key) {
          case "t": e.preventDefault(); openWindow("terminal"); break;
          case "e": e.preventDefault(); openWindow("execution"); break;
          case "v": e.preventDefault(); openWindow("vmmanager"); break;
          case "a": e.preventDefault(); openWindow("atomic"); break;
          case "p": e.preventDefault(); openWindow("ecosystem"); break;
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [booted, focusedWindow]);

  const openWindow = useCallback((id: AppId) => {
    setWindows(prev => ({
      ...prev,
      [id]: { ...prev[id], isOpen: true }
    }));
    setFocusedWindow(id);
  }, []);

  const closeWindow = useCallback((id: AppId) => {
    setWindows(prev => ({
      ...prev,
      [id]: { ...prev[id], isOpen: false }
    }));
    if (focusedWindow === id) {
      const openWindows = Object.values(windows).filter(w => w.isOpen && w.id !== id);
      setFocusedWindow(openWindows.length > 0 ? openWindows[openWindows.length - 1].id : null);
    }
  }, [focusedWindow, windows]);

  const renderWindowContent = (id: AppId) => {
    switch (id) {
      case "terminal":
        return <TerminalApp accentColor={accentColor} />;
      case "execution":
        return <ExecutionEngineApp accentColor={accentColor} />;
      case "vmmanager":
        return <VMManagerApp activeVM={activeVM} onVMChange={setActiveVM} accentColor={accentColor} />;
      case "atomic":
        return <AtomicLayerApp accentColor={accentColor} />;
      case "ecosystem":
        return <EcosystemApp accentColor={accentColor} />;
      case "overview":
        return <SystemOverviewApp />;
      default:
        return null;
    }
  };

  const getWindowTitle = (id: AppId) => {
    switch (id) {
      case "terminal": return "Terminal — Execution Interface";
      case "execution": return "Execution Engine";
      case "vmmanager": return "VM Manager";
      case "atomic": return "Atomic Layer";
      case "ecosystem": return "Process Monitor";
      case "overview": return "System Overview";
      default: return id;
    }
  };

  return (
    <div className="w-screen h-screen bg-zinc-950 overflow-hidden select-none">
      <AnimatePresence mode="wait">
        {!booted && <BootSequence onComplete={() => setBooted(true)} />}
      </AnimatePresence>

      {booted && (
        <>
          <SystemBar 
            activeVM={activeVM} 
            blockHeight={blockHeight} 
            onLogoClick={() => openWindow("overview")}
          />
          <Dock 
            activeApp={focusedWindow} 
            onAppClick={openWindow} 
          />
          
          {/* Desktop area */}
          <div className="fixed top-12 left-16 right-0 bottom-0">
            {/* Grid overlay (subtle) */}
            <div 
              className="absolute inset-0 opacity-[0.02] pointer-events-none"
              style={{
                backgroundImage: `
                  linear-gradient(to right, #fff 1px, transparent 1px),
                  linear-gradient(to bottom, #fff 1px, transparent 1px)
                `,
                backgroundSize: "calc(100% / 12) 40px"
              }}
            />

            {/* Windows */}
            <AnimatePresence>
              {Object.values(windows).filter(w => w.isOpen).map(win => (
                <Window
                  key={win.id}
                  title={getWindowTitle(win.id)}
                  isActive={focusedWindow === win.id}
                  onFocus={() => setFocusedWindow(win.id)}
                  onClose={() => closeWindow(win.id)}
                  position={win.position}
                  size={win.size}
                  accentColor={accentColor}
                  statusText={win.id === "terminal" ? "Ready" : undefined}
                >
                  {renderWindowContent(win.id)}
                </Window>
              ))}
            </AnimatePresence>

            {/* Empty state */}
            {!Object.values(windows).some(w => w.isOpen) && (
              <div className="absolute inset-0 flex items-center justify-center">
                <div className="text-center font-mono">
                  <div className="text-zinc-700 text-sm mb-4">No windows open</div>
                  <div className="text-zinc-800 text-xs">
                    Click dock icons or use keyboard shortcuts
                    <br />
                    <span className="text-zinc-600">⌘T</span> Terminal · 
                    <span className="text-zinc-600"> ⌘E</span> Execution · 
                    <span className="text-zinc-600"> ⌘V</span> VMs
                  </div>
                </div>
              </div>
            )}
          </div>
        </>
      )}
    </div>
  );
}
