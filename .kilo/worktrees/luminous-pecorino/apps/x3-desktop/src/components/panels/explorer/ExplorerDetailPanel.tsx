import React, { useState } from 'react';
import {
  Search,
  Box,
  ArrowRightLeft,
  User,
  CheckCircle,
  Copy,
  ChevronRight,
} from 'lucide-react';
import clsx from 'clsx';

type ExplorerTab = 'block' | 'transaction' | 'account';

const MOCK_BLOCK = {
  number: 1234567,
  hash: '0xabc123def456789012345678901234567890abcdef1234567890abcdef123456',
  parentHash: '0x987654321fedcba0987654321fedcba0987654321fedcba0987654321fedcba0',
  timestamp: '2026-02-10T14:32:18Z',
  extrinsicsCount: 12,
  stateRoot: '0x1111222233334444555566667777888899990000aaaabbbbccccddddeeee0000',
  validator: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  extrinsics: [
    { index: 0, hash: '0xext001...a1b2', module: 'Timestamp', call: 'set(1707572038)', status: 'Success' },
    { index: 1, hash: '0xext002...c3d4', module: 'Balances', call: 'transfer(5FHne..., 1500 X3)', status: 'Success' },
    { index: 2, hash: '0xext003...e5f6', module: 'GpuSwarm', call: 'submit_task(task_id: 892)', status: 'Success' },
  ],
};

const MOCK_TX = {
  hash: '0xtx789abcdef0123456789abcdef0123456789abcdef0123456789abcdef012345',
  blockNumber: 1234567,
  from: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  to: '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
  value: '1,500 X3',
  fee: '0.0012 X3',
  status: 'Finalized' as const,
  events: [
    { name: 'Balances.Transfer', data: 'from: 5Grwv... → to: 5FHne... amount: 1500 X3' },
    { name: 'System.ExtrinsicSuccess', data: 'weight: 234,567' },
    { name: 'Treasury.Deposit', data: 'fee: 0.0012 X3' },
  ],
};

const MOCK_ACCOUNT = {
  address: '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  freeBalance: '45,230.5 X3',
  reserved: '1,200.0 X3',
  nonce: 847,
  transactionCount: 1243,
  recentTxs: [
    { hash: '0xaaa...111', type: 'Send', amount: '1,500 X3', to: '5FHne...', time: '2m ago' },
    { hash: '0xbbb...222', type: 'Receive', amount: '3,200 X3', from: '5DAng...', time: '15m ago' },
    { hash: '0xccc...333', type: 'Send', amount: '500 X3', to: '5HGjW...', time: '1h ago' },
    { hash: '0xddd...444', type: 'Stake', amount: '10,000 X3', to: 'Validator Pool', time: '3h ago' },
    { hash: '0xeee...555', type: 'Receive', amount: '892.4 X3', from: '5Ckxo...', time: '6h ago' },
  ],
};

const truncate = (s: string, len = 16) =>
  s.length > len ? `${s.slice(0, len / 2 + 2)}...${s.slice(-(len / 2))}` : s;

const CopyBtn: React.FC<{ text: string }> = ({ text }) => {
  const [copied, setCopied] = useState(false);
  const copy = () => {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <button onClick={copy} className="text-gray-600 hover:text-white transition-colors">
      {copied ? <CheckCircle size={12} className="text-green-400" /> : <Copy size={12} />}
    </button>
  );
};

const Field: React.FC<{ label: string; value: string; mono?: boolean; copyable?: boolean }> = ({
  label,
  value,
  mono = false,
  copyable = false,
}) => (
  <div className="flex items-start justify-between py-2 border-b border-[#1a1a1a] last:border-0">
    <span className="text-xs text-gray-500 w-32 shrink-0">{label}</span>
    <div className="flex items-center gap-2 min-w-0">
      <span className={clsx('text-sm text-white text-right break-all', mono && 'font-mono text-xs')}>
        {value}
      </span>
      {copyable && <CopyBtn text={value} />}
    </div>
  </div>
);

const ExplorerDetailPanel: React.FC = () => {
  const [tab, setTab] = useState<ExplorerTab>('block');
  const [searchQuery, setSearchQuery] = useState('');
  const [blockInput, setBlockInput] = useState('1234567');
  const [txInput, setTxInput] = useState(MOCK_TX.hash);
  const [addrInput, setAddrInput] = useState(MOCK_ACCOUNT.address);
  const [showBlock, setShowBlock] = useState(true);
  const [showTx, setShowTx] = useState(true);
  const [showAccount, setShowAccount] = useState(true);

  const renderBlock = () => (
    <div className="space-y-4">
      <div className="flex gap-2">
        <input
          type="text"
          placeholder="Block number..."
          value={blockInput}
          onChange={(e) => setBlockInput(e.target.value)}
          className="flex-1 bg-[#111111] border border-[#1a1a1a] rounded-lg px-3 py-2 text-sm text-white placeholder-gray-600 outline-none focus:border-orange-500/40 font-mono"
        />
        <button
          onClick={() => setShowBlock(true)}
          className="bg-orange-500/20 text-orange-400 text-xs font-semibold px-4 py-2 rounded-lg hover:bg-orange-500/30 transition-colors"
        >
          Load
        </button>
      </div>

      {showBlock && (
        <>
          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <div className="flex items-center gap-2 mb-3">
              <Box size={14} className="text-orange-400" />
              <span className="text-sm font-semibold">Block #{MOCK_BLOCK.number.toLocaleString()}</span>
            </div>
            <Field label="Hash" value={MOCK_BLOCK.hash} mono copyable />
            <Field label="Parent Hash" value={MOCK_BLOCK.parentHash} mono copyable />
            <Field label="Timestamp" value={new Date(MOCK_BLOCK.timestamp).toLocaleString()} />
            <Field label="Extrinsics" value={String(MOCK_BLOCK.extrinsicsCount)} />
            <Field label="State Root" value={MOCK_BLOCK.stateRoot} mono copyable />
            <Field label="Validator" value={truncate(MOCK_BLOCK.validator, 24)} mono copyable />
          </div>

          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <h3 className="text-sm font-semibold mb-3">Extrinsics</h3>
            <div className="space-y-2">
              {MOCK_BLOCK.extrinsics.map((ext) => (
                <div
                  key={ext.index}
                  className="flex items-center gap-3 p-2.5 rounded-lg bg-[#0a0a0f] hover:bg-[#0f0f14] transition-colors cursor-pointer"
                >
                  <span className="text-xs text-gray-500 w-6 text-center">#{ext.index}</span>
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-medium text-white">{ext.module}</div>
                    <div className="text-[10px] text-gray-500 font-mono truncate">{ext.call}</div>
                  </div>
                  <span className="text-[10px] text-green-400">{ext.status}</span>
                  <ChevronRight size={12} className="text-gray-600" />
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );

  const renderTransaction = () => (
    <div className="space-y-4">
      <div className="flex gap-2">
        <input
          type="text"
          placeholder="Transaction hash..."
          value={txInput}
          onChange={(e) => setTxInput(e.target.value)}
          className="flex-1 bg-[#111111] border border-[#1a1a1a] rounded-lg px-3 py-2 text-sm text-white placeholder-gray-600 outline-none focus:border-orange-500/40 font-mono text-xs"
        />
        <button
          onClick={() => setShowTx(true)}
          className="bg-orange-500/20 text-orange-400 text-xs font-semibold px-4 py-2 rounded-lg hover:bg-orange-500/30 transition-colors"
        >
          Load
        </button>
      </div>

      {showTx && (
        <>
          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <div className="flex items-center gap-2 mb-3">
              <ArrowRightLeft size={14} className="text-blue-400" />
              <span className="text-sm font-semibold">Transaction Details</span>
              <span className="text-[10px] px-2 py-0.5 rounded-full bg-green-500/20 text-green-400 ml-auto">
                {MOCK_TX.status}
              </span>
            </div>
            <Field label="Hash" value={MOCK_TX.hash} mono copyable />
            <Field label="Block" value={`#${MOCK_TX.blockNumber.toLocaleString()}`} />
            <Field label="From" value={truncate(MOCK_TX.from, 24)} mono copyable />
            <Field label="To" value={truncate(MOCK_TX.to, 24)} mono copyable />
            <Field label="Value" value={MOCK_TX.value} />
            <Field label="Fee" value={MOCK_TX.fee} />
          </div>

          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <h3 className="text-sm font-semibold mb-3">Events</h3>
            <div className="space-y-2">
              {MOCK_TX.events.map((ev, i) => (
                <div key={i} className="p-2.5 rounded-lg bg-[#0a0a0f]">
                  <div className="text-xs font-medium text-white">{ev.name}</div>
                  <div className="text-[10px] text-gray-500 font-mono mt-0.5">{ev.data}</div>
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );

  const renderAccount = () => (
    <div className="space-y-4">
      <div className="flex gap-2">
        <input
          type="text"
          placeholder="Account address..."
          value={addrInput}
          onChange={(e) => setAddrInput(e.target.value)}
          className="flex-1 bg-[#111111] border border-[#1a1a1a] rounded-lg px-3 py-2 text-sm text-white placeholder-gray-600 outline-none focus:border-orange-500/40 font-mono text-xs"
        />
        <button
          onClick={() => setShowAccount(true)}
          className="bg-orange-500/20 text-orange-400 text-xs font-semibold px-4 py-2 rounded-lg hover:bg-orange-500/30 transition-colors"
        >
          Load
        </button>
      </div>

      {showAccount && (
        <>
          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <div className="flex items-center gap-2 mb-3">
              <User size={14} className="text-purple-400" />
              <span className="text-sm font-semibold">Account Details</span>
            </div>
            <Field label="Address" value={truncate(MOCK_ACCOUNT.address, 24)} mono copyable />
            <Field label="Free Balance" value={MOCK_ACCOUNT.freeBalance} />
            <Field label="Reserved" value={MOCK_ACCOUNT.reserved} />
            <Field label="Nonce" value={String(MOCK_ACCOUNT.nonce)} />
            <Field label="Transactions" value={MOCK_ACCOUNT.transactionCount.toLocaleString()} />
          </div>

          <div className="bg-[#111111] rounded-xl p-4 border border-[#1a1a1a]">
            <h3 className="text-sm font-semibold mb-3">Recent Transactions</h3>
            <div className="space-y-2">
              {MOCK_ACCOUNT.recentTxs.map((tx) => (
                <div
                  key={tx.hash}
                  className="flex items-center gap-3 p-2.5 rounded-lg bg-[#0a0a0f] hover:bg-[#0f0f14] transition-colors cursor-pointer"
                >
                  <div
                    className={clsx(
                      'w-7 h-7 rounded-full flex items-center justify-center text-xs',
                      tx.type === 'Send' && 'bg-red-500/10 text-red-400',
                      tx.type === 'Receive' && 'bg-green-500/10 text-green-400',
                      tx.type === 'Stake' && 'bg-purple-500/10 text-purple-400',
                    )}
                  >
                    {tx.type === 'Send' ? '↑' : tx.type === 'Receive' ? '↓' : '⊕'}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-medium text-white">{tx.type}</div>
                    <div className="text-[10px] text-gray-500 font-mono">{tx.hash}</div>
                  </div>
                  <div className="text-right">
                    <div className="text-xs text-white">{tx.amount}</div>
                    <div className="text-[10px] text-gray-600">{tx.time}</div>
                  </div>
                  <ChevronRight size={12} className="text-gray-600" />
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </div>
  );

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Search bar */}
      <div className="px-5 pt-4 pb-2">
        <div className="relative">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" />
          <input
            type="text"
            placeholder="Search blocks, transactions, accounts..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-[#111111] border border-[#1a1a1a] rounded-lg pl-9 pr-4 py-2.5 text-sm text-white placeholder-gray-600 outline-none focus:border-orange-500/40"
          />
        </div>
      </div>

      {/* Tabs */}
      <div className="px-5 py-2 flex items-center gap-1">
        {([
          ['block', Box, 'Block'],
          ['transaction', ArrowRightLeft, 'Transaction'],
          ['account', User, 'Account'],
        ] as const).map(([key, Icon, label]) => (
          <button
            key={key}
            onClick={() => setTab(key as ExplorerTab)}
            className={clsx(
              'flex items-center gap-1.5 px-3 py-1.5 rounded-md text-xs font-medium transition-colors',
              tab === key
                ? 'bg-orange-500/20 text-orange-400'
                : 'text-gray-500 hover:text-white',
            )}
          >
            <Icon size={12} /> {label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 px-5 pb-5 overflow-auto">
        {tab === 'block' && renderBlock()}
        {tab === 'transaction' && renderTransaction()}
        {tab === 'account' && renderAccount()}
      </div>
    </div>
  );
};

export default ExplorerDetailPanel;
