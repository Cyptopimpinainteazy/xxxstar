'use client';

import React, { useState, useEffect, useCallback, useRef } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import ChainStatusContainer from '@/components/ChainStatusContainer';

/* ═══════════════════════════════════════════════════════════════════════════════
   x3Star OS — Bloomberg Terminal Edition
   Production-Grade Execution Console
   
   NON-NEGOTIABLES:
   - This is an OPERATING SYSTEM, not a website
   - Desktop never scrolls
   - All content lives in windows
   - Motion is informative, not decorative
   - Copy is blunt, technical, unavoidable
   ═══════════════════════════════════════════════════════════════════════════════ */

/* ═══════════════════════════════════════════════════════════════════════════════
   SYSTEM CONSTANTS & COPY (AUTHORITATIVE)
   ═══════════════════════════════════════════════════════════════════════════════ */

const SYSTEM_VERSION = 'v1.0.3';
const NETWORK_STATE = 'TESTNET';

const BOOT_SEQUENCE = [
  'x3Star OS ' + SYSTEM_VERSION,
  'Initializing execution kernel…',
  'Loading VM abstraction layer…',
  'Mounting atomic state machine…',
  'Synchronizing cross-chain consensus…',
  'Calibrating MEV protection filters…',
  'System ready.',
];

const TERMINAL_COMMANDS: Record<string, string> = {
  status: `┌─────────────────────────────────────────┐
│ SYSTEM STATUS                           │
├─────────────────────────────────────────┤
│ NETWORK:     ${NETWORK_STATE.padEnd(26)}│
│ EXECUTION:   STABLE                     │
│ CONSENSUS:   FINALIZING                 │
│ LATENCY:     <50ms                      │
│ MEV SHIELD:  ACTIVE                     │
│ ATOMIC:      ENABLED                    │
└─────────────────────────────────────────┘`,

  vms: `┌─────────────────────────────────────────┐
│ AVAILABLE VIRTUAL MACHINES              │
├─────────────────────────────────────────┤
│ [EVM]  Ethereum Virtual Machine         │
│        Deterministic. Ubiquitous.       │
│        Compatible with all ERC standards│
├─────────────────────────────────────────┤
│ [SVM]  Solana Virtual Machine           │
│        Parallel. High-throughput.       │
│        Native Rust program execution    │
├─────────────────────────────────────────┤
│ [x3VM] Native Execution Engine          │
│        AI-aware. Constraint-native.     │
│        Intent-level abstraction         │
├─────────────────────────────────────────┤
│ [BTC]  Bitcoin Atomic Layer             │
│        Non-custodial. Trustless.        │
│        No bridges. Direct settlement.   │
└─────────────────────────────────────────┘`,

  atomic: `┌─────────────────────────────────────────┐
│ ATOMIC EXECUTION PROTOCOL               │
├─────────────────────────────────────────┤
│ MODE:         ENFORCED                  │
│ FAILURE:      GLOBAL ROLLBACK           │
│ PARTIAL EXEC: DISALLOWED                │
├─────────────────────────────────────────┤
│ CONSTRAINT:                             │
│                                         │
│   Either everything happens…            │
│   Or nothing does.                      │
│                                         │
│ This is not a feature.                  │
│ This is physics.                        │
└─────────────────────────────────────────┘`,

  why: `┌─────────────────────────────────────────┐
│ WHY x3STAR                              │
├─────────────────────────────────────────┤
│                                         │
│ Other chains execute transactions.      │
│ This executes intent.                   │
│                                         │
│ Other systems coordinate.               │
│ This guarantees.                        │
│                                         │
│ Other protocols bridge.                 │
│ This settles.                           │
│                                         │
│ Determinism is not optional.            │
│ Atomicity is not negotiable.            │
│ Execution is not approximate.           │
│                                         │
└─────────────────────────────────────────┘`,

  help: `AVAILABLE COMMANDS:
  status    System health and network state
  vms       List available virtual machines
  atomic    Atomic execution protocol status
  why       Core philosophy
  blocks    Recent block activity
  clear     Clear terminal output
  help      Show this message`,

  blocks: `RECENT BLOCKS:
  #1847293  EVM+SVM  42 txs  0.8s  ✓ FINALIZED
  #1847292  EVM      31 txs  0.9s  ✓ FINALIZED
  #1847291  x3VM     18 txs  0.7s  ✓ FINALIZED
  #1847290  EVM+BTC  27 txs  1.2s  ✓ FINALIZED
  #1847289  SVM      56 txs  0.6s  ✓ FINALIZED`,

  clear: 'CLEAR',
};

const VM_CONFIG = {
  EVM: {
    name: 'Ethereum VM',
    color: '#627EEA',
    status: 'ACTIVE',
    stats: { tps: 1247, gas: '21 gwei', blocks: 1847293 },
    description: 'Battle-tested execution. Maximum compatibility.',
  },
  SVM: {
    name: 'Solana VM',
    color: '#00FFA3',
    status: 'ACTIVE',
    stats: { tps: 4892, gas: '0.00025 SOL', blocks: 892341 },
    description: 'Parallelized execution. Speed without chaos.',
  },
  x3VM: {
    name: 'Native x3',
    color: '#FF6B00',
    status: 'ACTIVE',
    stats: { tps: 8421, gas: '0.001 X3', blocks: 234891 },
    description: 'Native execution. AI-first. Constraint-aware.',
  },
  BTC: {
    name: 'Bitcoin Atomic',
    color: '#F7931A',
    status: 'SYNC',
    stats: { tps: 7, gas: '12 sat/vB', blocks: 824891 },
    description: 'Atomic settlement. No bridges. No trust.',
  },
};

type VMKey = keyof typeof VM_CONFIG;
type WindowId = 'terminal' | 'execution' | 'vms' | 'atomic' | 'ecosystem' | 'overview';

interface WindowState {
  id: WindowId;
  isOpen: boolean;
  isFocused: boolean;
  position: { x: number; y: number };
  size: { width: number; height: number };
  zIndex: number;
}

interface BlockData {
  number: number;
  hash: string;
  vm: VMKey;
  txCount: number;
  time: number;
  status: 'pending' | 'confirmed' | 'finalized' | 'failed';
  gasUsed: string;
}

interface ProcessData {
  id: string;
  name: string;
  category: string;
  cpu: number;
  memory: string;
  status: 'running' | 'idle' | 'blocked';
}

/* ═══════════════════════════════════════════════════════════════════════════════
   UTILITY HOOKS
   ═══════════════════════════════════════════════════════════════════════════════ */

function useSystemClock() {
  const [time, setTime] = useState(new Date());
  useEffect(() => {
    const interval = setInterval(() => setTime(new Date()), 1000);
    return () => clearInterval(interval);
  }, []);
  return time.toUTCString().slice(17, 25);
}

function useBlockHeight() {
  const [height, setHeight] = useState(1847293);
  useEffect(() => {
    const interval = setInterval(() => {
      setHeight(h => h + Math.floor(Math.random() * 2));
    }, 6000);
    return () => clearInterval(interval);
  }, []);
  return height;
}

function useKeyboardShortcuts(handlers: Record<string, () => void>) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const key = `${e.ctrlKey || e.metaKey ? 'cmd+' : ''}${e.key.toLowerCase()}`;
      if (handlers[key]) {
        e.preventDefault();
        handlers[key]();
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [handlers]);
}

/* ═══════════════════════════════════════════════════════════════════════════════
   BOOT SEQUENCE COMPONENT
   ═══════════════════════════════════════════════════════════════════════════════ */

function BootSequence({ onComplete }: { onComplete: () => void }) {
  const [lineIndex, setLineIndex] = useState(0);
  const [charIndex, setCharIndex] = useState(0);
  const [displayedLines, setDisplayedLines] = useState<string[]>([]);

  useEffect(() => {
    if (lineIndex >= BOOT_SEQUENCE.length) {
      setTimeout(onComplete, 400);
      return;
    }

    const currentLine = BOOT_SEQUENCE[lineIndex];
    
    if (charIndex < currentLine.length) {
      const timer = setTimeout(() => {
        setDisplayedLines(prev => {
          const newLines = [...prev];
          newLines[lineIndex] = currentLine.slice(0, charIndex + 1);
          return newLines;
        });
        setCharIndex(c => c + 1);
      }, 15 + Math.random() * 10);
      return () => clearTimeout(timer);
    } else {
      const timer = setTimeout(() => {
        setLineIndex(l => l + 1);
        setCharIndex(0);
      }, 200);
      return () => clearTimeout(timer);
    }
  }, [lineIndex, charIndex, onComplete]);

  return (
    <motion.div
      className="fixed inset-0 bg-black z-[200] flex items-start justify-start p-8"
      exit={{ opacity: 0 }}
      transition={{ duration: 0.15 }}
    >
      <div className="font-mono text-[#00FF00] text-sm leading-relaxed">
        {displayedLines.map((line, i) => (
          <div key={i} className="whitespace-pre">
            {line}
            {i === lineIndex && lineIndex < BOOT_SEQUENCE.length && (
              <span className="animate-pulse">▮</span>
            )}
          </div>
        ))}
        {lineIndex >= BOOT_SEQUENCE.length && (
          <span className="animate-pulse">▮</span>
        )}
      </div>
    </motion.div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   SYSTEM BAR (PERSISTENT - Z=100)
   ═══════════════════════════════════════════════════════════════════════════════ */

function SystemBar({ 
  activeVM, 
  onLogoClick,
  blockHeight 
}: { 
  activeVM: VMKey;
  onLogoClick: () => void;
  blockHeight: number;
}) {
  const clock = useSystemClock();
  
  return (
    <div className="fixed top-0 left-0 right-0 h-12 bg-[#0a0a0a] border-b border-[#1a1a1a] z-[100] flex items-center px-4 font-mono text-xs select-none">
      {/* Left: Logo */}
      <button 
        onClick={onLogoClick}
        className="text-[#00FF00] font-bold tracking-wider hover:text-white transition-colors"
      >
        x3Star OS
      </button>
      
      {/* Center: Network Status */}
      <div className="flex-1 flex items-center justify-center gap-6">
        <div className="flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-[#00FF00] animate-pulse" />
          <span className="text-[#888]">{NETWORK_STATE}</span>
        </div>
        <div className="text-[#666]">│</div>
        <div className="text-[#888]">
          BLOCK <span className="text-white font-bold">#{blockHeight.toLocaleString()}</span>
        </div>
      </div>
      
      {/* Right: VM Indicators + Clock */}
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2">
          {(Object.keys(VM_CONFIG) as VMKey[]).map(vm => (
            <span
              key={vm}
              className={`px-2 py-0.5 text-[10px] rounded ${
                activeVM === vm 
                  ? 'bg-[#00FF00] text-black font-bold' 
                  : 'text-[#666] border border-[#333]'
              }`}
            >
              {vm}
            </span>
          ))}
        </div>
        <div className="text-[#666]">│</div>
        <div className="text-[#00FF00] tabular-nums">{clock} UTC</div>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   DOCK (LEFT NAVIGATION - Z=90)
   ═══════════════════════════════════════════════════════════════════════════════ */

const DOCK_ITEMS: { id: WindowId; label: string; icon: string }[] = [
  { id: 'terminal', label: 'Terminal', icon: '>_' },
  { id: 'execution', label: 'Execution', icon: '⚡' },
  { id: 'vms', label: 'VM Manager', icon: '◫' },
  { id: 'atomic', label: 'Atomic', icon: '⊕' },
  { id: 'ecosystem', label: 'Ecosystem', icon: '⊞' },
];

function Dock({ 
  activeWindow, 
  onSelect 
}: { 
  activeWindow: WindowId | null;
  onSelect: (id: WindowId) => void;
}) {
  const [hoveredItem, setHoveredItem] = useState<WindowId | null>(null);
  
  return (
    <div className="fixed left-0 top-12 bottom-0 w-16 bg-[#0a0a0a] border-r border-[#1a1a1a] z-[90] flex flex-col items-center py-4 gap-2">
      {DOCK_ITEMS.map(item => (
        <div key={item.id} className="relative">
          <button
            onClick={() => onSelect(item.id)}
            onMouseEnter={() => setHoveredItem(item.id)}
            onMouseLeave={() => setHoveredItem(null)}
            className={`w-12 h-12 flex items-center justify-center text-lg font-mono rounded transition-all ${
              activeWindow === item.id
                ? 'bg-[#00FF00] text-black'
                : 'text-[#666] hover:text-[#00FF00] hover:bg-[#1a1a1a]'
            }`}
          >
            {item.icon}
          </button>
          {activeWindow === item.id && (
            <div className="absolute left-0 top-1/2 -translate-y-1/2 w-0.5 h-6 bg-[#00FF00]" />
          )}
          <AnimatePresence>
            {hoveredItem === item.id && (
              <motion.div
                initial={{ opacity: 0, x: -4 }}
                animate={{ opacity: 1, x: 0 }}
                exit={{ opacity: 0, x: -4 }}
                transition={{ duration: 0.1 }}
                className="absolute left-14 top-1/2 -translate-y-1/2 px-2 py-1 bg-[#1a1a1a] text-[#00FF00] text-xs font-mono whitespace-nowrap border border-[#333] z-[100]"
              >
                {item.label}
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      ))}
      
      {/* Keyboard hint */}
      <div className="mt-auto text-[8px] text-[#333] text-center leading-tight px-2">
        ⌘T Terminal<br/>
        ⌘E Exec<br/>
        ESC Close
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   WINDOW COMPONENT (UNIVERSAL)
   ═══════════════════════════════════════════════════════════════════════════════ */

interface WindowProps {
  id: WindowId;
  title: string;
  isOpen: boolean;
  isFocused: boolean;
  position: { x: number; y: number };
  size: { width: number; height: number };
  zIndex: number;
  onFocus: () => void;
  onClose: () => void;
  children: React.ReactNode;
  statusBar?: React.ReactNode;
}

function Window({ 
  title, 
  isOpen, 
  isFocused, 
  position, 
  size, 
  zIndex, 
  onFocus, 
  onClose, 
  children,
  statusBar 
}: WindowProps) {
  if (!isOpen) return null;
  
  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.95, y: 10 }}
      animate={{ opacity: 1, scale: 1, y: 0 }}
      exit={{ opacity: 0, scale: 0.95, y: 10 }}
      transition={{ duration: 0.15, ease: 'linear' }}
      className={`absolute ${isFocused ? '' : 'blur-[1px] opacity-80'}`}
      style={{
        left: position.x,
        top: position.y,
        width: size.width,
        height: size.height,
        zIndex,
      }}
      onClick={onFocus}
    >
      <div className={`w-full h-full bg-[#0d0d0d] border ${isFocused ? 'border-[#00FF00]' : 'border-[#222]'} flex flex-col`}>
        {/* Title Bar */}
        <div className={`h-8 flex items-center justify-between px-3 ${isFocused ? 'bg-[#111]' : 'bg-[#0a0a0a]'} border-b border-[#1a1a1a] select-none`}>
          <span className={`font-mono text-xs ${isFocused ? 'text-[#00FF00]' : 'text-[#444]'}`}>
            {title}
          </span>
          <button
            onClick={(e) => { e.stopPropagation(); onClose(); }}
            className="w-4 h-4 flex items-center justify-center text-[#666] hover:text-[#ff5555] text-xs"
          >
            ×
          </button>
        </div>
        
        {/* Content */}
        <div className="flex-1 overflow-hidden">
          {children}
        </div>
        
        {/* Status Bar */}
        {statusBar && (
          <div className="h-6 bg-[#0a0a0a] border-t border-[#1a1a1a] px-3 flex items-center">
            {statusBar}
          </div>
        )}
      </div>
    </motion.div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   TERMINAL WINDOW
   ═══════════════════════════════════════════════════════════════════════════════ */

function TerminalApp({ isFocused }: { isFocused: boolean }) {
  const [history, setHistory] = useState<{ command: string; output: string }[]>([]);
  const [input, setInput] = useState('');
  const scrollRef = useRef<HTMLDivElement>(null);
  
  const executeCommand = useCallback((cmd: string) => {
    const trimmed = cmd.trim().toLowerCase();
    if (!trimmed) return;
    
    if (trimmed === 'clear') {
      setHistory([]);
      return;
    }
    
    const output = TERMINAL_COMMANDS[trimmed] || `ERROR: Unknown command '${trimmed}'. Type 'help' for available commands.`;
    setHistory(prev => [...prev, { command: cmd, output }]);
  }, []);
  
  useEffect(() => {
    scrollRef.current?.scrollTo(0, scrollRef.current.scrollHeight);
  }, [history]);
  
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      executeCommand(input);
      setInput('');
    }
  };
  
  return (
    <div className="h-full flex flex-col bg-black font-mono text-sm">
      {/* Output Area */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto p-4 space-y-4">
        <div className="text-[#00FF00]">
          x3Star Terminal {SYSTEM_VERSION}<br/>
          Type &apos;help&apos; for available commands.
        </div>
        {history.map((entry, i) => (
          <div key={i}>
            <div className="text-[#00FF00]">
              <span className="text-[#666]">x3@{NETWORK_STATE.toLowerCase()}</span>:~$ {entry.command}
            </div>
            <div className="text-[#888] whitespace-pre mt-1">{entry.output}</div>
          </div>
        ))}
      </div>
      
      {/* Input Area */}
      <div className="border-t border-[#1a1a1a] p-2 flex items-center gap-2">
        <span className="text-[#00FF00]">
          <span className="text-[#666]">x3@{NETWORK_STATE.toLowerCase()}</span>:~$
        </span>
        <input
          type="text"
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          className="flex-1 bg-transparent text-[#00FF00] outline-none"
          autoFocus={isFocused}
        />
        <span className="animate-pulse text-[#00FF00]">▮</span>
      </div>
      
      {/* Quick Commands */}
      <div className="border-t border-[#1a1a1a] p-2 flex gap-2">
        {['status', 'vms', 'atomic', 'why', 'blocks'].map(cmd => (
          <button
            key={cmd}
            onClick={() => executeCommand(cmd)}
            className="px-2 py-1 text-[10px] text-[#666] border border-[#333] hover:text-[#00FF00] hover:border-[#00FF00] transition-colors"
          >
            {cmd}
          </button>
        ))}
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   EXECUTION ENGINE (BLOOMBERG-STYLE)
   ═══════════════════════════════════════════════════════════════════════════════ */

function ExecutionEngineApp({ blocks }: { blocks: BlockData[] }) {
  const [selectedBlock, setSelectedBlock] = useState<BlockData | null>(null);
  
  const getStatusColor = (status: BlockData['status']) => {
    switch (status) {
      case 'finalized': return 'text-[#00FF00]';
      case 'confirmed': return 'text-[#00AAFF]';
      case 'pending': return 'text-[#FFAA00]';
      case 'failed': return 'text-[#FF5555]';
    }
  };
  
  const getVMColor = (vm: VMKey) => VM_CONFIG[vm].color;
  
  return (
    <div className="h-full flex flex-col bg-[#0a0a0a] font-mono text-xs">
      <div className="p-3">
        <ChainStatusContainer />
      </div>
      {/* Header Stats */}
      <div className="grid grid-cols-5 border-b border-[#1a1a1a]">
        {[
          { label: 'THROUGHPUT', value: '14,562 TPS', change: '+2.3%' },
          { label: 'PENDING', value: '847', change: '-12' },
          { label: 'FINALITY', value: '1.2s', change: '±0.1s' },
          { label: 'GAS AVG', value: '21 gwei', change: '+5%' },
          { label: 'MEV BLOCKED', value: '99.7%', change: '' },
        ].map((stat, i) => (
          <div key={i} className="p-3 border-r border-[#1a1a1a] last:border-r-0">
            <div className="text-[#666] text-[10px]">{stat.label}</div>
            <div className="text-white font-bold text-sm">{stat.value}</div>
            {stat.change && (
              <div className={stat.change.startsWith('+') ? 'text-[#00FF00]' : stat.change.startsWith('-') ? 'text-[#FF5555]' : 'text-[#666]'}>
                {stat.change}
              </div>
            )}
          </div>
        ))}
      </div>
      
      {/* Block Table */}
      <div className="flex-1 overflow-hidden flex">
        {/* Main Table */}
        <div className="flex-1 overflow-y-auto">
          <table className="w-full">
            <thead className="sticky top-0 bg-[#111] border-b border-[#1a1a1a]">
              <tr className="text-[#666] text-left">
                <th className="p-2 font-normal">BLOCK</th>
                <th className="p-2 font-normal">VM</th>
                <th className="p-2 font-normal">TXS</th>
                <th className="p-2 font-normal">GAS</th>
                <th className="p-2 font-normal">TIME</th>
                <th className="p-2 font-normal">STATUS</th>
              </tr>
            </thead>
            <tbody>
              {blocks.map((block, i) => (
                <tr
                  key={block.number}
                  onClick={() => setSelectedBlock(block)}
                  className={`border-b border-[#1a1a1a] cursor-pointer transition-colors ${
                    selectedBlock?.number === block.number ? 'bg-[#1a1a1a]' : 'hover:bg-[#111]'
                  }`}
                >
                  <td className="p-2 text-white">#{block.number.toLocaleString()}</td>
                  <td className="p-2">
                    <span 
                      className="px-1.5 py-0.5 rounded text-[10px] font-bold"
                      style={{ backgroundColor: getVMColor(block.vm) + '33', color: getVMColor(block.vm) }}
                    >
                      {block.vm}
                    </span>
                  </td>
                  <td className="p-2 text-[#888]">{block.txCount}</td>
                  <td className="p-2 text-[#888]">{block.gasUsed}</td>
                  <td className="p-2 text-[#888]">{block.time}s</td>
                  <td className={`p-2 ${getStatusColor(block.status)}`}>
                    {block.status === 'finalized' && '✓ '}
                    {block.status === 'failed' && '✗ '}
                    {block.status.toUpperCase()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
        
        {/* Block Detail Panel */}
        <div className="w-64 border-l border-[#1a1a1a] p-3 bg-[#0d0d0d]">
          <div className="text-[#666] text-[10px] mb-2">BLOCK DETAIL</div>
          {selectedBlock ? (
            <div className="space-y-3">
              <div>
                <div className="text-[#666] text-[10px]">NUMBER</div>
                <div className="text-white">#{selectedBlock.number.toLocaleString()}</div>
              </div>
              <div>
                <div className="text-[#666] text-[10px]">HASH</div>
                <div className="text-[#888] text-[10px] break-all">{selectedBlock.hash}</div>
              </div>
              <div>
                <div className="text-[#666] text-[10px]">VIRTUAL MACHINE</div>
                <div style={{ color: getVMColor(selectedBlock.vm) }}>{VM_CONFIG[selectedBlock.vm].name}</div>
              </div>
              <div>
                <div className="text-[#666] text-[10px]">TRANSACTIONS</div>
                <div className="text-white">{selectedBlock.txCount}</div>
              </div>
              <div>
                <div className="text-[#666] text-[10px]">STATUS</div>
                <div className={getStatusColor(selectedBlock.status)}>{selectedBlock.status.toUpperCase()}</div>
              </div>
            </div>
          ) : (
            <div className="text-[#444]">Select a block to view details</div>
          )}
        </div>
      </div>
      
      {/* Execution Timeline */}
      <div className="h-20 border-t border-[#1a1a1a] p-2">
        <div className="text-[#666] text-[10px] mb-1">EXECUTION TIMELINE</div>
        <div className="flex gap-1 h-12 items-end">
          {blocks.slice(0, 40).map((block, i) => (
            <div
              key={i}
              className="flex-1 rounded-sm transition-all hover:opacity-80"
              style={{
                height: `${20 + block.txCount}%`,
                backgroundColor: getVMColor(block.vm),
                opacity: block.status === 'failed' ? 0.3 : 0.7,
              }}
              title={`#${block.number} - ${block.txCount} txs`}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   VM MANAGER
   ═══════════════════════════════════════════════════════════════════════════════ */

function VMManagerApp({ 
  activeVM, 
  onVMChange 
}: { 
  activeVM: VMKey;
  onVMChange: (vm: VMKey) => void;
}) {
  return (
    <div className="h-full bg-[#0a0a0a] font-mono text-xs overflow-y-auto">
      <div className="grid grid-cols-2 gap-px bg-[#1a1a1a]">
        {(Object.keys(VM_CONFIG) as VMKey[]).map(vmKey => {
          const vm = VM_CONFIG[vmKey];
          const isActive = activeVM === vmKey;
          
          return (
            <button
              key={vmKey}
              onClick={() => onVMChange(vmKey)}
              className={`p-4 text-left transition-colors ${
                isActive ? 'bg-[#111]' : 'bg-[#0d0d0d] hover:bg-[#111]'
              }`}
            >
              <div className="flex items-center justify-between mb-3">
                <div className="flex items-center gap-2">
                  <span 
                    className="w-3 h-3 rounded-full"
                    style={{ backgroundColor: vm.color }}
                  />
                  <span className={isActive ? 'text-white font-bold' : 'text-[#888]'}>
                    {vmKey}
                  </span>
                </div>
                <span className={`text-[10px] px-1.5 py-0.5 rounded ${
                  vm.status === 'ACTIVE' ? 'bg-[#00FF00]/20 text-[#00FF00]' : 'bg-[#FFAA00]/20 text-[#FFAA00]'
                }`}>
                  {vm.status}
                </span>
              </div>
              
              <div className="text-[#666] text-[10px] mb-2">{vm.name}</div>
              <div className="text-[#888] text-[10px] mb-3">{vm.description}</div>
              
              <div className="grid grid-cols-3 gap-2 text-[10px]">
                <div>
                  <div className="text-[#666]">TPS</div>
                  <div className="text-white">{vm.stats.tps.toLocaleString()}</div>
                </div>
                <div>
                  <div className="text-[#666]">GAS</div>
                  <div className="text-white">{vm.stats.gas}</div>
                </div>
                <div>
                  <div className="text-[#666]">HEIGHT</div>
                  <div className="text-white">{vm.stats.blocks.toLocaleString()}</div>
                </div>
              </div>
              
              {isActive && (
                <div className="mt-3 pt-3 border-t border-[#1a1a1a]">
                  <div style={{ color: vm.color }} className="text-[10px]">
                    ▶ CURRENTLY SELECTED
                  </div>
                </div>
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   ATOMIC LAYER
   ═══════════════════════════════════════════════════════════════════════════════ */

function AtomicLayerApp() {
  const [atomicState, setAtomicState] = useState<'idle' | 'executing' | 'success' | 'failed'>('idle');
  const [progress, setProgress] = useState(0);
  
  const simulateExecution = () => {
    setAtomicState('executing');
    setProgress(0);
    
    const interval = setInterval(() => {
      setProgress(p => {
        if (p >= 100) {
          clearInterval(interval);
          const success = Math.random() > 0.2;
          setAtomicState(success ? 'success' : 'failed');
          setTimeout(() => setAtomicState('idle'), 3000);
          return 100;
        }
        return p + Math.random() * 15;
      });
    }, 200);
  };
  
  const lanes = [
    { vm: 'EVM', color: '#627EEA', progress: atomicState === 'executing' ? Math.min(progress * 1.2, 100) : atomicState === 'success' ? 100 : 0 },
    { vm: 'SVM', color: '#00FFA3', progress: atomicState === 'executing' ? Math.min(progress * 0.9, 100) : atomicState === 'success' ? 100 : 0 },
    { vm: 'BTC', color: '#F7931A', progress: atomicState === 'executing' ? Math.min(progress * 0.7, 100) : atomicState === 'success' ? 100 : 0 },
  ];
  
  return (
    <div className="h-full bg-[#0a0a0a] font-mono text-xs p-4 flex flex-col">
      {/* Header */}
      <div className="text-center mb-6">
        <div className="text-[#666] text-[10px] mb-1">ATOMIC EXECUTION PROTOCOL</div>
        <div className="text-white text-lg">
          {atomicState === 'idle' && 'READY'}
          {atomicState === 'executing' && 'EXECUTING...'}
          {atomicState === 'success' && '✓ COMMITTED'}
          {atomicState === 'failed' && '✗ ROLLED BACK'}
        </div>
      </div>
      
      {/* Timeline Lanes */}
      <div className="flex-1 space-y-4">
        {lanes.map((lane, i) => (
          <div key={lane.vm}>
            <div className="flex items-center justify-between mb-1">
              <span style={{ color: lane.color }}>{lane.vm}</span>
              <span className="text-[#666]">{Math.round(lane.progress)}%</span>
            </div>
            <div className="h-8 bg-[#111] rounded overflow-hidden relative">
              <motion.div
                className="h-full rounded"
                style={{ backgroundColor: lane.color }}
                initial={{ width: 0 }}
                animate={{ 
                  width: `${lane.progress}%`,
                  opacity: atomicState === 'failed' ? 0.3 : 1,
                }}
                transition={{ duration: 0.2 }}
              />
              {atomicState === 'failed' && lane.progress > 0 && (
                <motion.div
                  className="absolute inset-0 bg-[#FF5555]/50"
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                />
              )}
            </div>
          </div>
        ))}
      </div>
      
      {/* Constraint */}
      <div className="text-center py-4 border-t border-[#1a1a1a] mt-4">
        <div className="text-[#666] text-[10px] mb-2">ATOMIC CONSTRAINT</div>
        <div className="text-[#888]">
          Either everything happens…<br/>
          <span className="text-white">Or nothing does.</span>
        </div>
      </div>
      
      {/* Action */}
      <button
        onClick={simulateExecution}
        disabled={atomicState === 'executing'}
        className={`w-full py-3 rounded font-bold transition-colors ${
          atomicState === 'executing'
            ? 'bg-[#333] text-[#666] cursor-not-allowed'
            : 'bg-[#00FF00] text-black hover:bg-[#00DD00]'
        }`}
      >
        {atomicState === 'executing' ? 'EXECUTING...' : 'SIMULATE ATOMIC TX'}
      </button>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   ECOSYSTEM / PROCESS MONITOR
   ═══════════════════════════════════════════════════════════════════════════════ */

function EcosystemApp({ processes }: { processes: ProcessData[] }) {
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  
  const categories = ['All', 'Arbitrage', 'Quant', 'AI Agents', 'Infrastructure'];
  
  const filteredProcesses = selectedCategory && selectedCategory !== 'All'
    ? processes.filter(p => p.category === selectedCategory)
    : processes;
  
  const getStatusColor = (status: ProcessData['status']) => {
    switch (status) {
      case 'running': return 'text-[#00FF00]';
      case 'idle': return 'text-[#FFAA00]';
      case 'blocked': return 'text-[#FF5555]';
    }
  };
  
  return (
    <div className="h-full bg-[#0a0a0a] font-mono text-xs flex flex-col">
      {/* Category Tabs */}
      <div className="flex border-b border-[#1a1a1a]">
        {categories.map(cat => (
          <button
            key={cat}
            onClick={() => setSelectedCategory(cat)}
            className={`px-4 py-2 transition-colors ${
              (selectedCategory || 'All') === cat
                ? 'text-[#00FF00] border-b-2 border-[#00FF00]'
                : 'text-[#666] hover:text-[#888]'
            }`}
          >
            {cat}
          </button>
        ))}
      </div>
      
      {/* Process List */}
      <div className="flex-1 overflow-y-auto">
        <table className="w-full">
          <thead className="sticky top-0 bg-[#111] border-b border-[#1a1a1a]">
            <tr className="text-[#666] text-left">
              <th className="p-2 font-normal">PROCESS</th>
              <th className="p-2 font-normal">CATEGORY</th>
              <th className="p-2 font-normal">CPU</th>
              <th className="p-2 font-normal">MEM</th>
              <th className="p-2 font-normal">STATUS</th>
            </tr>
          </thead>
          <tbody>
            {filteredProcesses.map((process) => (
              <tr key={process.id} className="border-b border-[#1a1a1a] hover:bg-[#111]">
                <td className="p-2 text-white">{process.name}</td>
                <td className="p-2 text-[#888]">{process.category}</td>
                <td className="p-2">
                  <div className="flex items-center gap-2">
                    <div className="w-16 h-1 bg-[#1a1a1a] rounded overflow-hidden">
                      <div 
                        className="h-full bg-[#00FF00]" 
                        style={{ width: `${process.cpu}%` }}
                      />
                    </div>
                    <span className="text-[#888]">{process.cpu}%</span>
                  </div>
                </td>
                <td className="p-2 text-[#888]">{process.memory}</td>
                <td className={`p-2 ${getStatusColor(process.status)}`}>
                  {process.status.toUpperCase()}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      
      {/* Footer */}
      <div className="border-t border-[#1a1a1a] p-2 flex justify-between text-[#666]">
        <span>{filteredProcesses.length} processes</span>
        <span>These are not apps. These are workloads.</span>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   SYSTEM OVERVIEW WINDOW
   ═══════════════════════════════════════════════════════════════════════════════ */

function SystemOverviewApp() {
  return (
    <div className="h-full bg-black font-mono flex items-center justify-center p-8">
      <div className="max-w-md text-center space-y-8">
        <div className="text-[#00FF00] text-4xl font-bold tracking-wider">
          x3Star OS
        </div>
        
        <div className="text-[#888] text-sm leading-relaxed">
          A unified execution layer for deterministic,
          atomic, cross-chain computation.
        </div>
        
        <div className="space-y-4 text-sm">
          <div className="text-white">
            Other chains execute transactions.
          </div>
          <div className="text-[#00FF00]">
            This executes intent.
          </div>
        </div>
        
        <div className="pt-8 border-t border-[#1a1a1a] text-[#666] text-xs">
          EVM • SVM • x3VM • BTC Atomic
        </div>
        
        <div className="text-[10px] text-[#333]">
          Determinism is not optional.
        </div>
      </div>
    </div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   FAILURE MODE OVERLAY
   ═══════════════════════════════════════════════════════════════════════════════ */

function FailureModeOverlay({ 
  type, 
  onDismiss 
}: { 
  type: 'reorg' | 'halt' | 'abort' | null;
  onDismiss: () => void;
}) {
  if (!type) return null;
  
  const messages = {
    reorg: {
      title: 'CHAIN REORGANIZATION',
      subtitle: 'Block reverted. State rolling back.',
      color: '#FFAA00',
    },
    halt: {
      title: 'EXECUTION HALTED',
      subtitle: 'Consensus failure detected.',
      color: '#FF5555',
    },
    abort: {
      title: 'ATOMIC ABORT',
      subtitle: 'Transaction group collapsed. No state changed.',
      color: '#FF5555',
    },
  };
  
  const msg = messages[type];
  
  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 bg-black/90 z-[150] flex items-center justify-center"
      onClick={onDismiss}
    >
      <motion.div
        initial={{ scale: 0.9 }}
        animate={{ scale: 1 }}
        className="text-center"
      >
        <div 
          className="text-4xl font-mono font-bold mb-4 animate-pulse"
          style={{ color: msg.color }}
        >
          ⚠ {msg.title}
        </div>
        <div className="text-[#888] font-mono">
          {msg.subtitle}
        </div>
        <div className="mt-8 text-[#666] text-xs font-mono">
          Click anywhere to dismiss
        </div>
      </motion.div>
    </motion.div>
  );
}

/* ═══════════════════════════════════════════════════════════════════════════════
   MAIN OS SHELL
   ═══════════════════════════════════════════════════════════════════════════════ */

export default function X3StarOS() {
  const [booted, setBooted] = useState(false);
  const [activeVM, setActiveVM] = useState<VMKey>('EVM');
  const [failureMode, setFailureMode] = useState<'reorg' | 'halt' | 'abort' | null>(null);
  const blockHeight = useBlockHeight();
  
  // Window states
  const [windows, setWindows] = useState<Record<WindowId, WindowState>>({
    terminal: { id: 'terminal', isOpen: true, isFocused: true, position: { x: 96, y: 72 }, size: { width: 600, height: 400 }, zIndex: 60 },
    execution: { id: 'execution', isOpen: false, isFocused: false, position: { x: 350, y: 90 }, size: { width: 800, height: 500 }, zIndex: 50 },
    vms: { id: 'vms', isOpen: false, isFocused: false, position: { x: 120, y: 100 }, size: { width: 500, height: 400 }, zIndex: 50 },
    atomic: { id: 'atomic', isOpen: false, isFocused: false, position: { x: 300, y: 120 }, size: { width: 400, height: 500 }, zIndex: 50 },
    ecosystem: { id: 'ecosystem', isOpen: false, isFocused: false, position: { x: 200, y: 80 }, size: { width: 650, height: 450 }, zIndex: 50 },
    overview: { id: 'overview', isOpen: false, isFocused: false, position: { x: 250, y: 100 }, size: { width: 500, height: 400 }, zIndex: 50 },
  });
  
  // Mock data
  const [blocks, setBlocks] = useState<BlockData[]>([]);
  const [processes] = useState<ProcessData[]>([
    { id: '1', name: 'arb-executor-01', category: 'Arbitrage', cpu: 78, memory: '2.4 GB', status: 'running' },
    { id: '2', name: 'cross-chain-relay', category: 'Infrastructure', cpu: 45, memory: '1.2 GB', status: 'running' },
    { id: '3', name: 'mev-shield-daemon', category: 'Infrastructure', cpu: 92, memory: '4.1 GB', status: 'running' },
    { id: '4', name: 'quant-strategy-alpha', category: 'Quant', cpu: 67, memory: '8.2 GB', status: 'running' },
    { id: '5', name: 'ai-intent-parser', category: 'AI Agents', cpu: 34, memory: '3.8 GB', status: 'idle' },
    { id: '6', name: 'liquidation-bot', category: 'Arbitrage', cpu: 12, memory: '512 MB', status: 'idle' },
    { id: '7', name: 'consensus-validator', category: 'Infrastructure', cpu: 88, memory: '1.8 GB', status: 'running' },
    { id: '8', name: 'ai-execution-agent', category: 'AI Agents', cpu: 56, memory: '6.4 GB', status: 'running' },
  ]);
  
  // Generate mock blocks
  useEffect(() => {
    const vms: VMKey[] = ['EVM', 'SVM', 'x3VM', 'BTC'];
    const initialBlocks: BlockData[] = Array.from({ length: 20 }, (_, i) => ({
      number: blockHeight - i,
      hash: '0x' + Math.random().toString(16).slice(2, 10) + '...' + Math.random().toString(16).slice(2, 6),
      vm: vms[Math.floor(Math.random() * vms.length)],
      txCount: Math.floor(Math.random() * 80) + 5,
      time: +(Math.random() * 1.5 + 0.5).toFixed(1),
      status: i < 2 ? 'confirmed' : 'finalized',
      gasUsed: (Math.random() * 20 + 5).toFixed(1) + ' gwei',
    }));
    setBlocks(initialBlocks);
    
    const interval = setInterval(() => {
      setBlocks(prev => {
        const newBlock: BlockData = {
          number: prev[0].number + 1,
          hash: '0x' + Math.random().toString(16).slice(2, 10) + '...' + Math.random().toString(16).slice(2, 6),
          vm: vms[Math.floor(Math.random() * vms.length)],
          txCount: Math.floor(Math.random() * 80) + 5,
          time: +(Math.random() * 1.5 + 0.5).toFixed(1),
          status: 'pending',
          gasUsed: (Math.random() * 20 + 5).toFixed(1) + ' gwei',
        };
        
        // Update previous blocks
        const updated = prev.map((b, i) => ({
          ...b,
          status: i === 0 ? 'confirmed' as const : i === 1 ? 'finalized' as const : b.status,
        }));
        
        // Occasionally simulate failure
        if (Math.random() < 0.02) {
          newBlock.status = 'failed';
          setFailureMode('reorg');
        }
        
        return [newBlock, ...updated.slice(0, 19)];
      });
    }, 6000);
    
    return () => clearInterval(interval);
  }, [blockHeight]);
  
  // Window management
  const focusWindow = useCallback((id: WindowId) => {
    setWindows(prev => {
      const maxZ = Math.max(...Object.values(prev).map(w => w.zIndex));
      return {
        ...prev,
        ...Object.fromEntries(
          Object.entries(prev).map(([key, w]) => [
            key,
            { ...w, isFocused: key === id, zIndex: key === id ? maxZ + 1 : w.zIndex }
          ])
        ),
      };
    });
  }, []);
  
  const toggleWindow = useCallback((id: WindowId) => {
    setWindows(prev => {
      const window = prev[id];
      if (window.isOpen) {
        return { ...prev, [id]: { ...window, isOpen: false, isFocused: false } };
      }
      const maxZ = Math.max(...Object.values(prev).map(w => w.zIndex));
      return {
        ...prev,
        ...Object.fromEntries(
          Object.entries(prev).map(([key, w]) => [
            key,
            { ...w, isFocused: key === id }
          ])
        ),
        [id]: { ...window, isOpen: true, isFocused: true, zIndex: maxZ + 1 },
      };
    });
  }, []);
  
  // Keyboard shortcuts
  useKeyboardShortcuts({
    'cmd+t': () => toggleWindow('terminal'),
    'cmd+e': () => toggleWindow('execution'),
    'cmd+v': () => toggleWindow('vms'),
    'escape': () => {
      const focused = Object.values(windows).find(w => w.isFocused && w.isOpen);
      if (focused) toggleWindow(focused.id);
    },
  });
  
  const activeWindow = Object.values(windows).find(w => w.isFocused && w.isOpen)?.id || null;
  
  return (
    <div className="w-screen h-screen bg-[#050505] overflow-hidden select-none">
      <AnimatePresence>
        {!booted && <BootSequence onComplete={() => setBooted(true)} />}
      </AnimatePresence>
      
      {booted && (
        <>
          <SystemBar 
            activeVM={activeVM} 
            onLogoClick={() => toggleWindow('overview')}
            blockHeight={blockHeight}
          />
          
          <Dock activeWindow={activeWindow} onSelect={toggleWindow} />
          
          {/* Desktop Grid (visual reference) */}
          <div 
            className="fixed inset-0 pointer-events-none opacity-[0.02] z-0"
            style={{
              backgroundImage: 'linear-gradient(#333 1px, transparent 1px), linear-gradient(90deg, #333 1px, transparent 1px)',
              backgroundSize: '80px 80px',
              marginLeft: 64,
              marginTop: 48,
            }}
          />
          
          {/* Windows */}
          <AnimatePresence>
            <Window
              id="terminal"
              title="Terminal — Execution Interface"
              isOpen={windows.terminal.isOpen}
              isFocused={windows.terminal.isFocused}
              position={windows.terminal.position}
              size={windows.terminal.size}
              zIndex={windows.terminal.zIndex}
              onFocus={() => focusWindow('terminal')}
              onClose={() => toggleWindow('terminal')}
              statusBar={<span className="text-[#666] text-[10px]">Ready | {NETWORK_STATE}</span>}
            >
              <TerminalApp isFocused={windows.terminal.isFocused} />
            </Window>
            
            <Window
              id="execution"
              title="Execution Engine — Block Monitor"
              isOpen={windows.execution.isOpen}
              isFocused={windows.execution.isFocused}
              position={windows.execution.position}
              size={windows.execution.size}
              zIndex={windows.execution.zIndex}
              onFocus={() => focusWindow('execution')}
              onClose={() => toggleWindow('execution')}
              statusBar={<span className="text-[#00FF00] text-[10px]">● LIVE | {blocks.length} blocks loaded</span>}
            >
              <ExecutionEngineApp blocks={blocks} />
            </Window>
            
            <Window
              id="vms"
              title="VM Manager"
              isOpen={windows.vms.isOpen}
              isFocused={windows.vms.isFocused}
              position={windows.vms.position}
              size={windows.vms.size}
              zIndex={windows.vms.zIndex}
              onFocus={() => focusWindow('vms')}
              onClose={() => toggleWindow('vms')}
              statusBar={<span className="text-[10px]" style={{ color: VM_CONFIG[activeVM].color }}>Active: {activeVM}</span>}
            >
              <VMManagerApp activeVM={activeVM} onVMChange={setActiveVM} />
            </Window>
            
            <Window
              id="atomic"
              title="Atomic Layer"
              isOpen={windows.atomic.isOpen}
              isFocused={windows.atomic.isFocused}
              position={windows.atomic.position}
              size={windows.atomic.size}
              zIndex={windows.atomic.zIndex}
              onFocus={() => focusWindow('atomic')}
              onClose={() => toggleWindow('atomic')}
            >
              <AtomicLayerApp />
            </Window>
            
            <Window
              id="ecosystem"
              title="Ecosystem — Process Monitor"
              isOpen={windows.ecosystem.isOpen}
              isFocused={windows.ecosystem.isFocused}
              position={windows.ecosystem.position}
              size={windows.ecosystem.size}
              zIndex={windows.ecosystem.zIndex}
              onFocus={() => focusWindow('ecosystem')}
              onClose={() => toggleWindow('ecosystem')}
              statusBar={<span className="text-[#666] text-[10px]">{processes.filter(p => p.status === 'running').length} running</span>}
            >
              <EcosystemApp processes={processes} />
            </Window>
            
            <Window
              id="overview"
              title="System Overview"
              isOpen={windows.overview.isOpen}
              isFocused={windows.overview.isFocused}
              position={windows.overview.position}
              size={windows.overview.size}
              zIndex={windows.overview.zIndex}
              onFocus={() => focusWindow('overview')}
              onClose={() => toggleWindow('overview')}
            >
              <SystemOverviewApp />
            </Window>
          </AnimatePresence>
          
          {/* Failure Mode Overlay */}
          <AnimatePresence>
            <FailureModeOverlay type={failureMode} onDismiss={() => setFailureMode(null)} />
          </AnimatePresence>
        </>
      )}
    </div>
  );
}
