import React from "react";

export const PanelLoading: React.FC<{ label?: string }> = ({ label = "Gathering telemetry…" }) => (
  <div className="h-full flex items-center justify-center bg-[#05050c] text-[#999] text-[11px] font-mono">
    <div className="flex flex-col items-center gap-2">
      <div className="inline-block w-7 h-7 border-2 border-[#1a9fb5]/30 border-t-[#1a9fb5] rounded-full animate-spin" />
      <span>{label}</span>
    </div>
  </div>
);

export const PanelError: React.FC<{ message: string }> = ({ message }) => (
  <div className="h-full flex items-center justify-center bg-[#05050c] text-[#2ab4cc] text-[11px] font-mono">
    <div className="flex flex-col items-center gap-1">
      <div className="text-lg">⚠</div>
      <p className="text-center max-w-xs text--[#add9e8]">Could not load panel data: <span className="font-mono block">{message}</span></p>
    </div>
  </div>
);
