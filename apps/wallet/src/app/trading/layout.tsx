'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';

const tradingRoutes = [
  { href: '/trading/floor', label: '📊 Floor' },
  { href: '/trading/intents', label: '⚙️ Intents' },
  { href: '/trading/agents', label: '👥 Agents' },
  { href: '/trading/slashing', label: '⚡ Slashing' },
  { href: '/trading/proofs', label: '✓ Proofs' },
  { href: '/trading/bonds', label: '💰 Bonds' },
  { href: '/trading/rules', label: '📜 Rules' },
  { href: '/trading/arbitrage', label: '🔄 Arb' },
  { href: '/trading/guide', label: '❓ Guide' },
  { href: '/trading/why', label: 'ℹ️ Why' },
];

export default function TradingLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();

  return (
    <div className="min-h-screen bg-x3-black text-white">
      {/* Top Navigation */}
      <nav className="bg-x3-darker border-b border-x3-dark-gray sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 py-3 flex items-center gap-2 overflow-x-auto">
          {tradingRoutes.map((route) => (
            <Link
              key={route.href}
              href={route.href}
              className={`px-3 py-2 text-sm font-medium whitespace-nowrap rounded transition-colors ${
                pathname === route.href
                  ? 'bg-x3-orange text-white'
                  : 'text-gray-300 hover:bg-x3-dark-gray'
              }`}
            >
              {route.label}
            </Link>
          ))}
        </div>
      </nav>

      {/* Content */}
      <main className="max-w-7xl mx-auto">{children}</main>
    </div>
  );
}
