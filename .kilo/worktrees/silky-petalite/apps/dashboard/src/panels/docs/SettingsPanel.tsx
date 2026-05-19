import React, { useState } from 'react';
import { Settings, Moon, Bell, Lock, Database, Eye, EyeOff } from 'lucide-react';

interface Settings {
  theme: 'dark' | 'light';
  notifications: boolean;
  emailDigest: boolean;
  twoFactorAuth: boolean;
  autoBackup: boolean;
  dataRetention: number; // days
  privacyMode: boolean;
}

export const SettingsPanelComponent: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'general' | 'security' | 'data' | 'privacy'>('general');
  const [settings, setSettings] = useState<Settings>({
    theme: 'dark',
    notifications: true,
    emailDigest: true,
    twoFactorAuth: true,
    autoBackup: true,
    dataRetention: 90,
    privacyMode: false,
  });

  const handleSettingChange = (key: keyof Settings, value: any) => {
    setSettings({ ...settings, [key]: value });
  };

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Settings
            </h1>
            <p className="text-gray-400">Customize your preferences and configuration</p>
          </div>
          <Settings className="w-12 h-12 text-cyan-400" />
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['general', 'security', 'data', 'privacy'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'general' && 'General'}
              {tab === 'security' && 'Security'}
              {tab === 'data' && 'Data'}
              {tab === 'privacy' && 'Privacy'}
            </button>
          ))}
        </div>

        {/* General Settings */}
        {activeTab === 'general' && (
          <div className="space-y-4">
            {/* Theme */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-3">
                  <Moon className="w-5 h-5 text-cyan-400" />
                  <div>
                    <h3 className="text-white font-semibold">Theme</h3>
                    <p className="text-gray-400 text-sm">Choose your preferred color scheme</p>
                  </div>
                </div>
                <select
                  value={settings.theme}
                  onChange={(e) => handleSettingChange('theme', e.target.value)}
                  className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg px-4 py-2 text-white focus:outline-none focus:border-cyan-400"
                >
                  <option value="dark">Dark</option>
                  <option value="light">Light</option>
                </select>
              </div>
            </div>

            {/* Notifications */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Bell className="w-5 h-5 text-cyan-400" />
                  <div>
                    <h3 className="text-white font-semibold">Notifications</h3>
                    <p className="text-gray-400 text-sm">Receive alerts about important events</p>
                  </div>
                </div>
                <button
                  onClick={() => handleSettingChange('notifications', !settings.notifications)}
                  className={`px-4 py-2 rounded-full font-semibold transition ${
                    settings.notifications
                      ? 'bg-green-600 hover:bg-green-700 text-white'
                      : 'bg-gray-600 hover:bg-gray-700 text-white'
                  }`}
                >
                  {settings.notifications ? 'On' : 'Off'}
                </button>
              </div>
            </div>

            {/* Email Digest */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Bell className="w-5 h-5 text-cyan-400" />
                  <div>
                    <h3 className="text-white font-semibold">Email Digest</h3>
                    <p className="text-gray-400 text-sm">Receive weekly digest emails</p>
                  </div>
                </div>
                <button
                  onClick={() => handleSettingChange('emailDigest', !settings.emailDigest)}
                  className={`px-4 py-2 rounded-full font-semibold transition ${
                    settings.emailDigest
                      ? 'bg-green-600 hover:bg-green-700 text-white'
                      : 'bg-gray-600 hover:bg-gray-700 text-white'
                  }`}
                >
                  {settings.emailDigest ? 'On' : 'Off'}
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Security Settings */}
        {activeTab === 'security' && (
          <div className="space-y-4">
            {/* Two-Factor Authentication */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <Lock className="w-5 h-5 text-cyan-400" />
                  <div>
                    <h3 className="text-white font-semibold">Two-Factor Authentication</h3>
                    <p className="text-gray-400 text-sm">Add an extra layer of security to your account</p>
                  </div>
                </div>
                <button
                  onClick={() => handleSettingChange('twoFactorAuth', !settings.twoFactorAuth)}
                  className={`px-4 py-2 rounded-full font-semibold transition ${
                    settings.twoFactorAuth
                      ? 'bg-green-600 hover:bg-green-700 text-white'
                      : 'bg-gray-600 hover:bg-gray-700 text-white'
                  }`}
                >
                  {settings.twoFactorAuth ? 'Enabled' : 'Disabled'}
                </button>
              </div>

              {settings.twoFactorAuth && (
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <p className="text-gray-400 text-sm mb-3">Recovery codes:</p>
                  <div className="space-y-1 font-mono text-xs text-cyan-400">
                    <p>XXXX-XXXX-XXXX</p>
                    <p>XXXX-XXXX-XXXX</p>
                    <p>XXXX-XXXX-XXXX</p>
                  </div>
                </div>
              )}
            </div>

            {/* Password */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Lock className="w-5 h-5 text-cyan-400" />
                  <div>
                    <h3 className="text-white font-semibold">Change Password</h3>
                    <p className="text-gray-400 text-sm">Last changed 3 months ago</p>
                  </div>
                </div>
                <button className="px-4 py-2 bg-cyan-600 hover:bg-cyan-700 text-white rounded-lg font-semibold transition">
                  Update
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Data Settings */}
        {activeTab === 'data' && (
          <div className="space-y-4">
            {/* Auto Backup */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Database className="w-5 h-5 text-cyan-400" />
                  <div>
                    <h3 className="text-white font-semibold">Automatic Backups</h3>
                    <p className="text-gray-400 text-sm">Backup data automatically every day</p>
                  </div>
                </div>
                <button
                  onClick={() => handleSettingChange('autoBackup', !settings.autoBackup)}
                  className={`px-4 py-2 rounded-full font-semibold transition ${
                    settings.autoBackup
                      ? 'bg-green-600 hover:bg-green-700 text-white'
                      : 'bg-gray-600 hover:bg-gray-700 text-white'
                  }`}
                >
                  {settings.autoBackup ? 'On' : 'Off'}
                </button>
              </div>
            </div>

            {/* Data Retention */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <Database className="w-5 h-5 text-cyan-400" />
                  <div>
                    <h3 className="text-white font-semibold">Data Retention</h3>
                    <p className="text-gray-400 text-sm">How long to keep archived data</p>
                  </div>
                </div>
              </div>
              <div className="flex items-center gap-4">
                <input
                  type="range"
                  min="30"
                  max="365"
                  value={settings.dataRetention}
                  onChange={(e) => handleSettingChange('dataRetention', parseInt(e.target.value))}
                  className="flex-1 h-2 bg-[#0a0a0f] rounded-full appearance-none cursor-pointer accent-cyan-500"
                />
                <span className="text-white font-bold w-16 text-right">{settings.dataRetention} days</span>
              </div>
            </div>

            {/* Export Data */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  <Database className="w-5 h-5 text-cyan-400" />
                  <div>
                    <h3 className="text-white font-semibold">Export Data</h3>
                    <p className="text-gray-400 text-sm">Download your data in JSON format</p>
                  </div>
                </div>
                <button className="px-4 py-2 bg-cyan-600 hover:bg-cyan-700 text-white rounded-lg font-semibold transition">
                  Export
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Privacy Settings */}
        {activeTab === 'privacy' && (
          <div className="space-y-4">
            {/* Privacy Mode */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-3">
                  {settings.privacyMode ? (
                    <EyeOff className="w-5 h-5 text-cyan-400" />
                  ) : (
                    <Eye className="w-5 h-5 text-cyan-400" />
                  )}
                  <div>
                    <h3 className="text-white font-semibold">Privacy Mode</h3>
                    <p className="text-gray-400 text-sm">Hide sensitive information from view</p>
                  </div>
                </div>
                <button
                  onClick={() => handleSettingChange('privacyMode', !settings.privacyMode)}
                  className={`px-4 py-2 rounded-full font-semibold transition ${
                    settings.privacyMode
                      ? 'bg-green-600 hover:bg-green-700 text-white'
                      : 'bg-gray-600 hover:bg-gray-700 text-white'
                  }`}
                >
                  {settings.privacyMode ? 'On' : 'Off'}
                </button>
              </div>
            </div>

            {/* Data Sharing */}
            <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <div className="flex items-center gap-3">
                  <Eye className="w-5 h-5 text-cyan-400" />
                  <div>
                    <h3 className="text-white font-semibold">Analytics</h3>
                    <p className="text-gray-400 text-sm">Allow us to collect usage analytics</p>
                  </div>
                </div>
                <button className="px-4 py-2 rounded-full font-semibold bg-green-600 hover:bg-green-700 text-white transition">
                  Enabled
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Save Button */}
        <div className="mt-8 flex justify-end">
          <button className="px-6 py-3 bg-cyan-600 hover:bg-cyan-700 text-white font-bold rounded-lg transition">
            Save Changes
          </button>
        </div>
      </div>
    </div>
  );
};

export default SettingsPanelComponent;
