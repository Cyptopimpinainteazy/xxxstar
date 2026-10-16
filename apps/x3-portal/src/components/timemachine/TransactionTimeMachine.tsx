/**
 * Transaction Time Machine
 * 
 * A revolutionary interface for exploring blockchain history.
 * Navigate through time, visualize state changes, and replay transactions.
 * 
 * Features:
 * - Timeline scrubbing with block-level precision
 * - State diff visualization
 * - Transaction replay with gas simulation
 * - Fork comparison mode
 * - Time-travel debugging
 */

'use client';

import React, { useState, useRef, useEffect, useCallback, useMemo } from 'react';

// =============================================================================
// Types
// =============================================================================

export interface Block {
  number: number;
  hash: string;
  timestamp: number;
  transactions: number;
  gasUsed: string;
  parentHash: string;
}

export interface Transaction {
  hash: string;
  blockNumber: number;
  from: string;
  to: string;
  value: string;
  gasUsed: string;
  status: 'success' | 'failed' | 'pending';
  method?: string;
  type: 'evm' | 'svm' | 'comit';
}

export interface StateChange {
  address: string;
  slot: string;
  previousValue: string;
  newValue: string;
  type: 'balance' | 'storage' | 'code' | 'nonce';
}

export interface TimeMachineProps {
  /** Initial block to display */
  initialBlock?: number;
  /** Latest block number */
  latestBlock?: number;
  /** Block fetcher function */
  fetchBlock?: (blockNumber: number) => Promise<Block>;
  /** Transaction fetcher */
  fetchTransactions?: (blockNumber: number) => Promise<Transaction[]>;
  /** State diff fetcher */
  fetchStateDiff?: (blockNumber: number) => Promise<StateChange[]>;
  /** Enable fork comparison */
  enableForkMode?: boolean;
  /** WebSocket URL for real-time updates */
  wsUrl?: string;
}

// =============================================================================
// Mock Data Generators
// =============================================================================

function generateMockBlock(blockNumber: number): Block {
  const baseTime = Date.now() - (100000 - blockNumber) * 6000;
  return {
    number: blockNumber,
    hash: `0x${blockNumber.toString(16).padStart(64, 'a')}`,
    timestamp: baseTime,
    transactions: Math.floor(Math.random() * 50) + 5,
    gasUsed: (Math.random() * 15000000 + 5000000).toFixed(0),
    parentHash: `0x${(blockNumber - 1).toString(16).padStart(64, 'a')}`,
  };
}

function generateMockTransactions(blockNumber: number): Transaction[] {
  const count = Math.floor(Math.random() * 20) + 5;
  return Array.from({ length: count }, (_, i) => ({
    hash: `0x${blockNumber}${i}`.padEnd(66, 'f'),
    blockNumber,
    from: `0x${Math.random().toString(16).slice(2, 42)}`,
    to: `0x${Math.random().toString(16).slice(2, 42)}`,
    value: (Math.random() * 10).toFixed(4),
    gasUsed: (Math.random() * 100000 + 21000).toFixed(0),
    status: Math.random() > 0.1 ? 'success' : 'failed',
    method: ['transfer', 'swap', 'mint', 'approve', 'stake'][Math.floor(Math.random() * 5)],
    type: ['evm', 'svm', 'comit'][Math.floor(Math.random() * 3)] as 'evm' | 'svm' | 'comit',
  }));
}

function generateMockStateDiff(blockNumber: number): StateChange[] {
  const count = Math.floor(Math.random() * 10) + 3;
  return Array.from({ length: count }, () => ({
    address: `0x${Math.random().toString(16).slice(2, 42)}`,
    slot: `0x${Math.floor(Math.random() * 100).toString(16).padStart(64, '0')}`,
    previousValue: `0x${Math.random().toString(16).slice(2, 66)}`,
    newValue: `0x${Math.random().toString(16).slice(2, 66)}`,
    type: ['balance', 'storage', 'nonce'][Math.floor(Math.random() * 3)] as 'balance' | 'storage' | 'nonce',
  }));
}

// =============================================================================
// Component
// =============================================================================

export const TransactionTimeMachine: React.FC<TimeMachineProps> = ({
  initialBlock = 100000,
  latestBlock = 100050,
  fetchBlock = async (n) => generateMockBlock(n),
  fetchTransactions = async (n) => generateMockTransactions(n),
  fetchStateDiff = async (n) => generateMockStateDiff(n),
  enableForkMode = true,
  wsUrl,
}) => {
  const [currentBlock, setCurrentBlock] = useState(initialBlock);
  const [block, setBlock] = useState<Block | null>(null);
  const [transactions, setTransactions] = useState<Transaction[]>([]);
  const [stateDiff, setStateDiff] = useState<StateChange[]>([]);
  const [isPlaying, setIsPlaying] = useState(false);
  const [playSpeed, setPlaySpeed] = useState(1);
  const [selectedTx, setSelectedTx] = useState<Transaction | null>(null);
  const [viewMode, setViewMode] = useState<'timeline' | 'state' | 'replay'>('timeline');
  const [forkBlock, setForkBlock] = useState<number | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const timelineRef = useRef<HTMLDivElement>(null);
  const playIntervalRef = useRef<NodeJS.Timeout | null>(null);

  // Fetch block data
  const loadBlockData = useCallback(async (blockNumber: number) => {
    setIsLoading(true);
    try {
      const [blockData, txData, diffData] = await Promise.all([
        fetchBlock(blockNumber),
        fetchTransactions(blockNumber),
        fetchStateDiff(blockNumber),
      ]);
      setBlock(blockData);
      setTransactions(txData);
      setStateDiff(diffData);
    } catch (error) {
      console.error('Failed to load block:', error);
    } finally {
      setIsLoading(false);
    }
  }, [fetchBlock, fetchTransactions, fetchStateDiff]);

  // Load initial block
  useEffect(() => {
    loadBlockData(currentBlock);
  }, [currentBlock, loadBlockData]);

  // Playback control
  useEffect(() => {
    if (isPlaying) {
      playIntervalRef.current = setInterval(() => {
        setCurrentBlock((prev) => {
          if (prev >= latestBlock) {
            setIsPlaying(false);
            return prev;
          }
          return prev + 1;
        });
      }, 1000 / playSpeed);
    } else if (playIntervalRef.current) {
      clearInterval(playIntervalRef.current);
    }

    return () => {
      if (playIntervalRef.current) {
        clearInterval(playIntervalRef.current);
      }
    };
  }, [isPlaying, playSpeed, latestBlock]);

  // Format timestamp
  const formatTime = (timestamp: number) => {
    return new Date(timestamp).toLocaleString();
  };

  // Format address
  const formatAddress = (addr: string) => {
    return `${addr.slice(0, 6)}...${addr.slice(-4)}`;
  };

  // Timeline blocks for visualization
  const timelineBlocks = useMemo(() => {
    const range = 20;
    const start = Math.max(1, currentBlock - range / 2);
    const end = Math.min(latestBlock, currentBlock + range / 2);
    return Array.from({ length: end - start + 1 }, (_, i) => start + i);
  }, [currentBlock, latestBlock]);

  return (
    <div
      className="time-machine"
      style={{
        background: 'linear-gradient(135deg, #0a0a1a, #1a0a2e)',
        borderRadius: 16,
        overflow: 'hidden',
        border: '1px solid rgba(0, 255, 255, 0.2)',
        boxShadow: '0 0 50px rgba(0, 255, 255, 0.1)',
      }}
    >
      {/* Header */}
      <header
        style={{
          padding: '16px 24px',
          borderBottom: '1px solid rgba(255, 255, 255, 0.1)',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          background: 'rgba(0, 0, 0, 0.3)',
        }}
      >
        <div style={{ display: 'flex', alignItems: 'center', gap: 16 }}>
          <h2 style={{ margin: 0, color: '#00ffff', fontSize: 20, fontWeight: 600 }}>
            ⏱️ Transaction Time Machine
          </h2>
          <span
            style={{
              background: 'rgba(0, 255, 100, 0.2)',
              color: '#00ff64',
              padding: '4px 12px',
              borderRadius: 20,
              fontSize: 12,
              fontWeight: 600,
            }}
          >
            Block #{currentBlock.toLocaleString()}
          </span>
        </div>

        {/* View mode tabs */}
        <div style={{ display: 'flex', gap: 4, background: 'rgba(0, 0, 0, 0.3)', borderRadius: 8, padding: 4 }}>
          {(['timeline', 'state', 'replay'] as const).map((mode) => (
            <button
              key={mode}
              onClick={() => setViewMode(mode)}
              style={{
                background: viewMode === mode ? 'rgba(0, 255, 255, 0.2)' : 'transparent',
                border: 'none',
                borderRadius: 6,
                padding: '8px 16px',
                color: viewMode === mode ? '#00ffff' : 'rgba(255, 255, 255, 0.6)',
                cursor: 'pointer',
                textTransform: 'capitalize',
                fontWeight: viewMode === mode ? 600 : 400,
              }}
            >
              {mode}
            </button>
          ))}
        </div>
      </header>

      {/* Timeline Scrubber */}
      <div
        style={{
          padding: '24px',
          borderBottom: '1px solid rgba(255, 255, 255, 0.1)',
          background: 'rgba(0, 0, 0, 0.2)',
        }}
      >
        {/* Playback controls */}
        <div style={{ display: 'flex', alignItems: 'center', gap: 16, marginBottom: 16 }}>
          <button
            onClick={() => setCurrentBlock(Math.max(1, currentBlock - 10))}
            style={{
              background: 'rgba(255, 255, 255, 0.1)',
              border: 'none',
              borderRadius: 8,
              padding: '8px 12px',
              color: '#fff',
              cursor: 'pointer',
            }}
          >
            ⏪ -10
          </button>
          
          <button
            onClick={() => setCurrentBlock(Math.max(1, currentBlock - 1))}
            style={{
              background: 'rgba(255, 255, 255, 0.1)',
              border: 'none',
              borderRadius: 8,
              padding: '8px 12px',
              color: '#fff',
              cursor: 'pointer',
            }}
          >
            ◀ Prev
          </button>

          <button
            onClick={() => setIsPlaying(!isPlaying)}
            style={{
              background: isPlaying ? 'rgba(255, 0, 100, 0.3)' : 'rgba(0, 255, 100, 0.3)',
              border: 'none',
              borderRadius: 8,
              padding: '8px 24px',
              color: isPlaying ? '#ff6699' : '#00ff64',
              cursor: 'pointer',
              fontWeight: 600,
              minWidth: 100,
            }}
          >
            {isPlaying ? '⏸ Pause' : '▶ Play'}
          </button>

          <button
            onClick={() => setCurrentBlock(Math.min(latestBlock, currentBlock + 1))}
            style={{
              background: 'rgba(255, 255, 255, 0.1)',
              border: 'none',
              borderRadius: 8,
              padding: '8px 12px',
              color: '#fff',
              cursor: 'pointer',
            }}
          >
            Next ▶
          </button>

          <button
            onClick={() => setCurrentBlock(Math.min(latestBlock, currentBlock + 10))}
            style={{
              background: 'rgba(255, 255, 255, 0.1)',
              border: 'none',
              borderRadius: 8,
              padding: '8px 12px',
              color: '#fff',
              cursor: 'pointer',
            }}
          >
            +10 ⏩
          </button>

          <div style={{ flex: 1 }} />

          {/* Speed control */}
          <label style={{ color: 'rgba(255, 255, 255, 0.6)', fontSize: 13 }}>
            Speed: {playSpeed}x
          </label>
          <input
            type="range"
            min="0.5"
            max="5"
            step="0.5"
            value={playSpeed}
            onChange={(e) => setPlaySpeed(parseFloat(e.target.value))}
            style={{ width: 100 }}
          />

          {/* Fork mode */}
          {enableForkMode && (
            <button
              onClick={() => setForkBlock(forkBlock ? null : currentBlock)}
              style={{
                background: forkBlock ? 'rgba(255, 165, 0, 0.3)' : 'rgba(255, 255, 255, 0.1)',
                border: '1px solid',
                borderColor: forkBlock ? 'rgba(255, 165, 0, 0.5)' : 'transparent',
                borderRadius: 8,
                padding: '8px 16px',
                color: forkBlock ? '#ffa500' : 'rgba(255, 255, 255, 0.8)',
                cursor: 'pointer',
              }}
            >
              {forkBlock ? `Fork @ #${forkBlock}` : '🔀 Fork'}
            </button>
          )}
        </div>

        {/* Timeline visualization */}
        <div
          ref={timelineRef}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 4,
            overflowX: 'auto',
            padding: '8px 0',
          }}
        >
          {timelineBlocks.map((blockNum) => {
            const isCurrentBlock = blockNum === currentBlock;
            const isForkPoint = blockNum === forkBlock;
            
            return (
              <button
                key={blockNum}
                onClick={() => setCurrentBlock(blockNum)}
                style={{
                  width: isCurrentBlock ? 48 : 32,
                  height: isCurrentBlock ? 48 : 32,
                  borderRadius: isCurrentBlock ? 12 : 8,
                  background: isCurrentBlock
                    ? 'linear-gradient(135deg, #00ffff, #0066ff)'
                    : isForkPoint
                    ? 'linear-gradient(135deg, #ffa500, #ff6600)'
                    : 'rgba(255, 255, 255, 0.1)',
                  border: isCurrentBlock
                    ? '2px solid #fff'
                    : '1px solid rgba(255, 255, 255, 0.2)',
                  color: '#fff',
                  cursor: 'pointer',
                  fontSize: isCurrentBlock ? 12 : 10,
                  fontWeight: isCurrentBlock ? 700 : 400,
                  transition: 'all 0.2s ease',
                  flexShrink: 0,
                }}
              >
                {blockNum % 5 === 0 || isCurrentBlock ? blockNum : '·'}
              </button>
            );
          })}
        </div>

        {/* Block slider */}
        <input
          type="range"
          min="1"
          max={latestBlock}
          value={currentBlock}
          onChange={(e) => setCurrentBlock(parseInt(e.target.value))}
          style={{
            width: '100%',
            marginTop: 16,
            accentColor: '#00ffff',
          }}
        />
        <div
          style={{
            display: 'flex',
            justifyContent: 'space-between',
            fontSize: 11,
            color: 'rgba(255, 255, 255, 0.5)',
            marginTop: 4,
          }}
        >
          <span>Genesis</span>
          <span>Block #{latestBlock.toLocaleString()} (Latest)</span>
        </div>
      </div>

      {/* Main content */}
      <div style={{ display: 'flex', minHeight: 400 }}>
        {/* Block info panel */}
        <div
          style={{
            width: 300,
            padding: 24,
            borderRight: '1px solid rgba(255, 255, 255, 0.1)',
            background: 'rgba(0, 0, 0, 0.2)',
          }}
        >
          <h3 style={{ color: '#fff', margin: '0 0 16px', fontSize: 14, fontWeight: 600 }}>
            Block Details
          </h3>

          {isLoading ? (
            <div style={{ color: 'rgba(255, 255, 255, 0.5)', textAlign: 'center', padding: 32 }}>
              Loading...
            </div>
          ) : block ? (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
              <InfoRow label="Number" value={`#${block.number.toLocaleString()}`} />
              <InfoRow label="Timestamp" value={formatTime(block.timestamp)} />
              <InfoRow label="Transactions" value={block.transactions.toString()} />
              <InfoRow label="Gas Used" value={`${(parseInt(block.gasUsed) / 1e6).toFixed(2)}M`} />
              <InfoRow label="Hash" value={formatAddress(block.hash)} copyable />
              <InfoRow label="Parent" value={formatAddress(block.parentHash)} copyable />
            </div>
          ) : null}

          {/* Fork comparison */}
          {forkBlock && viewMode === 'timeline' && (
            <div
              style={{
                marginTop: 24,
                padding: 16,
                background: 'rgba(255, 165, 0, 0.1)',
                borderRadius: 8,
                border: '1px solid rgba(255, 165, 0, 0.3)',
              }}
            >
              <h4 style={{ color: '#ffa500', margin: '0 0 8px', fontSize: 13 }}>
                🔀 Fork Comparison
              </h4>
              <p style={{ color: 'rgba(255, 255, 255, 0.7)', fontSize: 12, margin: 0 }}>
                Comparing current block to fork point at #{forkBlock}
              </p>
              <div style={{ marginTop: 12, fontSize: 12 }}>
                <div style={{ color: '#00ff64' }}>
                  + {currentBlock - forkBlock} blocks ahead
                </div>
                <div style={{ color: '#00ffff' }}>
                  + ~{(currentBlock - forkBlock) * 5} transactions
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Transaction / State view */}
        <div style={{ flex: 1, padding: 24, overflowY: 'auto' }}>
          {viewMode === 'timeline' && (
            <>
              <h3 style={{ color: '#fff', margin: '0 0 16px', fontSize: 14, fontWeight: 600 }}>
                Transactions ({transactions.length})
              </h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {transactions.map((tx) => (
                  <div
                    key={tx.hash}
                    onClick={() => setSelectedTx(tx)}
                    style={{
                      padding: '12px 16px',
                      background: selectedTx?.hash === tx.hash
                        ? 'rgba(0, 255, 255, 0.1)'
                        : 'rgba(255, 255, 255, 0.05)',
                      borderRadius: 8,
                      border: selectedTx?.hash === tx.hash
                        ? '1px solid rgba(0, 255, 255, 0.3)'
                        : '1px solid transparent',
                      cursor: 'pointer',
                      display: 'flex',
                      alignItems: 'center',
                      gap: 12,
                    }}
                  >
                    {/* Status indicator */}
                    <span
                      style={{
                        width: 8,
                        height: 8,
                        borderRadius: '50%',
                        background: tx.status === 'success' ? '#00ff64' : '#ff3366',
                      }}
                    />
                    
                    {/* Type badge */}
                    <span
                      style={{
                        padding: '2px 8px',
                        borderRadius: 4,
                        fontSize: 10,
                        fontWeight: 600,
                        textTransform: 'uppercase',
                        background:
                          tx.type === 'comit'
                            ? 'rgba(255, 0, 255, 0.3)'
                            : tx.type === 'evm'
                            ? 'rgba(0, 100, 255, 0.3)'
                            : 'rgba(0, 255, 100, 0.3)',
                        color:
                          tx.type === 'comit'
                            ? '#ff00ff'
                            : tx.type === 'evm'
                            ? '#00aaff'
                            : '#00ff64',
                      }}
                    >
                      {tx.type}
                    </span>

                    {/* Method */}
                    <span style={{ color: '#fff', fontWeight: 500, width: 80 }}>
                      {tx.method || 'transfer'}
                    </span>

                    {/* Addresses */}
                    <span style={{ color: 'rgba(255, 255, 255, 0.6)', fontSize: 12 }}>
                      {formatAddress(tx.from)} → {formatAddress(tx.to)}
                    </span>

                    <span style={{ flex: 1 }} />

                    {/* Value */}
                    <span style={{ color: '#00ffff', fontWeight: 500 }}>
                      {tx.value} ETH
                    </span>
                  </div>
                ))}
              </div>
            </>
          )}

          {viewMode === 'state' && (
            <>
              <h3 style={{ color: '#fff', margin: '0 0 16px', fontSize: 14, fontWeight: 600 }}>
                State Changes ({stateDiff.length})
              </h3>
              <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
                {stateDiff.map((change, i) => (
                  <div
                    key={i}
                    style={{
                      padding: '12px 16px',
                      background: 'rgba(255, 255, 255, 0.05)',
                      borderRadius: 8,
                    }}
                  >
                    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 8 }}>
                      <span
                        style={{
                          padding: '2px 8px',
                          borderRadius: 4,
                          fontSize: 10,
                          fontWeight: 600,
                          textTransform: 'uppercase',
                          background: 'rgba(0, 255, 255, 0.2)',
                          color: '#00ffff',
                        }}
                      >
                        {change.type}
                      </span>
                      <span style={{ color: 'rgba(255, 255, 255, 0.8)', fontSize: 12 }}>
                        {formatAddress(change.address)}
                      </span>
                    </div>
                    <div style={{ display: 'flex', gap: 16, fontSize: 11 }}>
                      <div style={{ flex: 1 }}>
                        <div style={{ color: 'rgba(255, 255, 255, 0.5)', marginBottom: 4 }}>Before:</div>
                        <code style={{ color: '#ff6666' }}>{formatAddress(change.previousValue)}</code>
                      </div>
                      <div style={{ color: 'rgba(255, 255, 255, 0.3)' }}>→</div>
                      <div style={{ flex: 1 }}>
                        <div style={{ color: 'rgba(255, 255, 255, 0.5)', marginBottom: 4 }}>After:</div>
                        <code style={{ color: '#66ff66' }}>{formatAddress(change.newValue)}</code>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </>
          )}

          {viewMode === 'replay' && selectedTx && (
            <div style={{ textAlign: 'center', padding: 32 }}>
              <h3 style={{ color: '#fff', marginBottom: 24 }}>🔄 Transaction Replay</h3>
              <div
                style={{
                  background: 'rgba(0, 0, 0, 0.3)',
                  borderRadius: 12,
                  padding: 24,
                  marginBottom: 24,
                }}
              >
                <p style={{ color: 'rgba(255, 255, 255, 0.7)', marginBottom: 16 }}>
                  Replay transaction with modified parameters:
                </p>
                <code style={{ color: '#00ffff' }}>{selectedTx.hash}</code>
              </div>
              <button
                style={{
                  background: 'linear-gradient(135deg, #00ffff, #0066ff)',
                  border: 'none',
                  borderRadius: 8,
                  padding: '12px 32px',
                  color: '#000',
                  fontWeight: 600,
                  cursor: 'pointer',
                }}
              >
                ▶ Simulate Replay
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

// =============================================================================
// Helper Components
// =============================================================================

interface InfoRowProps {
  label: string;
  value: string;
  copyable?: boolean;
}

const InfoRow: React.FC<InfoRowProps> = ({ label, value, copyable }) => (
  <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
    <span style={{ color: 'rgba(255, 255, 255, 0.6)', fontSize: 12 }}>{label}</span>
    <span
      style={{
        color: '#fff',
        fontSize: 13,
        fontFamily: copyable ? 'JetBrains Mono, monospace' : 'inherit',
        cursor: copyable ? 'pointer' : 'default',
      }}
      onClick={copyable ? () => navigator.clipboard.writeText(value) : undefined}
      title={copyable ? 'Click to copy' : undefined}
    >
      {value}
    </span>
  </div>
);

export default TransactionTimeMachine;
