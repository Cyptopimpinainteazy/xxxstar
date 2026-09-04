import { useState } from 'react';
import { Code2, Play, Loader2, Copy, FileCode, FileType, Trash2 } from 'lucide-react';

const X3_EXAMPLE = `// Simple X3 token contract
contract MyToken {
    string name;
    string symbol;
    mapping(address => uint256) balances;

    function init(string _name, string _symbol) public {
        name = _name;
        symbol = _symbol;
    }

    function mint(address to, uint256 amount) public {
        balances[to] += amount;
    }

    function balanceOf(address owner) public view returns (uint256) {
        return balances[owner];
    }

    function transfer(address to, uint256 amount) public {
        require(balances[msg.sender] >= amount, "Insufficient balance");
        balances[msg.sender] -= amount;
        balances[to] += amount;
    }
}`;

const SOL_EXAMPLE = `// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

contract Storage {
    uint256 private value;

    event ValueChanged(uint256 newValue);

    function store(uint256 newValue) public {
        value = newValue;
        emit ValueChanged(newValue);
    }

    function retrieve() public view returns (uint256) {
        return value;
    }
}`;

export function CompilerPanel() {
  const [code, setCode] = useState(X3_EXAMPLE);
  const [language, setLanguage] = useState('x3');
  const [output, setOutput] = useState('');
  const [errors, setErrors] = useState('');
  const [compiling, setCompiling] = useState(false);
  const [success, setSuccess] = useState(false);

  const compile = async () => {
    setCompiling(true);
    setOutput('');
    setErrors('');
    setSuccess(false);
    try {
      const res = await fetch('http://127.0.0.1:8765/api/compile', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ code, language }),
      });
      const data = await res.json();
      setOutput(data.output || (data.success ? 'Compilation successful' : ''));
      setErrors(data.errors || '');
      setSuccess(data.success);
    } catch (e) {
      setErrors(`Error: ${e}`);
    } finally {
      setCompiling(false);
    }
  };

  const loadExample = (lang: string) => {
    setLanguage(lang);
    setCode(lang === 'x3' ? X3_EXAMPLE : SOL_EXAMPLE);
    setOutput('');
    setErrors('');
  };

  return (
    <div style={{ display: 'flex', height: '100%', color: '#d4d4d4' }}>
      <div style={{ width: 200, borderRight: '1px solid #333', overflow: 'auto', background: '#1e1e1e', flexShrink: 0 }}>
        <div style={{ padding: '6px 10px', fontSize: 11, color: '#888', borderBottom: '1px solid #333', background: '#252526' }}>
          COMPILER
        </div>
        <div style={{ padding: '8px' }}>
          <div style={{ fontSize: 11, color: '#888', marginBottom: 6 }}>Language</div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
            {[
              { id: 'x3', label: '.x3 (Native)' },
              { id: 'solidity', label: 'Solidity (EVM)' },
              { id: 'rust', label: 'Rust (SVM)' },
            ].map(l => (
              <div key={l.id} onClick={() => loadExample(l.id)}
                style={{
                  padding: '6px 8px', borderRadius: 4, cursor: 'pointer', fontSize: 12,
                  background: language === l.id ? '#37373d' : 'transparent',
                  color: language === l.id ? '#fff' : '#ccc',
                }}
                onMouseEnter={e => e.currentTarget.style.background = '#2a2a2a'}
                onMouseLeave={e => { if (language !== l.id) e.currentTarget.style.background = 'transparent' }}
              >
                <FileType size={12} style={{ marginRight: 6 }} />{l.label}
              </div>
            ))}
          </div>
        </div>
      </div>

      <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minWidth: 0 }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8, padding: '6px 12px', borderBottom: '1px solid #333', background: '#252526' }}>
          <Code2 size={14} />
          <span style={{ fontSize: 12 }}>{language === 'x3' ? 'contract.x3' : language === 'solidity' ? 'contract.sol' : 'contract.rs'}</span>
          <div style={{ marginLeft: 'auto', display: 'flex', gap: 6 }}>
            <button onClick={compile} disabled={compiling}
              style={{ display: 'flex', alignItems: 'center', gap: 4, padding: '4px 12px', border: 'none', borderRadius: 4, background: '#0e639c', color: '#fff', cursor: 'pointer', fontSize: 12, opacity: compiling ? 0.6 : 1 }}
            >{compiling ? <Loader2 size={12} className="spin" /> : <Play size={12} />} Compile</button>
          </div>
        </div>

        <div style={{ flex: 1, display: 'flex', flexDirection: 'column', minHeight: 0 }}>
          <textarea value={code} onChange={e => setCode(e.target.value)}
            spellCheck={false}
            style={{
              flex: 1, width: '100%', border: 'none', outline: 'none', resize: 'none',
              padding: 16, fontFamily: "'Fira Code', 'Consolas', monospace",
              fontSize: 14, lineHeight: 1.6, background: '#1e1e1e', color: '#d4d4d4',
              tabSize: 2,
            }}
          />

          {(output || errors) && (
            <div style={{
              height: 140, borderTop: '1px solid #333', background: '#1e1e1e',
              padding: '8px 16px', fontFamily: 'monospace', fontSize: 12,
              overflow: 'auto', whiteSpace: 'pre-wrap',
            }}>
              {output && <div style={{ color: success ? '#4ec9b0' : '#d4d4d4', marginBottom: 4 }}>{output}</div>}
              {errors && <div style={{ color: '#f48771' }}>{errors}</div>}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
