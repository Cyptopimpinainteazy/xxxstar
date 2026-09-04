import { useState } from 'react';
import { Blocks, ArrowRight, Loader2, ChevronRight, ChevronDown } from 'lucide-react';
import { useApi } from '../hooks/useApi';
import { api, type Block, type Transaction } from '../api/client';

type Tab = 'blocks' | 'transactions' | 'realtime';

export function ExplorerPanel() {
  const [tab, setTab] = useState<Tab>('blocks');
  const [selectedBlock, setSelectedBlock] = useState<number | null>(null);
  const [selectedTx, setSelectedTx] = useState<string | null>(null);
  const [expanded, setExpanded] = useState(true);

  const blocks = useApi(() => api.blocks(15), []);
  const txs = useApi(() => api.transactions(15), []);
  const blockDetail = useApi(
    () => selectedBlock ? api.block(selectedBlock) : Promise.reject(''),
    [selectedBlock]
  );
  const txDetail = useApi(
    () => selectedTx ? api.transaction(selectedTx) : Promise.reject(''),
    [selectedTx]
  );

  return (
    <div style={{ padding: 16, height: '100%', overflow: 'auto', color: '#d4d4d4' }}>
      <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 16 }}>
        <Blocks size={20} />
        <h2 style={{ margin: 0, fontSize: 16, fontWeight: 600 }}>X3 Chain Explorer</h2>
      </div>

      <div style={{ display: 'flex', gap: 4, marginBottom: 12, borderBottom: '1px solid #333', paddingBottom: 4 }}>
        {(['blocks', 'transactions', 'realtime'] as Tab[]).map(t => (
          <button key={t} onClick={() => { setTab(t); setSelectedBlock(null); setSelectedTx(null) }}
            style={{
              padding: '6px 16px', border: 'none', borderRadius: '4px 4px 0 0',
              background: tab === t ? '#2d2d2d' : 'transparent',
              color: tab === t ? '#fff' : '#888', cursor: 'pointer', fontSize: 13,
              fontWeight: tab === t ? 600 : 400,
            }}
          >
            {t === 'realtime' ? 'Real-time' : t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
      </div>

      {tab === 'blocks' && !selectedBlock && (
        <div>
          <div style={{ marginBottom: 8, display: 'flex', gap: 8, alignItems: 'center' }}>
            <input placeholder="Search block #..." style={{
              flex: 1, padding: '6px 10px', background: '#3c3c3c', border: '1px solid #555',
              borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace',
              outline: 'none',
            }} />
          </div>
          {blocks.loading && <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}><Loader2 size={14} className="spin" /> Loading...</div>}
          {blocks.error && <div style={{ color: '#f48771' }}>{blocks.error}</div>}
          {blocks.data?.map(block => (
            <div key={block.number} onClick={() => setSelectedBlock(block.number)}
              style={{ padding: '10px 12px', borderBottom: '1px solid #2a2a2a', cursor: 'pointer', borderRadius: 4 }}
              onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
              onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ color: '#569cd6', fontFamily: 'monospace', fontSize: 13 }}>#{block.number.toLocaleString()}</span>
                <span style={{ color: '#888', fontSize: 12 }}>{block.txCount} txns</span>
              </div>
              <div style={{ fontSize: 11, color: '#666', fontFamily: 'monospace', marginTop: 2, display: 'flex', gap: 6 }}>
                <span>{block.hash.slice(0, 18)}...</span>
                <span style={{ color: '#4ec9b0' }}>{block.producer.slice(0, 10)}</span>
              </div>
            </div>
          ))}
          {blocks.data && <button onClick={() => blocks.refresh()} style={{
            marginTop: 12, padding: '6px 16px', border: '1px solid #333',
            borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12,
          }}>Refresh</button>}
        </div>
      )}

      {tab === 'blocks' && selectedBlock && (
        <div>
          <button onClick={() => setSelectedBlock(null)} style={{
            marginBottom: 12, padding: '4px 10px', border: '1px solid #333',
            borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12,
          }}>← Back</button>
          {blockDetail.loading && <div>Loading...</div>}
          {blockDetail.error && !blockDetail.loading && <div style={{ color: '#f48771' }}>Block detail unavailable</div>}
          {blockDetail.data && (
            <div style={{ fontFamily: 'monospace', fontSize: 13, lineHeight: 1.8 }}>
              <div><span style={{ color: '#888' }}>Number:</span> #{blockDetail.data.number}</div>
              <div><span style={{ color: '#888' }}>Hash:</span> <span style={{ color: '#569cd6', fontSize: 12, wordBreak: 'break-all' }}>{blockDetail.data.hash}</span></div>
              <div><span style={{ color: '#888' }}>Time:</span> {new Date(blockDetail.data.timestamp).toLocaleString()}</div>
              <div><span style={{ color: '#888' }}>Txns:</span> {blockDetail.data.txCount}</div>
              <div><span style={{ color: '#888' }}>Producer:</span> {blockDetail.data.producer}</div>
            </div>
          )}
        </div>
      )}

      {tab === 'transactions' && !selectedTx && (
        <div>
          <div style={{ marginBottom: 8, display: 'flex', gap: 8, alignItems: 'center' }}>
            <input placeholder="Search tx hash..." style={{
              flex: 1, padding: '6px 10px', background: '#3c3c3c', border: '1px solid #555',
              borderRadius: 4, color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace', outline: 'none',
            }} />
          </div>
          {txs.loading && <div><Loader2 size={14} className="spin" /> Loading...</div>}
          {txs.error && <div style={{ color: '#f48771' }}>{txs.error}</div>}
          {txs.data?.map(tx => (
            <div key={tx.hash} onClick={() => setSelectedTx(tx.hash)}
              style={{ padding: '10px 12px', borderBottom: '1px solid #2a2a2a', cursor: 'pointer', borderRadius: 4 }}
              onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
              onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
            >
              <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ color: '#dcdcaa', fontFamily: 'monospace', fontSize: 12 }}>{tx.hash.slice(0, 14)}...</span>
                <span style={{ color: tx.status === 'confirmed' ? '#4ec9b0' : '#dcdcaa', fontSize: 11, padding: '1px 6px', borderRadius: 3, background: tx.status === 'confirmed' ? '#1a3a2a' : '#3a3a1a' }}>
                  {tx.status}
                </span>
              </div>
              <div style={{ fontSize: 11, color: '#666', marginTop: 2 }}>
                {tx.from.slice(0, 10)}... → {tx.to?.slice(0, 10)}... <span style={{ color: '#4ec9b0' }}>{tx.value}</span>
              </div>
            </div>
          ))}
          <button onClick={() => txs.refresh()} style={{
            marginTop: 12, padding: '6px 16px', border: '1px solid #333',
            borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12,
          }}>Refresh</button>
        </div>
      )}

      {tab === 'transactions' && selectedTx && (
        <div>
          <button onClick={() => setSelectedTx(null)} style={{
            marginBottom: 12, padding: '4px 10px', border: '1px solid #333',
            borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 12,
          }}>← Back</button>
          {txDetail.loading && <div>Loading...</div>}
          {txDetail.error && <div style={{ color: '#f48771' }}>Transaction detail unavailable</div>}
          {txDetail.data && (
            <div style={{ fontFamily: 'monospace', fontSize: 13, lineHeight: 1.8 }}>
              <div><span style={{ color: '#888' }}>Hash:</span> <span style={{ color: '#569cd6', fontSize: 12, wordBreak: 'break-all' }}>{txDetail.data.hash}</span></div>
              <div><span style={{ color: '#888' }}>Block:</span> #{txDetail.data.blockNumber}</div>
              <div><span style={{ color: '#888' }}>From:</span> {txDetail.data.from}</div>
              <div><span style={{ color: '#888' }}>To:</span> {txDetail.data.to}</div>
              <div><span style={{ color: '#888' }}>Value:</span> <span style={{ color: '#4ec9b0' }}>{txDetail.data.value} X3</span></div>
              <div><span style={{ color: '#888' }}>Status:</span> {txDetail.data.status}</div>
              <div><span style={{ color: '#888' }}>Time:</span> {new Date(txDetail.data.timestamp).toLocaleString()}</div>
            </div>
          )}
        </div>
      )}

      {tab === 'realtime' && (
        <div style={{ color: '#888', fontStyle: 'italic', padding: 20, textAlign: 'center' }}>
          <Blocks size={32} style={{ margin: '0 auto 12px', opacity: 0.3 }} />
          <div>Real-time block subscription</div>
          <div style={{ fontSize: 12, marginTop: 8 }}>Connect via WebSocket to ws://127.0.0.1:9944</div>
        </div>
      )}
    </div>
  );
}
