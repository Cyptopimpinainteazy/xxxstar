import { useState, useRef, useEffect } from 'react';
import { Search } from 'lucide-react';
import { api, type SearchResults } from '../api/client';

interface SearchBarProps {
  onNavigate: (panel: string, id?: string) => void;
}

export function SearchBar({ onNavigate }: SearchBarProps) {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchResults | null>(null);
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>();

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, []);

  const search = (q: string) => {
    setQuery(q);
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    if (q.length < 2) { setResults(null); setOpen(false); return; }
    timeoutRef.current = setTimeout(async () => {
      try {
        const res = await api.search(q);
        setResults(res);
        setOpen(true);
      } catch { setOpen(false) }
    }, 300);
  };

  const hasResults = results && (
    results.blocks.length > 0 || results.transactions.length > 0
  );

  return (
    <div ref={ref} style={{ position: 'relative', width: 300 }}>
      <div style={{
        display: 'flex', alignItems: 'center', gap: 6,
        background: '#3c3c3c', borderRadius: 4, padding: '4px 8px',
        border: '1px solid #555',
      }}>
        <Search size={14} color="#888" />
        <input
          value={query}
          onChange={e => search(e.target.value)}
          placeholder="Search blocks, txns, accounts..."
          style={{
            flex: 1, background: 'transparent', border: 'none', outline: 'none',
            color: '#d4d4d4', fontSize: 12, fontFamily: 'monospace',
          }}
          spellCheck={false}
        />
      </div>

      {open && hasResults && (
        <div style={{
          position: 'absolute', top: '100%', left: 0, right: 0, marginTop: 4,
          background: '#252526', border: '1px solid #333', borderRadius: 6,
          boxShadow: '0 8px 24px rgba(0,0,0,0.4)', zIndex: 100, overflow: 'hidden',
        }}>
          {results!.blocks.length > 0 && (
            <div>
              <div style={{ padding: '4px 8px', fontSize: 11, color: '#888', background: '#1e1e1e' }}>BLOCKS</div>
              {results!.blocks.map(b => (
                <div key={b.number} onClick={() => { setOpen(false); onNavigate('explorer', String(b.number)) }}
                  style={{ padding: '6px 8px', cursor: 'pointer', fontSize: 12, fontFamily: 'monospace' }}
                  onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
                  onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
                >
                  #{b.number}
                </div>
              ))}
            </div>
          )}
          {results!.transactions.length > 0 && (
            <div>
              <div style={{ padding: '4px 8px', fontSize: 11, color: '#888', background: '#1e1e1e' }}>TRANSACTIONS</div>
              {results!.transactions.map(tx => (
                <div key={tx.hash} onClick={() => { setOpen(false); onNavigate('explorer') }}
                  style={{ padding: '6px 8px', cursor: 'pointer', fontSize: 12, fontFamily: 'monospace' }}
                  onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
                  onMouseLeave={e => e.currentTarget.style.background = 'transparent'}
                >
                  {tx.hash.slice(0, 16)}...
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
