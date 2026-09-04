import { Shield, LogOut } from 'lucide-react';
import { api } from '../../api';

interface DashboardHeaderProps {
  onLogout: () => void;
  onAdmin?: () => void;
  onLeaderboard?: () => void;
}

export function DashboardHeader({ onLogout, onAdmin, onLeaderboard }: DashboardHeaderProps) {
  return (
    <div className="flex items-center justify-between mb-8">
      <div>
        <h1 className="text-3xl font-bold text-white mb-2">Inferstructor Dashboard</h1>
        <p className="text-gray-400">Validator: {api.getValidatorId()}</p>
      </div>
      <div className="flex items-center gap-3">
        {onLeaderboard && (
          <button
            onClick={onLeaderboard}
            className="flex items-center gap-2 px-4 py-2 text-yellow-300 hover:text-white bg-yellow-900/30 hover:bg-yellow-800/40 border border-yellow-700/50 rounded-lg transition-colors"
          >
            🏆
            TPS Leaderboard
          </button>
        )}
        {onAdmin && (
          <button
            onClick={onAdmin}
            className="flex items-center gap-2 px-4 py-2 text-red-300 hover:text-white bg-red-900/30 hover:bg-red-800/40 border border-red-700/50 rounded-lg transition-colors"
          >
            <Shield className="w-4 h-4" />
            Admin
          </button>
        )}
        <button
          onClick={onLogout}
          className="flex items-center gap-2 px-4 py-2 text-gray-300 hover:text-white hover:bg-gray-800 rounded-lg transition-colors"
        >
          <LogOut className="w-4 h-4" />
          Logout
        </button>
      </div>
    </div>
  );
}
