'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';

const xdexRoutes = [
  { href: '/polkadex/trading', label: '📊 Trading' },
  { href: '/polkadex/advanced', label: '⚡ Advanced' },
  { href: '/polkadex/bots', label: '🤖 Bot Traders' },
  { href: '/polkadex/scanner', label: '🎯 Scanner' },
  { href: '/polkadex/analytics', label: '📈 Analytics' },
  { href: '/polkadex/launchpad', label: '🚀 Launchpad' },
  { href: '/polkadex/orders', label: '📋 Orders' },
  { href: '/polkadex/portfolio', label: '💼 Portfolio' },
  { href: '/polkadex/settings', label: '⚙️ Settings' },
];

export default function XDEXLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();

  return (
    <div className="min-h-screen bg-x3-black text-white">
      {/* Top Navigation */}
      <nav className="bg-x3-darker border-b border-x3-dark-gray sticky top-0 z-50">
        <div className="max-w-7xl mx-auto px-4 py-3 flex items-center gap-2 overflow-x-auto">
          <span className="font-bold text-x3-orange mr-4 text-lg">X-DEX</span>
          {xdexRoutes.map((route) => (
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

          {/* Network selector (persists to localStorage) */}
          <select
            defaultValue={typeof window !== 'undefined' ? (window.localStorage.getItem('x3_active_network') || (process.env.NODE_ENV === 'development' ? 'local' : 'testnet')) : 'testnet'}
            onChange={(e) => { if (typeof window !== 'undefined') { window.localStorage.setItem('x3_active_network', e.target.value); window.location.reload(); } }}
            className="ml-auto px-2 py-1 text-sm bg-x3-dark-gray text-gray-200 rounded"
            title="Select network"
          >
            <option value="local">Local</option>
            <option value="testnet">Testnet</option>
            <option value="mainnet">Mainnet</option>
          </select>
        </div>
      </nav>

      {/* Content */}
      <main className="max-w-7xl mx-auto">{children}</main>
    </div>
  );
}
