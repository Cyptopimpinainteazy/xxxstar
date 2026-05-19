import React, { useState } from 'react';
import { Fingerprint, Eye, EyeOff, Copy, Shield, AlertCircle } from 'lucide-react';

interface Session {
  id: string;
  device: string;
  browser: string;
  ipAddress: string;
  location: string;
  loginTime: string;
  lastActivity: string;
  isCurrent: boolean;
  isVerified: boolean;
}

export const SessionSecurityPanel: React.FC = () => {
  const [sessions, setSessions] = useState<Session[]>([
    {
      id: '1',
      device: 'MacBook Pro 16"',
      browser: 'Chrome 122.0',
      ipAddress: '192.168.1.105',
      location: 'San Francisco, US',
      loginTime: '2024-01-22 09:30 AM',
      lastActivity: 'Just now',
      isCurrent: true,
      isVerified: true,
    },
    {
      id: '2',
      device: 'iPhone 15 Pro',
      browser: 'Safari',
      ipAddress: '203.0.113.42',
      location: 'San Francisco, US',
      loginTime: '2024-01-20 03:15 PM',
      lastActivity: '2 hours ago',
      isCurrent: false,
      isVerified: true,
    },
    {
      id: '3',
      device: 'Windows 11 Desktop',
      browser: 'Firefox 122.0',
      ipAddress: '198.51.100.89',
      location: 'London, UK',
      loginTime: '2024-01-15 07:45 AM',
      lastActivity: '5 days ago',
      isCurrent: false,
      isVerified: false,
    },
  ]);
  const [showAllIPs, setShowAllIPs] = useState(false);

  const handleRevokeSession = (id: string) => {
    setSessions(sessions.filter((s) => s.id !== id));
  };

  const maskIP = (ip: string) => {
    if (showAllIPs) return ip;
    const parts = ip.split('.');
    return `${parts[0]}.${parts[1]}.${parts[2]}.***`;
  };

  const handleCopyIP = (ip: string) => {
    navigator.clipboard.writeText(ip);
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Session Security
            </h1>
            <p className="text-gray-400">Manage your active sessions and device access</p>
          </div>
          <Shield className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Security Overview */}
        <div className="grid grid-cols-3 gap-4 mb-6">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs mb-2">ACTIVE SESSIONS</div>
            <div className="text-3xl font-bold text-cyan-400 mb-1">{sessions.length}</div>
            <div className="text-xs text-gray-500">1 current session</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs mb-2">VERIFIED DEVICES</div>
            <div className="text-3xl font-bold text-green-400 mb-1">
              {sessions.filter((s) => s.isVerified).length}/{sessions.length}
            </div>
            <div className="text-xs text-gray-500">2FA enabled</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-xs mb-2">SECURITY LEVEL</div>
            <div className="text-3xl font-bold text-teal-400 mb-1">High</div>
            <div className="text-xs text-gray-500">All checks passed</div>
          </div>
        </div>

        {/* IP Masking Toggle */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4 mb-6 flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Fingerprint className="w-5 h-5 text-cyan-400" />
            <div>
              <p className="text-white font-semibold">IP Address Display</p>
              <p className="text-gray-400 text-sm">
                {showAllIPs ? 'Showing full IP addresses' : 'IP addresses are partially hidden'}
              </p>
            </div>
          </div>
          <button
            onClick={() => setShowAllIPs(!showAllIPs)}
            className={`px-4 py-2 rounded font-semibold transition ${
              showAllIPs ? 'bg-red-600 hover:bg-red-700' : 'bg-cyan-600 hover:bg-cyan-700'
            } text-white flex items-center gap-2`}
          >
            {showAllIPs ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
            {showAllIPs ? 'Hide IPs' : 'Show IPs'}
          </button>
        </div>

        {/* Sessions List */}
        <div className="space-y-4">
          {sessions.map((session) => (
            <div
              key={session.id}
              className={`border rounded-lg p-6 transition ${
                session.isCurrent
                  ? 'bg-[#1a1a2e]/80 border-cyan-500/30 ring-1 ring-cyan-500/20'
                  : 'bg-[#1a1a2e] border-[#2a2a35] hover:border-[#3a3a45]'
              }`}
            >
              <div className="flex items-start justify-between mb-4">
                <div className="flex-1">
                  <div className="flex items-center gap-3 mb-1">
                    <h3 className="text-white font-bold text-lg">{session.device}</h3>
                    {session.isCurrent && (
                      <span className="text-xs bg-cyan-500/20 text-cyan-400 border border-cyan-500/30 rounded-full px-2 py-1">
                        Current Session
                      </span>
                    )}
                    {!session.isVerified && (
                      <span className="text-xs bg-orange-500/20 text-orange-400 border border-orange-500/30 rounded-full px-2 py-1 flex items-center gap-1">
                        <AlertCircle className="w-3 h-3" /> Unverified
                      </span>
                    )}
                  </div>
                  <p className="text-gray-400 text-sm">{session.browser}</p>
                </div>
                {!session.isCurrent && (
                  <button
                    onClick={() => handleRevokeSession(session.id)}
                    className="px-3 py-1 bg-red-600/20 hover:bg-red-600/30 text-red-400 rounded text-sm font-semibold transition"
                  >
                    Revoke
                  </button>
                )}
              </div>

              <div className="grid grid-cols-2 gap-4">
                {/* IP Address */}
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                  <p className="text-gray-400 text-xs mb-1">IP Address</p>
                  <div className="flex items-center justify-between gap-2">
                    <p className="text-white font-mono text-sm">{maskIP(session.ipAddress)}</p>
                    <button
                      onClick={() => handleCopyIP(session.ipAddress)}
                      className="text-cyan-400 hover:text-cyan-300 transition"
                      title="Copy IP"
                    >
                      <Copy className="w-4 h-4" />
                    </button>
                  </div>
                </div>

                {/* Location */}
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                  <p className="text-gray-400 text-xs mb-1">Location</p>
                  <p className="text-white font-semibold text-sm">{session.location}</p>
                </div>

                {/* Login Time */}
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                  <p className="text-gray-400 text-xs mb-1">Login Time</p>
                  <p className="text-white font-semibold text-sm">{session.loginTime}</p>
                </div>

                {/* Last Activity */}
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-3">
                  <p className="text-gray-400 text-xs mb-1">Last Activity</p>
                  <p className={`font-semibold text-sm ${session.isCurrent ? 'text-green-400' : 'text-gray-400'}`}>
                    {session.lastActivity}
                  </p>
                </div>
              </div>
            </div>
          ))}
        </div>

        {/* Security Tips */}
        <div className="bg-blue-500/10 border border-blue-500/20 rounded-lg p-4 mt-8">
          <div className="flex gap-3">
            <Shield className="w-5 h-5 text-blue-400 flex-shrink-0 mt-0.5" />
            <div>
              <p className="text-blue-400 font-semibold mb-2">Security Recommendations</p>
              <ul className="text-blue-300 text-sm space-y-1">
                <li>• Regularly review active sessions and revoke unknown devices</li>
                <li>• Enable two-factor authentication on all accounts</li>
                <li>• Use strong, unique passwords for each service</li>
                <li>• Monitor login attempts and location changes</li>
              </ul>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export default SessionSecurityPanel;
