import React, { useState, useCallback } from 'react';
import { SceneManager } from './scene/SceneManager';
import { AgentPanel } from './ui/AgentPanel';
import { Overlay } from './ui/Overlay';
import { useAgentStore } from './agents/AgentStore';
import { useBlockStore } from './blockchain/BlockStore';

export function App() {
  const [selectedAgentId, setSelectedAgentId] = useState<string | null>(null);
  const [showOverlay, setShowOverlay] = useState(true);
  const agents = useAgentStore((s) => s.agents);
  const blocks = useBlockStore((s) => s.recentBlocks);

  const handleAgentSelect = useCallback((id: string) => {
    setSelectedAgentId(id);
  }, []);

  const handleAgentDeselect = useCallback(() => {
    setSelectedAgentId(null);
  }, []);

  const selectedAgent = selectedAgentId
    ? agents.find((a) => a.id === selectedAgentId) ?? null
    : null;

  return (
    <div className="relative w-screen h-screen overflow-hidden bg-black">
      {/* Three.js Scene — full viewport */}
      <div className="absolute inset-0 z-0">
        <SceneManager
          selectedAgentId={selectedAgentId}
          onAgentSelect={handleAgentSelect}
          onAgentDeselect={handleAgentDeselect}
        />
      </div>

      {/* HUD Overlay — top bar */}
      {showOverlay && (
        <div className="absolute top-0 left-0 right-0 z-10 pointer-events-none">
          <Overlay
            agents={agents}
            blocks={blocks}
            onToggleOverlay={() => setShowOverlay(false)}
          />
        </div>
      )}

      {/* Agent Panel — side bar */}
      {selectedAgent && (
        <div className="absolute top-0 right-0 z-20 h-full pointer-events-none">
          <AgentPanel
            agent={selectedAgent}
            onClose={handleAgentDeselect}
          />
        </div>
      )}

      {/* HUD toggle hint */}
      {!showOverlay && (
        <button
          className="absolute top-4 left-4 z-50 px-3 py-1.5 text-xs bg-white/10 hover:bg-white/20 text-white/70 rounded border border-white/10"
          onClick={() => setShowOverlay(true)}
        >
          Show HUD
        </button>
      )}
    </div>
  );
}