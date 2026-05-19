/**
 * useCursorTracker — lightweight cursor position state (screenX, screenY).
 *
 * This is a simpler alternative to useEyeballTracking for non-3D components
 * that just need to know where the cursor is.
 */
import { useEffect, useState } from "react";

export interface CursorState {
  x: number;
  y: number;
  ndcX: number;
  ndcY: number;
  isInViewport: boolean;
}

export function useCursorTracker(): CursorState {
  const [state, setState] = useState<CursorState>({
    x: 0,
    y: 0,
    ndcX: 0,
    ndcY: 0,
    isInViewport: false,
  });

  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      setState({
        x: e.clientX,
        y: e.clientY,
        ndcX: (2 * e.clientX) / window.innerWidth - 1,
        ndcY: 1 - (2 * e.clientY) / window.innerHeight,
        isInViewport: true,
      });
    };

    const onLeave = () => {
      setState((s) => ({ ...s, isInViewport: false }));
    };

    document.addEventListener("mousemove", onMove, { passive: true });
    document.addEventListener("mouseleave", onLeave);

    return () => {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseleave", onLeave);
    };
  }, []);

  return state;
}
