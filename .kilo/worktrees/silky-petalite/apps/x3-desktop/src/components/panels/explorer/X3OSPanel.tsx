import React, { useState, useRef, useEffect, useCallback } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { Terminal, Monitor, Layers, ChevronRight, Circle } from "lucide-react";

/* ------------------------------------------------------------------ */
/*  VM definitions                                                     */
/* ------------------------------------------------------------------ */
const vms = [
  { id: "evm", name: "EVM", color: "text-cyan-400", bg: "bg-cyan-500/10", border: "border-cyan-500/30", status: "running", memory: "2.1 GB", uptime: "14d 6h" },
  { id: "svm", name: "SVM", color: "text-violet-400", bg: "bg-violet-500/10", border: "border-violet-500/30", status: "running", memory: "1.8 GB", uptime: "14d 6h" },
  { id: "x3vm", name: "x3VM", color: "text-amber-400", bg: "bg-amber-500/10", border: "border-amber-500/30", status: "running", memory: "3.4 GB", uptime: "14d 6h" },
  { id: "btc", name: "BTC", color: "text-orange-400", bg: "bg-orange-500/10", border: "border-orange-500/30", status: "idle", memory: "0.6 GB", uptime: "14d 6h" },
] as const;

type TabKey = "terminal" | "vm-manager" | "atomic";

const tabItems: { key: TabKey; label: string; icon: React.ReactNode }[] = [
  { key: "terminal", label: "Terminal", icon: <Terminal className="w-3.5 h-3.5" /> },
  { key: "vm-manager", label: "VM Manager", icon: <Monitor className="w-3.5 h-3.5" /> },
  { key: "atomic", label: "Atomic Pipeline", icon: <Layers className="w-3.5 h-3.5" /> },
];

/* ------------------------------------------------------------------ */
/*  Terminal command handler                                           */
/* ------------------------------------------------------------------ */
const COMMANDS: Record<string, string[]> = {
  status: [
    "╔══════════════════════════════════════╗",
    "║       x3Star OS v2.4.1 — Status     ║",
    "╚══════════════════════════════════════╝",
    " Chain:        X3 X3 Chain",
    " Block:        #1,847,293",
    " Finality:     6.0s",
    " VMs:          4 active (EVM, SVM, x3VM, BTC)",
    " Peers:        42",
    " Sync:         ✓ Fully synced",
  ],
  vms: [
    "┌──────┬─────────┬──────────┬───────────┐",
    "│  VM  │ Status  │  Memory  │  Uptime   │",
    "├──────┼─────────┼──────────┼───────────┤",
    "│ EVM  │ running │  2.1 GB  │  14d 6h   │",
    "│ SVM  │ running │  1.8 GB  │  14d 6h   │",
    "│ x3VM │ running │  3.4 GB  │  14d 6h   │",
    "│ BTC  │  idle   │  0.6 GB  │  14d 6h   │",
    "└──────┴─────────┴──────────┴───────────┘",
  ],
  atomic: [
    "Atomic Execution Pipeline:",
    "  [1] Prepare   ████████████████████ 100%",
    "  [2] Verify    ████████████████████ 100%",
    "  [3] Execute   ██████████░░░░░░░░░░  52%",
    "  [4] Finalize  ░░░░░░░░░░░░░░░░░░░░   0%",
    "",
    "  Status: Executing cross-VM transaction…",
  ],
  help: [
    "Available commands:",
    "  status    — Show chain status",
    "  vms       — List virtual machines",
    "  atomic    — Show atomic pipeline status",
    "  help      — Show this help",
    "  clear     — Clear terminal",
  ],
};

/* ================================================================== */
/*  X3OSPanel                                                          */
/* ================================================================== */
export default function X3OSPanel() {
  const [activeTab, setActiveTab] = useState<TabKey>("terminal");

  return (
    <div className="overflow-y-auto h-full bg-gradient-to-b from-[#0a0a14] via-[#0a0c18] to-black text-white flex flex-col">
      {/* ---- Sidebar + Main ---- */}
      <div className="flex flex-1 min-h-0">
        {/* Sidebar */}
        <div className="w-16 md:w-48 shrink-0 bg-slate-900/80 border-r border-slate-700/40 flex flex-col py-4 gap-2">
          <div className="px-3 mb-3 hidden md:block">
            <span className="text-[10px] uppercase tracking-widest text-slate-500 font-semibold">Virtual Machines</span>
          </div>
          {vms.map((vm) => (
            <div
              key={vm.id}
              className={`mx-2 px-3 py-2 rounded-lg ${vm.bg} border ${vm.border} flex items-center gap-2 cursor-pointer hover:brightness-125 transition`}
            >
              <Circle
                className={`w-2.5 h-2.5 ${vm.status === "running" ? "text-green-400 fill-green-400" : "text-slate-500 fill-slate-500"}`}
              />
              <span className={`${vm.color} font-semibold text-sm hidden md:inline`}>{vm.name}</span>
              <span className={`${vm.color} font-semibold text-xs md:hidden`}>{vm.name}</span>
            </div>
          ))}
        </div>

        {/* Main area */}
        <div className="flex-1 flex flex-col min-h-0">
          {/* Tab bar */}
          <div className="flex bg-slate-900/60 border-b border-slate-700/40">
            {tabItems.map((t) => (
              <button
                key={t.key}
                onClick={() => setActiveTab(t.key)}
                className={`flex items-center gap-1.5 px-4 py-2.5 text-xs font-medium transition border-b-2 ${
                  activeTab === t.key
                    ? "border-cyan-400 text-cyan-300"
                    : "border-transparent text-slate-500 hover:text-slate-300"
                }`}
              >
                {t.icon}
                {t.label}
              </button>
            ))}
          </div>

          {/* Content */}
          <div className="flex-1 min-h-0 overflow-hidden">
            <AnimatePresence mode="wait">
              {activeTab === "terminal" && (
                <motion.div key="terminal" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="h-full">
                  <TerminalWindow />
                </motion.div>
              )}
              {activeTab === "vm-manager" && (
                <motion.div key="vm-manager" initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} className="h-full overflow-y-auto p-5">
                  <VMManagerWindow />
                </motion.div>
              )}
              {activeTab === "atomic" && (
                <motion.div key="atomic" initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }} className="h-full overflow-y-auto p-5">
                  <AtomicPipelineWindow />
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </div>
      </div>
    </div>
  );
}

/* ================================================================== */
/*  Terminal                                                           */
/* ================================================================== */
function TerminalWindow() {
  const [lines, setLines] = useState<{ type: "input" | "output"; text: string }[]>([
    { type: "output", text: "x3Star OS v2.4.1 — Type 'help' for commands" },
  ]);
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [lines]);

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      const cmd = input.trim().toLowerCase();
      setInput("");
      const newLines = [
        ...lines,
        { type: "input" as const, text: `x3os@x3:~$ ${input}` },
      ];
      if (cmd === "clear") {
        setLines([]);
        return;
      }
      const output = COMMANDS[cmd];
      if (output) {
        output.forEach((line) => newLines.push({ type: "output", text: line }));
      } else if (cmd) {
        newLines.push({ type: "output", text: `x3os: command not found: ${cmd}` });
      }
      setLines(newLines);
    },
    [input, lines]
  );

  return (
    <div className="h-full flex flex-col bg-black/60 font-mono text-xs">
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 space-y-0.5">
        {lines.map((l, i) => (
          <div key={i} className={l.type === "input" ? "text-cyan-400" : "text-slate-300"}>
            {l.text}
          </div>
        ))}
      </div>
      <form onSubmit={handleSubmit} className="flex items-center border-t border-slate-800 px-4 py-2 bg-black/40">
        <span className="text-cyan-400 mr-2">x3os@x3:~$</span>
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          className="flex-1 bg-transparent outline-none text-slate-200 caret-cyan-400"
          autoFocus
        />
      </form>
    </div>
  );
}

/* ================================================================== */
/*  VM Manager                                                         */
/* ================================================================== */
function VMManagerWindow() {
  return (
    <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
      {vms.map((vm) => (
        <motion.div
          key={vm.id}
          initial={{ opacity: 0, scale: 0.95 }}
          animate={{ opacity: 1, scale: 1 }}
          transition={{ delay: vms.indexOf(vm) * 0.08 }}
          className={`${vm.bg} border ${vm.border} rounded-xl p-4 space-y-3`}
        >
          <div className="flex items-center justify-between">
            <span className={`${vm.color} font-bold text-lg`}>{vm.name}</span>
            <span
              className={`px-2 py-0.5 rounded-full text-[10px] font-semibold ${
                vm.status === "running"
                  ? "bg-green-500/20 text-green-400 border border-green-500/30"
                  : "bg-slate-500/20 text-slate-400 border border-slate-500/30"
              }`}
            >
              {vm.status}
            </span>
          </div>
          <div className="grid grid-cols-2 gap-3 text-xs">
            <div>
              <span className="text-slate-500">Memory</span>
              <div className="text-slate-200 font-medium">{vm.memory}</div>
            </div>
            <div>
              <span className="text-slate-500">Uptime</span>
              <div className="text-slate-200 font-medium">{vm.uptime}</div>
            </div>
          </div>
        </motion.div>
      ))}
    </div>
  );
}

/* ================================================================== */
/*  Atomic Pipeline                                                    */
/* ================================================================== */
const pipelineSteps = [
  { label: "Prepare", progress: 100, color: "bg-green-500" },
  { label: "Verify", progress: 100, color: "bg-cyan-500" },
  { label: "Execute", progress: 52, color: "bg-amber-500" },
  { label: "Finalize", progress: 0, color: "bg-violet-500" },
];

function AtomicPipelineWindow() {
  return (
    <div className="space-y-6">
      <h2 className="text-sm font-semibold text-slate-300">Atomic Execution Pipeline</h2>
      <div className="flex items-center gap-2">
        {pipelineSteps.map((step, i) => (
          <React.Fragment key={step.label}>
            <motion.div
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ delay: i * 0.12 }}
              className="flex-1 bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 text-center space-y-3"
            >
              <div className="text-xs font-semibold text-slate-300">{step.label}</div>
              <div className="w-full h-2 rounded-full bg-slate-700 overflow-hidden">
                <motion.div
                  className={`h-full rounded-full ${step.color}`}
                  initial={{ width: 0 }}
                  animate={{ width: `${step.progress}%` }}
                  transition={{ duration: 1, delay: i * 0.2 }}
                />
              </div>
              <div className="text-[10px] text-slate-500">{step.progress}%</div>
            </motion.div>
            {i < pipelineSteps.length - 1 && (
              <ChevronRight className="w-4 h-4 text-slate-600 shrink-0" />
            )}
          </React.Fragment>
        ))}
      </div>
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-4 text-xs text-slate-400 font-mono">
        <span className="text-amber-400">STATUS:</span> Executing cross-VM transaction EVM → x3VM → SVM
        <br />
        <span className="text-slate-500">TX: 0x7fa3…e91b │ Nonce: 847 │ Gas: 142,000</span>
      </div>
    </div>
  );
}
