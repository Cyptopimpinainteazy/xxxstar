import ValidatorGlobe from './ValidatorGlobe';
import { Trophy, BarChart3, TrendingUp, Shield, Zap } from 'lucide-react';
import { useState } from 'react';
import clsx from 'clsx';

interface Validator {
  id: string;
  name: string;
  location: string;
  uptime: number;
  blocksProduced: number;
  gpuScore: number;
  reputation: number;
  slashCount: number;
  stakersCount: number;
}

const MOCK_VALIDATORS: Validator[] = [
  { id: '1', name: 'ValidatorNode-1', location: 'New York', uptime: 99.97, blocksProduced: 45231, gpuScore: 95, reputation: 9800, slashCount: 0, stakersCount: 1245 },
  { id: '2', name: 'ValidatorNode-2', location: 'Tokyo', uptime: 99.95, blocksProduced: 44892, gpuScore: 92, reputation: 9650, slashCount: 0, stakersCount: 987 },
  { id: '3', name: 'ValidatorNode-3', location: 'London', uptime: 99.92, blocksProduced: 44521, gpuScore: 88, reputation: 9400, slashCount: 1, stakersCount: 654 },
  { id: '4', name: 'ValidatorNode-4', location: 'Frankfurt', uptime: 99.85, blocksProduced: 44012, gpuScore: 85, reputation: 9100, slashCount: 0, stakersCount: 543 },
  { id: '5', name: 'ValidatorNode-5', location: 'Singapore', uptime: 99.78, blocksProduced: 43541, gpuScore: 82, reputation: 8900, slashCount: 2, stakersCount: 432 },
];

export default function ValidatorsPanel() {
  const [showLeaderboard, setShowLeaderboard] = useState(false);
  const [selectedValidator, setSelectedValidator] = useState<Validator | null>(null);

  const renderLeaderboard = () => {
    return (
      <div className="flex-1 flex flex-col overflow-auto px-5 py-4">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-sm font-semibold text-gray-400">Validator Leaderboard</h2>
          <div className="text-xs text-gray-500">Top performers by GPU score & uptime</div>
        </div>

        <div className="bg-[#111111] rounded-xl border border-[#1a1a1a] overflow-hidden shadow-2xl">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-[#1a1a1a] text-gray-500 text-xs">
                <th className="text-left p-3">Validator</th>
                <th className="text-center p-3">Uptime</th>
                <th className="text-center p-3">Blocks Produced</th>
                <th className="text-center p-3">GPU Score</th>
                <th className="text-center p-3">Reputation</th>
                <th className="text-center p-3">Slashes</th>
                <th className="text-center p-3">Stakers</th>
              </tr>
            </thead>
            <tbody>
              {MOCK_VALIDATORS.map((val, idx) => (
                <tr
                  key={val.id}
                  onClick={() => setSelectedValidator(val)}
                  className="border-b border-[#1a1a1a] last:border-0 hover:bg-[#0f0f14] transition-colors cursor-pointer group"
                >
                  <td className="p-3">
                    <div className="flex items-center gap-2">
                      {idx < 3 && <Trophy size={14} className={clsx(
                        idx === 0 ? 'text-yellow-400' : idx === 1 ? 'text-gray-300' : 'text-orange-600'
                      )} />}
                      <div>
                        <div className="font-medium text-white group-hover:text-blue-400 transition-colors">{val.name}</div>
                        <div className="text-xs text-gray-500">{val.location}</div>
                      </div>
                    </div>
                  </td>
                  <td className="p-3 text-center">
                    <span className={clsx('font-semibold', val.uptime > 99.9 ? 'text-green-400' : val.uptime > 99.5 ? 'text-blue-400' : 'text-yellow-400')}>
                      {val.uptime.toFixed(2)}%
                    </span>
                  </td>
                  <td className="p-3 text-center text-white font-mono">{val.blocksProduced.toLocaleString()}</td>
                  <td className="p-3 text-center">
                    <div className="flex items-center justify-center gap-1">
                      <Zap size={12} className="text-purple-400" />
                      <span className="text-purple-400 font-semibold">{val.gpuScore}</span>
                    </div>
                  </td>
                  <td className="p-3 text-center text-green-400 font-semibold">{val.reputation}</td>
                  <td className="p-3 text-center">
                    {val.slashCount > 0 ? (
                      <span className="text-red-400 font-semibold">{val.slashCount}</span>
                    ) : (
                      <span className="text-green-400">—</span>
                    )}
                  </td>
                  <td className="p-3 text-center text-blue-400 font-semibold">{val.stakersCount}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Validator Detail Modal */}
        {selectedValidator && (
          <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
            <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-96 shadow-2xl">
              <h3 className="font-bold text-white mb-4 flex items-center gap-2">
                <BarChart3 size={18} className="text-blue-400" />
                {selectedValidator.name}
              </h3>

              <div className="space-y-3 mb-6">
                <div className="bg-[#0a0a0f] rounded-lg p-3">
                  <div className="text-xs text-gray-500 mb-1">Location</div>
                  <div className="font-semibold text-white">{selectedValidator.location}</div>
                </div>

                <div className="grid grid-cols-2 gap-2">
                  <div className="bg-[#0a0a0f] rounded-lg p-3">
                    <div className="text-xs text-gray-500 mb-1">Uptime</div>
                    <div className={clsx('font-bold text-lg', selectedValidator.uptime > 99.9 ? 'text-green-400' : 'text-yellow-400')}>
                      {selectedValidator.uptime.toFixed(2)}%
                    </div>
                  </div>
                  <div className="bg-[#0a0a0f] rounded-lg p-3">
                    <div className="text-xs text-gray-500 mb-1">GPU Score</div>
                    <div className="font-bold text-lg text-purple-400">{selectedValidator.gpuScore}</div>
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-2">
                  <div className="bg-[#0a0a0f] rounded-lg p-3">
                    <div className="text-xs text-gray-500 mb-1">Reputation</div>
                    <div className="font-bold text-lg text-green-400">{selectedValidator.reputation}</div>
                  </div>
                  <div className="bg-[#0a0a0f] rounded-lg p-3">
                    <div className="text-xs text-gray-500 mb-1">Blocks Produced</div>
                    <div className="font-bold text-lg text-blue-400">{selectedValidator.blocksProduced.toLocaleString()}</div>
                  </div>
                </div>

                <div className="bg-[#0a0a0f] rounded-lg p-3">
                  <div className="text-xs text-gray-500 mb-2">Staking Pool</div>
                  <div className="flex justify-between items-center">
                    <span className="text-white">{selectedValidator.stakersCount} stakers</span>
                    <button className="px-3 py-1 rounded-lg bg-gradient-to-r from-blue-500 to-blue-600 text-white text-xs font-semibold hover:from-blue-400 hover:to-blue-500 transition-all">
                      Delegate
                    </button>
                  </div>
                </div>

                {selectedValidator.slashCount > 0 && (
                  <div className="bg-red-500/10 border border-red-500/40 rounded-lg p-3">
                    <div className="flex items-center gap-2">
                      <Shield size={14} className="text-red-400" />
                      <span className="text-sm text-red-400">
                        {selectedValidator.slashCount} slashing incident{selectedValidator.slashCount > 1 ? 's' : ''}
                      </span>
                    </div>
                  </div>
                )}
              </div>

              <button
                onClick={() => setSelectedValidator(null)}
                className="w-full px-4 py-2 rounded-lg bg-gradient-to-r from-blue-500 to-blue-600 text-white font-semibold"
              >
                Close
              </button>
            </div>
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="w-full h-full flex flex-col bg-[#06080f]">
      {/* Toggle Buttons */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-[#1a1a1a]">
        <h1 className="font-bold text-white flex items-center gap-2">
          <BarChart3 size={18} className="text-blue-400" />
          Validators
        </h1>
        <div className="flex gap-2">
          <button
            onClick={() => setShowLeaderboard(false)}
            className={clsx(
              'px-3 py-1.5 rounded-lg text-xs font-medium transition-colors',
              !showLeaderboard ? 'bg-blue-500/20 text-blue-400' : 'text-gray-500 hover:text-white'
            )}
          >
            🌐 Globe
          </button>
          <button
            onClick={() => setShowLeaderboard(true)}
            className={clsx(
              'px-3 py-1.5 rounded-lg text-xs font-medium transition-colors',
              showLeaderboard ? 'bg-blue-500/20 text-blue-400' : 'text-gray-500 hover:text-white'
            )}
          >
            🏆 Leaderboard
          </button>
        </div>
      </div>

      {/* Content */}
      {showLeaderboard ? renderLeaderboard() : <ValidatorGlobe />}
    </div>
  );
}
