/**
 * Modal — a generic overlay dialog with backdrop blur, close button,
 * and keyboard escape support.
 */
import React, { useEffect, useRef } from "react";

export interface ModalProps {
  /** Whether the modal is visible */
  open: boolean;
  /** Called when the modal requests close */
  onClose: () => void;
  /** Dialog title */
  title: string;
  /** Content */
  children: React.ReactNode;
  /** Optional width override (default: 400px) */
  width?: number;
}

const Modal: React.FC<ModalProps> = ({
  open,
  onClose,
  title,
  children,
  width = 400,
}) => {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [open, onClose]);

  // Focus trap
  useEffect(() => {
    if (open && ref.current) {
      ref.current.focus();
    }
  }, [open]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[99998] flex items-center justify-center"
      onClick={onClose}
    >
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/60 backdrop-blur-sm" />

      {/* Dialog */}
      <div
        ref={ref}
        className="relative glass-panel rounded-xl shadow-window animate-fade-in
          flex flex-col max-h-[80vh]"
        style={{ width }}
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        tabIndex={-1}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-3 border-b border-border-default">
          <h2 className="text-sm font-semibold text-text-primary">{title}</h2>
          <button
            className="title-bar-btn hover:bg-red-500/20"
            onClick={onClose}
            aria-label="Close dialog"
          >
            <svg width="10" height="10" viewBox="0 0 10 10">
              <path
                d="M1 1L9 9M9 1L1 9"
                stroke="#a8a8a8"
                strokeWidth="1.5"
                strokeLinecap="round"
              />
            </svg>
          </button>
        </div>

        {/* Body */}
        <div className="px-5 py-4 overflow-y-auto text-sm text-text-primary">
          {children}
        </div>
      </div>
    </div>
  );
};

export default Modal;
