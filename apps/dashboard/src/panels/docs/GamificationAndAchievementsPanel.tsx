import React, { useState } from 'react';
import { Lightbulb, Zap, Award, Target, ArrowUp, Star } from 'lucide-react';

interface Achievement {
  id: string;
  title: string;
  description: string;
  icon: string;
  progress: number;
  maxProgress: number;
  unlocked: boolean;
  reward: string;
}

interface Leaderboard {
  rank: number;
  name: string;
  score: number;
  trend: 'up' | 'down' | 'same';
}

export const GamificationAndAchievementsPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'achievements' | 'leaderboard' | 'quests'>('achievements');
  const [achievements] = useState<Achievement[]>([
    {
      id: '1',
      title: 'First Steps',
      description: 'Complete your first validator setup',
      icon: '🚀',
      progress: 1,
      maxProgress: 1,
      unlocked: true,
      reward: '100 XP',
    },
    {
      id: '2',
      title: 'Block Producer',
      description: 'Produce 100 blocks',
      icon: '⛏️',
      progress: 73,
      maxProgress: 100,
      unlocked: false,
      reward: '250 XP',
    },
    {
      id: '3',
      title: 'Perfect Uptime',
      description: 'Maintain 30 days of 99.9% uptime',
      icon: '⚡',
      progress: 15,
      maxProgress: 30,
      unlocked: false,
      reward: '500 XP',
    },
    {
      id: '4',
      title: 'Charity Champion',
      description: 'Delegate 10 ETH to community pool',
      icon: '❤️',
      progress: 6,
      maxProgress: 10,
      unlocked: false,
      reward: '300 XP',
    },
    {
      id: '5',
      title: 'Developer Master',
      description: 'Deploy 5 smart contracts',
      icon: '💻',
      progress: 3,
      maxProgress: 5,
      unlocked: false,
      reward: '400 XP',
    },
    {
      id: '6',
      title: 'Security Expert',
      description: 'Pass all security audits',
      icon: '🔒',
      progress: 1,
      maxProgress: 1,
      unlocked: true,
      reward: '600 XP',
    },
  ]);

  const [leaderboard] = useState<Leaderboard[]>([
    { rank: 1, name: 'You', score: 2850, trend: 'up' },
    { rank: 2, name: 'ValidatorMax', score: 2745, trend: 'down' },
    { rank: 3, name: 'StakeHolder42', score: 2620, trend: 'up' },
    { rank: 4, name: 'BlockMaster', score: 2500, trend: 'same' },
    { rank: 5, name: 'CryptoNinja', score: 2385, trend: 'down' },
  ]);

  const totalXP = achievements.filter((a) => a.unlocked).length * 100 + 250 + 300;
  const completionPercentage = Math.round((achievements.filter((a) => a.unlocked).length / achievements.length) * 100);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-6xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-cyan-400 to-blue-500 mb-2">
              Gamification & Achievements
            </h1>
            <p className="text-gray-400">Earn rewards and climb the leaderboard</p>
          </div>
          <Award className="w-12 h-12 text-cyan-400" />
        </div>

        {/* XP & Progress Overview */}
        <div className="grid grid-cols-3 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <div className="text-gray-400 text-sm font-semibold mb-2">TOTAL XP</div>
            <div className="text-4xl font-bold text-cyan-400 mb-1">{totalXP.toLocaleString()}</div>
            <div className="text-xs text-gray-500">Level 15 • Next: 3200 XP</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <div className="text-gray-400 text-sm font-semibold mb-2">ACHIEVEMENTS</div>
            <div className="text-4xl font-bold text-blue-400 mb-1">
              {achievements.filter((a) => a.unlocked).length}/{achievements.length}
            </div>
            <div className="text-xs text-gray-500">{completionPercentage}% complete</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
            <div className="text-gray-400 text-sm font-semibold mb-2">LEADERBOARD RANK</div>
            <div className="text-4xl font-bold text-teal-400 mb-1">#1</div>
            <div className="text-xs text-gray-500">Top 1% of validators</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['achievements', 'leaderboard', 'quests'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-cyan-400 border-b-2 border-cyan-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'achievements' && 'Achievements'}
              {tab === 'leaderboard' && 'Leaderboard'}
              {tab === 'quests' && 'Quests'}
            </button>
          ))}
        </div>

        {/* Achievements Grid */}
        {activeTab === 'achievements' && (
          <div className="grid grid-cols-3 gap-4">
            {achievements.map((achievement) => (
              <div
                key={achievement.id}
                className={`border rounded-lg p-6 transition ${
                  achievement.unlocked
                    ? 'bg-[#1a1a2e] border-cyan-500/30'
                    : 'bg-[#1a1a2e]/50 border-[#2a2a35] opacity-75'
                }`}
              >
                <div className="flex items-start justify-between mb-3">
                  <div className="text-4xl">{achievement.icon}</div>
                  {achievement.unlocked && <Star className="w-5 h-5 text-yellow-400" />}
                </div>

                <h3 className={`font-bold mb-1 ${achievement.unlocked ? 'text-white' : 'text-gray-400'}`}>
                  {achievement.title}
                </h3>
                <p className="text-gray-400 text-sm mb-3">{achievement.description}</p>

                {/* Progress Bar */}
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-full h-2 overflow-hidden mb-2">
                  <div
                    className="h-full bg-gradient-to-r from-cyan-500 to-blue-500"
                    style={{
                      width: `${(achievement.progress / achievement.maxProgress) * 100}%`,
                    }}
                  />
                </div>

                <div className="flex items-center justify-between text-xs">
                  <span className="text-gray-500">
                    {achievement.progress}/{achievement.maxProgress}
                  </span>
                  <span className={achievement.unlocked ? 'text-green-400 font-semibold' : 'text-cyan-400'}>
                    {achievement.reward}
                  </span>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Leaderboard */}
        {activeTab === 'leaderboard' && (
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg overflow-hidden">
            <div className="bg-[#0a0a0f] border-b border-[#2a2a35] px-6 py-4">
              <h2 className="text-white font-bold">Top Validators</h2>
            </div>
            <div className="divide-y divide-[#2a2a35]">
              {leaderboard.map((entry) => (
                <div
                  key={entry.rank}
                  className={`p-4 flex items-center justify-between ${
                    entry.rank === 1 ? 'bg-cyan-500/10 border-l-2 border-cyan-500' : ''
                  }`}
                >
                  <div className="flex items-center gap-4 flex-1">
                    <div className="text-white font-bold text-lg w-8">#{entry.rank}</div>
                    <div className="flex-1">
                      <p className="text-white font-semibold">{entry.name}</p>
                    </div>
                  </div>

                  <div className="flex items-center gap-4">
                    <div className="text-right">
                      <div className="text-white font-bold">{entry.score.toLocaleString()}</div>
                      <div className="text-gray-500 text-xs">XP</div>
                    </div>
                    <div className="w-8 text-right">
                      {entry.trend === 'up' && <ArrowUp className="w-5 h-5 text-green-400 ml-auto" />}
                      {entry.trend === 'down' && (
                        <ArrowUp className="w-5 h-5 text-red-400 ml-auto rotate-180" />
                      )}
                      {entry.trend === 'same' && <span className="text-gray-500">-</span>}
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Quests */}
        {activeTab === 'quests' && (
          <div className="space-y-4">
            {[
              {
                title: 'Daily Challenge: Mine 10 Blocks',
                description: 'Complete 10 blocks today',
                progress: 7,
                reward: '50 XP',
                color: 'blue',
              },
              {
                title: 'Weekly Mission: Community Support',
                description: 'Help 5 new validators this week',
                progress: 2,
                reward: '200 XP',
                color: 'purple',
              },
              {
                title: 'Monthly Goal: Top 10 Ranking',
                description: 'Reach top 10 leaderboard',
                progress: 100,
                reward: '500 XP',
                color: 'cyan',
              },
            ].map((quest, idx) => (
              <div key={idx} className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
                <div className="flex items-start justify-between mb-3">
                  <div className="flex items-center gap-3">
                    <Target className={`w-5 h-5 text-${quest.color}-400`} />
                    <div>
                      <h3 className="text-white font-bold">{quest.title}</h3>
                      <p className="text-gray-400 text-sm">{quest.description}</p>
                    </div>
                  </div>
                  <span className={`text-${quest.color}-400 font-semibold`}>{quest.reward}</span>
                </div>
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-full h-2">
                  <div
                    className={`h-full bg-gradient-to-r from-${quest.color}-500 to-${quest.color}-400`}
                    style={{ width: `${quest.progress}%` }}
                  />
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};

export default GamificationAndAchievementsPanel;
