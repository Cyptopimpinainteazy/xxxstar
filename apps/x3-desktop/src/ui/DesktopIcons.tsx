import { useState } from 'react';

export type PanelTab =
  | 'arena'
  | 'validators'
  | 'network'
  | 'swarm'
  | 'supply'
  | 'crossvm'
  | 'phase5'
  | 'foundry'
  | 'wallet'
  | 'swap'
  | 'intelligence'
  | 'explorer';

const DESKTOP_ICONS: { name: string; label: string; tab?: PanelTab; url?: string; icon: string }[] = [
  { name: 'wallet', label: 'Wallet', tab: 'wallet', icon: '💼' },
  { name: 'swap', label: 'Swap', tab: 'swap', icon: '🔄' },
  { name: 'bridge', label: 'Bridge', tab: 'crossvm', icon: '🌉' },
  { name: 'validators', label: 'Validators', tab: 'validators', icon: '🌐' },
  { name: 'governance', label: 'Governance', tab: 'phase5', icon: '🏛️' },
  { name: 'network', label: 'Network', tab: 'network', icon: '🔗' },
  { name: 'supply', label: 'Supply', tab: 'supply', icon: '💰' },
  { name: 'swarm', label: 'AI Swarm', tab: 'swarm', icon: '🐝' },
  { name: 'foundry', label: 'Foundry', tab: 'foundry', icon: '⚒️' },
  { name: 'arena', label: 'Arena', tab: 'arena', icon: '🎮' },
  { name: 'intelligence', label: 'Intelligence', tab: 'intelligence', icon: '🧠' },
  { name: 'infra', label: 'Infra', tab: 'foundry', icon: '🔧' },
  { name: 'explorer', label: 'Explorer', tab: 'explorer', icon: '🔍' },
  { name: 'metrics', label: 'Metrics', tab: 'network', icon: '📊' },
];

interface DesktopIconsProps {
  onNavigate?: (tab: PanelTab) => void;
}

export default function DesktopIcons({ onNavigate }: DesktopIconsProps) {
  const [hovered, setHovered] = useState<string | null>(null);

  const handleClick = (ic: typeof DESKTOP_ICONS[0]) => {
    if (ic.tab && onNavigate) {
      onNavigate(ic.tab);
    } else if (ic.url) {
      window.open(ic.url, '_blank');
    }
  };

  return (
    <div className="fixed bottom-0 left-0 right-0 z-40 flex justify-center pb-3 pointer-events-none">
      <div
        className="flex items-end gap-1 px-4 py-2 rounded-2xl pointer-events-auto"
        style={{
          background: 'rgba(10, 10, 30, 0.6)',
          backdropFilter: 'blur(16px)',
          WebkitBackdropFilter: 'blur(16px)',
          border: '1px solid rgba(255,255,255,0.08)',
        }}
      >
        {DESKTOP_ICONS.map((ic) => (
          <button
            key={ic.name}
            className="flex flex-col items-center gap-1 px-2 py-2 transition-all duration-200 rounded-xl min-w-[64px]"
            style={{
              background:
                hovered === ic.name
                  ? 'rgba(255,255,255,0.08)'
                  : 'transparent',
              transform:
                hovered === ic.name
                  ? 'translateY(-4px) scale(1.05)'
                  : 'translateY(0) scale(1)',
            }}
            onMouseEnter={() => setHovered(ic.name)}
            onMouseLeave={() => setHovered(null)}
            onClick={() => handleClick(ic)}
            title={ic.label}
          >
            <span className="w-8 h-8 flex items-center justify-center text-xl drop-shadow-lg"
              style={{
                filter:
                  hovered === ic.name
                    ? 'brightness(1.3) drop-shadow(0 0 6px rgba(0,200,255,0.5))'
                    : 'brightness(0.9)',
              }}
            >
              {ic.icon}
            </span>
            <span className="text-[10px] text-white/80 font-medium leading-tight text-center drop-shadow-lg">
              {ic.label}
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}
