import React, { useState } from 'react';
import { Loader } from 'lucide-react';
import { api } from '../../api';
import { useToast } from '../toast-context';
import {
  FAUCET_DEFAULT_RATE_LIMIT,
  FAUCET_DEFAULT_MAX_PER_ADDRESS,
  FAUCET_DEFAULT_COOLDOWN_HOURS,
} from '../../constants';

interface FaucetPanelProps {
  onSettingsSaved?: () => void;
}

export const FaucetPanel: React.FC<FaucetPanelProps> = ({ onSettingsSaved }) => {
  const { addToast } = useToast();
  const [faucetRateLimit, setFaucetRateLimit] = useState(FAUCET_DEFAULT_RATE_LIMIT);
  const [faucetMaxPerAddress, setFaucetMaxPerAddress] = useState(FAUCET_DEFAULT_MAX_PER_ADDRESS);
  const [faucetCooldown, setFaucetCooldown] = useState(FAUCET_DEFAULT_COOLDOWN_HOURS);
  const [savingSettings, setSavingSettings] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSaveFaucetSettings = async () => {
    try {
      setSavingSettings(true);
      setError(null);
      
      await api.adminAction('faucet_config', {
         rate_limit: parseInt(faucetRateLimit),
         max_per_address: parseInt(faucetMaxPerAddress),
         cooldown_hours: parseInt(faucetCooldown),
       });
       
       addToast('Faucet settings saved successfully', 'success');
       onSettingsSaved?.();
     } catch (err) {
       const errorMsg = err instanceof Error ? err.message : 'Failed to save faucet settings';
       setError(errorMsg);
       addToast(errorMsg, 'error');
     } finally {
       setSavingSettings(false);
     }
  };

  return (
    <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
      <h2 className="text-xl font-bold text-white mb-4">Faucet Configuration</h2>
      <div className="space-y-6">
        {error && (
          <div className="p-4 bg-red-900/20 border border-red-700 rounded-lg">
            <p className="text-red-300 text-sm">{error}</p>
          </div>
        )}
        <div>
          <label className="block text-gray-300 text-sm font-medium mb-2">Rate Limit (tokens/hour)</label>
          <input
            type="number"
            value={faucetRateLimit}
            onChange={(e) => setFaucetRateLimit(e.target.value)}
            className="w-full px-4 py-2 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg text-white focus:outline-none focus:border-blue-500"
          />
        </div>
        <div>
          <label className="block text-gray-300 text-sm font-medium mb-2">Max Per Address</label>
          <input
            type="text"
            value={faucetMaxPerAddress}
            onChange={(e) => setFaucetMaxPerAddress(e.target.value)}
            className="w-full px-4 py-2 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg text-white focus:outline-none focus:border-blue-500"
          />
        </div>
        <div>
          <label className="block text-gray-300 text-sm font-medium mb-2">Cooldown Period (hours)</label>
          <input
            type="number"
            value={faucetCooldown}
            onChange={(e) => setFaucetCooldown(e.target.value)}
            className="w-full px-4 py-2 bg-[#0a0a0f] border border-[#2a2a35] rounded-lg text-white focus:outline-none focus:border-blue-500"
          />
        </div>
        <button
          onClick={handleSaveFaucetSettings}
          disabled={savingSettings}
          className="px-4 py-2 bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 text-white rounded-lg transition-colors flex items-center gap-2"
        >
          {savingSettings && <Loader className="w-4 h-4 animate-spin" />}
          {savingSettings ? 'Saving...' : 'Save Settings'}
        </button>
      </div>
    </div>
  );
};
