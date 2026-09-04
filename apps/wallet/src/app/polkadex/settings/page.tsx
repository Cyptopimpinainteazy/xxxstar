'use client';

import { useState } from 'react';
import { Button } from '@/components/x3/UIComponents';

export default function SettingsPage() {
  const [settings, setSettings] = useState({
    slippageTolerance: 0.5,
    defaultOrderType: 'limit' as 'limit' | 'market',
    priceAlerts: true,
    twoFactorAuth: false,
    emailNotifications: true,
  });

  const handleSave = () => {
    alert('Settings saved successfully!');
  };

  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-3xl font-bold">Settings</h1>
        <span className="text-gray-400">Configure your POLKADEX trading preferences</span>
      </div>

      <div className="max-w-2xl space-y-6">
        {/* Trading Preferences */}
        <div className="bg-x3-dark p-6 rounded border border-x3-dark-gray">
          <h2 className="text-xl font-bold mb-4">Trading Preferences</h2>
          <div className="space-y-4">
            <div>
              <label className="block text-sm font-medium mb-2">Slippage Tolerance (%)</label>
              <div className="flex gap-2">
                <input
                  type="number"
                  value={settings.slippageTolerance}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      slippageTolerance: Number(e.target.value),
                    })
                  }
                  className="flex-1 bg-x3-dark-gray text-white p-2 rounded border border-x3-dark-gray"
                  min="0.1"
                  max="5"
                  step="0.1"
                />
                <span className="text-gray-400 text-sm leading-8">%</span>
              </div>
              <p className="text-xs text-gray-500 mt-1">
                Maximum price change you accept for swap execution
              </p>
            </div>

            <div>
              <label className="block text-sm font-medium mb-2">Default Order Type</label>
              <select
                value={settings.defaultOrderType}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    defaultOrderType: e.target.value as 'limit' | 'market',
                  })
                }
                className="w-full bg-x3-dark-gray text-white p-2 rounded border border-x3-dark-gray"
              >
                <option value="limit">Limit Order</option>
                <option value="market">Market Order</option>
              </select>
            </div>
          </div>
        </div>

        {/* Notifications */}
        <div className="bg-x3-dark p-6 rounded border border-x3-dark-gray">
          <h2 className="text-xl font-bold mb-4">Notifications</h2>
          <div className="space-y-3">
            {[
              {
                key: 'priceAlerts',
                label: 'Price Alerts',
                desc: 'Get notified when price targets are reached',
              },
              {
                key: 'emailNotifications',
                label: 'Email Notifications',
                desc: 'Receive email updates on order fills',
              },
            ].map((item) => (
              <div key={item.key} className="flex items-start gap-3">
                <input
                  type="checkbox"
                  checked={settings[item.key as keyof typeof settings] as boolean}
                  onChange={(e) =>
                    setSettings({
                      ...settings,
                      [item.key]: e.target.checked,
                    })
                  }
                  className="w-4 h-4 mt-1 accent-x3-orange"
                />
                <div>
                  <div className="font-medium">{item.label}</div>
                  <div className="text-sm text-gray-400">{item.desc}</div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Security */}
        <div className="bg-x3-dark p-6 rounded border border-x3-dark-gray">
          <h2 className="text-xl font-bold mb-4">Security</h2>
          <div className="space-y-3">
            <div className="flex items-start gap-3">
              <input
                type="checkbox"
                checked={settings.twoFactorAuth}
                onChange={(e) =>
                  setSettings({
                    ...settings,
                    twoFactorAuth: e.target.checked,
                  })
                }
                className="w-4 h-4 mt-1 accent-x3-orange"
              />
              <div>
                <div className="font-medium">Two-Factor Authentication</div>
                <div className="text-sm text-gray-400">
                  Require 2FA for large orders (&gt;$10,000)
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* API Settings */}
        <div className="bg-x3-dark p-6 rounded border border-x3-dark-gray">
          <h2 className="text-xl font-bold mb-4">API Keys</h2>
          <p className="text-sm text-gray-400 mb-4">
            Create API keys to trade programmatically via our REST/WebSocket APIs
          </p>
          <Button variant="secondary">Generate API Key</Button>
        </div>

        {/* Save Button */}
        <div className="flex gap-2">
          <Button variant="primary" onClick={handleSave}>
            Save Settings
          </Button>
          <Button variant="secondary">Reset to Defaults</Button>
        </div>
      </div>
    </div>
  );
}
