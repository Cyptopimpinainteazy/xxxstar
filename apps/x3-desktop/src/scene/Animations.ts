import * as THREE from 'three';
import { useFrame } from '@react-three/fiber';
import { useRef } from 'react';

/**
 * Animation utility hook — returns delta-scaled sine oscillator for use in
 * agent idle animations (breathing, floating).
 */
export function useFloatAnimation(speed = 1.0, amplitude = 0.1): number {
  const t = useRef(0);
  useFrame((_, delta) => {
    t.current += delta * speed;
  });
  return Math.sin(t.current) * amplitude;
}

/**
 * Pulse animation for selection glow.
 */
export function usePulseAnimation(speed = 4.0): number {
  const t = useRef(0);
  useFrame((_, delta) => {
    t.current += delta * speed;
  });
  return 0.5 + 0.5 * Math.sin(t.current);
}

/**
 * Ease-in-out utility for manual tweening.
 */
export function easeInOutCubic(t: number): number {
  return t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
}

/**
 * Linear interpolation.
 */
export function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}