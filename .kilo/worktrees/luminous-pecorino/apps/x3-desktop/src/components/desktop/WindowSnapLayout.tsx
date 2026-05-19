// Window snap layout system for tiling windows
// Supports: 2x2 grid, 1 large + 2 small, fullscreen

import React, { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export enum SnapLayout {
  TwoByTwo = 'two-by-two',      // 2x2 equal grid
  OneAndTwo = 'one-and-two',    // 1 large (left) + 2 small (right)
  FullScreen = 'fullscreen',
  None = 'none',
}

interface SnapGridConfig {
  columns: number;
  rows: number;
  gaps: number; // px
  padding: number; // px
}

interface WindowPosition {
  x: number;
  y: number;
  width: number;
  height: number;
  gridX?: number;
  gridY?: number;
  gridSpanX?: number;
  gridSpanY?: number;
}

interface SnapState {
  activeLayout: SnapLayout;
  windows: Map<string, WindowPosition>;
  gridConfig: SnapGridConfig;
  screenWidth: number;
  screenHeight: number;
}

export const WindowSnapLayout: React.FC = () => {
  const [state, setState] = useState<SnapState>({
    activeLayout: SnapLayout.None,
    windows: new Map(),
    gridConfig: {
      columns: 2,
      rows: 2,
      gaps: 12,
      padding: 16,
    },
    screenWidth: window.innerWidth,
    screenHeight: window.innerHeight,
  });

  const [hoveredEdge, setHoveredEdge] = useState<string | null>(null);

  // Detect screen resize
  useEffect(() => {
    const handleResize = () => {
      setState(prev => ({
        ...prev,
        screenWidth: window.innerWidth,
        screenHeight: window.innerHeight,
      }));
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  // Detect edge drag for snap
  useEffect(() => {
    const handleMouseMove = (e: MouseEvent) => {
      const edge = detectScreenEdge(e.clientX, e.clientY);
      setHoveredEdge(edge);

      // Edge snap triggers at 50px from screen edge
      if (e.clientX < 50 || e.clientX > state.screenWidth - 50 ||
          e.clientY < 50 || e.clientY > state.screenHeight - 50) {
        // Snap indicator would show here
      }
    };

    window.addEventListener('mousemove', handleMouseMove);
    return () => window.removeEventListener('mousemove', handleMouseMove);
  }, [state.screenWidth, state.screenHeight]);

  const detectScreenEdge = (x: number, y: number): string | null => {
    const threshold = 50;
    if (x < threshold) return 'left';
    if (x > state.screenWidth - threshold) return 'right';
    if (y < threshold) return 'top';
    if (y > state.screenHeight - threshold) return 'bottom';
    return null;
  };

  // Calculate grid positions for given layout
  const calculateLayoutPositions = (layout: SnapLayout): Map<string, WindowPosition> => {
    const positions = new Map<string, WindowPosition>();
    const cfg = state.gridConfig;
    const usableWidth = state.screenWidth - (cfg.padding * 2);
    const usableHeight = state.screenHeight - (cfg.padding * 2);

    if (layout === SnapLayout.TwoByTwo) {
      const cellWidth = (usableWidth - cfg.gaps) / 2;
      const cellHeight = (usableHeight - cfg.gaps) / 2;

      const cells = [
        { x: 0, y: 0 }, { x: 1, y: 0 },
        { x: 0, y: 1 }, { x: 1, y: 1 },
      ];

      cells.forEach((cell, idx) => {
        positions.set(`window-${idx}`, {
          x: cfg.padding + (cell.x * (cellWidth + cfg.gaps)),
          y: cfg.padding + (cell.y * (cellHeight + cfg.gaps)),
          width: cellWidth,
          height: cellHeight,
          gridX: cell.x,
          gridY: cell.y,
          gridSpanX: 1,
          gridSpanY: 1,
        });
      });
    }

    if (layout === SnapLayout.OneAndTwo) {
      const leftWidth = usableWidth * 0.6;
      const rightWidth = usableWidth - leftWidth - cfg.gaps;
      const cellHeight = (usableHeight - cfg.gaps) / 2;

      // Left window (large)
      positions.set('window-0', {
        x: cfg.padding,
        y: cfg.padding,
        width: leftWidth,
        height: usableHeight,
        gridSpanX: 2,
        gridSpanY: 2,
      });

      // Right windows (small)
      positions.set('window-1', {
        x: cfg.padding + leftWidth + cfg.gaps,
        y: cfg.padding,
        width: rightWidth,
        height: cellHeight,
        gridSpanX: 1,
        gridSpanY: 1,
      });

      positions.set('window-2', {
        x: cfg.padding + leftWidth + cfg.gaps,
        y: cfg.padding + cellHeight + cfg.gaps,
        width: rightWidth,
        height: cellHeight,
        gridSpanX: 1,
        gridSpanY: 1,
      });
    }

    if (layout === SnapLayout.FullScreen) {
      positions.set('window-0', {
        x: cfg.padding,
        y: cfg.padding,
        width: usableWidth,
        height: usableHeight,
        gridSpanX: 2,
        gridSpanY: 2,
      });
    }

    return positions;
  };

  // Apply layout to windows
  const applyLayout = async (layout: SnapLayout) => {
    try {
      const positions = calculateLayoutPositions(layout);
      
      // Send to Tauri backend
      const results = Array.from(positions.entries()).map(([windowId, pos]) => ({
        window_id: windowId,
        x: Math.floor(pos.x),
        y: Math.floor(pos.y),
        width: Math.floor(pos.width),
        height: Math.floor(pos.height),
      }));

      await invoke('apply_window_snap_layout', { windows: results });

      setState(prev => ({
        ...prev,
        activeLayout: layout,
        windows: positions,
      }));
    } catch (error) {
      console.error('Failed to apply snap layout:', error);
    }
  };

  // Keyboard shortcuts for layouts
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey) {
        if (e.key === '1') applyLayout(SnapLayout.TwoByTwo);
        if (e.key === '2') applyLayout(SnapLayout.OneAndTwo);
        if (e.key === 'F') applyLayout(SnapLayout.FullScreen);
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  return (
    <div className="window-snap-layout-control">
      <div className="layout-buttons">
        <button
          className={`snap-btn ${state.activeLayout === SnapLayout.TwoByTwo ? 'active' : ''}`}
          onClick={() => applyLayout(SnapLayout.TwoByTwo)}
          title="Ctrl+Shift+1"
        >
          2×2 Grid
        </button>
        <button
          className={`snap-btn ${state.activeLayout === SnapLayout.OneAndTwo ? 'active' : ''}`}
          onClick={() => applyLayout(SnapLayout.OneAndTwo)}
          title="Ctrl+Shift+2"
        >
          1+2
        </button>
        <button
          className={`snap-btn ${state.activeLayout === SnapLayout.FullScreen ? 'active' : ''}`}
          onClick={() => applyLayout(SnapLayout.FullScreen)}
          title="Ctrl+Shift+F"
        >
          Fullscreen
        </button>
      </div>

      <div className={`snap-grid-preview ${state.activeLayout}`}>
        {Array.from(state.windows.entries()).map(([id, pos]) => (
          <div
            key={id}
            className="snap-window-preview"
            style={{
              left: `${(pos.x / state.screenWidth) * 100}%`,
              top: `${(pos.y / state.screenHeight) * 100}%`,
              width: `${(pos.width / state.screenWidth) * 100}%`,
              height: `${(pos.height / state.screenHeight) * 100}%`,
            }}
          >
            {id}
          </div>
        ))}
      </div>

      <div className="layout-hints">
        <p>💡 Drag window to edge to snap • Ctrl+Shift+1 for 2×2 • Ctrl+Shift+2 for 1+2</p>
      </div>
    </div>
  );
};

export default WindowSnapLayout;
