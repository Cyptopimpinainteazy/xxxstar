import { useState } from 'react';
import { useWorkspaceStore, useSettingsStore } from '../../store';

export default function ContractVerificationPanel() {
  const wp = useWorkspaceStore(s => s.workspacePath);
  const sourcifyUrl = useSettingsStore(s => s.sourcifyApiUrl);
  const [address, setAddress] = useState('');
  const [chain, setChain] = useState('1');
  const [contractPath, setContractPath] = useState('');
  const [compilerVersion, setCompilerVersion] = useState('v0.8.24+commit.e11b9ed9');
  const [output, setOutput] = useState('');
  const [status, setStatus] = useState('');

  const verifySourcify = async () => {
    if (!wp || !address || !contractPath) return;
    setStatus('Verifying with Sourcify...');
    setOutput('');
    try {
      const metadataPath = wp + '/' + contractPath.replace('.sol', '.json').replace('contracts/', 'out/');
      let metadata;
      try {
        const raw = await window.x3studio.fs.readFile(metadataPath);
        metadata = JSON.parse(raw);
      } catch {
        metadata = { compiler: { version: compilerVersion }, sources: {} };
      }

      const formData = new FormData();
      formData.append('address', address);
      formData.append('chain', chain);
      const sources = metadata.sources || {};
      for (const [path, info] of Object.entries(sources)) {
        const fullPath = wp + '/' + path;
        try {
          const content = await window.x3studio.fs.readFile(fullPath);
          formData.append('files', new Blob([content]), path.replace(/^.*[\\/]/, ''));
        } catch {}
      }

      const res = await fetch(`${sourcifyUrl}/verify`, {
        method: 'POST',
        body: formData,
      });
      const data = await res.json();
      setOutput(JSON.stringify(data, null, 2));
      setStatus(data.status === 'perfect' ? '✓ Verified (perfect match)' : data.status === 'partial' ? '⚠ Partial match' : '✗ Verification failed');
    } catch (e: any) { setStatus('Error: ' + e.message); }
  };

  const verifyEtherscan = async () => {
    setStatus('Etherscan verification requires API key. Opening browser...');
    window.x3studio.shell.openExternal(`https://etherscan.io/verifyContract?a=${address}`);
  };

  const generateSourcifyJson = async () => {
    if (!wp || !address || !contractPath) return;
    try {
      const content = await window.x3studio.fs.readFile(wp + '/' + contractPath);
      const verificationJson = {
        address,
        chain: parseInt(chain),
        source: content,
        compilerVersion,
        metadata: { sources: {} },
      };
      const outPath = wp + '/x3-proof/verification-' + address + '.json';
      await window.x3studio.fs.writeFile(outPath, JSON.stringify(verificationJson, null, 2));
      setStatus(`✓ Verification data written to ${outPath}`);
    } catch (e: any) { setStatus('Error: ' + e.message); }
  };

  return (
    <div style={{ padding: 8, overflow: 'auto', height: '100%' }}>
      <div className="panel-header" style={{ margin: '-8px -8px 8px -8px' }}>Contract Verification</div>
      <p style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 8 }}>
        Verify deployed contracts on Sourcify or Etherscan.
      </p>

      <div className="form-group">
        <label style={{ fontSize: 'var(--font-size-sm)' }}>Contract Address</label>
        <input className="input-field" value={address} onChange={e => setAddress(e.target.value)} placeholder="0x..." />
      </div>
      <div className="form-group">
        <label style={{ fontSize: 'var(--font-size-sm)' }}>Chain ID</label>
        <select className="select-field" value={chain} onChange={e => setChain(e.target.value)}>
          <option value="1">Ethereum (1)</option>
          <option value="8453">Base (8453)</option>
          <option value="42161">Arbitrum (42161)</option>
          <option value="137">Polygon (137)</option>
          <option value="10">Optimism (10)</option>
          <option value="49009">X3 Local (49009)</option>
        </select>
      </div>
      <div className="form-group">
        <label style={{ fontSize: 'var(--font-size-sm)' }}>Contract Source Path (relative)</label>
        <input className="input-field" value={contractPath} onChange={e => setContractPath(e.target.value)} placeholder="contracts/MyContract.sol" />
      </div>
      <div className="form-group">
        <label style={{ fontSize: 'var(--font-size-sm)' }}>Compiler Version</label>
        <input className="input-field" value={compilerVersion} onChange={e => setCompilerVersion(e.target.value)} placeholder="v0.8.24+commit.e11b9ed9" />
      </div>

      <div style={{ display: 'flex', gap: 4, marginBottom: 8, flexWrap: 'wrap' }}>
        <button className="btn btn-primary" onClick={verifySourcify} disabled={!address || !contractPath}>Verify with Sourcify</button>
        <button className="btn" onClick={verifyEtherscan}>Etherscan (Browser)</button>
        <button className="btn" onClick={generateSourcifyJson}>Generate Verification File</button>
      </div>

      {status && <div style={{ fontSize: 11, marginBottom: 8, color: status.includes('✓') ? 'var(--pass-color)' : status.includes('⚠') ? 'var(--warn-color)' : 'var(--fail-color)' }}>{status}</div>}

      {output && (
        <>
          <div className="section-title">Response</div>
          <pre style={{ background: 'var(--bg-surface)', padding: 8, borderRadius: 'var(--radius)', fontSize: 10, maxHeight: 200, overflow: 'auto', whiteSpace: 'pre-wrap' }}>{output}</pre>
        </>
      )}
    </div>
  );
}
