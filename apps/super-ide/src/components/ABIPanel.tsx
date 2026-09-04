import { useState } from 'react';
import { Search, FileText, Loader2, ChevronDown, ChevronRight, ExternalLink, Copy } from 'lucide-react';
import { useApi } from '../hooks/useApi';
import { api, type ABIInfo } from '../api/client';

export function ABIPanel() {
  const { data: abis, loading, refresh } = useApi(() => api.abis(), []);
  const [selected, setSelected] = useState<string | null>(null);
  const [abiDetail, setAbiDetail] = useState<{ name: string; abi: unknown[]; bytecode: unknown } | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [expandedMethod, setExpandedMethod] = useState<string | null>(null);
  const [filter, setFilter] = useState('');

  const selectABI = async (name: string) => {
    setSelected(name);
    setLoadingDetail(true);
    try {
      const data = await api.abi(name);
      setAbiDetail(data);
    } catch {
      setAbiDetail(null);
    } finally {
      setLoadingDetail(false);
    }
  };

  const filteredAbis = abis?.filter(a =>
    a.name.toLowerCase().includes(filter.toLowerCase())
  ) || [];

  const copyABI = () => {
    if (abiDetail) navigator.clipboard.writeText(JSON.stringify(abiDetail.abi, null, 2));
  };

  return (
    <div style={{ display: 'flex', height: '100%', color: '#d4d4d4' }}>
      <div style={{ width: 240, borderRight: '1px solid #333', overflow: 'auto', background: '#1e1e1e', flexShrink: 0 }}>
        <div style={{ padding: '6px 10px', fontSize: 11, color: '#888', borderBottom: '1px solid #333', background: '#252526' }}>
          CONTRACT ABIs ({abis?.length || 0})
        </div>
        <div style={{ padding: '6px 8px' }}>
          <input value={filter} onChange={e => setFilter(e.target.value)}
            placeholder="Filter..."
            style={{ width: '100%', padding: '4px 6px', background: '#3c3c3c', border: '1px solid #555', borderRadius: 4, color: '#d4d4d4', fontSize: 11, outline: 'none' }}
          />
        </div>
        {loading && <div style={{ padding: 12 }}><Loader2 size={14} className="spin" /> Loading...</div>}
        {filteredAbis.map(a => (
          <div key={a.name} onClick={() => selectABI(a.name)}
            style={{
              padding: '8px 10px', borderBottom: '1px solid #2a2a2a', cursor: 'pointer',
              background: selected === a.name ? '#2a2a2a' : 'transparent',
            }}
            onMouseEnter={e => { if (selected !== a.name) e.currentTarget.style.background = '#2a2a2a' }}
            onMouseLeave={e => { if (selected !== a.name) e.currentTarget.style.background = 'transparent' }}
          >
            <div style={{ fontWeight: 500, fontSize: 12, color: '#569cd6' }}>{a.name}</div>
            <div style={{ fontSize: 10, color: '#888' }}>{a.methods?.length || 0} methods · {a.hasBytecode ? '✓ bytecode' : 'no bytecode'}</div>
          </div>
        ))}
      </div>

      <div style={{ flex: 1, overflow: 'auto', padding: 16 }}>
        {!selected && (
          <div style={{ color: '#666', fontStyle: 'italic', textAlign: 'center', marginTop: 40 }}>
            Select a contract ABI to inspect
          </div>
        )}
        {loadingDetail && <div><Loader2 size={14} className="spin" /> Loading...</div>}
        {abiDetail && (
          <>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 12 }}>
              <FileText size={18} color="#569cd6" />
              <h3 style={{ margin: 0, fontSize: 15 }}>{abiDetail.name}</h3>
              <button onClick={copyABI}
                style={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 4, padding: '4px 8px', border: '1px solid #333', borderRadius: 4, background: '#2d2d2d', color: '#ccc', cursor: 'pointer', fontSize: 11 }}
              ><Copy size={11} /> Copy ABI</button>
            </div>

            <div style={{ fontFamily: 'monospace', fontSize: 12 }}>
              {Array.isArray(abiDetail.abi) && (abiDetail.abi as Array<{ type: string; name?: string; stateMutability?: string; inputs?: Array<{ name: string; type: string }>; outputs?: Array<{ name: string; type: string }> }>).map((item, i) => {
                if (item.type === 'function') {
                  const key = item.name || `fn_${i}`;
                  const isExpanded = expandedMethod === key;
                  return (
                    <div key={i} style={{ marginBottom: 4, border: '1px solid #333', borderRadius: 4, overflow: 'hidden' }}>
                      <div onClick={() => setExpandedMethod(isExpanded ? null : key)}
                        style={{ display: 'flex', alignItems: 'center', gap: 6, padding: '6px 10px', cursor: 'pointer', background: '#252526' }}
                      >
                        {isExpanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
                        <span style={{ color: '#569cd6' }}>function</span>
                        <span style={{ color: '#dcdcaa' }}>{item.name}</span>
                        <span style={{ color: '#666' }}>({(item.inputs || []).map(i => i.type).join(', ')})</span>
                        <span style={{ color: '#4ec9b0', marginLeft: 'auto', fontSize: 10 }}>{item.stateMutability || 'nonpayable'}</span>
                      </div>
                      {isExpanded && (
                        <div style={{ padding: '6px 10px', borderTop: '1px solid #333', fontSize: 11 }}>
                          {item.inputs && item.inputs.length > 0 && (
                            <div style={{ marginBottom: 4 }}>
                              <span style={{ color: '#888' }}>Inputs:</span>
                              {item.inputs.map((inp, j) => (
                                <span key={j}> {inp.name}: <span style={{ color: '#4ec9b0' }}>{inp.type}</span>{j < item.inputs!.length - 1 ? ',' : ''}</span>
                              ))}
                            </div>
                          )}
                          {item.outputs && item.outputs.length > 0 && (
                            <div>
                              <span style={{ color: '#888' }}>Outputs:</span>
                              {item.outputs.map((out, j) => (
                                <span key={j}> {out.name || `return${j}`}: <span style={{ color: '#4ec9b0' }}>{out.type}</span>{j < item.outputs!.length - 1 ? ',' : ''}</span>
                              ))}
                            </div>
                          )}
                        </div>
                      )}
                    </div>
                  );
                }
                if (item.type === 'event') {
                  return (
                    <div key={i} style={{ padding: '4px 10px', marginBottom: 2, fontFamily: 'monospace', fontSize: 11, color: '#dcdcaa', background: '#252526', borderRadius: 4 }}>
                      <span style={{ color: '#888' }}>event</span> {item.name}({(item.inputs || []).map((i: { type: string }) => i.type).join(', ')})
                    </div>
                  );
                }
                if (item.type === 'constructor') {
                  return (
                    <div key={i} style={{ padding: '4px 10px', marginBottom: 2, fontFamily: 'monospace', fontSize: 11, color: '#888', background: '#252526', borderRadius: 4 }}>
                      <span style={{ color: '#888' }}>constructor</span>({(item.inputs || []).map((i: { type: string }) => i.type).join(', ')})
                    </div>
                  );
                }
                return null;
              })}
            </div>

            <div style={{ marginTop: 16 }}>
              <h4 style={{ fontSize: 12, color: '#888', margin: '0 0 8px' }}>Bytecode</h4>
              <pre style={{
                margin: 0, padding: 8, background: '#252526', border: '1px solid #333',
                borderRadius: 4, fontSize: 11, overflow: 'auto', maxHeight: 100,
                fontFamily: 'monospace', wordBreak: 'break-all',
              }}>
                {(() => {
                    const bc = abiDetail.bytecode;
                    if (typeof bc === 'object' && bc !== null) {
                      const obj = bc as Record<string, unknown>;
                      return String(obj.object || JSON.stringify(bc));
                    }
                    return String(bc || 'No bytecode');
                  })()}
              </pre>
            </div>
          </>
        )}
      </div>
    </div>
  );
}
