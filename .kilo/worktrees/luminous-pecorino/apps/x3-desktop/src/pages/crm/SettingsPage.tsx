import React, { useEffect, useState } from "react";
import { useCrmStore } from "@/stores/crmStore";
import type { SaveSmtpConfigInput } from "@/stores/crmStore";

interface VpnProxyConfig {
  vpnEnabled: boolean;
  vpnEndpoint: string;
  vpnProtocol: 'wireguard' | 'openvpn' | 'ipsec';
  proxyEnabled: boolean;
  proxyType: 'socks5' | 'http' | 'https';
  proxyHost: string;
  proxyPort: number;
  proxyAuth: boolean;
  proxyUsername: string;
  proxyPassword: string;
}

const DEFAULT_VPN_PROXY: VpnProxyConfig = {
  vpnEnabled: false,
  vpnEndpoint: '',
  vpnProtocol: 'wireguard',
  proxyEnabled: false,
  proxyType: 'socks5',
  proxyHost: '',
  proxyPort: 1080,
  proxyAuth: false,
  proxyUsername: '',
  proxyPassword: '',
};

const loadVpnProxy = (): VpnProxyConfig => {
  try {
    const raw = localStorage.getItem('x3-crm-vpn-proxy');
    return raw ? { ...DEFAULT_VPN_PROXY, ...JSON.parse(raw) } : DEFAULT_VPN_PROXY;
  } catch { return DEFAULT_VPN_PROXY; }
};

const SettingsPage: React.FC = () => {
  const { smtpConfig, loadSmtpConfig, saveSmtpConfig, loading, error } = useCrmStore();
  const [saved, setSaved] = useState(false);
  const [testStatus, setTestStatus] = useState<string | null>(null);

  // VPN / Proxy state
  const [vpnProxy, setVpnProxy] = useState<VpnProxyConfig>(loadVpnProxy);
  const [vpnSaved, setVpnSaved] = useState(false);

  const updateVpn = (patch: Partial<VpnProxyConfig>) => setVpnProxy(prev => ({ ...prev, ...patch }));
  const saveVpnProxy = () => {
    localStorage.setItem('x3-crm-vpn-proxy', JSON.stringify(vpnProxy));
    setVpnSaved(true);
    setTimeout(() => setVpnSaved(false), 3000);
  };

  const [form, setForm] = useState<SaveSmtpConfigInput>({
    host: "", port: 587, username: "", password: "",
    fromName: "", fromEmail: "", useTls: true,
  });

  useEffect(() => { loadSmtpConfig(); }, [loadSmtpConfig]);

  useEffect(() => {
    if (smtpConfig) {
      setForm({
        host: smtpConfig.host,
        port: smtpConfig.port,
        username: smtpConfig.username,
        password: "", // never sent back
        fromName: smtpConfig.fromName,
        fromEmail: smtpConfig.fromEmail,
        useTls: smtpConfig.useTls,
      });
    }
  }, [smtpConfig]);

  const handleSave = async () => {
    if (!form.host || !form.username || !form.fromEmail) return;
    setSaved(false);
    await saveSmtpConfig(form);
    if (!error) {
      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    }
  };

  const handleTest = async () => {
    if (!smtpConfig) {
      setTestStatus("Save SMTP config first");
      return;
    }
    setTestStatus("Sending test email...");
    try {
      const { sendEmail } = useCrmStore.getState();
      await sendEmail({
        toEmail: form.fromEmail,
        subject: "X3 CRM — SMTP Test",
        body: "<h2>SMTP Test</h2><p>If you received this email, your SMTP configuration is working correctly!</p>",
      });
      setTestStatus("✅ Test email sent! Check your inbox.");
    } catch (err: any) {
      setTestStatus(`❌ Failed: ${err}`);
    }
  };

  return (
    <div className="crm-page">
      <div className="crm-page-header">
        <h1>Settings</h1>
      </div>

      {/* SMTP Configuration */}
      <div className="crm-card" style={{ maxWidth: 600 }}>
        <h2>📧 SMTP Email Server</h2>
        <p className="crm-help-text">
          Configure your outgoing email server for sending emails from the CRM. 
          Common providers: Gmail (smtp.gmail.com:587), Outlook (smtp-mail.outlook.com:587), 
          SendGrid (smtp.sendgrid.net:587).
        </p>

        {saved && <div className="crm-success-banner">✅ SMTP settings saved!</div>}
        {error && <div className="crm-error-banner">❌ {error}</div>}

        <div className="crm-form">
          <div className="crm-form-row">
            <div style={{ flex: 2 }}>
              <label>SMTP Host *</label>
              <input
                value={form.host}
                onChange={(e) => setForm({ ...form, host: e.target.value })}
                placeholder="smtp.gmail.com"
              />
            </div>
            <div style={{ flex: 1 }}>
              <label>Port</label>
              <input
                type="number"
                value={form.port ?? 587}
                onChange={(e) => setForm({ ...form, port: +e.target.value })}
              />
            </div>
          </div>

          <div className="crm-form-row">
            <div>
              <label>Username *</label>
              <input
                value={form.username}
                onChange={(e) => setForm({ ...form, username: e.target.value })}
                placeholder="your@email.com"
              />
            </div>
            <div>
              <label>Password *</label>
              <input
                type="password"
                value={form.password}
                onChange={(e) => setForm({ ...form, password: e.target.value })}
                placeholder={smtpConfig ? "••••••••" : "App password"}
              />
            </div>
          </div>

          <div className="crm-form-row">
            <div>
              <label>From Name</label>
              <input
                value={form.fromName}
                onChange={(e) => setForm({ ...form, fromName: e.target.value })}
                placeholder="X3 CRM"
              />
            </div>
            <div>
              <label>From Email *</label>
              <input
                type="email"
                value={form.fromEmail}
                onChange={(e) => setForm({ ...form, fromEmail: e.target.value })}
                placeholder="noreply@yourdomain.com"
              />
            </div>
          </div>

          <label className="crm-checkbox-row">
            <input
              type="checkbox"
              checked={form.useTls ?? true}
              onChange={(e) => setForm({ ...form, useTls: e.target.checked })}
            />
            Use TLS (recommended)
          </label>

          <div className="crm-form-actions">
            <button className="crm-btn" onClick={handleTest}>🧪 Send Test Email</button>
            <div style={{ flex: 1 }} />
            <button className="crm-btn primary" onClick={handleSave} disabled={loading}>
              {loading ? "Saving..." : "💾 Save Settings"}
            </button>
          </div>

          {testStatus && (
            <div className="crm-info-banner" style={{ marginTop: 12 }}>{testStatus}</div>
          )}
        </div>
      </div>

      {/* VPN / Proxy Configuration */}
      <div className="crm-card" style={{ maxWidth: 600, marginTop: 16 }}>
        <h2>🔒 VPN / Proxy</h2>
        <p className="crm-help-text">
          Route CRM traffic through a VPN tunnel or proxy server for enhanced privacy and security.
        </p>

        {vpnSaved && <div className="crm-success-banner">✅ VPN/Proxy settings saved!</div>}

        <div className="crm-form">
          {/* VPN Toggle */}
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '8px 0', borderBottom: '1px solid var(--border-default, #333)' }}>
            <div>
              <div style={{ fontWeight: 600, fontSize: '0.9rem' }}>🛡️ VPN</div>
              <div style={{ fontSize: '0.75rem', opacity: 0.6 }}>Encrypt all CRM network traffic</div>
            </div>
            <label style={{ position: 'relative', display: 'inline-block', width: 48, height: 26 }}>
              <input type="checkbox" checked={vpnProxy.vpnEnabled} onChange={e => updateVpn({ vpnEnabled: e.target.checked })}
                style={{ opacity: 0, width: 0, height: 0 }} />
              <span style={{
                position: 'absolute', cursor: 'pointer', inset: 0, borderRadius: 13,
                background: vpnProxy.vpnEnabled ? '#4caf50' : '#555',
                transition: 'background 0.3s',
              }}>
                <span style={{
                  position: 'absolute', height: 20, width: 20, left: vpnProxy.vpnEnabled ? 24 : 4, bottom: 3,
                  background: 'white', borderRadius: '50%', transition: 'left 0.3s',
                }} />
              </span>
            </label>
          </div>

          {vpnProxy.vpnEnabled && (
            <div className="crm-form-row" style={{ marginTop: 8 }}>
              <div style={{ flex: 2 }}>
                <label>VPN Endpoint</label>
                <input value={vpnProxy.vpnEndpoint} onChange={e => updateVpn({ vpnEndpoint: e.target.value })}
                  placeholder="vpn.example.com:51820" />
              </div>
              <div style={{ flex: 1 }}>
                <label>Protocol</label>
                <select value={vpnProxy.vpnProtocol} onChange={e => updateVpn({ vpnProtocol: e.target.value as VpnProxyConfig['vpnProtocol'] })}
                  style={{ width: '100%', padding: '6px 8px', background: 'var(--bg-primary, #111)', border: '1px solid var(--border-default, #333)', borderRadius: 6, color: 'inherit' }}>
                  <option value="wireguard">WireGuard</option>
                  <option value="openvpn">OpenVPN</option>
                  <option value="ipsec">IPSec</option>
                </select>
              </div>
            </div>
          )}

          {/* Proxy Toggle */}
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', padding: '12px 0 8px', borderBottom: '1px solid var(--border-default, #333)', marginTop: 8 }}>
            <div>
              <div style={{ fontWeight: 600, fontSize: '0.9rem' }}>🌐 Proxy</div>
              <div style={{ fontSize: '0.75rem', opacity: 0.6 }}>Route traffic through a proxy server</div>
            </div>
            <label style={{ position: 'relative', display: 'inline-block', width: 48, height: 26 }}>
              <input type="checkbox" checked={vpnProxy.proxyEnabled} onChange={e => updateVpn({ proxyEnabled: e.target.checked })}
                style={{ opacity: 0, width: 0, height: 0 }} />
              <span style={{
                position: 'absolute', cursor: 'pointer', inset: 0, borderRadius: 13,
                background: vpnProxy.proxyEnabled ? '#4caf50' : '#555',
                transition: 'background 0.3s',
              }}>
                <span style={{
                  position: 'absolute', height: 20, width: 20, left: vpnProxy.proxyEnabled ? 24 : 4, bottom: 3,
                  background: 'white', borderRadius: '50%', transition: 'left 0.3s',
                }} />
              </span>
            </label>
          </div>

          {vpnProxy.proxyEnabled && (
            <>
              <div className="crm-form-row" style={{ marginTop: 8 }}>
                <div style={{ flex: 1 }}>
                  <label>Type</label>
                  <select value={vpnProxy.proxyType} onChange={e => updateVpn({ proxyType: e.target.value as VpnProxyConfig['proxyType'] })}
                    style={{ width: '100%', padding: '6px 8px', background: 'var(--bg-primary, #111)', border: '1px solid var(--border-default, #333)', borderRadius: 6, color: 'inherit' }}>
                    <option value="socks5">SOCKS5</option>
                    <option value="http">HTTP</option>
                    <option value="https">HTTPS</option>
                  </select>
                </div>
                <div style={{ flex: 2 }}>
                  <label>Host</label>
                  <input value={vpnProxy.proxyHost} onChange={e => updateVpn({ proxyHost: e.target.value })}
                    placeholder="127.0.0.1" />
                </div>
                <div style={{ flex: 1 }}>
                  <label>Port</label>
                  <input type="number" value={vpnProxy.proxyPort} onChange={e => updateVpn({ proxyPort: +e.target.value })} />
                </div>
              </div>

              <label className="crm-checkbox-row" style={{ marginTop: 8 }}>
                <input type="checkbox" checked={vpnProxy.proxyAuth} onChange={e => updateVpn({ proxyAuth: e.target.checked })} />
                Proxy requires authentication
              </label>

              {vpnProxy.proxyAuth && (
                <div className="crm-form-row">
                  <div>
                    <label>Username</label>
                    <input value={vpnProxy.proxyUsername} onChange={e => updateVpn({ proxyUsername: e.target.value })} placeholder="proxy-user" />
                  </div>
                  <div>
                    <label>Password</label>
                    <input type="password" value={vpnProxy.proxyPassword} onChange={e => updateVpn({ proxyPassword: e.target.value })} placeholder="••••••••" />
                  </div>
                </div>
              )}
            </>
          )}

          <div className="crm-form-actions" style={{ marginTop: 12 }}>
            <div style={{ flex: 1 }} />
            <button className="crm-btn primary" onClick={saveVpnProxy}>
              💾 Save VPN/Proxy
            </button>
          </div>
        </div>
      </div>

      {/* Info Card */}
      <div className="crm-card" style={{ maxWidth: 600, marginTop: 16 }}>
        <h2>ℹ️ About X3 CRM</h2>
        <p className="crm-help-text">
          X3 CRM is a built-in contact and calendar management system. 
          Manage contacts, track deals through your pipeline, schedule events, 
          and send emails — all from your X3 Desktop.
        </p>
        <ul className="crm-feature-list">
          <li>👥 Contact management with stages and priorities</li>
          <li>📅 Full calendar with event types and reminders</li>
          <li>💰 Deal pipeline with kanban board view</li>
          <li>✉️ SMTP email with templates</li>
          <li>📊 Activity tracking and statistics</li>
        </ul>
      </div>
    </div>
  );
};

export default SettingsPage;
