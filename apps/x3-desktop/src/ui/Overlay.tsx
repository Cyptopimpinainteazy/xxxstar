import React from 'react';
import { AgentState } from '../agents/AgentStore';
import { BlockEntry } from '../blockchain/BlockStore';
import { toggleAIController } from '../agents/AIController';

interface OverlayProps {
  agents: AgentState[];
  blocks: BlockEntry[];
  onToggleOverlay: () => void;
}

/**
 * HUD Overlay — top bar showing arena stats and controls.
 */
export function Overlay({ agents, blocks, onToggleOverlay }: OverlayProps) {
  const [aiRunning, setAiRunning] = React.useState(false);

  const handleToggleAI = () => {
    const running = toggleAIController();
    setAiRunning(running);
  };

  const totalPnl = agents.reduce((sum, a) => sum + a.pnl, 0);
  const avgHealth = agents.reduce((sum, a) => sum + a.health, 0) / agents.length;
  const activeAgents = agents.filter((a) => a.status !== 'idle').length;

  return (
    <div className="pointer-events-auto">
      {/* Main HUD bar */}
      <div className="flex items-center justify-between px-6 py-3 bg-black/50 backdrop-blur-md border-b border-white/10">
        {/* Left: Title */}
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-full bg-gradient-to-br from-cyan-400 to-blue-600 flex items-center justify-center text-white font-bold text-xs">
            X3
          </div>
          <div>
            <h1 className="text-white font-semibold text-sm tracking-wide">COMBAT ARENA</h1>
            <p className="text-white/40 text-[10px]">Agent Battle Simulator</p>
          </div>
        </div>

        {/* Center: Stats */}
        <div className="flex items-center gap-8 text-sm">
          <StatItem label="Agents" value={agents.length.toString()} color="text-cyan-400" />
          <StatItem label="Active" value={activeAgents.toString()} color="text-green-400" />
          <StatItem label="Blocks" value={blocks.length.toString()} color="text-yellow-400" />
          <StatItem label="Total PnL" value={`$${totalPnl.toLocaleString(undefined, { maximumFractionDigits: 0 })}`} color="text-white" />
          <StatItem label="Avg Health" value={`${avgHealth.toFixed(0)}%`} color={avgHealth > 60 ? 'text-green-400' : avgHealth > 30 ? 'text-yellow-400' : 'text-red-400'} />
        </div>

        {/* Right: Controls */}
        <div className="flex items-center gap-2">
          <button
            onClick={handleToggleAI}
            className={`px-4 py-1.5 rounded text-xs font-medium transition-all ${
              aiRunning
                ? 'bg-green-600/30 text-green-400 border border-green-500/30'
                : 'bg-white/10 text-white/70 border border-white/20 hover:bg-white/20'
            }`}
          >
            {aiRunning ? 'AI RUNNING' : 'START AI'}
          </button>
          <button
            onClick={onToggleOverlay}
            className="px-3 py-1.5 rounded text-xs bg-white/10 text-white/60 border border-white/10 hover:bg-white/20"
          >
            HIDE
          </button>
        </div>
      </div>

      {/* Block feed strip */}
      <div className="flex gap-2 px-6 py-2 bg-black/30 backdrop-blur-sm overflow-x-auto">
        {blocks.length === 0 && (
          <span className="text-white/30 text-xs italic">No blocks yet — start AI to generate activity</span>
        )}
        {blocks.slice(-10).map((block) => (
          <div
            key={block.id}
            className={`flex items-center gap-1 px-2 py-0.5 rounded text-[10px] font-mono ${
              block.status === 'confirmed'
                ? 'bg-green-900/30 text-green-400'
                : block.status === 'pending'
                  ? 'bg-yellow-900/30 text-yellow-400'
                  : 'bg-red-900/30 text-red-400'
            }`}
          >
            <span>#{block.height}</span>
            <span className="opacity-50">|</span>
            <span>{block.status}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function StatItem({ label, value, color }: { label: string; value: string; color: string }) {
  return (
    <div className="flex items-center gap-1.5">
      <span className="text-white/40 text-xs">{label}</span>
      <span className={`text-xs font-mono font-bold ${color}`}>{value}</span>
    </div>
  );
}