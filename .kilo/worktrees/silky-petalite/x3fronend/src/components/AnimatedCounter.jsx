import { useState, useEffect, useRef } from 'react';

/**
 * Animated number counter component with smooth transitions
 * @param {number} value - Target value to animate to
 * @param {number} duration - Animation duration in milliseconds (default: 2000)
 * @param {string} prefix - Prefix to display before the number (e.g., "$")
 * @param {string} suffix - Suffix to display after the number (e.g., "%")
 */
export default function AnimatedCounter({ value, duration = 2000, prefix = '', suffix = '' }) {
  const [displayValue, setDisplayValue] = useState(0);
  const animationRef = useRef(null);
  const previousValueRef = useRef(0);

  useEffect(() => {
    const start = previousValueRef.current;
    const end = value;
    const startTime = performance.now();

    // PATTERN: Use requestAnimationFrame for smooth 60fps animation
    const animate = (currentTime) => {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);

      // Easing function for smooth animation (ease-out cubic)
      const eased = 1 - Math.pow(1 - progress, 3);
      const currentValue = Math.floor(start + (end - start) * eased);
      
      setDisplayValue(currentValue);

      if (progress < 1) {
        animationRef.current = requestAnimationFrame(animate);
      } else {
        // Animation complete, update previous value reference
        previousValueRef.current = end;
      }
    };

    // Cancel any existing animation
    if (animationRef.current) {
      cancelAnimationFrame(animationRef.current);
    }

    animationRef.current = requestAnimationFrame(animate);

    // CRITICAL: Cleanup animation on unmount or value change
    return () => {
      if (animationRef.current) {
        cancelAnimationFrame(animationRef.current);
      }
    };
  }, [value, duration]);

  // Format number with commas for readability
  const formatNumber = (num) => {
    if (num >= 1000000000) {
      return (num / 1000000000).toFixed(2) + 'B';
    }
    if (num >= 1000000) {
      return (num / 1000000).toFixed(2) + 'M';
    }
    if (num >= 1000) {
      return num.toLocaleString();
    }
    return num.toString();
  };

  return (
    <span className="animated-counter">
      {prefix}
      <span className="counter-value">{formatNumber(displayValue)}</span>
      {suffix}
    </span>
  );
}