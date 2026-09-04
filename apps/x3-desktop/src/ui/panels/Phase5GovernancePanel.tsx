/**
 * Phase 5 Governance Panel — absorbed from `phase5_panel/`.
 *
 * Connects directly to md_supervisor WebSocket at ws://localhost:8765
 * for real-time AST node heatmap, rollback events, and node votes.
 * Fallback stub data when supervisor is unreachable.
 */
import { useEffect, useState, useCallback, useRef } from 'react';

interface ASTNodeData {
  nodeId: string;
  pnl: number;
  votes?: Record<string, boolean>;
  runtime?: unknown;
  merged?: boolean;
}

type ConnectionState = 'connecting' | 'connected' | 'disconnected';

function Phase5GovernancePanel() {
  const [nodes, setNodes] = useState<ASTNodeData[]>([]);
  const [connState, setConnState] = useState<ConnectionState>('connecting');
  const wsRef = useRef<WebSocket | null>(null);

  useEffect(() => {
    let socket: WebSocket | null = null;
    let reconnectTimer: ReturnType<typeof setTimeout> | null = null;

    const connect = () => {
      try {
        socket = new WebSocket('ws://localhost:8765');
        wsRef.current = socket;
        setConnState('connecting');

        socket.onopen = () => {
          setConnState('connected');
        };

        socket.onclose = () => {
          setConnState('disconnected');
          wsRef.current = null;
          reconnectTimer = setTimeout(connect, 3000);
        };

        socket.onerror = () => {
          setConnState('disconnected');
        };

        socket.onmessage = (event: MessageEvent) => {
          try {
            const msg = JSON.parse(event.data);
            if (msg.type === 'autopilot_update' || msg.type === 'replay_update' || msg.type === 'rollback_update') {
              setNodes((prev) => {
                const filtered = prev.filter((n) => n.nodeId !== msg.payload.nodeId);
                return [...filtered, msg.payload as ASTNodeData];
              });
            }
          } catch {
            // ignore malformed messages
          }
        };
      } catch {
        setConnState('disconnected');
        reconnectTimer = setTimeout(connect, 3000);
      }
    };

    connect();

    return () => {
      if (reconnectTimer) clearTimeout(reconnectTimer);
      if (socket) {
        socket.onopen = null;
        socket.onclose = null;
        socket.close();
      }
    };
  }, []);

  const getColor = useCallback((pnl: number): string => {
    if (pnl >= 2) return 'bg-green-600';
    if (pnl >= 0) return 'bg-green-400/70';
    if (pnl >= -1) return 'bg-yellow-500';
    return 'bg-red-500';
  }, []);

  const mergedCount = nodes.filter((n) => n.merged === true).length;
  const blockedCount = nodes.filter((n) => n.merged === false).length;

  return (
    <div className="view p-6">
      <div className="flex items-center justify-between mb-4">
        <div>
          <h2 className="text-xl font-bold text-white">Phase 5 Governance</h2>
          <p className="text-gray-400 text-sm">AST node heatmap + rollback events from md_supervisor</p>
        </div>
        <div className="flex items-center gap-2">
          <div
            className={`w-2 h-2 rounded-full ${
              connState === 'connected'
                ? 'bg-green-400'
                : connState === 'connecting'
                  ? 'bg-yellow-400 animate-pulse'
                  : 'bg-red-400'
            }`}
          />
          <span
            className={`text-xs font-mono ${
              connState === 'connected'
                ? 'text-green-400'
                : connState === 'connecting'
                  ? 'text-yellow-400'
                  : 'text-red-400'
            }`}
          >
            {connState === 'connected' ? 'LIVE' : connState === 'connecting' ? 'CONNECTING' : 'OFFLINE'}
          </span>
          <span className="text-gray-600 text-xs">ws://localhost:8765</span>
        </div>
      </div>

      <div className="grid grid-cols-3 gap-3 mb-6">
        <div className="bg-gray-800/40 rounded-lg p-3 border border-gray-700/50">
          <div className="text-gray-500 text-xs">Total Nodes</div>
          <div className="text-white font-mono text-xl">{nodes.length}</div>
        </div>
        <div className="bg-gray-800/40 rounded-lg p-3 border border-green-700/30">
          <div className="text-gray-500 text-xs">Merged</div>
          <div className="text-green-400 font-mono text-xl">{mergedCount}</div>
        </div>
        <div className="bg-gray-800/40 rounded-lg p-3 border border-red-700/30">
          <div className="text-gray-500 text-xs">Blocked</div>
          <div className="text-red-400 font-mono text-xl">{blockedCount}</div>
        </div>
      </div>

      <div className="max-h-96 overflow-y-auto">
        {nodes.length === 0 ? (
          <div className="text-center py-12 text-gray-500">
            Waiting for data...<br />
            <span className="text-xs">Start md_supervisor on :8765 to populate.</span>
          </div>
        ) : (
          nodes.map((node) => (
            <div
              key={node.nodeId}
              className={`flex items-center justify-between px-3 py-2 my-1 rounded-md ${getColor(node.pnl)} text-gray-900 text-sm font-medium`}
            >
              <span className="font-mono text-xs">{node.nodeId}</span>
              <div className="flex items-center gap-4">
                <span className="font-mono">PnL: {node.pnl.toFixed(2)}</span>
                {node.merged !== undefined && (
                  <span className="text-xs font-bold">
                    {node.merged ? '✅ Merged' : '⛔ Blocked'}
                  </span>
                )}
              </div>
            </div>
          ))
        )}
      </div>

      <div className="mt-3 text-xs text-gray-600">
        WebSocket: ws://localhost:8765 (md_supervisor) — realtime autopilot/replay/rollback events
      </div>
    </div>
  );
}

export default Phase5GovernancePanel;