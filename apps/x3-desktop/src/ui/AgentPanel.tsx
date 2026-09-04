import React from 'react';
import { AgentState } from '../agents/AgentStore';
import { strategyCore } from '../agents/StrategyCore';

interface AgentPanelProps {
  agent: AgentState;
  onClose: () => void;
}

/**
 * Agent Panel — side bar showing selected agent details.
 * Displays strategy info, PnL, health, last action, and chain.
 */
export function AgentPanel({ agent, onClose }: AgentPanelProps) {
  const strategy = strategyCore.getStrategy(agent.strategyId);

  const statusColors: Record<string, string> = {
    idle: 'text-gray-400 bg-gray-500/20',
    attacking: 'text-red-400 bg-red-500/20',
    defending: 'text-blue-400 bg-blue-500/20',
    strategy_executing: 'text-yellow-400 bg-yellow-500/20',
    hedging: 'text-orange-400 bg-orange-500/20',
  };

  const statusLabel = agent.status.replace('_', ' ').toUpperCase();

  return (
    <div className="h-full w-80 bg-black/70 backdrop-blur-md border-l border-white/10 flex flex-col pointer-events-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-white/10">
        <div className="flex items-center gap-2">
          <div
            className="w-4 h-4 rounded-full"
            style={{ backgroundColor: agent.color }}
          />
          <h2 className="text-white font-semibold text-sm">{agent.name}</h2>
        </div>
        <button
          onClick={onClose}
          className="text-white/40 hover:text-white/80 text-lg leading-none"
        >
          &times;
        </button>
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-4 text-sm">
        {/* Status badge */}
        <div className="flex items-center gap-2">
          <span className={`px-2 py-0.5 rounded text-[10px] font-mono font-bold ${statusColors[agent.status] || 'text-gray-400 bg-gray-500/20'}`}>
            {statusLabel}
          </span>
          <span className="text-white/40 text-[10px] font-mono">
            {agent.chain}
          </span>
        </div>

        {/* Health bar */}
        <div>
          <div className="flex justify-between text-xs mb-1">
            <span className="text-white/60">Health</span>
            <span className="text-white font-mono">{agent.health}%</span>
          </div>
          <div className="h-2 bg-white/10 rounded-full overflow-hidden">
            <div
              className="h-full rounded-full transition-all duration-300"
              style={{
                width: `${agent.health}%`,
                background: agent.health > 50
                  ? 'linear-gradient(90deg, #00cc66, #00ff88)'
                  : agent.health > 25
                    ? 'linear-gradient(90deg, #ffaa00, #ffcc00)'
                    : 'linear-gradient(90deg, #ff3333, #ff6666)',
              }}
            />
          </div>
        </div>

        {/* Stats grid */}
        <div className="grid grid-cols-2 gap-3">
          <PanelStat label="PnL" value={`$${agent.pnl.toLocaleString(undefined, { maximumFractionDigits: 2 })}`} />
          <PanelStat label="XP" value={agent.xp.toLocaleString()} />
          <PanelStat label="Chain" value={agent.chain} />
          <PanelStat label="Status" value={statusLabel} />
        </div>

        {/* Strategy info */}
        {strategy && (
          <div className="space-y-2">
            <h3 className="text-white/50 text-[10px] font-semibold uppercase tracking-wider">Strategy</h3>
            <div className="bg-white/5 rounded-lg p-3 space-y-2">
              <div className="flex justify-between">
                <span className="text-white/60 text-xs">Name</span>
                <span className="text-white font-mono text-xs">{strategy.name}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-white/60 text-xs">Type</span>
                <span className="text-white font-mono text-xs capitalize">{strategy.type.replace('_', ' ')}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-white/60 text-xs">Risk</span>
                <span className="text-xs font-mono capitalize"
                  style={{
                    color: strategy.riskProfile === 'low' ? '#00cc66' : strategy.riskProfile === 'medium' ? '#ffaa00' : strategy.riskProfile === 'high' ? '#ff6600' : '#ff3333',
                  }}
                >
                  {strategy.riskProfile}
                </span>
              </div>
              <div className="flex justify-between">
                <span className="text-white/60 text-xs">Win Rate</span>
                <span className="text-white font-mono text-xs">{(strategy.winRate * 100).toFixed(0)}%</span>
              </div>
              <div className="flex justify-between">
                <span className="text-white/60 text-xs">Sharpe</span>
                <span className="text-white font-mono text-xs">{strategy.sharpeRatio.toFixed(2)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-white/60 text-xs">Confidence</span>
                <span className="text-white font-mono text-xs">{(strategy.confidence * 100).toFixed(0)}%</span>
              </div>
            </div>
          </div>
        )}

        {/* Last action */}
        <div>
          <h3 className="text-white/50 text-[10px] font-semibold uppercase tracking-wider mb-1">Last Action</h3>
          <p className="text-white/80 text-xs leading-relaxed bg-white/5 rounded-lg p-3">
            {agent.lastAction}
          </p>
        </div>

        {/* Next action */}
        {strategy && (
          <div>
            <h3 className="text-white/50 text-[10px] font-semibold uppercase tracking-wider mb-1">Next Action</h3>
            <p className="text-cyan-400 text-xs leading-relaxed bg-white/5 rounded-lg p-3">
              {strategy.nextAction}
            </p>
          </div>
        )}
      </div>
    </div>
  );
}

function PanelStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="bg-white/5 rounded-lg p-2">
      <div className="text-white/40 text-[10px]">{label}</div>
      <div className="text-white font-mono text-xs font-bold truncate">{value}</div>
    </div>
  );
}