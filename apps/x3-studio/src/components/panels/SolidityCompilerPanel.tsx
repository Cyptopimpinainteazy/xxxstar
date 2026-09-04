import { useState } from 'react';
import { useSolidityCompilerStore, useSettingsStore } from '../../store';

const DEFAULT_SOURCE = `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

contract Counter {
    uint256 private count;

    function increment() public {
        count++;
    }

    function getCount() public view returns (uint256) {
        return count;
    }
}`;

const EVM_VERSIONS = ['default', 'london', 'paris', 'shanghai', 'cancun'];

export default function SolidityCompilerPanel() {
  const output = useSolidityCompilerStore(s => s.output);
  const isCompiling = useSolidityCompilerStore(s => s.isCompiling);
  const compileError = useSolidityCompilerStore(s => s.error);
  const setOutput = useSolidityCompilerStore(s => s.setOutput);
  const setCompiling = useSolidityCompilerStore(s => s.setCompiling);
  const setError = useSolidityCompilerStore(s => s.setError);
  const rpcUrl = useSettingsStore(s => s.chainRpcUrl);

  const [source, setSource] = useState(DEFAULT_SOURCE);
  const [remappings, setRemappings] = useState('');
  const [optimizerEnabled, setOptimizerEnabled] = useState(true);
  const [optimizerRuns, setOptimizerRuns] = useState('200');
  const [evmVersion, setEvmVersion] = useState('default');
  const [deployStatus, setDeployStatus] = useState('');

  const buildInput = () => {
    const settings: any = {
      optimizer: { enabled: optimizerEnabled, runs: parseInt(optimizerRuns) || 200 },
      outputSelection: {
        '*': { '*': ['abi', 'evm.bytecode.object', 'evm.deployedBytecode.object'] },
      },
    };
    if (remappings.trim()) {
      settings.remappings = remappings.split('\n').map(r => r.trim()).filter(Boolean);
    }
    if (evmVersion !== 'default') {
      settings.evmVersion = evmVersion;
    }
    return JSON.stringify({
      language: 'Solidity',
      sources: { 'Contract.sol': { content: source } },
      settings,
    });
  };

  const handleCompile = async () => {
    setCompiling(true);
    setError(null);
    setOutput(null);
    setDeployStatus('');
    try {
      const result = await window.x3studio.solidity.compile(buildInput(), '0.8.24');
      setOutput(result);
    } catch (err: any) {
      setError(err.message || String(err));
    } finally {
      setCompiling(false);
    }
  };

  const handleDeploy = async () => {
    if (!output?.contracts) return;
    const contract = output.contracts['Contract.sol'];
    if (!contract) return;
    const bytecode = contract.Counter?.evm?.bytecode?.object;
    if (!bytecode) {
      setDeployStatus('No bytecode found. Compile first.');
      return;
    }
    setDeployStatus('Deploying...');
    try {
      const txHash = await window.x3studio.chain.rpcCall(rpcUrl, 'eth_sendRawTransaction', ['0x' + bytecode]);
      setDeployStatus(`Deployed tx: ${txHash}`);
    } catch (err: any) {
      setDeployStatus(`Deploy failed: ${err.message || String(err)}`);
    }
  };

  const errors = output?.errors?.filter(e => e.severity === 'error') || [];
  const warnings = output?.errors?.filter(e => e.severity === 'warning') || [];

  const contracts = output?.contracts?.['Contract.sol'];
  const contractNames = contracts ? Object.keys(contracts) : [];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div className="panel-header">
        <span>Solidity Compiler</span>
      </div>
      <div className="panel-body" style={{ padding: '8px', overflow: 'auto' }}>
        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)' }}>Source Code</label>
          <textarea
            className="input-field"
            style={{ width: '100%', minHeight: 180, fontFamily: 'var(--font-mono)', fontSize: 11, resize: 'vertical' }}
            value={source}
            onChange={e => setSource(e.target.value)}
          />
        </div>

        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)' }}>Remappings (one per line)</label>
          <textarea
            className="input-field"
            style={{ width: '100%', minHeight: 50, fontFamily: 'var(--font-mono)', fontSize: 11, resize: 'vertical' }}
            value={remappings}
            onChange={e => setRemappings(e.target.value)}
            placeholder="@openzeppelin=node_modules/@openzeppelin"
          />
        </div>

        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)', display: 'flex', alignItems: 'center', gap: 4 }}>
            <input type="checkbox" checked={optimizerEnabled} onChange={e => setOptimizerEnabled(e.target.checked)} />
            Optimizer Enabled
          </label>
        </div>

        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)' }}>Optimizer Runs</label>
          <input
            className="input-field"
            type="number"
            style={{ width: 80, fontSize: 11 }}
            value={optimizerRuns}
            onChange={e => setOptimizerRuns(e.target.value)}
            min={1}
          />
        </div>

        <div className="form-group">
          <label style={{ fontSize: 'var(--font-size-sm)' }}>EVM Version</label>
          <select className="select-field" value={evmVersion} onChange={e => setEvmVersion(e.target.value)}>
            {EVM_VERSIONS.map(v => (
              <option key={v} value={v}>{v}</option>
            ))}
          </select>
        </div>

        <div style={{ display: 'flex', gap: 4, marginBottom: 8, flexWrap: 'wrap' }}>
          <button className="btn btn-primary" onClick={handleCompile} disabled={isCompiling}>
            {isCompiling ? 'Compiling...' : 'Compile'}
          </button>
          <button className="btn" onClick={handleDeploy} disabled={!output?.contracts}>
            Deploy
          </button>
        </div>

        {compileError && (
          <div style={{ color: 'var(--color-error, #ef4444)', background: 'rgba(239,68,68,0.1)', padding: 6, borderRadius: 'var(--radius)', marginBottom: 8, fontSize: 11, whiteSpace: 'pre-wrap' }}>
            {compileError}
          </div>
        )}

        {errors.length > 0 && (
          <div style={{ marginBottom: 8 }}>
            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)', color: 'var(--color-error, #ef4444)' }}>
              Errors ({errors.length})
            </div>
            {errors.map((e, i) => (
              <div key={i} style={{ color: 'var(--color-error, #ef4444)', fontSize: 11, marginBottom: 2, whiteSpace: 'pre-wrap' }}>
                {e.message}
              </div>
            ))}
          </div>
        )}

        {warnings.length > 0 && (
          <div style={{ marginBottom: 8 }}>
            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)', color: 'var(--color-warning, #eab308)' }}>
              Warnings ({warnings.length})
            </div>
            {warnings.map((w, i) => (
              <div key={i} style={{ color: 'var(--color-warning, #eab308)', fontSize: 11, marginBottom: 2, whiteSpace: 'pre-wrap' }}>
                {w.message}
              </div>
            ))}
          </div>
        )}

        {contractNames.length > 0 && (
          <div style={{ marginBottom: 8 }}>
            <div className="section-title" style={{ fontSize: 'var(--font-size-sm)' }}>
              Compiled Contracts: {contractNames.join(', ')}
            </div>
            {contractNames.map(name => {
              const c = contracts?.[name];
              const abi = c?.abi;
              const bytecodeObj = c?.evm?.bytecode?.object;
              const deployed = c?.evm?.deployedBytecode?.object;
              return (
                <div key={name} style={{ background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)', marginBottom: 6 }}>
                  <div style={{ fontSize: 11, fontWeight: 600, marginBottom: 4 }}>{name}</div>
                  {bytecodeObj && (
                    <div style={{ fontSize: 10, fontFamily: 'var(--font-mono)', marginBottom: 2 }}>
                      Bytecode: {bytecodeObj.slice(0, 50)}... ({bytecodeObj.length / 2} bytes)
                    </div>
                  )}
                  {deployed && (
                    <div style={{ fontSize: 10, fontFamily: 'var(--font-mono)', marginBottom: 4 }}>
                      Deployed: {deployed.slice(0, 50)}... ({deployed.length / 2} bytes)
                    </div>
                  )}
                  {abi && (
                    <details>
                      <summary style={{ fontSize: 11, cursor: 'pointer' }}>ABI ({abi.length} entries)</summary>
                      <pre style={{ fontSize: 10, fontFamily: 'var(--font-mono)', whiteSpace: 'pre-wrap', marginTop: 4, maxHeight: 200, overflow: 'auto', background: 'var(--bg-surface)', borderRadius: 'var(--radius)' }}>
                        {JSON.stringify(abi, null, 2)}
                      </pre>
                    </details>
                  )}
                </div>
              );
            })}
          </div>
        )}

        {deployStatus && (
          <div style={{ marginBottom: 8, fontSize: 11, padding: 6, borderRadius: 'var(--radius)', background: deployStatus.includes('failed') ? 'rgba(239,68,68,0.1)' : 'rgba(34,197,94,0.1)', color: deployStatus.includes('failed') ? 'var(--color-error, #ef4444)' : 'var(--color-success, #22c55e)' }}>
            {deployStatus}
          </div>
        )}
      </div>
    </div>
  );
}
