'use client';

import React, { useEffect, useRef, useState, useCallback } from 'react';
import { motion } from 'framer-motion';

interface TerminalLine {
  id: string;
  type: 'input' | 'output' | 'system' | 'error' | 'success' | 'warning';
  text: string;
  timestamp: Date;
}

const SYSTEM_MESSAGES = [
  '[ NEURAL ] Validator mesh synchronized across 142 nodes',
  '[ QUANTUM ] Entanglement verification: PASSED',
  '[ SHIELD ] Threat detection scan: 0 anomalies',
  '[ SYNC ] Block #8,924,518 validated in 0.47s',
  '[ AI ] Pattern recognition model updated',
  '[ NET ] Peer discovery: 14 new connections',
  '[ CRYPTO ] Zero-knowledge proof generated',
  '[ CACHE ] State trie optimized: 23% improvement',
  '[ AUDIT ] Transaction batch verified: 1,247 tx',
  '[ LAYER2 ] Rollup proof submitted to L1',
];

const COMMANDS: Record<string, () => string[]> = {
  'help': () => [
    '╔══════════════════════════════════════════════════════════════╗',
    '║  NEURAL VALIDATOR TERMINAL v3.0.1 - QUANTUM EDITION          ║',
    '╠══════════════════════════════════════════════════════════════╣',
    '║  Available Commands:                                         ║',
    '║                                                              ║',
    '║  status      - Show network status                          ║',
    '║  nodes       - List active validators                       ║',
    '║  threats     - View threat intelligence                     ║',
    '║  metrics     - Display performance metrics                  ║',
    '║  quantum     - Quantum state information                    ║',
    '║  clear       - Clear terminal                               ║',
    '║  help        - Show this help message                       ║',
    '╚══════════════════════════════════════════════════════════════╝',
  ],
  'status': () => [
    '┌─────────────────────────────────────────────────────┐',
    '│  NETWORK STATUS: OPTIMAL                            │',
    '├─────────────────────────────────────────────────────┤',
    `│  Block Height:      8,924,517                       │`,
    `│  Active Validators: 142 / 150                       │`,
    `│  TPS:               1,247                           │`,
    `│  Finality:          0.47s                           │`,
    `│  Network Health:    99.7%                           │`,
    `│  Threat Level:      LOW                             │`,
    '└─────────────────────────────────────────────────────┘',
  ],
  'nodes': () => [
    '┌──────────────────┬──────────┬────────┬─────────┐',
    '│ VALIDATOR ID     │ LOCATION │ SCORE  │ STATUS  │',
    '├──────────────────┼──────────┼────────┼─────────┤',
    '│ VAL-NYC-001      │ New York │  97    │ 🟢 LIVE │',
    '│ VAL-TOK-012      │ Tokyo    │  95    │ 🟢 LIVE │',
    '│ VAL-SGP-007      │ Singapore│  94    │ 🟢 LIVE │',
    '│ VAL-FRA-003      │ Frankfurt│  92    │ 🟡 SYNC │',
    '│ VAL-SYD-019      │ Sydney   │  91    │ 🟢 LIVE │',
    '│ ... and 137 more validators                    │',
    '└──────────────────┴──────────┴────────┴─────────┘',
  ],
  'threats': () => [
    '┌─────────────────────────────────────────────────────┐',
    '│  THREAT INTELLIGENCE REPORT                         │',
    '├─────────────────────────────────────────────────────┤',
    '│  [✓] DDoS Protection: ACTIVE                        │',
    '│  [✓] Sybil Detection: MONITORING                    │',
    '│  [✓] Eclipse Attack Prevention: ENABLED             │',
    '│  [✓] 51% Attack Detection: WATCHING                 │',
    '│                                                     │',
    '│  Last 24H:                                          │',
    '│  • 0 DDoS attempts blocked                          │',
    '│  • 3 suspicious IPs flagged                         │',
    '│  • 0 Sybil patterns detected                        │',
    '│                                                     │',
    '│  Neural AI Confidence: 99.2%                        │',
    '└─────────────────────────────────────────────────────┘',
  ],
  'metrics': () => [
    '┌─────────────────────────────────────────────────────┐',
    '│  PERFORMANCE METRICS                                │',
    '├─────────────────────────────────────────────────────┤',
    '│                                                     │',
    `│  CPU Usage:    ████████░░░░░░░░  47%               │`,
    `│  Memory:       ██████████░░░░░░  62%               │`,
    `│  Network I/O:  █████████████░░░  84%               │`,
    `│  Disk:         ████░░░░░░░░░░░░  28%               │`,
    '│                                                     │',
    `│  Avg Block Time:    0.47s                          │`,
    `│  Avg Gas Used:      12.4M                          │`,
    `│  Pending TX Pool:   847                            │`,
    `│  Peer Connections:  142                            │`,
    '└─────────────────────────────────────────────────────┘',
  ],
  'quantum': () => [
    '┌─────────────────────────────────────────────────────┐',
    '│  QUANTUM STATE INFORMATION                          │',
    '├─────────────────────────────────────────────────────┤',
    '│                                                     │',
    '│  Entanglement Status: STABLE                        │',
    '│  Qubit Coherence: 99.847%                          │',
    '│  Superposition Layers: 7                           │',
    '│  Quantum Error Rate: 0.0023%                       │',
    '│                                                     │',
    '│  Cryptography Mode: CRYSTALS-Kyber-1024            │',
    '│  Post-Quantum Ready: YES                           │',
    '│                                                     │',
    '│  ⚛️  Quantum Random Beacon: ACTIVE                  │',
    '│  🔐 QKD Channel: SECURED                           │',
    '└─────────────────────────────────────────────────────┘',
  ],
};

export default function QuantumTerminal() {
  const [lines, setLines] = useState<TerminalLine[]>([]);
  const [input, setInput] = useState('');
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const terminalRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  
  // Add system messages periodically
  useEffect(() => {
    const addSystemMessage = () => {
      const msg = SYSTEM_MESSAGES[Math.floor(Math.random() * SYSTEM_MESSAGES.length)];
      setLines(prev => [...prev, {
        id: `sys-${Date.now()}`,
        type: 'system',
        text: msg,
        timestamp: new Date()
      }]);
    };
    
    // Initial boot sequence
    const bootSequence = [
      '╔══════════════════════════════════════════════════════════════╗',
      '║                                                              ║',
      '║   ███╗   ██╗███████╗██╗   ██╗██████╗  █████╗ ██╗             ║',
      '║   ████╗  ██║██╔════╝██║   ██║██╔══██╗██╔══██╗██║             ║',
      '║   ██╔██╗ ██║█████╗  ██║   ██║██████╔╝███████║██║             ║',
      '║   ██║╚██╗██║██╔══╝  ██║   ██║██╔══██╗██╔══██║██║             ║',
      '║   ██║ ╚████║███████╗╚██████╔╝██║  ██║██║  ██║███████╗        ║',
      '║   ╚═╝  ╚═══╝╚══════╝ ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═╝╚══════╝        ║',
      '║                                                              ║',
      '║   NEURAL VALIDATOR TERMINAL v3.0.1 - QUANTUM EDITION         ║',
      '║   Copyright © 2060 Neural Validator Network                  ║',
      '║                                                              ║',
      '╚══════════════════════════════════════════════════════════════╝',
      '',
      '[ BOOT ] Initializing quantum substrate...',
      '[ BOOT ] Loading neural network models...',
      '[ BOOT ] Connecting to validator mesh...',
      '[ BOOT ] Synchronizing entangled state...',
      '[ BOOT ] System ready.',
      '',
      'Type "help" for available commands.',
      '',
    ];
    
    let i = 0;
    const bootInterval = setInterval(() => {
      if (i < bootSequence.length) {
        setLines(prev => [...prev, {
          id: `boot-${i}`,
          type: 'output',
          text: bootSequence[i],
          timestamp: new Date()
        }]);
        i++;
      } else {
        clearInterval(bootInterval);
      }
    }, 100);
    
    const systemInterval = setInterval(addSystemMessage, 5000);
    
    return () => {
      clearInterval(bootInterval);
      clearInterval(systemInterval);
    };
  }, []);
  
  // Auto-scroll to bottom
  useEffect(() => {
    if (terminalRef.current) {
      terminalRef.current.scrollTop = terminalRef.current.scrollHeight;
    }
  }, [lines]);
  
  const handleCommand = useCallback((cmd: string) => {
    const trimmed = cmd.trim().toLowerCase();
    
    // Add input line
    setLines(prev => [...prev, {
      id: `input-${Date.now()}`,
      type: 'input',
      text: `> ${cmd}`,
      timestamp: new Date()
    }]);
    
    // Add to history
    setHistory(prev => [...prev, cmd]);
    setHistoryIndex(-1);
    
    // Handle command
    if (trimmed === 'clear') {
      setLines([]);
      return;
    }
    
    if (COMMANDS[trimmed]) {
      const output = COMMANDS[trimmed]();
      output.forEach((line, i) => {
        setTimeout(() => {
          setLines(prev => [...prev, {
            id: `output-${Date.now()}-${i}`,
            type: 'output',
            text: line,
            timestamp: new Date()
          }]);
        }, i * 50);
      });
    } else if (trimmed) {
      setLines(prev => [...prev, {
        id: `error-${Date.now()}`,
        type: 'error',
        text: `Command not found: ${trimmed}. Type "help" for available commands.`,
        timestamp: new Date()
      }]);
    }
    
    setInput('');
  }, []);
  
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleCommand(input);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (history.length > 0) {
        const newIndex = historyIndex < history.length - 1 ? historyIndex + 1 : historyIndex;
        setHistoryIndex(newIndex);
        setInput(history[history.length - 1 - newIndex] || '');
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndex > 0) {
        const newIndex = historyIndex - 1;
        setHistoryIndex(newIndex);
        setInput(history[history.length - 1 - newIndex] || '');
      } else {
        setHistoryIndex(-1);
        setInput('');
      }
    }
  };
  
  const getLineColor = (type: TerminalLine['type']) => {
    switch (type) {
      case 'input': return 'text-cyan-400';
      case 'output': return 'text-gray-300';
      case 'system': return 'text-purple-400';
      case 'error': return 'text-red-400';
      case 'success': return 'text-green-400';
      case 'warning': return 'text-yellow-400';
      default: return 'text-gray-300';
    }
  };
  
  return (
    <div 
      className="bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950 rounded-2xl border border-cyan-500/30 shadow-2xl shadow-cyan-500/10 overflow-hidden"
      onClick={() => inputRef.current?.focus()}
    >
      {/* Terminal Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-gradient-to-r from-slate-900 to-slate-800 border-b border-cyan-500/20">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-red-500 hover:brightness-125 transition-all cursor-pointer" />
          <div className="w-3 h-3 rounded-full bg-yellow-500 hover:brightness-125 transition-all cursor-pointer" />
          <div className="w-3 h-3 rounded-full bg-green-500 hover:brightness-125 transition-all cursor-pointer" />
        </div>
        <div className="flex items-center gap-2">
          <span className="text-sm font-mono text-gray-400">neural-validator</span>
          <span className="text-xs text-green-400">●</span>
        </div>
        <div className="flex items-center gap-2 text-gray-500 text-xs">
          <span className="font-mono">v3.0.1</span>
        </div>
      </div>
      
      {/* Terminal Body */}
      <div 
        ref={terminalRef}
        className="h-[400px] overflow-y-auto p-4 font-mono text-sm leading-relaxed scrollbar-thin scrollbar-thumb-cyan-500/30 scrollbar-track-slate-800"
      >
        {lines.map((line) => (
          <motion.div
            key={line.id}
            initial={{ opacity: 0, x: -10 }}
            animate={{ opacity: 1, x: 0 }}
            className={`${getLineColor(line.type)} whitespace-pre-wrap mb-1`}
          >
            {line.text}
          </motion.div>
        ))}
        
        {/* Input Line */}
        <div className="flex items-center gap-2 mt-2">
          <span className="text-cyan-400">❯</span>
          <input
            ref={inputRef}
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            className="flex-1 bg-transparent border-none outline-none text-cyan-100 caret-cyan-400"
            autoFocus
            spellCheck={false}
          />
          <motion.div
            className="w-2 h-5 bg-cyan-400"
            animate={{ opacity: [1, 0] }}
            transition={{ duration: 0.8, repeat: Infinity }}
          />
        </div>
      </div>
      
      {/* Terminal Footer */}
      <div className="flex items-center justify-between px-4 py-2 bg-slate-900/50 border-t border-cyan-500/20 text-xs text-gray-500">
        <div className="flex gap-4">
          <span>Lines: {lines.length}</span>
          <span>|</span>
          <span>History: {history.length}</span>
        </div>
        <div className="flex gap-4">
          <span>↑↓ History</span>
          <span>|</span>
          <span>⏎ Execute</span>
        </div>
      </div>
    </div>
  );
}
