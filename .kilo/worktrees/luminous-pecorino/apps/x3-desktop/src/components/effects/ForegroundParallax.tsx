/**
 * ForegroundParallax.tsx — Cursor-reactive foreground vignette
 * 
 * Creates depth illusion by slightly moving/scaling the dark vignette
 * in response to cursor position.
 */
import { useEffect, useRef } from 'react';

export function ForegroundParallax() {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    // Get reference to foreground element
    // Pyramid stays fixed - only foreground moves for depth effect

    // Set initial state
    el.style.transform = 'translate(0px, 0px) scale(1.1)';

    const handleMouseMove = (e: MouseEvent) => {
      // Normalized cursor position (-1 to 1)
      const x = (e.clientX / window.innerWidth - 0.5) * 2;
      const y = (e.clientY / window.innerHeight - 0.5) * 2;

      // Foreground vignette moves OPPOSITE to cursor (creates depth illusion)
      // This dark frame shifts so pyramid appears to be in background
      const fgOffsetX = x * -60;
      const fgOffsetY = y * -60;

      // Scale change based on cursor distance from center
      const dist = Math.sqrt(x * x + y * y);
      const scale = 1.1 + dist * 0.08;

      el.style.transform = `translate(${fgOffsetX}px, ${fgOffsetY}px) scale(${scale})`;
      
      // Pyramid stays fixed - only foreground moves
    };

    const handleMouseLeave = () => {
      el.style.transform = 'translate(0px, 0px) scale(1.1)';
    };

    window.addEventListener('mousemove', handleMouseMove, { passive: true });
    window.addEventListener('mouseleave', handleMouseLeave);

    return () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseleave', handleMouseLeave);
    };
  }, []);

  return (
    <div
      ref={ref}
      className="scene-foreground"
      aria-hidden="true"
      style={{ transform: 'translate(0px, 0px) scale(1.1)' }}
    />
  );
}
