'use client';

import React, { useEffect, useMemo, useRef, useState } from 'react';
import { usePathname, useRouter } from 'next/navigation';
import { motion, AnimatePresence } from 'framer-motion';
import { Terminal, X, CornerDownLeft } from 'lucide-react';

type Line = {
  id: string;
  kind: 'input' | 'output' | 'error' | 'system';
  text: string;
};

const STORAGE_KEY = 'atlas.neuralTerminal.v1';

function nowId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function clamp(n: number, min: number, max: number) {
  return Math.max(min, Math.min(max, n));
}

export default function FloatingNeuralTerminal() {
  const router = useRouter();
  const pathname = usePathname();

  const [open, setOpen] = useState(true);
  const [pos, setPos] = useState({ x: 18, y: 18 });
  const [dragging, setDragging] = useState(false);

  const [lines, setLines] = useState<Line[]>(() => [
    { id: nowId(), kind: 'system', text: '[NEURAL] Terminal online. Type `help`.' },
  ]);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return;
      const parsed = JSON.parse(raw) as { open?: boolean; x?: number; y?: number };
      if (typeof parsed.open === 'boolean') setOpen(parsed.open);
      if (typeof parsed.x === 'number' && typeof parsed.y === 'number') {
        setPos({ x: clamp(parsed.x, 8, 2000), y: clamp(parsed.y, 8, 2000) });
      }
    } catch {
      // ignore
    }
  }, []);

  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ open, x: pos.x, y: pos.y }));
    } catch {
      // ignore
    }
  }, [open, pos.x, pos.y]);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      // Ctrl/Cmd + K toggles
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        setOpen(v => !v);
        return;
      }
      // ` focuses input when open
      if (e.key === '`' && open) {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [open]);

  useEffect(() => {
    if (open) {
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  }, [open]);

  const helpText = useMemo(() => {
    return [
      'Commands:',
      '  help                Show this help',
      '  clear               Clear output',
      '  go <path>           Navigate (e.g. go /quantum)',
      '  open <path>         Open in new tab',
      '  ask <question>      Ask AI about this codebase',
      '',
      'Shortcuts:',
      '  Ctrl/Cmd+K          Toggle terminal',
      '  ` (backtick)        Focus input',
      '',
      'Safety:',
      '  This terminal cannot access your real OS terminal.',
    ].join('\n');
  }, []);

  function push(kind: Line['kind'], text: string) {
    setLines(prev => [...prev, { id: nowId(), kind, text }]);
  }

  async function handleAsk(question: string) {
    setBusy(true);
    try {
      const res = await fetch('/api/ai/ask', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ question, pagePath: pathname }),
      });
      const json = await res.json();
      if (!res.ok) {
        push('error', json?.error || 'AI request failed');
        return;
      }
      if (json?.answer) push('output', json.answer);
      if (Array.isArray(json?.sources) && json.sources.length) {
        const sources = json.sources
          .slice(0, 6)
          .map((s: any) => `${s.path}#L${s.startLine}-L${s.endLine}`)
          .join('\n');
        push('system', `Sources:\n${sources}`);
      }
    } catch (e) {
      push('error', e instanceof Error ? e.message : 'AI request failed');
    } finally {
      setBusy(false);
    }
  }

  async function runCommand(raw: string) {
    const cmd = raw.trim();
    if (!cmd) return;

    push('input', `> ${cmd}`);
    setInput('');

    const [head, ...rest] = cmd.split(' ');
    const arg = rest.join(' ').trim();
    const verb = head.toLowerCase();

    if (verb === 'clear') {
      setLines([{ id: nowId(), kind: 'system', text: '[NEURAL] Cleared.' }]);
      return;
    }

    if (verb === 'help') {
      push('output', helpText);
      return;
    }

    if (verb === 'go') {
      if (!arg.startsWith('/')) {
        push('error', 'Path must start with /. Example: go /quantum');
        return;
      }
      router.push(arg);
      push('system', `[NAV] ${arg}`);
      return;
    }

    if (verb === 'open') {
      if (!arg.startsWith('/')) {
        push('error', 'Path must start with /. Example: open /docs');
        return;
      }
      window.open(arg, '_blank', 'noopener,noreferrer');
      push('system', `[OPEN] ${arg}`);
      return;
    }

    if (verb === 'ask') {
      if (!arg) {
        push('error', 'Usage: ask <question>');
        return;
      }
      await handleAsk(arg);
      return;
    }

    // Convenience: allow entering a raw path to navigate
    if (cmd.startsWith('/')) {
      router.push(cmd);
      push('system', `[NAV] ${cmd}`);
      return;
    }

    push('error', `Unknown command: ${verb}. Type help.`);
  }

  // Drag behavior (simple, not fancy)
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    let startX = 0;
    let startY = 0;
    let startPos = { x: 0, y: 0 };

    const onPointerDown = (e: PointerEvent) => {
      const target = e.target as HTMLElement | null;
      if (target?.closest('input,button,a,textarea')) return;
      if (!target?.closest('[data-drag-handle]')) return;

      setDragging(true);
      startX = e.clientX;
      startY = e.clientY;
      startPos = { ...pos };
      (e.target as Element).setPointerCapture?.(e.pointerId);
    };

    const onPointerMove = (e: PointerEvent) => {
      if (!dragging) return;
      const dx = e.clientX - startX;
      const dy = e.clientY - startY;
      setPos({ x: startPos.x + dx, y: startPos.y + dy });
    };

    const onPointerUp = () => setDragging(false);

    el.addEventListener('pointerdown', onPointerDown);
    window.addEventListener('pointermove', onPointerMove);
    window.addEventListener('pointerup', onPointerUp);

    return () => {
      el.removeEventListener('pointerdown', onPointerDown);
      window.removeEventListener('pointermove', onPointerMove);
      window.removeEventListener('pointerup', onPointerUp);
    };
  }, [dragging, pos]);

  return (
    <div
      ref={containerRef}
      className="fixed z-[9999]"
      style={{ right: pos.x, bottom: pos.y }}
    >
      <AnimatePresence initial={false}>
        {open ? (
          <motion.div
            key="panel"
            initial={{ opacity: 0, y: 8, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 8, scale: 0.98 }}
            transition={{ duration: 0.15 }}
            className="w-[360px] max-w-[calc(100vw-24px)] rounded-2xl border border-slate-700/60 bg-slate-950/85 backdrop-blur-xl shadow-2xl"
          >
            <div
              data-drag-handle
              className="flex items-center justify-between gap-2 px-3 py-2 border-b border-slate-700/50 cursor-grab active:cursor-grabbing select-none"
            >
              <div className="flex items-center gap-2">
                <div className="h-8 w-8 rounded-xl bg-gradient-to-br from-cyan-500/30 to-fuchsia-500/20 border border-slate-700/60 flex items-center justify-center">
                  <Terminal className="h-4 w-4 text-cyan-200" />
                </div>
                <div className="leading-tight">
                  <div className="text-sm font-semibold text-slate-100">Neural Terminal</div>
                  <div className="text-[11px] text-slate-400">Navigator + codebase Q&A</div>
                </div>
              </div>
              <div className="flex items-center gap-1">
                <button
                  type="button"
                  className="h-8 w-8 rounded-xl hover:bg-slate-800/60 border border-transparent hover:border-slate-700/60 text-slate-300"
                  onClick={() => setOpen(false)}
                  aria-label="Close terminal"
                >
                  <X className="h-4 w-4 mx-auto" />
                </button>
              </div>
            </div>

            <div className="px-3 py-2">
              <div className="max-h-[220px] overflow-auto pr-1 text-[12px] leading-relaxed">
                {lines.slice(-120).map(l => (
                  <pre
                    key={l.id}
                    className={
                      l.kind === 'input'
                        ? 'text-slate-200'
                        : l.kind === 'error'
                          ? 'text-red-300'
                          : l.kind === 'system'
                            ? 'text-cyan-300/90'
                            : 'text-slate-100'
                    }
                    style={{ whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}
                  >
                    {l.text}
                  </pre>
                ))}
              </div>

              <div className="mt-2 flex items-center gap-2">
                <div className="flex-1">
                  <input
                    ref={inputRef}
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') runCommand(input);
                    }}
                    placeholder={busy ? 'Thinking…' : 'help | go /quantum | ask what is atlas kernel?'}
                    disabled={busy}
                    className="w-full h-10 rounded-xl bg-slate-900/70 border border-slate-700/60 px-3 text-sm text-slate-100 placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-cyan-500/30"
                  />
                </div>
                <button
                  type="button"
                  onClick={() => runCommand(input)}
                  disabled={busy}
                  className="h-10 w-10 rounded-xl bg-slate-900/70 border border-slate-700/60 hover:border-cyan-500/40 hover:bg-slate-900 text-slate-200 disabled:opacity-60"
                  aria-label="Run command"
                >
                  <CornerDownLeft className="h-4 w-4 mx-auto" />
                </button>
              </div>
            </div>
          </motion.div>
        ) : (
          <motion.button
            key="fab"
            initial={{ opacity: 0, y: 6, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1 }}
            exit={{ opacity: 0, y: 6, scale: 0.98 }}
            transition={{ duration: 0.15 }}
            onClick={() => setOpen(true)}
            className="h-12 w-12 rounded-2xl border border-slate-700/60 bg-slate-950/75 backdrop-blur-xl shadow-2xl hover:border-cyan-500/40 flex items-center justify-center"
            aria-label="Open Neural Terminal"
          >
            <Terminal className="h-5 w-5 text-cyan-200" />
          </motion.button>
        )}
      </AnimatePresence>
    </div>
  );
}
