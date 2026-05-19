import { useEffect, useRef, useState } from 'react';
import { SceneManager } from '@/lib/three/SceneManager';

interface ThreeSceneProps {
  onSceneReady?: (scene: SceneManager) => void;
}

export function ThreeScene({ onSceneReady }: ThreeSceneProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<SceneManager | null>(null);
  const [scrollY, setScrollY] = useState(0);
  const animationIdRef = useRef<number>();

  useEffect(() => {
    if (!containerRef.current) return;

    // Initialize scene
    const scene = new SceneManager(containerRef.current);
    sceneRef.current = scene;
    onSceneReady?.(scene);

    // Animation loop
    const animate = () => {
      scene.update(scrollY);
      animationIdRef.current = requestAnimationFrame(animate);
    };
    animate();

    // Scroll listener
    const handleScroll = () => {
      setScrollY(window.scrollY);
    };
    window.addEventListener('scroll', handleScroll, false);

    // Cleanup
    return () => {
      window.removeEventListener('scroll', handleScroll);
      if (animationIdRef.current) {
        cancelAnimationFrame(animationIdRef.current);
      }
      scene.dispose();
    };
  }, [onSceneReady]);

  useEffect(() => {
    // Update scroll in scene
    if (sceneRef.current) {
      sceneRef.current.update(scrollY);
    }
  }, [scrollY]);

  return (
    <div
      ref={containerRef}
      className="fixed inset-0 w-full h-full pointer-events-none"
      style={{ zIndex: 0, background: '#050508' }}
    >
      {/* Metatron's Cube sacred geometry pattern */}
      <div
        className="absolute inset-0 pointer-events-none metatron-cube"
        style={{ zIndex: 0, opacity: 0.25 }}
      />
      {/* Spiral emanating from center */}
      <div
        className="absolute inset-0 pointer-events-none center-spiral"
        style={{ zIndex: 0 }}
      />
      {/* Cinematic vignette overlay */}
      <div
        className="absolute inset-0 pointer-events-none"
        style={{
          background: 'radial-gradient(ellipse at center, transparent 50%, rgba(5,5,8,0.5) 100%)',
          zIndex: 1,
        }}
      />
    </div>
  );
}
