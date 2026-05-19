import { useState } from "react";
import {
  Gift,
  Star,
  Zap,
  TrendingUp,
  Users,
  Trophy,
  Crown,
  Award,
} from "lucide-react";

const earnActivities = [
  { name: "Bridge", points: "100 pts/tx", multiplier: "2x", icon: Zap, color: "from-blue-600 to-cyan-600" },
  { name: "Swap", points: "50 pts/tx", multiplier: "1.5x", icon: TrendingUp, color: "from-purple-600 to-blue-600" },
  { name: "Stake", points: "200 pts/day", multiplier: "3x", icon: Star, color: "from-orange-600 to-amber-600" },
  { name: "LP Provide", points: "150 pts/day", multiplier: "2.5x", icon: Gift, color: "from-green-600 to-emerald-600" },
  { name: "Referral", points: "500 pts/signup", multiplier: "5x", icon: Users, color: "from-pink-600 to-rose-600" },
  { name: "More Coming", points: "???", multiplier: "?x", icon: Award, color: "from-slate-600 to-slate-500" },
];

const tierLevels = [
  { name: "Bronze", minPts: 0, icon: Award, color: "text-orange-700" },
  { name: "Silver", minPts: 1000, icon: Star, color: "text-slate-300" },
  { name: "Gold", minPts: 5000, icon: Trophy, color: "text-amber-400" },
  { name: "Platinum", minPts: 25000, icon: Crown, color: "text-cyan-400" },
  { name: "Diamond", minPts: 100000, icon: Zap, color: "text-purple-400" },
];

const leaderboard = [
  { rank: 1, name: "0x1a2b...9f4e", points: "847,320", tier: "Diamond" },
  { rank: 2, name: "0x3c4d...7a2b", points: "623,140", tier: "Diamond" },
  { rank: 3, name: "0x5e6f...1c8d", points: "512,870", tier: "Platinum" },
  { rank: 4, name: "0x7g8h...3e0f", points: "398,450", tier: "Platinum" },
  { rank: 5, name: "0x9i0j...5g2h", points: "284,120", tier: "Gold" },
];

export default function EarnPanel() {
  const [userPoints] = useState(12450);

  const currentTier = [...tierLevels].reverse().find((t) => userPoints >= t.minPts) ?? tierLevels[0];
  const nextTier = tierLevels[tierLevels.indexOf(currentTier) + 1];
  const progress = nextTier
    ? ((userPoints - currentTier.minPts) / (nextTier.minPts - currentTier.minPts)) * 100
    : 100;

  return (
    <div className="overflow-y-auto h-full bg-slate-900 text-white p-6 space-y-6">
      {/* Header */}
      <div>
        <h1 className="text-2xl font-bold bg-gradient-to-r from-orange-400 to-amber-400 bg-clip-text text-transparent">
          Earn Points & Rewards
        </h1>
        <p className="text-sm text-slate-400 mt-1">
          Complete activities to earn points and climb the leaderboard
        </p>
      </div>

      {/* Season Banner */}
      <div className="bg-gradient-to-r from-orange-600/20 to-amber-600/20 border border-orange-500/30 rounded-xl p-5">
        <div className="flex items-center gap-3">
          <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-orange-500 to-amber-500 flex items-center justify-center">
            <Trophy className="w-6 h-6 text-white" />
          </div>
          <div className="flex-1">
            <div className="flex items-center gap-2">
              <h2 className="text-lg font-bold">Season 1: Genesis</h2>
              <span className="px-2 py-0.5 text-xs rounded-full bg-green-500/20 text-green-400">
                Active
              </span>
            </div>
            <p className="text-sm text-slate-300 mt-0.5">
              Earn bonus multipliers during the Genesis season
            </p>
          </div>
          <div className="text-right">
            <div className="text-sm text-slate-400">Your Points</div>
            <div className="text-2xl font-bold text-orange-400">
              {userPoints.toLocaleString()}
            </div>
          </div>
        </div>
      </div>

      {/* Tier Progress */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center gap-2">
            <currentTier.icon className={`w-5 h-5 ${currentTier.color}`} />
            <span className="font-semibold">{currentTier.name}</span>
          </div>
          {nextTier && (
            <div className="flex items-center gap-2 text-sm text-slate-400">
              <span>Next: {nextTier.name}</span>
              <nextTier.icon className={`w-4 h-4 ${nextTier.color}`} />
            </div>
          )}
        </div>
        <div className="w-full bg-slate-700/50 rounded-full h-3 overflow-hidden">
          <div
            className="h-full rounded-full bg-gradient-to-r from-orange-500 to-amber-500 transition-all"
            style={{ width: `${Math.min(progress, 100)}%` }}
          />
        </div>
        {nextTier && (
          <div className="mt-2 text-xs text-slate-400 flex justify-between">
            <span>{userPoints.toLocaleString()} pts</span>
            <span>{nextTier.minPts.toLocaleString()} pts</span>
          </div>
        )}
      </div>

      {/* Earn Activities */}
      <div>
        <h2 className="text-lg font-semibold mb-3">Earn Activities</h2>
        <div className="grid grid-cols-3 gap-3">
          {earnActivities.map((activity) => (
            <div
              key={activity.name}
              className="bg-slate-800/60 border border-slate-700/50 rounded-xl p-4 hover:border-orange-500/30 transition-colors"
            >
              <div
                className={`w-10 h-10 rounded-lg bg-gradient-to-br ${activity.color} flex items-center justify-center mb-3`}
              >
                <activity.icon className="w-5 h-5 text-white" />
              </div>
              <div className="font-semibold text-sm">{activity.name}</div>
              <div className="text-xs text-slate-400 mt-1">{activity.points}</div>
              <div className="mt-2 inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-orange-500/10 text-orange-400 text-xs">
                <Zap className="w-3 h-3" />
                {activity.multiplier}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Tier Levels */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <h2 className="text-lg font-semibold mb-3">Tier Levels</h2>
        <div className="flex items-center justify-between gap-2">
          {tierLevels.map((tier) => (
            <div
              key={tier.name}
              className={`flex-1 text-center p-3 rounded-lg ${
                tier.name === currentTier.name
                  ? "bg-orange-500/10 border border-orange-500/30"
                  : "bg-slate-900/50"
              }`}
            >
              <tier.icon className={`w-5 h-5 mx-auto mb-1 ${tier.color}`} />
              <div className="text-xs font-semibold">{tier.name}</div>
              <div className="text-xs text-slate-500 mt-0.5">
                {tier.minPts === 0 ? "0" : `${(tier.minPts / 1000).toFixed(0)}K`}+ pts
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Leaderboard */}
      <div className="bg-slate-800/40 border border-slate-700/50 rounded-xl p-5">
        <div className="flex items-center gap-2 mb-4">
          <Crown className="w-5 h-5 text-amber-400" />
          <h2 className="text-lg font-semibold">Leaderboard</h2>
        </div>
        <div className="space-y-2">
          {leaderboard.map((entry) => (
            <div
              key={entry.rank}
              className="flex items-center gap-3 bg-slate-900/50 rounded-lg p-3"
            >
              <div
                className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold ${
                  entry.rank === 1
                    ? "bg-amber-500/20 text-amber-400"
                    : entry.rank === 2
                    ? "bg-slate-400/20 text-slate-300"
                    : entry.rank === 3
                    ? "bg-orange-700/20 text-orange-400"
                    : "bg-slate-700/50 text-slate-400"
                }`}
              >
                {entry.rank}
              </div>
              <span className="flex-1 text-sm font-mono text-slate-300">{entry.name}</span>
              <span className="text-sm font-semibold">{entry.points}</span>
              <span className="text-xs text-slate-400">{entry.tier}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
