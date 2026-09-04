import { useState, useEffect } from 'react';
import { useDebuggerStore, useWorkspaceStore, useSettingsStore } from '../../store';

export default function DebuggerPanel() {
  const wp = useWorkspaceStore(s => s.workspacePath);
  const isAttached = useDebuggerStore(s => s.isAttached);
  const setAttached = useDebuggerStore(s => s.setAttached);
  const breakpoints = useDebuggerStore(s => s.breakpoints);
  const addBreakpoint = useDebuggerStore(s => s.addBreakpoint);
  const removeBreakpoint = useDebuggerStore(s => s.removeBreakpoint);
  const currentFile = useDebuggerStore(s => s.currentFile);
  const currentLine = useDebuggerStore(s => s.currentLine);
  const variables = useDebuggerStore(s => s.variables);
  const callStack = useDebuggerStore(s => s.callStack);
  const setVariables = useDebuggerStore(s => s.setVariables);
  const setCallStack = useDebuggerStore(s => s.setCallStack);
  const setLocation = useDebuggerStore(s => s.setLocation);
  const debuggerId = useDebuggerStore(s => s.debuggerId);
  const setDebuggerId = useDebuggerStore(s => s.setDebuggerId);
  const sessionOutput = useDebuggerStore(s => s.sessionOutput);
  const appendOutput = useDebuggerStore(s => s.appendOutput);
  const clearOutput = useDebuggerStore(s => s.clearOutput);

  const [target, setTarget] = useState('forge test --debug');
  const [bpFile, setBpFile] = useState('contracts/HTLC.sol');
  const [bpLine, setBpLine] = useState('45');

  const handleAttach = async () => {
    if (!wp || isAttached) return;
    clearOutput();
    appendOutput(`Starting debugger with: ${target}`);
    const result = await window.x3studio.debugger.start(target, wp);
    appendOutput(result.stdout + '\n' + result.stderr);
    setAttached(true);
    setDebuggerId(`dbg-${Date.now()}`);
    const vars = await window.x3studio.debugger.getVariables('1');
    setVariables(vars);
    setCallStack([{ file: 'contracts/HTLC.sol', line: 45, function: 'claim' }]);
    setLocation('contracts/HTLC.sol', 45);
  };

  const handleDetach = async () => {
    if (debuggerId) await window.x3studio.debugger.stop(debuggerId);
    setAttached(false); setVariables([]); setCallStack([]); setLocation(null, null); setDebuggerId(null);
    appendOutput('Debugger detached.');
  };

  const handleStep = async () => {
    if (!isAttached || !debuggerId) return;
    const result = await window.x3studio.debugger.step(debuggerId);
    if (result.file) setLocation(result.file, result.line);
    setVariables(result.variables);
    setCallStack(result.callStack);
    appendOutput(`Stepped to ${result.file}:${result.line}`);
  };

  const handleContinue = async () => {
    if (!isAttached || !debuggerId) return;
    const result = await window.x3studio.debugger.continue(debuggerId);
    setLocation(result.file, result.line);
    setVariables(result.variables);
    setCallStack(result.callStack);
    appendOutput('Continuing...');
  };

  const handleSetBp = async () => {
    const line = parseInt(bpLine);
    if (!bpFile || isNaN(line)) return;
    const id = `bp-${Date.now()}`;
    addBreakpoint({ id, file: bpFile, line, enabled: true });
    if (debuggerId) await window.x3studio.debugger.setBreakpoint(debuggerId, bpFile, line);
    appendOutput(`Breakpoint set: ${bpFile}:${line}`);
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Debugger</span>
        <span className={`badge badge-${isAttached ? 'pass' : 'info'}`}>{isAttached ? 'Attached' : 'Detached'}</span>
      </div>
      <div className="panel-body" style={{ padding: '8px', overflow: 'auto' }}>
        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)' }}>Launch Target</label>
          <select className="select-field" value={target} onChange={e => setTarget(e.target.value)}>
            <option value="forge test --debug">forge test --debug</option>
            <option value="forge script script/Deploy.s.sol">forge script</option>
            <option value="cargo test">cargo test</option>
            <option value="pnpm test">pnpm test</option>
            <option value="gdb">gdb (generic)</option>
            <option value="lldb">lldb (generic)</option>
            <option value="foundry test --debug">foundry test --debug</option>
          </select>
        </div>

        <div style={{ display: 'flex', gap: 4, marginBottom: 8, flexWrap: 'wrap' }}>
          {!isAttached ? (
            <button className="btn btn-primary" onClick={handleAttach} disabled={!wp}>▶ Attach</button>
          ) : (
            <>
              <button className="btn" onClick={handleStep}>⤵ Step</button>
              <button className="btn" onClick={handleContinue}>▶ Continue</button>
              <button className="btn btn-danger" onClick={handleDetach}>■ Detach</button>
            </>
          )}
        </div>

        <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Add Breakpoint</div>
        <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
          <input className="input-field" style={{ flex: 1, fontSize: 11 }} value={bpFile} onChange={e => setBpFile(e.target.value)} placeholder="File path" />
          <input className="input-field" style={{ width: 60, fontSize: 11 }} value={bpLine} onChange={e => setBpLine(e.target.value)} placeholder="Line" />
          <button className="btn" onClick={handleSetBp}>+ BP</button>
        </div>

        <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Breakpoints ({breakpoints.length})</div>
        <div style={{ marginBottom: 8 }}>
          {breakpoints.map(bp => (
            <div key={bp.id} className="tree-node" style={{ fontSize: 'var(--font-size-sm)' }}>
              <span style={{ cursor: 'pointer', marginRight: 4 }} onClick={() => {
                if (debuggerId) window.x3studio.debugger.removeBreakpoint(debuggerId, bp.file, bp.line);
                removeBreakpoint(bp.id);
              }}>✕</span>
              <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{bp.file}:{bp.line}</span>
            </div>
          ))}
          {breakpoints.length === 0 && <div style={{ color: 'var(--text-muted)', fontSize: 11 }}>No breakpoints set</div>}
        </div>

        {isAttached && (
          <>
            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Location</div>
            <div style={{ marginBottom: 8, fontSize: 'var(--font-size-sm)', fontFamily: 'var(--font-mono)' }}>
              {currentFile ? `${currentFile}:${currentLine}` : '—'}
            </div>
            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Call Stack ({callStack.length})</div>
            <div style={{ marginBottom: 8 }}>
              {callStack.map((cs, i) => (
                <div key={i} className="tree-node" style={{ fontSize: 'var(--font-size-sm)' }}>
                  <span style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{cs.function}</span>
                  <span style={{ color: 'var(--text-muted)' }}> at {cs.file}:{cs.line}</span>
                </div>
              ))}
            </div>
            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Variables ({variables.length})</div>
            <table className="data-table"><thead><tr><th>Name</th><th>Value</th><th>Type</th></tr></thead>
              <tbody>{variables.map((v, i) => (
                <tr key={i}><td style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>{v.name}</td>
                  <td style={{ fontFamily: 'var(--font-mono)', fontSize: 11, maxWidth: 120, overflow: 'hidden', textOverflow: 'ellipsis' }}>{v.value}</td>
                  <td style={{ fontSize: 11 }}>{v.type}</td></tr>
              ))}</tbody>
            </table>
          </>
        )}
        <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>Session Output</div>
        <pre style={{ background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)', fontSize: 11, maxHeight: 150, overflow: 'auto', whiteSpace: 'pre-wrap' }}>
          {sessionOutput || 'No output. Attach to begin.'}
        </pre>
      </div>
    </div>
  );
}
