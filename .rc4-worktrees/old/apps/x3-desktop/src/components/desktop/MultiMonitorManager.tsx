// Multi-monitor support: detect displays, lock windows to monitors, span across displays

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize, PhysicalPosition } from '@tauri-apps/api/window';

interface Monitor {
  id: number;
  name: string;
  width: number;
  height: number;
  x: number;
  y: number;
  scale_factor: number;
  is_primary: boolean;
}

interface WindowConfiguration {
  window_id: string;
  locked_to_monitor?: number;
  span_monitors?: number[];
  position_mode: 'locked' | 'floating' | 'spanned';
}

interface MultiMonitorState {
  monitors: Monitor[];
  configurations: Map<string, WindowConfiguration>;
  activeWindow: string | null;
  detectionStatus: 'scanning' | 'ready' | 'error';
}

export const MultiMonitorManager: React.FC = () => {
  const [state, setState] = useState<MultiMonitorState>({
    monitors: [],
    configurations: new Map(),
    activeWindow: null,
    detectionStatus: 'scanning',
  });

  const [selectedMonitor, setSelectedMonitor] = useState<number | null>(null);
  const [windowPreview, setWindowPreview] = useState<boolean>(false);

  // Detect available monitors
  useEffect(() => {
    const detectMonitors = async () => {
      try {
        const monitors = await invoke<Monitor[]>('get_monitors');
        setState(prev => ({
          ...prev,
          monitors: monitors.sort((a: Monitor, b: Monitor) => a.x - b.x),
          detectionStatus: 'ready',
        }));
      } catch (error) {
        console.error('Failed to detect monitors:', error);
        setState(prev => ({
          ...prev,
          detectionStatus: 'error',
        }));
      }
    };

    detectMonitors();

    // Redetect on physical display change (e.g., via docking)
    const detectionInterval = setInterval(detectMonitors, 5000);
    return () => clearInterval(detectionInterval);
  }, []);

  // Move window to specific monitor
  const moveWindowToMonitor = async (windowId: string, monitorId: number) => {
    try {
      const monitor = state.monitors.find(m => m.id === monitorId);
      if (!monitor) return;

      // Center window on monitor
      const x = monitor.x + (monitor.width - 800) / 2;
      const y = monitor.y + (monitor.height - 600) / 2;

      const targetWindow = getCurrentWindow();
      await targetWindow.setPosition(new PhysicalPosition(x, y));

      // Lock to this monitor
      setState(prev => {
        const newConfigs = new Map(prev.configurations);
        newConfigs.set(windowId, {
          window_id: windowId,
          locked_to_monitor: monitorId,
          position_mode: 'locked',
        });
        return { ...prev, configurations: newConfigs };
      });

      // Persist configuration
      await invoke('save_window_monitor_config', {
        window_id: windowId,
        monitor_id: monitorId,
      });
    } catch (error) {
      console.error('Failed to move window:', error);
    }
  };

  // Span window across multiple monitors
  const spanWindowAcrossMonitors = async (windowId: string, monitorIds: number[]) => {
    try {
      if (monitorIds.length < 2) return;

      // Calculate combined bounds
      const selectedMonitors = state.monitors.filter(m =>
        monitorIds.includes(m.id)
      );

      const minX = Math.min(...selectedMonitors.map(m => m.x));
      const maxX = Math.max(...selectedMonitors.map(m => m.x + m.width));
      const minY = Math.min(...selectedMonitors.map(m => m.y));
      const maxY = Math.max(...selectedMonitors.map(m => m.y + m.height));

      const spans = {
        x: minX,
        y: minY,
        width: maxX - minX,
        height: maxY - minY,
      };

      const targetWindow = getCurrentWindow();
      await targetWindow.setPosition(new PhysicalPosition(spans.x, spans.y));
      await targetWindow.setSize(
        new LogicalSize(spans.width, spans.height)
      );

      // Update configuration
      setState(prev => {
        const newConfigs = new Map(prev.configurations);
        newConfigs.set(windowId, {
          window_id: windowId,
          span_monitors: monitorIds,
          position_mode: 'spanned',
        });
        return { ...prev, configurations: newConfigs };
      });

      await invoke('save_window_monitor_config', {
        window_id: windowId,
        span_monitors: monitorIds,
      });
    } catch (error) {
      console.error('Failed to span window:', error);
    }
  };

  // Restore window to monitor if previously locked
  const restoreWindowPosition = async (windowId: string) => {
    try {
      const config = state.configurations.get(windowId);
      if (!config || !config.locked_to_monitor) return;

      await moveWindowToMonitor(windowId, config.locked_to_monitor);
    } catch (error) {
      console.error('Failed to restore window position:', error);
    }
  };

  // Handle monitor disconnection
  const handleMonitorDisconnect = async (monitorId: number) => {
    // Move any windows locked to this monitor to primary monitor
    const primaryMonitor = state.monitors.find(m => m.is_primary);
    if (!primaryMonitor) return;

    const affectedWindows = Array.from(state.configurations.entries())
      .filter(([_, config]) => config.locked_to_monitor === monitorId)
      .map(([windowId]) => windowId);

    for (const windowId of affectedWindows) {
      await moveWindowToMonitor(windowId, primaryMonitor.id);
    }
  };

  return (
    <div className="multi-monitor-manager">
      <h3>Multi-Monitor Configuration</h3>

      {/* Status */}
      <div className={`detection-status ${state.detectionStatus}`}>
        {state.detectionStatus === 'scanning' && (
          <span>🔍 Scanning for displays...</span>
        )}
        {state.detectionStatus === 'ready' && (
          <span>✅ {state.monitors.length} display(s) detected</span>
        )}
        {state.detectionStatus === 'error' && (
          <span>❌ Failed to detect displays</span>
        )}
      </div>

      {/* Monitor Grid Visualization */}
      {state.detectionStatus === 'ready' && (
        <div className="monitor-grid-container">
          <div className="monitor-visualization">
            {state.monitors.map(monitor => (
              <div
                key={monitor.id}
                className={`monitor ${monitor.is_primary ? 'primary' : ''} ${
                  selectedMonitor === monitor.id ? 'selected' : ''
                }`}
                style={{
                  width: `${(monitor.width / 1920) * 100}%`,
                  aspectRatio: `${monitor.width} / ${monitor.height}`,
                  order: monitor.id,
                }}
                onClick={() => setSelectedMonitor(monitor.id)}
                title={`${monitor.name} (${monitor.width}×${monitor.height})`}
              >
                <div className="monitor-label">
                  {monitor.is_primary ? '⭐ ' : ''}{monitor.name}
                </div>
                <div className="monitor-resolution">
                  {monitor.width}×{monitor.height}
                </div>
                <div className="monitor-scale">
                  {(monitor.scale_factor * 100).toFixed(0)}%
                </div>
              </div>
            ))}
          </div>

          <div className="monitor-info">
            <h4>Display Information</h4>
            <div className="monitor-list">
              {state.monitors.map(monitor => (
                <div key={monitor.id} className="monitor-item">
                  <div className="monitor-header">
                    <strong>{monitor.name}</strong>
                    {monitor.is_primary && <span className="primary-badge">Primary</span>}
                  </div>
                  <div className="monitor-details">
                    <p>Resolution: {monitor.width} × {monitor.height}</p>
                    <p>Position: ({monitor.x}, {monitor.y})</p>
                    <p>Scale: {(monitor.scale_factor * 100).toFixed(0)}%</p>
                    <div className="monitor-buttons">
                      <button
                        onClick={() => moveWindowToMonitor('active-window', monitor.id)}
                        className="btn-move"
                      >
                        Move to This Monitor
                      </button>
                      <button
                        onClick={() => setSelectedMonitor(monitor.id)}
                        className="btn-select"
                      >
                        Select
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Multi-Monitor Spanning */}
      {selectedMonitor !== null && (
        <div className="span-controls">
          <h4>Window Spanning</h4>
          <div className="span-options">
            <label>
              <input
                type="checkbox"
                onChange={e => {
                  if (e.target.checked) {
                    setWindowPreview(true);
                  }
                }}
              />
              Preview span
            </label>
            <button
              onClick={() => {
                const monitorIds = state.monitors
                  .filter(m => m.x >= (state.monitors.find(m => m.id === selectedMonitor)?.x || 0))
                  .map(m => m.id)
                  .slice(0, 2);

                if (monitorIds.length >= 2) {
                  spanWindowAcrossMonitors('active-window', monitorIds);
                }
              }}
              className="btn-span"
            >
              Span 2 Monitors
            </button>
            <button
              onClick={() => {
                const allMonitorIds = state.monitors.map(m => m.id);
                if (allMonitorIds.length > 1) {
                  spanWindowAcrossMonitors('active-window', allMonitorIds);
                }
              }}
              className="btn-span-all"
            >
              Span All Monitors
            </button>
          </div>
        </div>
      )}

      {/* Configuration Summary */}
      {state.configurations.size > 0 && (
        <div className="config-summary">
          <h4>Active Configurations</h4>
          <ul>
            {Array.from(state.configurations.entries()).map(([windowId, config]) => (
              <li key={windowId}>
                <strong>{windowId}</strong>:{' '}
                {config.position_mode === 'locked' && (
                  `Locked to ${state.monitors.find(m => m.id === config.locked_to_monitor)?.name || 'Unknown'}`
                )}
                {config.position_mode === 'spanned' && (
                  `Spanning ${config.span_monitors?.length || 0} monitors`
                )}
              </li>
            ))}
          </ul>
        </div>
      )}

      <style>{`
        .multi-monitor-manager {
          padding: 20px;
          background: #f5f5f5;
          border-radius: 8px;
        }

        .detection-status {
          padding: 10px 15px;
          border-radius: 4px;
          margin-bottom: 20px;
          font-size: 14px;
          font-weight: 500;
        }

        .detection-status.ready {
          background: #d4edda;
          color: #155724;
        }

        .detection-status.scanning {
          background: #fff3cd;
          color: #856404;
        }

        .detection-status.error {
          background: #f8d7da;
          color: #721c24;
        }

        .monitor-grid-container {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 20px;
          margin-bottom: 20px;
        }

        .monitor-visualization {
          display: flex;
          gap: 10px;
          padding: 20px;
          background: white;
          border-radius: 8px;
          overflow-x: auto;
        }

        .monitor {
          flex-shrink: 0;
          border: 3px solid #ccc;
          border-radius: 8px;
          padding: 15px;
          background: #fafafa;
          cursor: pointer;
          transition: all 0.3s ease;
          display: flex;
          flex-direction: column;
          justify-content: space-between;
          align-items: center;
        }

        .monitor.selected {
          border-color: #0066cc;
          background: #e6f2ff;
        }

        .monitor.primary {
          border: 3px solid #28a745;
        }

        .monitor-label {
          font-weight: 600;
          font-size: 12px;
          text-align: center;
        }

        .monitor-resolution {
          font-size: 11px;
          color: #666;
        }

        .monitor-scale {
          font-size: 10px;
          color: #999;
        }

        .monitor-info {
          background: white;
          border-radius: 8px;
          padding: 15px;
          max-height: 400px;
          overflow-y: auto;
        }

        .monitor-list {
          display: flex;
          flex-direction: column;
          gap: 12px;
        }

        .monitor-item {
          border: 1px solid #ddd;
          border-radius: 6px;
          padding: 12px;
          background: #fafafa;
        }

        .monitor-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 8px;
        }

        .primary-badge {
          font-size: 11px;
          background: #28a745;
          color: white;
          padding: 3px 8px;
          border-radius: 3px;
        }

        .monitor-details p {
          margin: 4px 0;
          font-size: 12px;
          color: #666;
        }

        .monitor-buttons {
          display: flex;
          gap: 8px;
          margin-top: 10px;
        }

        .monitor-buttons button {
          flex: 1;
          padding: 6px 12px;
          font-size: 12px;
          border: 1px solid #ddd;
          border-radius: 4px;
          cursor: pointer;
          background: white;
          transition: all 0.2s;
        }

        .monitor-buttons button:hover {
          background: #f0f0f0;
          border-color: #999;
        }

        .span-controls {
          background: white;
          border-radius: 8px;
          padding: 15px;
          margin-bottom: 20px;
        }

        .span-options {
          display: flex;
          gap: 10px;
          align-items: center;
        }

        .span-options label {
          display: flex;
          align-items: center;
          gap: 8px;
          font-size: 14px;
          cursor: pointer;
        }

        .btn-span,
        .btn-span-all {
          padding: 8px 16px;
          background: #0066cc;
          color: white;
          border: none;
          border-radius: 4px;
          cursor: pointer;
          font-size: 14px;
          transition: all 0.2s;
        }

        .btn-span:hover,
        .btn-span-all:hover {
          background: #0052a3;
        }

        .config-summary {
          background: white;
          border-radius: 8px;
          padding: 15px;
        }

        .config-summary ul {
          list-style: none;
          padding: 0;
          margin: 0;
        }

        .config-summary li {
          padding: 8px 0;
          font-size: 13px;
          border-bottom: 1px solid #eee;
        }

        .config-summary li:last-child {
          border-bottom: none;
        }
      `}</style>
    </div>
  );
};

export default MultiMonitorManager;
