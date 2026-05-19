/**
 * PerformanceMonitor.tsx — Real-time FPS and frame time monitor
 * 
 * Press 'Ctrl+P' to toggle visibility
 * Shows:
 *  - Current FPS
 *  - Frame time (ms)
 *  - Memory usage (if available)
 *  - GPU info
 *  - Toggle buttons to disable features for testing
 *  - Mode switch (Normal / Exclusive)
 */
import { useEffect, useRef, useState } from 'react';
import { useAppMode } from '@/contexts/AppModeContext';

interface PerfStats {
  fps: number;
  frameTime: number;
  memory?: number;
  gpuInfo?: string;
}

// Global flags for toggling features (accessible from console)
declare global {
  interface Window {
    __PERF_FLAGS__: {
      disableSceneManager: boolean;
      disableMetatron: boolean;
      disableSpiral: boolean;
      disableBloom: boolean;
      disablePyramidGlow: boolean;
      disableForeground: boolean;
      disablePyramid: boolean;
    };
  }
}

window.__PERF_FLAGS__ = window.__PERF_FLAGS__ || {
  disableSceneManager: false,
  disableMetatron: false,
  disableSpiral: false,
  disableBloom: false,
  disablePyramidGlow: false,
  disableForeground: false,
  disablePyramid: false,
};

export function PerformanceMonitor() {
  const [visible, setVisible] = useState(false);
  const [stats, setStats] = useState<PerfStats>({ fps: 0, frameTime: 0 });
  const [flags, setFlags] = useState({ ...window.__PERF_FLAGS__ });
  const [baseline, setBaseline] = useState<number | null>(null);
  const { setMode, isExclusive } = useAppMode();
  const frameTimesRef = useRef<number[]>([]);
  const lastTimeRef = useRef(performance.now());
  const frameCountRef = useRef(0);

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() === 'p' && e.ctrlKey) {
        e.preventDefault();
        setVisible(v => !v);
      }
    };
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  }, []);

  // Apply CSS toggles
  useEffect(() => {
    const metatron = document.querySelector('.metatron-cube') as HTMLElement;
    const spiral = document.querySelector('.center-spiral') as HTMLElement;
    // Find the SceneManager's canvas (inside ThreeScene container)
    const threeContainer = document.querySelector('[class*="fixed"][class*="inset-0"]')?.querySelector('canvas') as HTMLCanvasElement;
    
    if (metatron) {
      metatron.style.display = flags.disableMetatron ? 'none' : '';
    }
    if (spiral) {
      spiral.style.display = flags.disableSpiral ? 'none' : '';
    }
    if (threeContainer) {
      threeContainer.style.visibility = flags.disableSceneManager ? 'hidden' : '';
    }
    
    // Toggle pyramid glow
    const pyramidGlow = document.querySelector('.pyramid-glow') as HTMLElement;
    if (pyramidGlow) {
      pyramidGlow.style.display = flags.disablePyramidGlow ? 'none' : '';
    }
    
    // Toggle foreground vignette
    const foreground = document.querySelector('.scene-foreground') as HTMLElement;
    if (foreground) {
      foreground.style.display = flags.disableForeground ? 'none' : '';
    }
    
    // Toggle pyramid layers
    const pyramidBg = document.querySelector('.pyramid-bg') as HTMLElement;
    const pyramidShadow = document.querySelector('.pyramid-shadow') as HTMLElement;
    if (pyramidBg) {
      pyramidBg.style.display = flags.disablePyramid ? 'none' : '';
    }
    if (pyramidShadow) {
      pyramidShadow.style.display = flags.disablePyramid ? 'none' : '';
    }
    
    window.__PERF_FLAGS__ = flags;
  }, [flags]);

  useEffect(() => {
    let running = true;
    let gpuInfo = 'Unknown';

    // Try to get GPU info
    try {
      const canvas = document.createElement('canvas');
      const gl = canvas.getContext('webgl') || canvas.getContext('experimental-webgl');
      if (gl) {
        const debugInfo = (gl as WebGLRenderingContext).getExtension('WEBGL_debug_renderer_info');
        if (debugInfo) {
          gpuInfo = (gl as WebGLRenderingContext).getParameter(debugInfo.UNMASKED_RENDERER_WEBGL);
        }
      }
    } catch (e) {
      // Ignore
    }

    const tick = () => {
      if (!running) return;

      const now = performance.now();
      const delta = now - lastTimeRef.current;
      lastTimeRef.current = now;
      frameCountRef.current++;

      // Track frame times for averaging
      frameTimesRef.current.push(delta);
      if (frameTimesRef.current.length > 60) {
        frameTimesRef.current.shift();
      }

      // Update stats every 500ms
      if (frameCountRef.current % 30 === 0) {
        const avgFrameTime = frameTimesRef.current.reduce((a, b) => a + b, 0) / frameTimesRef.current.length;
        const fps = 1000 / avgFrameTime;

        // Memory (Chrome only)
        const memory = (performance as any).memory?.usedJSHeapSize / 1024 / 1024;

        setStats({
          fps: Math.round(fps),
          frameTime: Math.round(avgFrameTime * 10) / 10,
          memory: memory ? Math.round(memory) : undefined,
          gpuInfo,
        });
      }

      requestAnimationFrame(tick);
    };

    requestAnimationFrame(tick);

    return () => {
      running = false;
    };
  }, []);

  const toggleFlag = (key: keyof typeof flags) => {
    setFlags(f => ({ ...f, [key]: !f[key] }));
  };

  if (!visible) return null;

  const fpsColor = stats.fps >= 55 ? '#00ff00' : stats.fps >= 30 ? '#ffff00' : '#ff0000';

  return (
    <div
      style={{
        position: 'fixed',
        top: 10,
        left: 10,
        background: 'rgba(0,0,0,0.9)',
        color: '#fff',
        padding: '12px 16px',
        borderRadius: 8,
        fontFamily: 'monospace',
        fontSize: 12,
        zIndex: 9999,
        minWidth: 240,
        border: '1px solid rgba(255,255,255,0.2)',
        pointerEvents: 'auto',
      }}
    >
      <div style={{ marginBottom: 8, fontWeight: 'bold', color: '#888' }}>
        Performance Monitor (P to hide)
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
        <span>FPS:</span>
        <span style={{ color: fpsColor, fontWeight: 'bold', fontSize: 16 }}>{stats.fps}</span>
      </div>
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
        <span>Frame Time:</span>
        <span>{stats.frameTime}ms</span>
      </div>
      {stats.memory && (
        <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
          <span>JS Heap:</span>
          <span>{stats.memory}MB</span>
        </div>
      )}
      <div style={{ marginTop: 8, fontSize: 10, color: '#666', wordBreak: 'break-all' }}>
        GPU: {stats.gpuInfo?.substring(0, 35)}
      </div>
      
      <div style={{ marginTop: 12, borderTop: '1px solid rgba(255,255,255,0.1)', paddingTop: 8 }}>
        {/* Mode Toggle */}
        <div style={{ marginBottom: 12 }}>
          <div style={{ color: '#888', marginBottom: 6 }}>Frontend Mode:</div>
          <div style={{ display: 'flex', gap: 8 }}>
            <button
              onClick={() => setMode('normal')}
              style={{
                flex: 1,
                padding: '6px 8px',
                borderRadius: 4,
                border: '1px solid',
                borderColor: !isExclusive ? '#00ff88' : '#444',
                background: !isExclusive ? 'rgba(0, 255, 136, 0.15)' : '#222',
                color: !isExclusive ? '#00ff88' : '#888',
                fontSize: 11,
                fontWeight: !isExclusive ? 'bold' : 'normal',
                cursor: 'pointer',
              }}
            >
              Normal
            </button>
            <button
              onClick={() => setMode('exclusive')}
              style={{
                flex: 1,
                padding: '6px 8px',
                borderRadius: 4,
                border: '1px solid',
                borderColor: isExclusive ? '#ffaa00' : '#444',
                background: isExclusive ? 'rgba(255, 170, 0, 0.15)' : '#222',
                color: isExclusive ? '#ffaa00' : '#888',
                fontSize: 11,
                fontWeight: isExclusive ? 'bold' : 'normal',
                cursor: 'pointer',
              }}
            >
              Exclusive
            </button>
          </div>
          <div style={{ fontSize: 9, color: '#555', marginTop: 4 }}>
            {isExclusive ? 'Pyramid + Metatron + Eye' : 'Eye only'}
          </div>
        </div>
        
        <div style={{ color: '#888', marginBottom: 8 }}>Toggle to find bottleneck:</div>
        
        <label style={{ display: 'flex', alignItems: 'center', marginBottom: 6, cursor: 'pointer' }}>
          <input 
            type="checkbox" 
            checked={flags.disableSceneManager}
            onChange={() => toggleFlag('disableSceneManager')}
            style={{ marginRight: 8 }}
          />
          <span style={{ color: flags.disableSceneManager ? '#ff6b6b' : '#fff' }}>
            Hide 3D Scene (hide canvas)
          </span>
        </label>
        
        <label style={{ display: 'flex', alignItems: 'center', marginBottom: 6, cursor: 'pointer' }}>
          <input 
            type="checkbox" 
            checked={flags.disableMetatron}
            onChange={() => toggleFlag('disableMetatron')}
            style={{ marginRight: 8 }}
          />
          <span style={{ color: flags.disableMetatron ? '#ff6b6b' : '#fff' }}>
            Disable Metatron Cube
          </span>
        </label>
        
        <label style={{ display: 'flex', alignItems: 'center', marginBottom: 6, cursor: 'pointer' }}>
          <input 
            type="checkbox" 
            checked={flags.disableSpiral}
            onChange={() => toggleFlag('disableSpiral')}
            style={{ marginRight: 8 }}
          />
          <span style={{ color: flags.disableSpiral ? '#ff6b6b' : '#fff' }}>
            Disable Spiral
          </span>
        </label>
        
        <label style={{ display: 'flex', alignItems: 'center', marginBottom: 6, cursor: 'pointer' }}>
          <input 
            type="checkbox" 
            checked={flags.disablePyramidGlow}
            onChange={() => toggleFlag('disablePyramidGlow')}
            style={{ marginRight: 8 }}
          />
          <span style={{ color: flags.disablePyramidGlow ? '#ff6b6b' : '#fff' }}>
            Disable Pyramid Glow
          </span>
        </label>
        
        <label style={{ display: 'flex', alignItems: 'center', marginBottom: 6, cursor: 'pointer' }}>
          <input 
            type="checkbox" 
            checked={flags.disableForeground}
            onChange={() => toggleFlag('disableForeground')}
            style={{ marginRight: 8 }}
          />
          <span style={{ color: flags.disableForeground ? '#ff6b6b' : '#fff' }}>
            Disable Foreground Vignette
          </span>
        </label>
        
        <label style={{ display: 'flex', alignItems: 'center', marginBottom: 6, cursor: 'pointer' }}>
          <input 
            type="checkbox" 
            checked={flags.disablePyramid}
            onChange={() => toggleFlag('disablePyramid')}
            style={{ marginRight: 8 }}
          />
          <span style={{ color: flags.disablePyramid ? '#ff6b6b' : '#fff' }}>
            Disable Pyramid
          </span>
        </label>
        
        <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
          <button
            onClick={() => setBaseline(stats.fps)}
            style={{
              background: '#333',
              border: '1px solid #555',
              color: '#fff',
              padding: '4px 8px',
              borderRadius: 4,
              cursor: 'pointer',
              fontSize: 10,
            }}
          >
            Save Baseline
          </button>
          {baseline && (
            <span style={{ fontSize: 10, color: '#aaa', alignSelf: 'center' }}>
              Baseline: {baseline} FPS {stats.fps > baseline ? `(+${stats.fps - baseline})` : stats.fps < baseline ? `(${stats.fps - baseline})` : ''}
            </span>
          )}
        </div>
        
        <div style={{ marginTop: 8, fontSize: 10, color: '#888' }}>
          SceneManager: 4000 stars, 1200 nebula,<br/>
          600 dust, 35 nodes + bloom post-proc
        </div>
      </div>
    </div>
  );
}
