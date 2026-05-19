import { useEffect, useRef, useState } from 'react';
import { executeCommand, formatTerminalOutput } from './terminalCommands';
import { lazy, Suspense } from 'react';

const TerminalChatBot = lazy(() => import('./TerminalChatBot'));

interface TerminalEntry {
  type: 'command' | 'output' | 'error';
  content: string;
  timestamp: Date;
}

interface TerminalProps {
  isOpen?: boolean;
  onClose?: () => void;
}

export function Terminal({ isOpen = true, onClose }: TerminalProps) {
  const [entries, setEntries] = useState<TerminalEntry[]>([
    {
      type: 'output',
      content: 'X3 Desktop Terminal v1.0.0\nType "help" for available commands',
      timestamp: new Date(),
    },
  ]);
  const [input, setInput] = useState('');
  const [isExecuting, setIsExecuting] = useState(false);
  const terminalEndRef = useRef<HTMLDivElement>(null);
  const historyRef = useRef<string[]>([]);
  const historyIndexRef = useRef<number>(-1);

  // Auto-scroll to bottom
  useEffect(() => {
    terminalEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [entries]);

  const handleExecuteCommand = async (cmd: string) => {
    if (!cmd.trim()) return;

    // Add to history
    historyRef.current.push(cmd);
    historyIndexRef.current = historyRef.current.length;

    // Add command to terminal
    setEntries((prev) => [...prev, { type: 'command', content: cmd, timestamp: new Date() }]);

    setIsExecuting(true);
    const { output, error } = await executeCommand(cmd);

    if (cmd === 'clear') {
      setEntries([]);
    } else {
      setEntries((prev) => [
        ...prev,
        {
          type: error ? 'error' : 'output',
          content: error || formatTerminalOutput(output),
          timestamp: new Date(),
        },
      ]);
    }

    setInput('');
    setIsExecuting(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleExecuteCommand(input);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      if (historyIndexRef.current > 0) {
        historyIndexRef.current--;
        setInput(historyRef.current[historyIndexRef.current]);
      }
    } else if (e.key === 'ArrowDown') {
      e.preventDefault();
      if (historyIndexRef.current < historyRef.current.length - 1) {
        historyIndexRef.current++;
        setInput(historyRef.current[historyIndexRef.current]);
      } else {
        historyIndexRef.current = historyRef.current.length;
        setInput('');
      }
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed bottom-32 right-4 w-[480px] max-w-[95vw] h-[300px] max-h-[60vh]
      glass-panel border border-[#1a9fb5]/30 shadow-2xl z-40 flex flex-col rounded-lg"
      style={{
        boxShadow: '0 0 30px rgba(26, 159, 181, 0.5), 0 0 15px rgba(26, 159, 181, 0.3), inset 0 0 15px rgba(26, 159, 181, 0.1)'
      }}>
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-2 border-b border-[#1a9fb5]/20 bg-[#0a0e27]/80">
        <h3 className="font-mono font-bold text-[#2ab4cc] text-xs tracking-wide">X3 TERMINAL</h3>
        <button
          onClick={onClose}
          className="text-[#2ab4cc] hover:text-[#1a9fb5] transition text-xs font-mono px-2 py-1 rounded border border-[#1a9fb5]/30 hover:border-[#1a9fb5]/60"
        >
          close
        </button>
      </div>

      {/* Terminal Output */}
      <div className="flex-1 overflow-y-auto px-4 py-2 space-y-2 font-mono text-xs scrollbar-thin scrollbar-thumb-[#1a9fb5]/30 scrollbar-track-transparent">
        {entries.map((entry, idx) => (
          <div
            key={idx}
            className={`flex flex-col gap-1 ${
              entry.type === 'command'
                ? 'text-[#2ab4cc]/80'
                : entry.type === 'error'
                  ? 'text-[#ff3366]'
                  : 'text-[#e0e0e0]/70'
            }`}
          >
            {entry.type === 'command' && <span className="text-[#2ab4cc]">{`>`} {entry.content}</span>}
            {entry.type !== 'command' && (
              <pre className="font-mono whitespace-pre-wrap break-words text-xs opacity-90">{entry.content}</pre>
            )}
          </div>
        ))}
        <div ref={terminalEndRef} />
      </div>

      {/* Input */}
      <div className="px-4 py-2 border-t border-[#1a9fb5]/20 bg-[#0a0e27]/80">
        <div className="flex items-center gap-2">
          <span className="text-[#2ab4cc] font-mono font-bold text-sm">{`>`}</span>
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            disabled={isExecuting}
            placeholder="Type command or 'help' for options..."
            className="flex-1 bg-transparent text-[#2ab4cc] font-mono text-xs outline-none placeholder:text-[#2ab4cc]/30 disabled:opacity-50"
            autoFocus
          />
          {isExecuting && <span className="text-[#1a9fb5] font-mono text-xs animate-pulse">executing...</span>}
        </div>
        {/* Chatbot below input */}
        <div className="mt-2">
          {/* Lazy load to avoid circular import issues */}
          <Suspense fallback={<div className="text-[#2ab4cc]/50 text-xs font-mono">Loading chatbot...</div>}>
            <TerminalChatBot />
          </Suspense>
        </div>
      </div>
    </div>
  );
}
