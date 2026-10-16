'use client';

import React, { useEffect, useMemo, useState } from 'react';
import dynamic from 'next/dynamic';
import Image from 'next/image';
import Link from 'next/link';
import { AnimatePresence, motion, useScroll, useTransform } from 'framer-motion';

type NetworkStats = {
  chain: string;
  nodeName: string;
  nodeVersion: string;
  blockNumber: number;
  blockHash: string;
  timestamp: number;
  peerCount: number;
  isSyncing: boolean;
  authorityCount: number;
};

type ExtrinsicInfo = {
  hash: string;
  index: number;
  blockNumber: number;
  section: string;
  method: string;
  signer: string | null;
  success: boolean;
  timestamp: number;
};

type ValidatorInfo = {
  address: string;
  isActive: boolean;
};

type MarqueeItem = { icon: string; text: string; value: string };
type OrderRow = { price: string; amount: string; total: string; quantum?: boolean };
type OrderbookSnapshot = {
  pair: string;
  bids: OrderRow[];
  asks: OrderRow[];
  updatedAt: number;
  source: 'mock' | 'live';
};

const ValidatorGlobe = dynamic(() => import('@/components/quantum/ValidatorGlobe'), {
  ssr: false,
  loading: () => <GlobePlaceholder />,
});

const HolographicCardShowcase = dynamic(
  () => import('@atlas-sphere/shared/components/quantum-ui/HolographicCard').then(mod => mod.HolographicCardShowcase) as any,
  { ssr: false, loading: () => <SectionPlaceholder title="VALIDATOR NFT CARDS" /> }
);

function SectionPlaceholder({ title }: { title: string }) {
  return (
    <div className="rounded-2xl border border-white/10 bg-white/5 h-[320px] flex items-center justify-center backdrop-blur-sm">
      <div className="text-center">
        <div className="text-sm tracking-widest text-white/60">{title}</div>
        <div className="text-white/40 text-sm mt-2">Loading…</div>
      </div>
    </div>
  );
}

function GlobePlaceholder() {
  return (
    <div className="h-[520px] flex items-center justify-center bg-white/5">
      <div className="text-white/60">Loading validator globe…</div>
    </div>
  );
}

function shortHash(value: string, head: number = 10, tail: number = 6) {
  if (!value) return '—';
  if (value.length <= head + tail + 2) return value;
  return `${value.slice(0, head)}…${value.slice(-tail)}`;
}

function formatBlockNumber(n?: number) {
  if (typeof n !== 'number') return '—';
  return n.toLocaleString();
}

function stableScore(input: string) {
  let hash = 0;
  for (let i = 0; i < input.length; i++) hash = (hash * 31 + input.charCodeAt(i)) >>> 0;
  return 70 + (hash % 30); // 70..99
}

function useLiveNetworkStats(pollMs: number = 6000) {
  const [stats, setStats] = useState<NetworkStats | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const tick = async () => {
      try {
        const res = await fetch('/api/blockchain?type=stats', { cache: 'no-store' });
        if (!res.ok) throw new Error(`stats ${res.status}`);
        const json = (await res.json()) as NetworkStats;
        if (!isMounted) return;
        setStats(json);
        setError(null);
      } catch (e) {
        if (!isMounted) return;
        setError(e instanceof Error ? e.message : 'Failed to load stats');
      } finally {
        if (!isMounted) return;
        timer = setTimeout(tick, pollMs);
      }
    };

    tick();
    return () => {
      isMounted = false;
      if (timer) clearTimeout(timer);
    };
  }, [pollMs]);

  return { stats, error };
}

function useLiveExtrinsics(count: number = 20, pollMs: number = 4500, paused: boolean = false) {
  const [items, setItems] = useState<ExtrinsicInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const tick = async () => {
      if (paused) {
        timer = setTimeout(tick, pollMs);
        return;
      }
      try {
        const res = await fetch(`/api/blockchain?type=extrinsics&count=${count}`, { cache: 'no-store' });
        if (!res.ok) throw new Error(`extrinsics ${res.status}`);
        const json = (await res.json()) as ExtrinsicInfo[];
        if (!isMounted) return;
        setItems(Array.isArray(json) ? json : []);
        setError(null);
      } catch (e) {
        if (!isMounted) return;
        setError(e instanceof Error ? e.message : 'Failed to load extrinsics');
      } finally {
        if (!isMounted) return;
        timer = setTimeout(tick, pollMs);
      }
    };

    tick();
    return () => {
      isMounted = false;
      if (timer) clearTimeout(timer);
    };
  }, [count, pollMs, paused]);

  return { items, error };
}

function useLiveAuthorities(pollMs: number = 30000) {
  const [items, setItems] = useState<ValidatorInfo[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const tick = async () => {
      try {
        const res = await fetch('/api/blockchain?type=authorities', { cache: 'no-store' });
        if (!res.ok) throw new Error(`authorities ${res.status}`);
        const json = (await res.json()) as ValidatorInfo[];
        if (!isMounted) return;
        setItems(Array.isArray(json) ? json : []);
        setError(null);
      } catch (e) {
        if (!isMounted) return;
        setError(e instanceof Error ? e.message : 'Failed to load validators');
      } finally {
        if (!isMounted) return;
        timer = setTimeout(tick, pollMs);
      }
    };

    tick();
    return () => {
      isMounted = false;
      if (timer) clearTimeout(timer);
    };
  }, [pollMs]);

  return { items, error };
}

function useOrderbook(pair: string = 'ATLAS/USDC', pollMs: number = 2500, paused: boolean = false) {
  const [snapshot, setSnapshot] = useState<OrderbookSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const tick = async () => {
      if (paused) {
        timer = setTimeout(tick, pollMs);
        return;
      }
      try {
        const res = await fetch(`/api/orderbook?pair=${encodeURIComponent(pair)}`, { cache: 'no-store' });
        if (!res.ok) throw new Error(`orderbook ${res.status}`);
        const json = (await res.json()) as OrderbookSnapshot;
        if (!isMounted) return;
        setSnapshot(json);
        setError(null);
      } catch (e) {
        if (!isMounted) return;
        setError(e instanceof Error ? e.message : 'Failed to load orderbook');
      } finally {
        if (!isMounted) return;
        timer = setTimeout(tick, pollMs);
      }
    };

    tick();
    return () => {
      isMounted = false;
      if (timer) clearTimeout(timer);
    };
  }, [pair, pollMs, paused]);

  return { snapshot, error };
}

function LiveMarquee({ items }: { items: MarqueeItem[] }) {
  // Pure CSS marquee (lightweight, stable).
  const content = useMemo(() => [...items, ...items], [items]);

  return (
    <div className="relative overflow-hidden">
      <div className="absolute inset-0 bg-gradient-to-r from-black/60 via-transparent to-black/60 pointer-events-none" />
      <div className="flex whitespace-nowrap will-change-transform animate-[marquee_28s_linear_infinite]">
        {content.map((item, idx) => (
          <div key={idx} className="flex items-center gap-3 px-6 py-2 text-sm text-white/70">
            <span>{item.icon}</span>
            <span className="font-mono">{item.text}</span>
            <span className="text-white/40">{item.value}</span>
            <span className="mx-2 text-white/10">|</span>
          </div>
        ))}
      </div>
      <style jsx>{`
        @keyframes marquee {
          0% { transform: translateX(0); }
          100% { transform: translateX(-50%); }
        }
      `}</style>
    </div>
  );
}

function QuantumStatCard({ label, value, icon }: { label: string; value: string; icon: string }) {
  return (
    <div className="rounded-2xl border border-white/10 bg-white/5 p-4">
      <div className="flex items-center justify-between">
        <div className="text-xs tracking-widest text-white/50">{label}</div>
        <div className="text-lg">{icon}</div>
      </div>
      <div className="mt-3 text-2xl font-semibold tracking-tight text-white">{value}</div>
    </div>
  );
}

function LiveExplorerFeed({ paused }: { paused: boolean }) {
  const { items } = useLiveExtrinsics(24, 4500, paused);

  const rows = useMemo(() => {
    return items.map((x) => ({
      id: `${x.blockNumber}-${x.index}`,
      type: `${x.section}::${x.method}`.toUpperCase(),
      signer: x.signer,
      success: x.success,
      blockNumber: x.blockNumber,
    }));
  }, [items]);

  const iconFor = (type: string) => {
    if (type.includes('ATLASKERNEL')) return '⚛️';
    if (type.includes('BALANCES')) return '💸';
    if (type.includes('STAKING')) return '🔒';
    if (type.includes('UTILITY')) return '🧩';
    return '📄';
  };

  return (
    <div className="rounded-3xl border border-white/10 bg-white/5 backdrop-blur-xl overflow-hidden">
      <div className="p-6 border-b border-white/10 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <span className="text-2xl">🔍</span>
          <div>
            <div className="text-lg font-semibold">Live Explorer</div>
            <div className="text-xs text-white/50">Newest extrinsics (real chain data)</div>
          </div>
        </div>
        <div className="text-xs text-white/50">{paused ? 'PAUSED' : 'LIVE'}</div>
      </div>

      <div className="max-h-[520px] overflow-y-auto">
        <AnimatePresence mode="popLayout">
          {rows.map((row) => (
            <motion.div
              key={row.id}
              initial={{ opacity: 0, x: -30, height: 0 }}
              animate={{ opacity: 1, x: 0, height: 'auto' }}
              exit={{ opacity: 0, x: 30 }}
              className="flex items-center justify-between p-4 border-b border-white/5 hover:bg-white/5 transition-colors"
            >
              <div className="flex items-center gap-4">
                <div className="text-2xl">{iconFor(row.type)}</div>
                <div>
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-white/60 text-sm">#{row.blockNumber}</span>
                    <span className="px-2 py-0.5 bg-white/10 rounded text-white/70 text-xs font-mono">
                      {row.type}
                    </span>
                  </div>
                  <div className="mt-1 text-sm text-white/60">
                    Signer: <span className="font-mono text-white/80">{row.signer ? shortHash(row.signer) : '—'}</span>
                  </div>
                </div>
              </div>
              <div className="text-sm font-mono">
                {row.success ? <span className="text-emerald-300">🟢 SUCCESS</span> : <span className="text-rose-300">🔴 FAILED</span>}
              </div>
            </motion.div>
          ))}
        </AnimatePresence>
      </div>

      <div className="p-4 border-t border-white/10 bg-black/10 flex justify-around">
        <div className="text-center">
          <div className="font-semibold text-white">{rows.length}</div>
          <div className="text-xs text-white/50">IN VIEW</div>
        </div>
        <div className="text-center">
          <div className="font-semibold text-white">/api/blockchain</div>
          <div className="text-xs text-white/50">SOURCE</div>
        </div>
      </div>
    </div>
  );
}

function QuantumOrderBook({ paused }: { paused: boolean }) {
  const { snapshot } = useOrderbook('ATLAS/USDC', 2500, paused);
  const bids = snapshot?.bids ?? [];
  const asks = snapshot?.asks ?? [];
  const entanglement = snapshot?.source === 'live' ? 82 : 64;

  return (
    <div className="rounded-3xl border border-white/10 bg-white/5 backdrop-blur-xl overflow-hidden">
      <div className="p-6 border-b border-white/10 flex items-center gap-3">
        <span className="text-2xl">📈</span>
        <div>
          <div className="text-lg font-semibold">Quantum Orderbook</div>
          <div className="text-xs text-white/50">{snapshot?.pair ?? 'ATLAS/USDC'} • {snapshot?.source === 'live' ? 'LIVE' : 'MOCK'}</div>
        </div>
      </div>

      <div className="grid grid-cols-2 gap-4 p-4">
        <div>
          <div className="text-emerald-300 text-sm mb-2 uppercase tracking-widest">Bids</div>
          <div className="space-y-1">
            {bids.map((order, i) => (
              <div
                key={i}
                className={`flex justify-between p-2 rounded text-sm font-mono ${order.quantum ? 'bg-emerald-500/10 border-l-2 border-emerald-400' : 'bg-black/10'}`}
              >
                <span className="text-emerald-200">{order.price}</span>
                <span className="text-white/60">{order.amount}</span>
                <span className="text-white/40">{order.total}</span>
              </div>
            ))}
          </div>
        </div>

        <div>
          <div className="text-rose-300 text-sm mb-2 uppercase tracking-widest">Asks</div>
          <div className="space-y-1">
            {asks.map((order, i) => (
              <div
                key={i}
                className={`flex justify-between p-2 rounded text-sm font-mono ${order.quantum ? 'bg-rose-500/10 border-l-2 border-rose-400' : 'bg-black/10'}`}
              >
                <span className="text-rose-200">{order.price}</span>
                <span className="text-white/60">{order.amount}</span>
                <span className="text-white/40">{order.total}</span>
              </div>
            ))}
          </div>
        </div>
      </div>

      <div className="p-4 border-t border-white/10 bg-black/10">
        <div className="flex items-center justify-between">
          <span className="text-white/60 text-sm">Depth / signal</span>
          <div className="flex items-center gap-2">
            <div className="w-32 h-2 bg-white/10 rounded-full overflow-hidden">
              <div
                className="h-full bg-gradient-to-r from-emerald-400 via-sky-400 to-violet-400"
                style={{ width: `${entanglement}%` }}
              />
            </div>
            <span className="text-white/70 font-mono text-sm">{entanglement}%</span>
          </div>
        </div>
      </div>
    </div>
  );
}

function ValidatorLeaderboard({ blockNumber }: { blockNumber?: number }) {
  const { items } = useLiveAuthorities(30_000);

  const rows = useMemo(() => {
    const drift = typeof blockNumber === 'number' ? blockNumber % 7 : 0;
    return items
      .map((v) => {
        const base = stableScore(v.address);
        const score = Math.min(99, base + drift);
        return { address: v.address, score, status: v.isActive ? 'ACTIVE' : 'INACTIVE' };
      })
      .sort((a, b) => b.score - a.score)
      .slice(0, 10);
  }, [items, blockNumber]);

  const rankColor = (rank: number) => {
    if (rank === 1) return 'text-amber-300';
    if (rank === 2) return 'text-white/80';
    if (rank === 3) return 'text-orange-300';
    return 'text-white/60';
  };

  const scoreColor = (score: number) => {
    if (score >= 90) return 'text-emerald-300';
    if (score >= 70) return 'text-amber-300';
    return 'text-rose-300';
  };

  return (
    <div className="rounded-3xl border border-white/10 bg-white/5 backdrop-blur-xl overflow-hidden">
      <div className="p-6 border-b border-white/10 flex items-center gap-3">
        <span className="text-2xl">🏆</span>
        <div>
          <div className="text-lg font-semibold">Validator Leaderboard</div>
          <div className="text-xs text-white/50">On-chain authority set (scoring is placeholder until launch metrics)</div>
        </div>
      </div>

      <div className="p-4">
        <div className="grid grid-cols-6 gap-4 text-xs text-white/50 uppercase tracking-widest mb-2 px-2">
          <span>Rank</span>
          <span className="col-span-3">Validator</span>
          <span>Score</span>
          <span>Status</span>
        </div>

        <div className="space-y-2">
          {rows.map((v, i) => (
            <div
              key={v.address}
              className={`grid grid-cols-6 gap-4 p-3 rounded-xl items-center ${i === 0 ? 'bg-white/10 border border-white/20' : 'bg-black/10 hover:bg-white/5'} transition-colors`}
            >
              <span className={`font-semibold ${rankColor(i + 1)}`}>#{i + 1}</span>
              <span className="col-span-3 font-mono text-white">{shortHash(v.address, 10, 8)}</span>
              <div className="flex items-center gap-2">
                <div className="w-16 h-2 bg-white/10 rounded-full overflow-hidden">
                  <div
                    className={`h-full ${v.score >= 90 ? 'bg-emerald-400' : v.score >= 70 ? 'bg-amber-300' : 'bg-rose-400'}`}
                    style={{ width: `${v.score}%` }}
                  />
                </div>
                <span className={`font-mono text-sm ${scoreColor(v.score)}`}>{Math.round(v.score)}</span>
              </div>
              <span className={`text-sm ${v.status === 'ACTIVE' ? 'text-emerald-300' : 'text-amber-300'}`}>
                {v.status === 'ACTIVE' ? '🟢' : '🟡'} {v.status}
              </span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

function EcosystemTile({ title, desc, href }: { title: string; desc: string; href: string }) {
  return (
    <Link href={href} className="group rounded-2xl border border-white/10 bg-white/5 p-6 hover:bg-white/10 transition-colors">
      <div className="text-xs tracking-widest text-white/50">MODULE</div>
      <div className="text-lg font-semibold mt-2 group-hover:text-white">{title}</div>
      <div className="text-sm text-white/70 mt-2">{desc}</div>
      <div className="text-sm text-white/60 mt-4">Open →</div>
    </Link>
  );
}

export default function QuantumLandingPage() {
  const { scrollY } = useScroll();
  const heroOpacity = useTransform(scrollY, [0, 280], [1, 0.92]);
  const heroScale = useTransform(scrollY, [0, 280], [1, 0.98]);

  const [paused, setPaused] = useState(false);
  const { stats } = useLiveNetworkStats(6000);
  const { items: extrinsics } = useLiveExtrinsics(12, 4500, paused);

  const marqueeItems: MarqueeItem[] = useMemo(() => {
    if (!extrinsics.length) return [{ icon: '⛓️', text: 'Connecting to chain…', value: '—' }];
    return extrinsics.map((x) => ({
      icon: x.success ? '🟢' : '🔴',
      text: `${x.section}::${x.method} • #${x.blockNumber}`,
      value: x.signer ? shortHash(x.signer) : '—',
    }));
  }, [extrinsics]);

  const activeValidators = stats?.authorityCount ?? 0;
  const health = stats?.isSyncing ? 'SYNCING' : 'OK';

  return (
    <div className="min-h-screen bg-[#0b0d12] text-white">
      <header className="fixed top-0 left-0 right-0 z-50 border-b border-white/10 bg-black/40 backdrop-blur-xl">
        <div className="mx-auto max-w-7xl px-4 py-3 flex items-center gap-4">
          <Link href="/" className="font-semibold tracking-tight">Atlas Sphere</Link>
          <nav className="hidden md:flex items-center gap-5 text-sm text-white/70">
            <Link href="/explorer" className="hover:text-white">Explorer</Link>
            <Link href="/network" className="hover:text-white">Network</Link>
            <Link href="/swap" className="hover:text-white">Swap</Link>
            <Link href="/treasury" className="hover:text-white">Treasury</Link>
            <Link href="/ai-swarm" className="hover:text-white">GPU Swarm</Link>
            <Link href="/developers" className="hover:text-white">Developers</Link>
          </nav>
          <div className="ml-auto flex items-center gap-3 text-xs text-white/60">
            <span className="font-mono text-white/80">{stats?.chain ?? '—'}</span>
            <span className="text-white/20">•</span>
            <span className="font-mono text-white/80">#{formatBlockNumber(stats?.blockNumber)}</span>
          </div>
        </div>
        <div className="border-t border-white/5">
          <LiveMarquee items={marqueeItems} />
        </div>
      </header>

      <motion.section className="relative min-h-[85vh] flex items-center justify-center pt-28" style={{ opacity: heroOpacity, scale: heroScale }}>
        <div className="absolute inset-0">
          <div className="absolute inset-0 bg-gradient-to-b from-black/60 via-[#0b0d12] to-[#0b0d12]" />
          <div className="absolute inset-0 opacity-20">
            <Image
              alt="Atlas Sphere background"
              src="/images/branding/hero-landing-visual-ultra-wide-cinematic-shot-of-a-zero-gravity-cryp-heTp3mFmkc.png"
              fill
              className="object-cover"
              priority
            />
          </div>
        </div>

        <div className="relative z-10 px-4 max-w-5xl text-center">
          <motion.h1
            className="text-5xl md:text-7xl font-semibold tracking-tight mb-6"
            initial={{ opacity: 0, y: 40 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.8 }}
          >
            Neural Validator
          </motion.h1>

          <motion.p
            className="text-lg md:text-xl text-white/70 mx-auto mb-10"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ delay: 0.3 }}
          >
            Real-time chain telemetry, validators, and market surfaces.
            <span className="text-white/50"> Use the floating terminal (Ctrl/Cmd+K) to navigate or ask questions.</span>
          </motion.p>

          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <QuantumStatCard label="Active Validators" value={`${activeValidators || '—'}`} icon="🌐" />
            <QuantumStatCard label="Network" value={health} icon="💚" />
            <QuantumStatCard label="Block Height" value={formatBlockNumber(stats?.blockNumber)} icon="📦" />
            <QuantumStatCard label="Threat" value="LOW" icon="🛡️" />
          </div>

          <div className="mt-10 flex flex-col sm:flex-row gap-3 justify-center">
            <Link href="/network" className="px-8 py-3 bg-white text-black rounded-xl font-semibold">View Network</Link>
            <Link href="/explorer" className="px-8 py-3 border border-white/20 rounded-xl text-white/80 hover:bg-white/5">Open Explorer</Link>
            <button
              onClick={() => setPaused((p) => !p)}
              className="px-8 py-3 border border-white/20 rounded-xl text-white/80 hover:bg-white/5"
            >
              {paused ? 'Resume Live Feed' : 'Pause Live Feed'}
            </button>
          </div>

          <div className="mt-8 flex justify-center">
            <div className="rounded-2xl border border-white/10 bg-black/20 px-4 py-2 text-xs text-white/60">
              Node: <span className="font-mono text-white/80">{stats?.nodeName ?? '—'}</span>
              <span className="mx-2">•</span>
              Peers: <span className="font-mono text-white/80">{stats?.peerCount ?? '—'}</span>
              <span className="mx-2">•</span>
              Sync: <span className="font-mono text-white/80">{stats?.isSyncing ? 'YES' : 'NO'}</span>
            </div>
          </div>
        </div>
      </motion.section>

      <section className="relative px-4">
        <div className="max-w-7xl mx-auto">
          <div className="relative overflow-hidden rounded-3xl border border-white/10 bg-white/5">
            <div className="absolute inset-0 opacity-40">
              <Image
                alt="Experience the future"
                src="/images/branding/152195cd-2b22-4577-a46d-a1fbdec99cac.jpg"
                fill
                className="object-cover"
              />
            </div>
            <div className="relative p-8 md:p-10">
              <div className="flex flex-col md:flex-row md:items-center md:justify-between gap-6">
                <div>
                  <div className="text-xs tracking-widest text-white/60">EXPERIENCE</div>
                  <h2 className="text-2xl md:text-3xl font-semibold tracking-tight mt-2">The Future of Execution: EVM + SVM</h2>
                  <p className="text-white/70 mt-2 max-w-2xl">Cross-VM atomic transactions, validator visibility, and a GPU swarm built for AI workloads.</p>
                </div>
                <div className="flex gap-3">
                  <Link href="/x3" className="px-5 py-2 rounded-xl bg-white text-black font-semibold">X3</Link>
                  <Link href="/ai-swarm" className="px-5 py-2 rounded-xl border border-white/20 text-white/80 hover:bg-white/5">GPU Swarm</Link>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section className="relative py-16 px-4">
        <div className="max-w-7xl mx-auto">
          <h2 className="text-3xl md:text-4xl font-semibold tracking-tight text-center mb-10">Live Validator Pulse</h2>
          <div className="relative rounded-3xl border border-white/10 overflow-hidden">
            <ValidatorGlobe />
            <div className="absolute bottom-0 left-0 right-0 bg-gradient-to-t from-black via-black/70 to-transparent p-8">
              <div className="flex justify-around flex-wrap gap-6">
                <div className="text-center">
                  <div className="text-3xl font-semibold">{stats?.authorityCount ?? '—'}</div>
                  <div className="text-sm text-white/60 uppercase tracking-widest">Authorities</div>
                </div>
                <div className="text-center">
                  <div className="text-3xl font-semibold">6s</div>
                  <div className="text-sm text-white/60 uppercase tracking-widest">Block Time</div>
                </div>
                <div className="text-center">
                  <div className="text-3xl font-semibold">{stats?.peerCount ?? '—'}</div>
                  <div className="text-sm text-white/60 uppercase tracking-widest">Peers</div>
                </div>
                <div className="text-center">
                  <div className="text-3xl font-semibold">{stats?.isSyncing ? 'SYNC' : 'OK'}</div>
                  <div className="text-sm text-white/60 uppercase tracking-widest">Status</div>
                </div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section className="relative py-16 px-4">
        <div className="max-w-7xl mx-auto grid lg:grid-cols-2 gap-8">
          <LiveExplorerFeed paused={paused} />
          <div className="space-y-8">
            <QuantumOrderBook paused={paused} />
          </div>
        </div>
      </section>

      <section className="relative py-16 px-4">
        <div className="max-w-5xl mx-auto">
          <ValidatorLeaderboard blockNumber={stats?.blockNumber} />
        </div>
      </section>

      <section className="relative py-16 px-4">
        <div className="max-w-7xl mx-auto">
          <h2 className="text-3xl md:text-4xl font-semibold tracking-tight text-center mb-10">Quantum Ecosystem</h2>
          <div className="grid md:grid-cols-3 gap-4">
            <EcosystemTile title="Network" desc="Validators, health, and chain status" href="/network" />
            <EcosystemTile title="Explorer" desc="Blocks, extrinsics, accounts" href="/explorer" />
            <EcosystemTile title="Swap" desc="DEX surfaces and pools" href="/swap" />
            <EcosystemTile title="Treasury" desc="Protocol treasury + spending" href="/treasury" />
            <EcosystemTile title="GPU Swarm" desc="Distributed compute marketplace" href="/ai-swarm" />
            <EcosystemTile title="Developers" desc="Docs, APIs, and specs" href="/developers" />
          </div>
        </div>
      </section>

      <section className="relative py-16 px-4">
        <div className="max-w-7xl mx-auto">
          <h2 className="text-3xl md:text-4xl font-semibold tracking-tight text-center mb-10">Validator NFT Cards</h2>
          <HolographicCardShowcase />
        </div>
      </section>

      <section className="relative py-16 px-4">
        <div className="max-w-5xl mx-auto rounded-3xl border border-white/10 bg-white/5 p-10 text-center backdrop-blur-xl">
          <h2 className="text-3xl md:text-4xl font-semibold tracking-tight mb-4">Launch-Ready Surfaces</h2>
          <p className="text-base md:text-lg text-white/70 mb-6">
            This page now only keeps what you called out as “cool”. Next wiring work is plugging orderbook + threat feed + GPU swarm into live backends.
          </p>
          <div className="flex flex-col sm:flex-row gap-3 justify-center">
            <Link href="/developers" className="px-8 py-3 bg-white text-black rounded-xl font-semibold">Developer Docs</Link>
            <Link href="/network" className="px-8 py-3 border border-white/20 rounded-xl text-white/80 hover:bg-white/5">Network</Link>
          </div>
        </div>
      </section>

      <footer className="border-t border-white/10 py-10 px-4">
        <div className="max-w-7xl mx-auto flex flex-col md:flex-row justify-between items-center gap-4">
          <div className="text-sm text-white/60">Atlas Sphere • Neural Validator</div>
          <div className="flex gap-6 text-sm text-white/60">
            <Link href="/privacy" className="hover:text-white">Privacy</Link>
            <Link href="/terms" className="hover:text-white">Terms</Link>
            <Link href="/developers" className="hover:text-white">Developers</Link>
            <Link href="/ecosystem" className="hover:text-white">Ecosystem</Link>
          </div>
        </div>
      </footer>
    </div>
  );
}
