/**
 * ApplicationIcon — renders a single app icon with hover effects,
 * running indicator, click handlers, and resize functionality.
 *
 * Single-click → tooltip. Double-click → launch application.
 * Drag bottom-right corner → resize icon.
 */
import React, { useCallback, useState, useRef } from "react";
import type { Application, ApplicationCategory } from "@/types/application";
import { CATEGORY_COLORS, CATEGORY_LABELS } from "@/types/application";

export interface ApplicationIconProps {
  /** Application manifest */
  app: Application;
  /** Whether the application is currently running */
  isRunning: boolean;
  /** Called on double-click to launch */
  onLaunch: (appId: string) => void;
  /** Icon display size */
  size: "small" | "medium" | "large";
  /** Called when icon is resized */
  onResize?: (appId: string, newSize: "small" | "medium" | "large") => void;
}

const SIZE_MAP = {
  small: { icon: 48, text: "text-[10px]", gap: "gap-1" },
  medium: { icon: 64, text: "text-xs", gap: "gap-1.5" },
  large: { icon: 96, text: "text-sm", gap: "gap-2" },
} as const;

/**
 * Generate a placeholder icon with category indicator and accent colour.
 */
function PlaceholderIcon({
  category,
  color,
  size,
}: {
  category: ApplicationCategory;
  color?: string;
  size: number;
}) {
  const bg = color ?? CATEGORY_COLORS[category];
  const emoji = CATEGORY_LABELS[category];

  return (
    <div
      className="rounded-xl flex items-center justify-center"
      style={{
        width: size,
        height: size,
        background: `linear-gradient(135deg, ${bg}33 0%, ${bg}18 100%)`,
        border: `4px solid ${bg}77`,
        boxShadow: `0 0 15px ${bg}44, inset 0 0 15px ${bg}22`,
      }}
    >
      <span style={{ fontSize: size * 0.4 }}>{emoji}</span>
    </div>
  );
}

const ApplicationIcon: React.FC<ApplicationIconProps> = ({
  app,
  isRunning,
  onLaunch,
  size,
  onResize,
}) => {
  const [showTooltip, setShowTooltip] = useState(false);
  const [isResizing, setIsResizing] = useState(false);
  const [tooltipPosition, setTooltipPosition] = useState({ x: 0, y: 0 });
  const resizeRef = useRef<{ startX: number; startY: number; startSize: number } | null>(null);
  const iconRef = useRef<HTMLDivElement>(null);
  const dims = SIZE_MAP[size];

  const handleClick = useCallback(() => {
    // Single click → toggle tooltip
    if (iconRef.current) {
      const rect = iconRef.current.getBoundingClientRect();
      setTooltipPosition({
        x: rect.left + rect.width / 2,
        y: rect.bottom + 8,
      });
    }
    setShowTooltip((v) => !v);
  }, []);

  const handleDoubleClick = useCallback(() => {
    // Double click → launch
    setShowTooltip(false); // Hide tooltip when launching
    onLaunch(app.id);
  }, [app.id, onLaunch]);

  const handleResizeStart = useCallback(
    (e: React.MouseEvent) => {
      if (!onResize) return;
      e.preventDefault();
      e.stopPropagation();
      setIsResizing(true);

      const sizeIndex = { small: 0, medium: 1, large: 2 }[size];
      resizeRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        startSize: sizeIndex,
      };

      const handleMouseMove = (ev: MouseEvent) => {
        if (!resizeRef.current) return;
        const deltaX = ev.clientX - resizeRef.current.startX;
        const deltaY = ev.clientY - resizeRef.current.startY;
        const delta = Math.max(deltaX, deltaY); // Use the larger delta for size change
        const sizeStep = Math.floor(delta / 30); // Change size every 30px of drag
        const newSizeIndex = Math.max(0, Math.min(2, resizeRef.current.startSize + sizeStep));
        const newSizes: Array<"small" | "medium" | "large"> = ["small", "medium", "large"];
        const newSize = newSizes[newSizeIndex];
        if (newSize !== size) {
          onResize(app.id, newSize);
        }
      };

      const handleMouseUp = () => {
        setIsResizing(false);
        resizeRef.current = null;
        document.removeEventListener("mousemove", handleMouseMove);
        document.removeEventListener("mouseup", handleMouseUp);
      };

      document.addEventListener("mousemove", handleMouseMove);
      document.addEventListener("mouseup", handleMouseUp);
    },
    [app.id, onResize, size],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        onLaunch(app.id);
      }
    },
    [app.id, onLaunch],
  );

  return (
    <div
      ref={iconRef}
      className={`relative flex flex-col items-center ${dims.gap} cursor-pointer
        transition-transform duration-150 hover:scale-110 focus-visible:scale-110
        group`}
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
      onKeyDown={handleKeyDown}
      onMouseLeave={() => setShowTooltip(false)}
      role="button"
      tabIndex={0}
      aria-label={`${app.name}${isRunning ? " (running)" : ""}`}
    >
      {/* Icon */}
      {app.icon.type === "file" && app.icon.path ? (
        <img
          src={app.icon.path}
          alt={app.name}
          className="rounded-xl object-contain"
          style={{ 
            width: dims.icon, 
            height: dims.icon, 
            border: `4px solid ${app.icon.color ?? '#1a9fb5'}77`,
            boxShadow: `0 0 15px ${app.icon.color ?? '#1a9fb5'}44, inset 0 0 15px ${app.icon.color ?? '#1a9fb5'}22`
          }}
          draggable={false}
        />
      ) : (
        <PlaceholderIcon
          category={app.category}
          color={app.icon.color}
          size={dims.icon}
        />
      )}

      {/* Enhanced Glow on hover */}
      <div
        className="absolute inset-0 rounded-xl opacity-0 group-hover:opacity-100
          transition-all duration-300 pointer-events-none"
        style={{
          boxShadow: `0 0 25px ${app.icon.color ?? '#1a9fb5'}88, 0 0 50px ${app.icon.color ?? '#1a9fb5'}44, 0 0 75px ${app.icon.color ?? '#1a9fb5'}22`,
          filter: 'blur(1px)',
        }}
      />

      {/* Resize handle */}
      {onResize && (
        <div
          className={`absolute bottom-0 right-0 w-3 h-3 cursor-se-resize opacity-0 group-hover:opacity-100
            transition-opacity duration-200 ${isResizing ? 'opacity-100' : ''}`}
          onMouseDown={handleResizeStart}
          role="separator"
          aria-label="Resize icon"
        >
          <svg
            className="absolute bottom-0.5 right-0.5"
            width="6"
            height="6"
            viewBox="0 0 6 6"
          >
            <path d="M5 1L1 5M5 3L3 5M5 5L5 5" stroke="#888" strokeWidth="1" strokeLinecap="round" />
          </svg>
        </div>
      )}

      {/* Running indicator */}
      {isRunning && <span className="running-dot" />}

      {/* Label */}
      <span
        className={`${dims.text} text-text-secondary text-center truncate max-w-[80px]
          group-hover:text-text-accent transition-colors`}
      >
        {app.name}
      </span>

      {/* Tooltip */}
      {showTooltip && app.description && (
        <div
          className="fixed glass-panel rounded-lg px-3 py-1.5 text-xs text-text-primary whitespace-nowrap z-[100] animate-fade-in pointer-events-none"
          style={{
            left: tooltipPosition.x,
            top: tooltipPosition.y,
            transform: 'translateX(-50%)',
          }}
        >
          {app.description}
          {isRunning && (
            <span className="ml-2 text-accent-primary font-medium">
              Running
            </span>
          )}
        </div>
      )}
    </div>
  );
};

export default React.memo(ApplicationIcon);
