import React, { useState, useRef, useEffect, useCallback } from "react";

/* ------------------------------------------------------------------ */
/*  Boot lines                                                         */
/* ------------------------------------------------------------------ */
const BOOT_LINES = [
  "x3Star Terminal v3.1.0 — X3 Chain Kernel",
  "Loading runtime modules ██████████████████ OK",
  "Connecting to X3 X3 Chain (peers: 42) … OK",
  "System ready. Type 'help' for available commands.",
];

/* ------------------------------------------------------------------ */
/*  Commands                                                           */
/* ------------------------------------------------------------------ */
const COMMANDS: Record<string, string[]> = {
  status: [
    "┌──────────────────────────────────────────┐",
    "│           X3 X3 Chain Status           │",
    "├──────────────────────────────────────────┤",
    "│  Block Height   :  #1,847,293            │",
    "│  Finality       :  6.0s                  │",
    "│  Validators     :  64                    │",
    "│  TPS            :  12,847                │",
    "│  Peers          :  42                    │",
    "│  Sync           :  ✓ Fully synced        │",
    "│  Runtime        :  x3-chain v2.4.1   │",
    "└──────────────────────────────────────────┘",
  ],
  vms: [
    "┌──────┬─────────┬──────────┬───────────┐",
    "│  VM  │ Status  │  Memory  │  Uptime   │",
    "├──────┼─────────┼──────────┼───────────┤",
    "│ EVM  │ running │  2.1 GB  │  14d 06h  │",
    "│ SVM  │ running │  1.8 GB  │  14d 06h  │",
    "│ x3VM │ running │  3.4 GB  │  14d 06h  │",
    "│ BTC  │  idle   │  0.6 GB  │  14d 06h  │",
    "└──────┴─────────┴──────────┴───────────┘",
  ],
  help: [
    "┌─────────────────────────────────────┐",
    "│         Available Commands          │",
    "├─────────────────────────────────────┤",
    "│  status  — Chain status overview    │",
    "│  vms     — List virtual machines    │",
    "│  help    — Display this help        │",
    "│  clear   — Clear terminal           │",
    "└─────────────────────────────────────┘",
  ],
};

/* ------------------------------------------------------------------ */
/*  Quick stats                                                        */
/* ------------------------------------------------------------------ */
const quickStats = [
  { label: "Block", value: "#1,847,293" },
  { label: "Validators", value: "64" },
  { label: "TPS", value: "12,847" },
  { label: "Uptime", value: "99.99%" },
];

/* ================================================================== */
/*  X3StarPanel                                                        */
/* ================================================================== */
export default function X3StarPanel() {
  const [bootDone, setBootDone] = useState(false);
  const [bootIndex, setBootIndex] = useState(0);
  const [lines, setLines] = useState<{ type: "boot" | "input" | "output"; text: string }[]>([]);
  const [input, setInput] = useState("");
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  /* ---- Boot sequence ---- */
  useEffect(() => {
    if (bootIndex < BOOT_LINES.length) {
      const timer = setTimeout(() => {
        setLines((prev) => [...prev, { type: "boot", text: BOOT_LINES[bootIndex] }]);
        setBootIndex((i) => i + 1);
      }, 400);
      return () => clearTimeout(timer);
    } else if (!bootDone) {
      setBootDone(true);
    }
  }, [bootIndex, bootDone]);

  /* ---- Auto-scroll ---- */
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight });
  }, [lines]);

  /* ---- Focus input ---- */
  useEffect(() => {
    if (bootDone) inputRef.current?.focus();
  }, [bootDone]);

  /* ---- Submit command ---- */
  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      if (!bootDone) return;
      const cmd = input.trim().toLowerCase();
      setInput("");
      const next = [...lines, { type: "input" as const, text: `x3@x3star:~$ ${input}` }];
      if (cmd === "clear") {
        setLines([]);
        return;
      }
      const output = COMMANDS[cmd];
      if (output) {
        output.forEach((line) => next.push({ type: "output", text: line }));
      } else if (cmd) {
        next.push({ type: "output", text: `x3star: command not found: ${cmd}` });
      }
      setLines(next);
    },
    [input, lines, bootDone]
  );

  return (
    <div className="overflow-hidden h-full bg-black text-green-400 font-mono flex flex-col">
      {/* ---- Header bar ---- */}
      <div className="flex items-center justify-between px-4 py-2 bg-[#0a0a0a] border-b border-green-900/40 text-xs">
        <div className="flex items-center gap-2">
          <span className="w-2.5 h-2.5 rounded-full bg-green-500 animate-pulse" />
          <span className="text-green-500 font-bold">x3Star Terminal</span>
          <span className="text-green-800">v3.1.0</span>
        </div>
        <span className="text-green-800">x3-chain</span>
      </div>

      {/* ---- Body ---- */}
      <div className="flex flex-1 min-h-0">
        {/* Terminal */}
        <div
          className="flex-1 flex flex-col min-h-0 cursor-text"
          onClick={() => inputRef.current?.focus()}
        >
          <div ref={scrollRef} className="flex-1 overflow-y-auto px-4 py-3 space-y-0.5 text-xs leading-relaxed">
            {lines.map((l, i) => (
              <div
                key={i}
                className={
                  l.type === "boot"
                    ? "text-green-600"
                    : l.type === "input"
                    ? "text-green-300"
                    : "text-green-500/90"
                }
              >
                {l.text}
              </div>
            ))}
          </div>
          {bootDone && (
            <form
              onSubmit={handleSubmit}
              className="flex items-center px-4 py-2 border-t border-green-900/30 bg-black/60 shrink-0"
            >
              <span className="text-green-400 mr-2 text-xs">x3@x3star:~$</span>
              <input
                ref={inputRef}
                value={input}
                onChange={(e) => setInput(e.target.value)}
                className="flex-1 bg-transparent outline-none text-green-300 caret-green-400 text-xs"
                spellCheck={false}
              />
            </form>
          )}
        </div>

        {/* Side panel — quick stats */}
        <div className="hidden md:flex flex-col w-44 bg-[#050505] border-l border-green-900/30 px-3 py-4 gap-4 text-xs shrink-0">
          <div className="text-green-700 font-semibold uppercase tracking-widest text-[10px]">
            Quick Stats
          </div>
          {quickStats.map((s) => (
            <div key={s.label}>
              <div className="text-green-800 text-[10px]">{s.label}</div>
              <div className="text-green-400 font-bold">{s.value}</div>
            </div>
          ))}
          <div className="flex-1" />
          <div className="text-green-900 text-[10px] leading-snug">
            ┌───────────┐
            <br />│ {'  '}CONNECTED{'  '}│
            <br />└───────────┘
          </div>
        </div>
      </div>
    </div>
  );
}
