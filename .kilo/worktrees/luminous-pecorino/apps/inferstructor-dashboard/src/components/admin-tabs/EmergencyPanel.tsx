import React, { useState } from 'react';
import { ToggleRight, ToggleLeft } from 'lucide-react';
import { api } from '../../api';

interface EmergencyPanelProps {
  onStateChange?: (active: boolean) => void;
}

export const EmergencyPanel: React.FC<EmergencyPanelProps> = ({ onStateChange }) => {
  const [emergencyPauseActive, setEmergencyPauseActive] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleEmergencyPause = async () => {
    try {
      setError(null);
      const newState = !emergencyPauseActive;
      await api.adminAction('emergency_pause', { active: newState });
      setEmergencyPauseActive(newState);
      onStateChange?.(newState);
    } catch (err) {
      const errorMsg = err instanceof Error ? err.message : 'Failed to update emergency pause';
      setError(errorMsg);
    }
  };

  return (
    <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
      <h2 className="text-xl font-bold text-white mb-4">Emergency Controls</h2>
      <div className="space-y-4">
        {error && (
          <div className="p-4 bg-red-900/20 border border-red-700 rounded-lg">
            <p className="text-red-300 text-sm">{error}</p>
          </div>
        )}
        <div className="flex items-center justify-between p-4 bg-red-900/20 border border-red-700 rounded-lg">
          <div>
            <p className="text-white font-medium">Emergency Pause</p>
            <p className="text-gray-400 text-sm">Immediately halt all validator operations and swaps</p>
          </div>
          <button
            onClick={handleEmergencyPause}
            className={`px-4 py-2 rounded-lg font-medium flex items-center gap-2 transition-colors ${
              emergencyPauseActive
                ? 'bg-red-600 hover:bg-red-700 text-white'
                : 'bg-gray-700 hover:bg-gray-600 text-white'
            }`}
          >
            {emergencyPauseActive ? (
              <>
                <ToggleRight className="w-4 h-4" />
                ACTIVE
              </>
            ) : (
              <>
                <ToggleLeft className="w-4 h-4" />
                INACTIVE
              </>
            )}
          </button>
        </div>
        {emergencyPauseActive && (
          <div className="p-4 bg-yellow-900/20 border border-yellow-700 rounded-lg">
            <p className="text-yellow-300 text-sm">
              ⚠️ Emergency pause is active. System operations are halted. All users are notified.
            </p>
          </div>
        )}
      </div>
    </div>
  );
};
