import { useEffect, useRef, useCallback } from 'react';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { useTerminalStore, useWorkspaceStore } from '../../store';

export default function XTerminal() {
  const terminals = useTerminalStore(s => s.terminals);
  const activeTerminalId = useTerminalStore(s => s.activeTerminalId);
  const addTerminal = useTerminalStore(s => s.addTerminal);
  const removeTerminal = useTerminalStore(s => s.removeTerminal);
  const setActive = useTerminalStore(s => s.setActive);
  const workspacePath = useWorkspaceStore(s => s.workspacePath);

  const termInstances = useRef<Map<string, { term: Terminal; fitAddon: FitAddon }>>(new Map());
  const containerRef = useRef<HTMLDivElement>(null);
  const activeIdRef = useRef<string | null>(null);
  const workspaceRef = useRef<string | null>(null);
  const initialMount = useRef(true);

  activeIdRef.current = activeTerminalId;
  workspaceRef.current = workspacePath;

  const createXterm = useCallback((container: HTMLElement): { term: Terminal; fitAddon: FitAddon } => {
    const fitAddon = new FitAddon();
    const term = new Terminal({
      cursorBlink: true,
      cursorStyle: 'block',
      fontSize: 13,
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', 'Source Code Pro', monospace",
      theme: { background: '#0d1117', foreground: '#c9d1d9', cursor: '#58a6ff', selectionBackground: '#264f78' },
      allowProposedApi: true,
    });
    term.loadAddon(fitAddon);
    term.open(container);
    setTimeout(() => fitAddon.fit(), 50);
    const ro = new ResizeObserver(() => { try { fitAddon.fit(); } catch {} });
    ro.observe(container);
    (term as any)._ro = ro;
    return { term, fitAddon };
  }, []);

  const spawnTerminal = useCallback(async (id: string, cwd: string) => {
    const inst = termInstances.current.get(id);
    if (!inst) return;
    const { term, fitAddon } = inst;
    term.clear();
    term.writeln(`\x1b[33mX3 Studio Terminal [${id}] — ${cwd}\x1b[0m`);
    await window.x3studio.terminal.create(id, cwd);
    window.x3studio.terminal.onData((tid, data) => {
      if (tid === activeIdRef.current) {
        const t = termInstances.current.get(tid);
        if (t) t.term.write(data);
      }
    });
    window.x3studio.terminal.onExit((tid, code) => {
      const t = termInstances.current.get(tid);
      if (t) t.term.writeln(`\r\n\x1b[33mProcess exited with code ${code}\x1b[0m`);
    });
    window.x3studio.terminal.onError((tid, msg) => {
      const t = termInstances.current.get(tid);
      if (t) t.term.writeln(`\r\n\x1b[31mTerminal error: ${msg}\x1b[0m`);
    });
    term.onData((data) => {
      window.x3studio.terminal.write(activeIdRef.current || id, data);
    });
    // Resize tracking
    term.onResize(({ cols, rows }) => {
      window.x3studio.terminal.resize(activeIdRef.current || id, cols, rows);
    });
    setTimeout(() => fitAddon.fit(), 100);
  }, []);

  // Initialize first terminal or create one
  useEffect(() => {
    if (!initialMount.current) return;
    initialMount.current = false;
    if (terminals.length === 0) {
      const id = 'term-0';
      addTerminal(id, 'bash');
      setActive(id);
    }
  }, []);

  // When workspace path changes, spawn terminal
  useEffect(() => {
    const id = activeTerminalId || 'term-0';
    if (!id || !containerRef.current) return;
    let inst = termInstances.current.get(id);
    if (!inst) {
      const termContainer = document.createElement('div');
      termContainer.style.width = '100%';
      termContainer.style.height = '100%';
      termContainer.id = `xterm-${id}`;
      containerRef.current.innerHTML = '';
      containerRef.current.appendChild(termContainer);
      inst = createXterm(termContainer);
      termInstances.current.set(id, inst);
      const cwd = workspacePath || process?.env?.HOME || '/tmp';
      spawnTerminal(id, cwd);
    } else {
      containerRef.current.innerHTML = '';
      const termContainer = document.createElement('div');
      termContainer.style.width = '100%';
      termContainer.style.height = '100%';
      termContainer.id = `xterm-${id}`;
      containerRef.current.appendChild(termContainer);
      inst.term.open(termContainer);
      setTimeout(() => inst.fitAddon.fit(), 50);
    }
    return () => {};
  }, [activeTerminalId]);

  const handleAddTerminal = () => {
    const id = `term-${Date.now()}`;
    addTerminal(id, 'bash');
    setActive(id);
    const cwd = workspacePath || process?.env?.HOME || '/tmp';
    const termContainer = document.createElement('div');
    termContainer.style.width = '100%';
    termContainer.style.height = '100%';
    termContainer.id = `xterm-${id}`;
    if (containerRef.current) {
      containerRef.current.innerHTML = '';
      containerRef.current.appendChild(termContainer);
    }
    const inst = createXterm(termContainer);
    termInstances.current.set(id, inst);
    spawnTerminal(id, cwd);
  };

  const handleKillTerminal = async (id: string) => {
    const inst = termInstances.current.get(id);
    if (inst) {
      (inst.term as any)._ro?.disconnect();
      inst.term.dispose();
      termInstances.current.delete(id);
    }
    await window.x3studio.terminal.kill(id);
    removeTerminal(id);
  };

  const activeId = activeTerminalId || 'term-0';

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ display: 'flex', gap: 0, background: '#161b22', borderBottom: '1px solid #30363d', minHeight: 28, alignItems: 'center' }}>
        {terminals.map(t => (
          <div key={t.id}
            onClick={() => setActive(t.id)}
            style={{
              padding: '4px 12px', cursor: 'pointer', fontSize: 11, color: t.id === activeId ? '#e6edf3' : '#8b949e',
              background: t.id === activeId ? '#0d1117' : 'transparent',
              borderBottom: t.id === activeId ? '2px solid #58a6ff' : '2px solid transparent',
              display: 'flex', alignItems: 'center', gap: 6, userSelect: 'none',
            }}>
            <span>{t.name}</span>
            <span onClick={(e) => { e.stopPropagation(); handleKillTerminal(t.id); }}
              style={{ fontSize: 12, opacity: 0.7, cursor: 'pointer', padding: '0 2px', borderRadius: 2 }}
              onMouseEnter={e => (e.target as HTMLElement).style.background = '#30363d'}
              onMouseLeave={e => (e.target as HTMLElement).style.background = 'transparent'}>×</span>
          </div>
        ))}
        <div onClick={handleAddTerminal}
          style={{ padding: '4px 8px', cursor: 'pointer', color: '#8b949e', fontSize: 16, lineHeight: 1 }}
          title="New Terminal">+</div>
      </div>
      <div ref={containerRef} style={{ flex: 1, overflow: 'hidden' }} />
    </div>
  );
}
