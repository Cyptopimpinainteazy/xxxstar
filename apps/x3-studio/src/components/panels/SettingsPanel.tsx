import { useSettingsStore } from '../../store';

export default function SettingsPanel() {
  const settings = useSettingsStore();
  const saveSettings = async () => {
    try {
      await window.x3studio.fs.writeFile(
        '.x3studio/settings.json',
        JSON.stringify(settings, null, 2)
      );
    } catch {}
  };

  const exportSettings = () => {
    const blob = new Blob([JSON.stringify(settings, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'x3studio-settings.json';
    a.click();
    URL.revokeObjectURL(url);
  };

  const importSettings = () => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.json';
    input.onchange = async (e: any) => {
      const file = e.target.files?.[0];
      if (!file) return;
      const text = await file.text();
      try {
        const imported = JSON.parse(text);
        settings.update(imported);
        saveSettings();
      } catch {
        alert('Invalid settings file');
      }
    };
    input.click();
  };

  const Toggle = ({ value, onChange, label }: { value: boolean; onChange: (v: boolean) => void; label: string }) => (
    <div className="form-group" style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
      <label style={{ marginBottom: 0 }}>{label}</label>
      <label className="toggle">
        <input type="checkbox" checked={value} onChange={e => { onChange(e.target.checked); saveSettings(); }} />
        <span className="toggle-slider" />
      </label>
    </div>
  );

  return (
    <div style={{ padding: '8px', height: '100%', overflowY: 'auto' }}>
      <div className="panel-header">Settings</div>

      <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
        <button className="btn" onClick={exportSettings} style={{ fontSize: 10, padding: '2px 6px' }}>Export</button>
        <button className="btn" onClick={importSettings} style={{ fontSize: 10, padding: '2px 6px' }}>Import</button>
        <button className="btn" onClick={() => { settings.reset(); saveSettings(); }} style={{ fontSize: 10, padding: '2px 6px' }}>Reset</button>
      </div>

      <div className="section-title">Proof Mode</div>
      <Toggle value={settings.proofMode} onChange={v => settings.update({ proofMode: v })} label="Enable Proof Mode" />
      <Toggle value={settings.strictMainnet} onChange={v => settings.update({ strictMainnet: v })} label="Strict Mainnet Mode" />
      <Toggle value={settings.allowMocks} onChange={v => settings.update({ allowMocks: v })} label="Allow Mocks" />
      <Toggle value={settings.allowStubs} onChange={v => settings.update({ allowStubs: v })} label="Allow Stubs" />

      <div className="form-group">
        <label>Proof Output Directory</label>
        <input className="input-field" value={settings.proofOutputDir} onChange={e => { settings.update({ proofOutputDir: e.target.value }); saveSettings(); }} />
      </div>

      <div className="section-title">AI Provider</div>
      <div className="form-group">
        <label>Provider</label>
        <select className="select-field" value={settings.aiProvider} onChange={e => { settings.update({ aiProvider: e.target.value }); saveSettings(); }}>
          <option value="ollama">Ollama</option>
          <option value="lm-studio">LM Studio</option>
          <option value="openai">OpenAI-compatible</option>
          <option value="anthropic">Anthropic-compatible</option>
        </select>
      </div>
      <div className="form-group">
        <label>API Endpoint</label>
        <input className="input-field" value={settings.aiEndpoint} onChange={e => { settings.update({ aiEndpoint: e.target.value }); saveSettings(); }} />
      </div>
      <div className="form-group">
        <label>Model</label>
        <input className="input-field" value={settings.aiModel} onChange={e => { settings.update({ aiModel: e.target.value }); saveSettings(); }} />
      </div>

      <div className="section-title">AI Conversations</div>
      <Toggle value={settings.saveConversations} onChange={v => settings.update({ saveConversations: v })} label="Save Conversation History" />
      <div className="form-group">
        <label>Conversation Directory</label>
        <input className="input-field" value={settings.conversationDir} onChange={e => { settings.update({ conversationDir: e.target.value }); saveSettings(); }} />
      </div>

      <div className="section-title">Chain Connection</div>
      <div className="form-group">
        <label>RPC URL</label>
        <input className="input-field" value={settings.chainRpcUrl} onChange={e => { settings.update({ chainRpcUrl: e.target.value }); saveSettings(); }}
          placeholder="http://localhost:8545" />
      </div>

      <div className="section-title">Tools</div>
      <div className="form-group">
        <label>Forge Path</label>
        <input className="input-field" value={settings.forgePath} onChange={e => { settings.update({ forgePath: e.target.value }); saveSettings(); }} placeholder="forge" />
      </div>
      <div className="form-group">
        <label>Sourcify API URL</label>
        <input className="input-field" value={settings.sourcifyApiUrl} onChange={e => { settings.update({ sourcifyApiUrl: e.target.value }); saveSettings(); }} />
      </div>
      <div className="form-group">
        <label>Explorer API URL</label>
        <input className="input-field" value={settings.explorerApiUrl} onChange={e => { settings.update({ explorerApiUrl: e.target.value }); saveSettings(); }} placeholder="https://api.etherscan.io/api" />
      </div>

      <div className="section-title">Editor</div>
      <Toggle value={settings.autosave} onChange={v => settings.update({ autosave: v })} label="Autosave" />
      <div className="form-group">
        <label>Theme</label>
        <select className="select-field" value={settings.theme} onChange={e => settings.update({ theme: e.target.value as any })}>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </div>

      <div className="section-title">Commands</div>
      <div className="form-group">
        <label>Build/Verify Command</label>
        <input className="input-field" value={settings.verifyCommand} onChange={e => { settings.update({ verifyCommand: e.target.value }); saveSettings(); }} />
      </div>
      <div className="form-group">
        <label>Testnet Gate Command</label>
        <input className="input-field" value={settings.testnetGateCommand} onChange={e => { settings.update({ testnetGateCommand: e.target.value }); saveSettings(); }} />
      </div>
      <div className="form-group">
        <label>Mainnet Gate Command</label>
        <input className="input-field" value={settings.mainnetGateCommand} onChange={e => { settings.update({ mainnetGateCommand: e.target.value }); saveSettings(); }} />
      </div>
      <div className="form-group">
        <label>Command Timeout (seconds)</label>
        <input className="input-field" type="number" value={settings.commandTimeout} onChange={e => { settings.update({ commandTimeout: parseInt(e.target.value) || 120 }); saveSettings(); }} />
      </div>
    </div>
  );
}
