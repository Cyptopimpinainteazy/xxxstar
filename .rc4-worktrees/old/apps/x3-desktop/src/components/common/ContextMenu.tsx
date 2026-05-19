/**
 * ContextMenu — right-click context menu for the desktop.
 *
 * Shows desktop operations: settings, theme toggle, layout change, refresh.
 */
import React, { useEffect, useRef } from "react";

export interface ContextMenuItem {
  label: string;
  icon?: string;
  shortcut?: string;
  action: () => void;
  divider?: boolean;
  disabled?: boolean;
}

export interface ContextMenuProps {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
}

const ContextMenu: React.FC<ContextMenuProps> = ({ x, y, items, onClose }) => {
  const ref = useRef<HTMLDivElement>(null);

  // Close on click outside or Escape
  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };

    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [onClose]);

  // Ensure menu stays within viewport
  const style: React.CSSProperties = {
    left: Math.min(x, window.innerWidth - 200),
    top: Math.min(y, window.innerHeight - items.length * 36 - 16),
  };

  return (
    <div
      ref={ref}
      className="fixed glass-panel rounded-lg py-1 min-w-[180px] shadow-window
        animate-fade-in z-[99999]"
      style={style}
      role="menu"
    >
      {items.map((item, i) => (
        <React.Fragment key={i}>
          {item.divider && <div className="h-px bg-border-default my-1 mx-2" />}
          <button
            className={`flex items-center gap-2 w-full px-3 py-1.5 text-xs text-left
              transition-colors rounded-sm
              ${
                item.disabled
                  ? "text-text-secondary/40 cursor-not-allowed"
                  : "text-text-primary hover:bg-white/8 hover:text-text-accent"
              }`}
            onClick={() => {
              if (!item.disabled) {
                item.action();
                onClose();
              }
            }}
            disabled={item.disabled}
            role="menuitem"
          >
            {item.icon && <span className="w-4 text-center">{item.icon}</span>}
            <span className="flex-1">{item.label}</span>
            {item.shortcut && (
              <span className="text-text-secondary/50 text-[10px] ml-3">
                {item.shortcut}
              </span>
            )}
          </button>
        </React.Fragment>
      ))}
    </div>
  );
};

export default ContextMenu;
