/**
 * Tooltip — a simple hover tooltip that follows the element position.
 */
import React from "react";

export interface TooltipProps {
  content: string;
  children: React.ReactNode;
  position?: "top" | "bottom" | "left" | "right";
}

const POSITION_CLASSES = {
  top: "bottom-full left-1/2 -translate-x-1/2 mb-2",
  bottom: "top-full left-1/2 -translate-x-1/2 mt-2",
  left: "right-full top-1/2 -translate-y-1/2 mr-2",
  right: "left-full top-1/2 -translate-y-1/2 ml-2",
};

const Tooltip: React.FC<TooltipProps> = ({
  content,
  children,
  position = "top",
}) => {
  return (
    <div className="relative group inline-block">
      {children}
      <div
        className={`absolute ${POSITION_CLASSES[position]} glass-panel rounded-md
          px-2 py-1 text-[10px] text-text-primary whitespace-nowrap
          opacity-0 group-hover:opacity-100 transition-opacity duration-200
          pointer-events-none z-50`}
        role="tooltip"
      >
        {content}
      </div>
    </div>
  );
};

export default Tooltip;
