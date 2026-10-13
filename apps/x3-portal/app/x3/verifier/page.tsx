'use client';

import { useState, useEffect } from 'react';
import Link from 'next/link';

interface VerifierStats {
  total_receipts: number;
  verified_receipts: number;
  disputed_receipts: number;
  total_rewards_distributed: string;
  avg_verification_time_ms: number;
}

interface Receipt {
  id: string;
  job_id: string;
  executor: string;
  success: boolean;
  gas_used: number;
  state_root: string;
  status: 'verified' | 'pending' | 'disputed';
  submitted_at: number;
  reward: string;
}

export default function VerifierDashboard() {
  const [stats, setStats] = useState<VerifierStats | null>(null);
  const [receipts, setReceipts] = useState<Receipt[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setStats({
      total_receipts: 15234,
      verified_receipts: 15187,
      disputed_receipts: 12,
      total_rewards_distributed: '1,234.56 ATLAS',
      avg_verification_time_ms: 23,
    });

    setReceipts([
      {
        id: '0xabc123...',
        job_id: '0xdef456...',
        executor: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
        success: true,
        gas_used: 125000,
        state_root: '0x1234567890abcdef...',
        status: 'verified',
        submitted_at: Date.now() - 30000,
        reward: '0.05 ATLAS',
      },
      {
        id: '0xbcd234...',
        job_id: '0xefg567...',
        executor: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
        success: true,
        gas_used: 89000,
        state_root: '0xabcdef1234567890...',
        status: 'pending',
        submitted_at: Date.now() - 5000,
        reward: '0.03 ATLAS',
      },
      {
        id: '0xcde345...',
        job_id: '0xfgh678...',
        executor: '5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy',
        success: false,
        gas_used: 50000,
        state_root: '0x0000000000000000...',
        status: 'disputed',
        submitted_at: Date.now() - 120000,
        reward: '0 ATLAS',
      },
    ]);

    setLoading(false);
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen bg-gradient-to-br from-slate-900 via-green-900 to-slate-900 flex items-center justify-center">
        <div className="animate-spin rounded-full h-32 w-32 border-t-2 border-b-2 border-green-500"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-slate-900 via-green-900 to-slate-900">
      {/* Header */}
      <header className="border-b border-green-800/30 backdrop-blur-xl bg-black/20">
        <div className="container mx-auto px-6 py-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-4">
              <Link href="/" className="text-2xl font-bold bg-gradient-to-r from-green-400 to-emerald-400 bg-clip-text text-transparent">
                X3 Sphere
              </Link>
              <span className="text-green-400/60">/</span>
              <span className="text-white font-medium">Receipt Verifier</span>
            </div>
            <nav className="flex items-center space-x-6">
              <Link href="/x3/evolution" className="text-gray-400 hover:text-white transition">Evolution</Link>
              <Link href="/x3/swarm" className="text-gray-400 hover:text-white transition">Swarm</Link>
              <Link href="/x3/scripts" className="text-gray-400 hover:text-white transition">Scripts</Link>
            </nav>
          </div>
        </div>
      </header>

      <main className="container mx-auto px-6 py-8">
        {/* Stats */}
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-5 gap-4 mb-8">
          <StatCard title="Total Receipts" value={stats?.total_receipts.toLocaleString() || '0'} icon="📜" />
          <StatCard title="Verified" value={stats?.verified_receipts.toLocaleString() || '0'} icon="✅" highlight />
          <StatCard title="Disputed" value={stats?.disputed_receipts.toString() || '0'} icon="⚠️" warning />
          <StatCard title="Rewards Paid" value={stats?.total_rewards_distributed || '0'} icon="💰" />
          <StatCard title="Avg Verify Time" value={`${stats?.avg_verification_time_ms}ms`} icon="⏱️" />
        </div>

        {/* Verification Process */}
        <div className="bg-black/40 rounded-2xl border border-green-800/30 p-6 mb-8">
          <h2 className="text-xl font-semibold text-white mb-6 flex items-center gap-2">
            <span className="text-2xl">🔍</span> Verification Process
          </h2>
          <div className="flex items-center justify-between">
            <ProcessStep step={1} title="Receipt Submitted" description="Executor submits execution receipt" active />
            <Arrow />
            <ProcessStep step={2} title="Merkle Verification" description="Verify state root matches claimed changes" />
            <Arrow />
            <ProcessStep step={3} title="Signature Check" description="Validate executor signature" />
            <Arrow />
            <ProcessStep step={4} title="Reward Distribution" description="Pay executor if valid" />
          </div>
        </div>

        {/* Recent Receipts */}
        <div className="bg-black/40 rounded-2xl border border-green-800/30 p-6">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-semibold text-white flex items-center gap-2">
              <span className="text-2xl">📋</span> Recent Receipts
            </h2>
            <div className="flex gap-2">
              <button className="px-3 py-1 bg-green-600/20 text-green-400 rounded-lg text-sm">All</button>
              <button className="px-3 py-1 bg-black/40 text-gray-400 rounded-lg text-sm hover:bg-green-600/10">Verified</button>
              <button className="px-3 py-1 bg-black/40 text-gray-400 rounded-lg text-sm hover:bg-green-600/10">Pending</button>
              <button className="px-3 py-1 bg-black/40 text-gray-400 rounded-lg text-sm hover:bg-green-600/10">Disputed</button>
            </div>
          </div>

          <div className="overflow-x-auto">
            <table className="w-full">
              <thead>
                <tr className="text-left text-gray-400 text-sm border-b border-green-800/30">
                  <th className="pb-4">Receipt ID</th>
                  <th className="pb-4">Job ID</th>
                  <th className="pb-4">Executor</th>
                  <th className="pb-4">Result</th>
                  <th className="pb-4">Gas Used</th>
                  <th className="pb-4">Status</th>
                  <th className="pb-4">Reward</th>
                </tr>
              </thead>
              <tbody>
                {receipts.map((receipt) => (
                  <tr key={receipt.id} className="border-b border-green-800/20 last:border-0">
                    <td className="py-4 font-mono text-sm text-white">{receipt.id}</td>
                    <td className="py-4 font-mono text-sm text-gray-400">{receipt.job_id}</td>
                    <td className="py-4 font-mono text-sm text-gray-400">
                      {receipt.executor.slice(0, 8)}...
                    </td>
                    <td className="py-4">
                      {receipt.success ? (
                        <span className="text-green-400">✓ Success</span>
                      ) : (
                        <span className="text-red-400">✗ Failed</span>
                      )}
                    </td>
                    <td className="py-4 text-white">{receipt.gas_used.toLocaleString()}</td>
                    <td className="py-4">
                      <StatusBadge status={receipt.status} />
                    </td>
                    <td className="py-4 text-emerald-400">{receipt.reward}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Dispute Resolution */}
        <div className="mt-8 grid grid-cols-1 lg:grid-cols-2 gap-8">
          <div className="bg-black/40 rounded-2xl border border-green-800/30 p-6">
            <h2 className="text-xl font-semibold text-white mb-6 flex items-center gap-2">
              <span className="text-2xl">⚖️</span> Active Disputes
            </h2>
            {stats?.disputed_receipts === 0 ? (
              <div className="text-center py-8 text-gray-500">
                <p className="text-4xl mb-2">🎉</p>
                <p>No active disputes</p>
              </div>
            ) : (
              <div className="space-y-4">
                <DisputeCard
                  receiptId="0xcde345..."
                  reason="State root mismatch"
                  challenger="5HGjWAeFDfF..."
                  stake="100 ATLAS"
                />
              </div>
            )}
          </div>

          <div className="bg-black/40 rounded-2xl border border-green-800/30 p-6">
            <h2 className="text-xl font-semibold text-white mb-6 flex items-center gap-2">
              <span className="text-2xl">📊</span> Verification Stats
            </h2>
            <div className="space-y-4">
              <StatRow label="Success Rate" value="99.92%" />
              <StatRow label="Avg Gas per Receipt" value="95,234" />
              <StatRow label="Total Disputes" value="47" />
              <StatRow label="Disputes Won by Executor" value="35" />
              <StatRow label="Disputes Won by Challenger" value="12" />
            </div>
          </div>
        </div>
      </main>
    </div>
  );
}

function StatCard({ title, value, icon, highlight, warning }: {
  title: string;
  value: string;
  icon: string;
  highlight?: boolean;
  warning?: boolean;
}) {
  const borderClass = warning 
    ? 'border-yellow-500/50' 
    : highlight 
      ? 'border-green-500/50' 
      : 'border-green-800/30';
  
  const valueClass = warning
    ? 'text-yellow-400'
    : highlight
      ? 'text-green-400'
      : 'text-white';

  return (
    <div className={`bg-black/40 rounded-xl border ${borderClass} p-4`}>
      <div className="flex items-center justify-between mb-1">
        <span className="text-gray-400 text-xs">{title}</span>
        <span className="text-lg">{icon}</span>
      </div>
      <div className={`text-2xl font-bold ${valueClass}`}>{value}</div>
    </div>
  );
}

function ProcessStep({ step, title, description, active }: {
  step: number;
  title: string;
  description: string;
  active?: boolean;
}) {
  return (
    <div className="text-center">
      <div className={`w-12 h-12 rounded-full flex items-center justify-center mx-auto mb-2 ${
        active ? 'bg-green-500 text-white' : 'bg-green-900/50 text-green-400'
      }`}>
        {step}
      </div>
      <h3 className="text-white font-medium text-sm">{title}</h3>
      <p className="text-gray-500 text-xs mt-1 max-w-[150px]">{description}</p>
    </div>
  );
}

function Arrow() {
  return (
    <div className="text-green-600 text-2xl">→</div>
  );
}

function StatusBadge({ status }: { status: 'verified' | 'pending' | 'disputed' }) {
  const styles = {
    verified: 'bg-green-500/20 text-green-400 border-green-500/30',
    pending: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
    disputed: 'bg-red-500/20 text-red-400 border-red-500/30',
  };

  return (
    <span className={`px-2 py-1 rounded-full text-xs border ${styles[status]}`}>
      {status.toUpperCase()}
    </span>
  );
}

function DisputeCard({ receiptId, reason, challenger, stake }: {
  receiptId: string;
  reason: string;
  challenger: string;
  stake: string;
}) {
  return (
    <div className="bg-red-900/20 border border-red-800/30 rounded-lg p-4">
      <div className="flex items-center justify-between mb-2">
        <span className="font-mono text-sm text-white">{receiptId}</span>
        <span className="text-red-400 text-sm">{stake} at stake</span>
      </div>
      <p className="text-gray-400 text-sm mb-2">{reason}</p>
      <p className="text-gray-500 text-xs">Challenged by: {challenger}</p>
    </div>
  );
}

function StatRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center justify-between py-2 border-b border-green-800/20 last:border-0">
      <span className="text-gray-400">{label}</span>
      <span className="text-white font-medium">{value}</span>
    </div>
  );
}
