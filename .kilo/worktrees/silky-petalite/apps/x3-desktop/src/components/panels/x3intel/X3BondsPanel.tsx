import { useState } from 'react';

interface PendingWithdrawal { id: string; amount: string; requestedAt: string; availableAt: string }
interface BondHistoryEntry { id: string; type: 'Deposit' | 'Withdrawal'; amount: string; date: string; txHash: string }

const pendingWithdrawals: PendingWithdrawal[] = [
  { id: 'w-001', amount: '2,000 USDC', requestedAt: '2026-02-08 14:22', availableAt: '2026-02-15 14:22' },
  { id: 'w-002', amount: '500 USDC',   requestedAt: '2026-02-09 09:05', availableAt: '2026-02-16 09:05' },
];

const bondHistory: BondHistoryEntry[] = [
  { id: 'bh-1', type: 'Deposit',    amount: '20,000 USDC', date: '2026-01-15', txHash: '0xabc1…def1' },
  { id: 'bh-2', type: 'Deposit',    amount: '15,000 USDC', date: '2026-01-22', txHash: '0xabc2…def2' },
  { id: 'bh-3', type: 'Deposit',    amount: '20,000 USDC', date: '2026-01-30', txHash: '0xabc3…def3' },
  { id: 'bh-4', type: 'Withdrawal', amount: '3,000 USDC',  date: '2026-02-05', txHash: '0xabc4…def4' },
  { id: 'bh-5', type: 'Withdrawal', amount: '2,000 USDC',  date: '2026-02-08', txHash: '0xabc5…def5' },
];

export default function X3BondsPanel() {
  const [depositAmt, setDepositAmt] = useState('');
  const [withdrawAmt, setWithdrawAmt] = useState('');

  const locked = true;
  const lockUntil = '2026-03-15 00:00 UTC';

  return (
    <div className="min-h-full bg-[#0a0a0f] text-gray-300 p-6 space-y-6 overflow-auto">
      <h1 className="text-xl font-semibold text-white">Bond Management</h1>

      {/* Balance + Lock Row */}
      <div className="grid grid-cols-2 gap-4">
        <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-5">
          <p className="text-xs text-gray-500 uppercase tracking-wide">Bond Balance</p>
          <p className="text-3xl font-mono text-white mt-2">50,000 <span className="text-sm text-gray-500">USDC</span></p>
        </div>
        <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-5">
          <p className="text-xs text-gray-500 uppercase tracking-wide">Lock Status</p>
          {locked
            ? <p className="text-lg font-mono text-[#ffaa00] mt-2">🔒 Locked until {lockUntil}</p>
            : <p className="text-lg font-mono text-[#00d4aa] mt-2">🔓 Unlocked</p>
          }
        </div>
      </div>

      {/* Actions Row */}
      <div className="grid grid-cols-2 gap-4">
        {/* Deposit */}
        <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-5 space-y-3">
          <p className="text-sm text-gray-400 font-medium">Deposit</p>
          <div className="flex gap-2">
            <input value={depositAmt} onChange={e => setDepositAmt(e.target.value)} placeholder="Amount (USDC)"
              className="flex-1 bg-[#0a0a0f] border border-[#222] rounded px-3 py-2 text-sm font-mono text-gray-300 focus:outline-none focus:border-[#00d4aa]" />
            <button className="px-4 py-2 bg-[#00d4aa] text-black text-sm font-medium rounded hover:brightness-110 transition">
              Deposit
            </button>
          </div>
        </div>
        {/* Withdraw */}
        <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-5 space-y-3">
          <p className="text-sm text-gray-400 font-medium">Withdraw</p>
          <div className="flex gap-2">
            <input value={withdrawAmt} onChange={e => setWithdrawAmt(e.target.value)} placeholder="Amount (USDC)"
              className="flex-1 bg-[#0a0a0f] border border-[#222] rounded px-3 py-2 text-sm font-mono text-gray-300 focus:outline-none focus:border-[#ffaa00]" />
            <button className="px-4 py-2 bg-[#ffaa00] text-black text-sm font-medium rounded hover:brightness-110 transition">
              Request Withdrawal
            </button>
          </div>
        </div>
      </div>

      {/* Pending Withdrawals */}
      <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg p-4">
        <p className="text-sm text-gray-400 mb-3">Pending Withdrawals</p>
        {pendingWithdrawals.length === 0 ? (
          <p className="text-gray-500 text-sm">No pending withdrawals.</p>
        ) : (
          <div className="space-y-2">
            {pendingWithdrawals.map(w => (
              <div key={w.id} className="flex items-center justify-between bg-[#0a0a0f] border border-[#222] rounded px-4 py-3">
                <div>
                  <span className="font-mono text-sm text-white">{w.amount}</span>
                  <span className="text-xs text-gray-500 ml-2">requested {w.requestedAt}</span>
                </div>
                <span className="text-xs text-[#ffaa00]">available {w.availableAt}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Bond History */}
      <div className="bg-[#111116] border border-[#1a1a1a] rounded-lg overflow-hidden">
        <p className="text-sm text-gray-400 p-4 pb-2">Bond History</p>
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-[#222] text-gray-500 text-xs uppercase">
              <th className="text-left p-3">Type</th><th className="text-right p-3">Amount</th>
              <th className="text-right p-3">Date</th><th className="text-right p-3">Tx Hash</th>
            </tr>
          </thead>
          <tbody>
            {bondHistory.map(h => (
              <tr key={h.id} className="border-b border-[#1a1a1a] hover:bg-[#ffffff04]">
                <td className="p-3">
                  <span className={`px-2 py-0.5 rounded text-xs font-medium ${h.type === 'Deposit' ? 'bg-[#00d4aa22] text-[#00d4aa]' : 'bg-[#ffaa0022] text-[#ffaa00]'}`}>
                    {h.type}
                  </span>
                </td>
                <td className="p-3 text-right font-mono">{h.amount}</td>
                <td className="p-3 text-right text-gray-500 font-mono text-xs">{h.date}</td>
                <td className="p-3 text-right font-mono text-xs text-gray-500">{h.txHash}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
