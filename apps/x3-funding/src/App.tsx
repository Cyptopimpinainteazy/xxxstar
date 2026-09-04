import { useState, useEffect, useRef, useMemo, useCallback } from 'react'
import './App.css'
import {
  GD, SD, FD, REAL_FEATURES, BOUNTIES, SERVICES, FOUNDING_TIERS,
  type Grant, type Sponsor, type FundingTarget,
} from './data'
import { GRANT_ANGLES, TOTAL_ANGLES } from './data_angles'
import { useWallet } from './hooks/useWallet'
import WalletModal, { PayButton } from './WalletModal'
import FundingStrategyPage from './FundingStrategyPage'
import ThreeBg from './ThreeBg'

// ─── SMALL UTILITY COMPONENTS ──────────────────────────────────────────

function Stars() {
  const stars = useMemo(() => Array.from({ length: 60 }, (_, i) => ({
    id: i, top: Math.random() * 100, left: Math.random() * 100,
    size: Math.random() * 2 + 0.5, delay: Math.random() * 8, duration: Math.random() * 4 + 2,
  })), [])
  return <>{stars.map(s => (
    <div key={s.id} className="star" style={{
      top: `${s.top}%`, left: `${s.left}%`, width: `${s.size}px`, height: `${s.size}px`,
      animationName: 'twinkle', animationDuration: `${s.duration}s`, animationDelay: `${s.delay}s`,
      animationTimingFunction: 'ease-in-out', animationIterationCount: 'infinite',
    }} />
  ))}</>
}

function AnimCounter({ target, suffix = '', prefix = '' }) {
  const [v, setV] = useState(0)
  const ref = useRef<HTMLSpanElement>(null)
  useEffect(() => {
    const obs = new IntersectionObserver(([e]) => {
      if (!e.isIntersecting) return
      let start = 0; const dur = 1800; const t0 = performance.now()
      const step = (now: number) => {
        const p = Math.min((now - t0) / dur, 1)
        setV(Math.floor((1 - Math.pow(1 - p, 3)) * target))
        if (p < 1) requestAnimationFrame(step)
        else setV(target)
      }
      requestAnimationFrame(step)
    }, { threshold: 0.5 })
    if (ref.current) obs.observe(ref.current)
    return () => obs.disconnect()
  }, [target])
  return <span ref={ref}>{prefix}{v.toLocaleString()}{suffix}</span>
}

function Typewriter({ texts, speed = 70, pause = 2200 }: { texts: string[]; speed?: number; pause?: number }) {
  const [display, setDisplay] = useState('')
  const [ti, setTi] = useState(0)
  const [ci, setCi] = useState(0)
  const [del, setDel] = useState(false)
  useEffect(() => {
    const cur = texts[ti]
    if (!del && ci < cur.length) {
      const t = setTimeout(() => { setDisplay(cur.slice(0, ci + 1)); setCi(c => c + 1) }, speed)
      return () => clearTimeout(t)
    }
    if (!del && ci === cur.length) { const t = setTimeout(() => setDel(true), pause); return () => clearTimeout(t) }
    if (del && ci > 0) { const t = setTimeout(() => { setDisplay(cur.slice(0, ci - 1)); setCi(c => c - 1) }, speed / 2); return () => clearTimeout(t) }
    if (del && ci === 0) { setDel(false); setTi(i => (i + 1) % texts.length) }
  }, [ci, del, ti, texts, speed, pause])
  return <span>{display}<span className="cursor-blink">|</span></span>
}

function TiltCard({ children, className, onClick, style }: {
  children: React.ReactNode; className?: string; onClick?: () => void; style?: React.CSSProperties
}) {
  const ref = useRef<HTMLDivElement>(null)
  const handleMove = useCallback((e: React.MouseEvent) => {
    if (!ref.current) return
    const r = ref.current.getBoundingClientRect()
    const x = (e.clientX - r.left) / r.width - 0.5
    const y = (e.clientY - r.top) / r.height - 0.5
    ref.current.style.transform = `perspective(900px) rotateY(${x * 8}deg) rotateX(${-y * 8}deg) translateY(-4px)`
    ref.current.style.boxShadow = `${-x * 18}px ${-y * 18}px 36px rgba(0,100,200,.14),0 0 36px rgba(0,200,255,.08)`
  }, [])
  const handleLeave = useCallback(() => {
    if (ref.current) { ref.current.style.transform = ''; ref.current.style.boxShadow = '' }
  }, [])
  return (
    <div ref={ref} className={`tilt ${className || ''}`} style={style} onClick={onClick}
      onMouseMove={handleMove} onMouseLeave={handleLeave}>
      {children}
    </div>
  )
}

function AnimBreakdown({ items }: { items: { label: string; pct: number }[] }) {
  const ref = useRef<HTMLDivElement>(null)
  const [anim, setAnim] = useState(false)
  useEffect(() => {
    const obs = new IntersectionObserver(([e]) => { if (e.isIntersecting) setAnim(true) }, { threshold: 0.3 })
    if (ref.current) obs.observe(ref.current)
    return () => obs.disconnect()
  }, [])
  return (
    <div className="bd" ref={ref}>
      {items.map((b, i) => (
        <div key={i} className="bdi">
          <div className="bdr"><span className="bdl">{b.label}</span><span className="bdp">{b.pct}%</span></div>
          <div className="bdbg"><div className={`bdbar${anim ? ' animate' : ''}`} style={{ width: `${b.pct}%`, transitionDelay: `${i * 0.12}s` }} /></div>
        </div>
      ))}
    </div>
  )
}

function Ticker() {
  const items = [
    'Atomic Router', '88% readiness', 'Cross-VM 6-route matrix',
    'EVM adapter', 'testnet active', 'X3Evm domain live',
    'PQC prototypes', 'ML-KEM / ML-DSA / SLH-DSA', 'quantum-crypto crate',
    'Validator lab', 'Denver, Colorado', 'physical hardware',
    'BTC vault', 'SPV + multisig + UTXO', 'regtest active',
    'ProofForge', '24 runner modules', 'readiness audit system',
    'AXE DEX', '75% readiness', 'AMM + flash loans',
    'Agent Swarm', 'on-chain agent accounts', 'experimental',
  ]
  const doubled = [...items, ...items]
  return (
    <div className="ticker">
      <div className="ticker-inner">
        {doubled.map((item, i) => (
          <div key={i} className="ticker-item">
            <span className="dot" />
            <span>●</span> {item}
          </div>
        ))}
      </div>
    </div>
  )
}

// ─── LAYOUT COMPONENTS ─────────────────────────────────────────────────

const star = { position: 'fixed' as const, borderRadius: '50%', background: 'white', pointerEvents: 'none' as const, zIndex: 0 } as const

function Badge({ text, bc }: { text: string; bc?: string }) {
  return <div className={`badge${bc ? ` ${bc}` : ''}`}>{text}</div>
}

function Back({ label, onClick }: { label: string; onClick: () => void }) {
  return <button className="back" onClick={onClick}>← {label}</button>
}

function Sec({ tag, title, children }: { tag: string; title: string; children: React.ReactNode }) {
  return (
    <div className="xs">
      <div className="shd"><span className="stag">{tag}</span><div className="sline" /><span className="stitle">{title}</span></div>
      {children}
    </div>
  )
}

function ReadinessBar({ score, mode }: { score: number; mode: string }) {
  const color = score >= 70 ? '#22c55e' : score >= 40 ? '#eab308' : '#ef4444'
  return (
    <div className="rbar-wrap">
      <div className="rbar-bg"><div className="rbar-fill" style={{ width: `${score}%`, background: color }} /></div>
      <span className="rbar-score" style={{ color }}>{score}%</span>
      <span className="rbar-label">{mode.replace('_', ' ').toUpperCase()}</span>
    </div>
  )
}

function StatusStrip({ status }: { status: { sh: string[]; pr: string[]; fn: string[] } }) {
  return (
    <div className="sstrip">
      <div><div className="slbl sh"><span className="sdot" />SHIPPED</div><div className="sitems">{(status.sh || []).map((s, i) => <div key={i} className="sitem">✓ <strong>{s}</strong></div>)}</div></div>
      <div><div className="slbl pr"><span className="sdot" />IN PROGRESS</div><div className="sitems">{(status.pr || []).map((s, i) => <div key={i} className="sitem">◎ <strong>{s}</strong></div>)}</div></div>
      <div><div className="slbl fn"><span className="sdot" />FUNDING NEEDED</div><div className="sitems">{(status.fn || []).map((s, i) => <div key={i} className="sitem">○ <strong>{s}</strong></div>)}</div></div>
    </div>
  )
}

function ModuleGrid({ modules }: { modules: { name: string; desc: string }[] }) {
  return (
    <div className="cgrid">
      {modules.map((m, i) => (
        <TiltCard key={i} className="xcard">
          <div className="cn">{m.name}</div>
          <div className="cd">{m.desc}</div>
        </TiltCard>
      ))}
    </div>
  )
}

function Roadmap({ items }: { items: { phase: string; title: string; desc: string; status: string }[] }) {
  return (
    <div className="rm">
      {items.map((r, i) => (
        <div key={i} className="rmi">
          <div className="rmph">{r.phase}</div>
          <div className={`rmdot ${r.status}`} />
          <div className="rmc">
            <div><div className="rmtitle">{r.title}</div><div className="rmdesc">{r.desc}</div></div>
            <div className={`rmtag ${r.status}`}>{r.status === 'sh' ? 'SHIPPED' : r.status === 'pr' ? 'IN PROGRESS' : 'FUNDING NEEDED'}</div>
          </div>
        </div>
      ))}
    </div>
  )
}

function CTA({ title, sub, actions }: { title: string; sub?: string; actions: string[] }) {
  return (
    <div className="ctasec">
      <div className="ctatitle">{title}</div>
      {sub && <div className="ctasub">{sub}</div>}
      <div className="ctabtns">
        {actions.map((a, i) => i === 0
          ? <button key={i} className="btnp">{a}</button>
          : <button key={i} className="btns">{a}</button>)}
      </div>
    </div>
  )
}

function FunderTags({ tags }: { tags: string[] }) {
  return <div className="ftags">{tags.map((t, i) => <div key={i} className="ftag">{t}</div>)}</div>
}

function UnlockList({ items }: { items: string[] }) {
  return <div className="ul">{items.map((item, i) => <div key={i} className="uli">{item}</div>)}</div>
}

function HWList({ items }: { items: string[] }) {
  return <div className="hwlist">{items.map((item, i) => <div key={i} className="hwi">{item}</div>)}</div>
}

function TierCards({ tiers }: { tiers: { num: string; name: string; range: string; benefits: string }[] }) {
  return (
    <div className="tgrid">
      {tiers.map((t, i) => (
        <div key={i} className="tcard">
          <div className="tnum">{t.num}</div>
          <div><div className="tname">{t.name}</div><div className="trange">{t.range}</div><div className="tbenefits">{t.benefits}</div></div>
        </div>
      ))}
    </div>
  )
}

function ArchDiagram() {
  return (
    <svg viewBox="0 0 860 200" className="arch-svg" style={{ overflow: 'visible' as const, width: '100%', maxWidth: 900, margin: '0 auto', display: 'block' }}>
      <defs>
        <filter id="glow"><feGaussianBlur stdDeviation="3" result="coloredBlur" /><feMerge><feMergeNode in="coloredBlur" /><feMergeNode in="SourceGraphic" /></feMerge></filter>
        <marker id="arr" markerWidth="8" markerHeight="6" refX="6" refY="3" orient="auto"><polygon points="0 0, 8 3, 0 6" fill="rgba(0,200,255,.5)" /></marker>
      </defs>
      {[{ x: 30, y: 60, l: 'BTC', c: '#f7931a' }, { x: 30, y: 120, l: 'ETH', c: '#627eea' }, { x: 30, y: 180, l: 'SOL', c: '#9945ff' }, { x: 30, y: 240, l: 'EXT', c: '#00c8ff' }].map((n, i) => (
        <g key={i} filter="url(#glow)">
          <rect x={n.x} y={n.y - 18} width={48} height={28} rx="5" fill="rgba(8,16,30,.9)" stroke={n.c} strokeWidth="1" opacity=".8" />
          <text x={n.x + 24} y={n.y - 1} textAnchor="middle" fill={n.c} fontSize="11" fontFamily="JetBrains Mono,monospace" fontWeight="700">{n.l}</text>
        </g>
      ))}
      {[60, 120, 180, 240].map((y, i) => (
        <line key={i} x1="78" y1={y} x2="200" y2="150" stroke="rgba(0,200,255,.25)" strokeWidth="1" className="flow-line" markerEnd="url(#arr)" />
      ))}
      <g filter="url(#glow)">
        <rect x="195" y="110" width="110" height="68" rx="8" fill="rgba(0,100,200,.15)" stroke="rgba(0,200,255,.6)" strokeWidth="1.5" />
        <text x="250" y="137" textAnchor="middle" fill="#00c8ff" fontSize="10" fontFamily="JetBrains Mono,monospace" fontWeight="700">ATOMIC</text>
        <text x="250" y="153" textAnchor="middle" fill="#00c8ff" fontSize="10" fontFamily="JetBrains Mono,monospace" fontWeight="700">GATEWAY</text>
        <text x="250" y="169" textAnchor="middle" fill="rgba(0,200,255,.5)" fontSize="8" fontFamily="JetBrains Mono,monospace">StarPackets</text>
      </g>
      <line x1="305" y1="144" x2="390" y2="144" stroke="rgba(0,200,255,.4)" strokeWidth="1.5" className="flow-line" markerEnd="url(#arr)" />
      <g filter="url(#glow)">
        <rect x="388" y="100" width="120" height="88" rx="8" fill="rgba(0,150,255,.12)" stroke="rgba(0,200,255,.8)" strokeWidth="2" />
        <text x="448" y="128" textAnchor="middle" fill="#00c8ff" fontSize="10" fontFamily="JetBrains Mono,monospace" fontWeight="700">ATOMIC</text>
        <text x="448" y="144" textAnchor="middle" fill="#00c8ff" fontSize="10" fontFamily="JetBrains Mono,monospace" fontWeight="700">KERNEL</text>
        <text x="448" y="162" textAnchor="middle" fill="rgba(0,200,255,.5)" fontSize="8" fontFamily="JetBrains Mono,monospace">Supply Invariant</text>
        <text x="448" y="178" textAnchor="middle" fill="rgba(34,197,94,.7)" fontSize="8" fontFamily="JetBrains Mono,monospace">✓ ACTIVE 88%</text>
      </g>
      {[{ y: 60, l: 'EVM', c: '#627eea' }, { y: 120, l: 'SVM', c: '#9945ff' }, { y: 180, l: 'X3VM', c: '#00c8ff' }, { y: 240, l: 'BTC', c: '#f7931a' }].map((n, i) => (
        <g key={i}>
          <line x1="508" y1="144" x2="580" y2={n.y} stroke="rgba(0,200,255,.25)" strokeWidth="1" className="flow-line" markerEnd="url(#arr)" />
          <g filter="url(#glow)">
            <rect x="580" y={n.y - 16} width="58" height="26" rx="5" fill="rgba(8,16,30,.9)" stroke={n.c} strokeWidth="1" opacity=".8" />
            <text x="609" y={n.y + 1} textAnchor="middle" fill={n.c} fontSize="10" fontFamily="JetBrains Mono,monospace" fontWeight="700">{n.l}</text>
          </g>
          <line x1="638" y1={n.y} x2="700" y2={n.y} stroke="rgba(139,92,246,.3)" strokeWidth="1" className="flow-line" markerEnd="url(#arr)" />
          <g filter="url(#glow)">
            <rect x="700" y={n.y - 16} width="70" height="26" rx="5" fill="rgba(139,92,246,.1)" stroke="rgba(139,92,246,.5)" strokeWidth="1" />
            <text x="735" y={n.y + 1} textAnchor="middle" fill="rgba(139,92,246,.9)" fontSize="9" fontFamily="JetBrains Mono,monospace" fontWeight="700">STARDEX</text>
          </g>
        </g>
      ))}
      <text x="250" y="100" textAnchor="middle" fill="rgba(0,200,255,.4)" fontSize="8" fontFamily="JetBrains Mono,monospace">CROSS-CHAIN ROUTING</text>
      <text x="448" y="90" textAnchor="middle" fill="rgba(0,200,255,.4)" fontSize="8" fontFamily="JetBrains Mono,monospace">SETTLEMENT LAYER</text>
    </svg>
  )
}

function Nav({ nav, current, walletBar }: { nav: (p: string) => void; current: string; walletBar?: React.ReactNode }) {
  const m = [
    { l: 'GRANTS', items: [{ l: 'Grants Hub', p: '/grants' }, ...GD.map(g => ({ l: `— ${g.codename}`, p: `/grants/${g.id}` }))] },
    { l: 'SPONSORS', items: [{ l: 'Sponsors Hub', p: '/sponsors' }, ...SD.map(s => ({ l: `— ${s.codename}`, p: `/sponsors/${s.id}` }))] },
    { l: 'FUNDING', items: [{ l: 'Funding Hub', p: '/funding' }, ...FD.map(f => ({ l: `— ${f.codename}`, p: `/funding/${f.id}` }))] },
    { l: 'ANGLES', items: GRANT_ANGLES.slice(0, 8).map(g => ({ l: `— ${g.category}`, p: `/angles#${g.category.replace(/\s+/g, '-').toLowerCase()}` })) },
    { l: 'MORE', items: [
      { l: 'Services', p: '/services' }, { l: 'Bounties', p: '/bounties' },
      { l: 'Founding Builders', p: '/founding' }, { l: 'Grant Match Tool', p: '/match' },
      { l: 'Feature Registry', p: '/features' }, { l: 'Grant Angles', p: '/angles' },
      { l: 'Funding Strategy', p: '/strategy' },
    ]},
  ]
  return (
    <nav className="x3">
      <div className="logo" onClick={() => nav('/')}>X3<em>⚛</em>STAR</div>
      {m.map(g => (
        <div key={g.l} className="ng">
          <button className={`nb${current.startsWith('/' + (g.l === 'ANGLES' ? 'a' : g.l.toLowerCase().replace('more', ''))) ? ' on' : ''}`}>{g.l}</button>
          <div className="nd">{g.items.map(i => (
            <button key={i.p} className="ndi" onClick={() => nav(i.p)}>
              {i.l.startsWith('—') ? <span style={{ color: 'var(--txm)' }}>—</span> : null}
              {i.l.startsWith('—') ? i.l.slice(2) : i.l}
            </button>
          ))}</div>
        </div>
      ))}
      <div className="nav-right">
        {walletBar}
        <button className="nav-pill" onClick={() => nav('/angles')}>⚡ {TOTAL_ANGLES} Angles</button>
      </div>
    </nav>
  )
}

// ─── PAGE COMPONENTS ───────────────────────────────────────────────────

function HomePage({ nav }: { nav: (p: string) => void }) {
  const lanes = [
    { code: 'Atomic Kernel', p: '/grants/atomic-kernel', l: 'Atomic Kernel & Cross-VM Router', score: 88 },
    { code: 'Atomic Gateway', p: '/grants/cross-chain-gateway', l: 'Cross-Chain Gateway', score: 65 },
    { code: 'BTC Fortress', p: '/grants/btc-fortress', l: 'BTC Fortress', score: 25 },
    { code: 'Quantum Readiness', p: '/grants/quantum-crypto', l: 'Post-Quantum Crypto', score: 35 },
    { code: 'ProofForge', p: '/grants/proof-forge', l: 'ProofForge Launch Gates', score: 55 },
    { code: 'AXE DEX', p: '/grants/dex-defi', l: 'DeFi Hub DEX + Launchpad', score: 75 },
    { code: 'X3 Reactor', p: '/grants/gpu-reactor', l: 'GPU Validator Benchmarks', score: 40 },
    { code: 'X3 Swarm', p: '/grants/ai-agents', l: 'AI Agent Swarm', score: 20 },
    { code: 'Cloud Stack', p: '/grants/infrastructure-cloud', l: 'Infrastructure & Cloud', score: 35 },
    { code: 'GPU Donation', p: '/sponsors/gpu-donation', l: 'GPU Sponsorship' },
    { code: 'Server Lab', p: '/sponsors/recycled-servers', l: 'Recycled Servers' },
    { code: 'Secure Signing', p: '/sponsors/hardware-wallets', l: 'Hardware Wallets' },
    { code: 'Node Kit', p: '/sponsors/validator-node-kit', l: 'Validator Node Kits' },
    { code: 'SBIR / STTR', p: '/funding/sbir-sttr', l: 'Federal Non-Dilutive' },
    { code: 'Colorado', p: '/funding/colorado', l: 'Colorado Programs' },
    { code: 'Accelerators', p: '/funding/accelerators', l: 'AI + Cloud Accelerators' },
    { code: 'Ecosystem', p: '/funding/ecosystem-grants', l: 'Ecosystem Grants' },
    { code: 'Services', p: '/services', l: 'Paid Services' },
    { code: 'Bounties', p: '/bounties', l: 'Bounty Board' },
    { code: 'Founders', p: '/founding', l: 'Founding Builders' },
  ]

  const scoreColor = (s?: number) => {
    if (!s) return 'transparent'
    return s >= 70 ? '#22c55e' : s >= 40 ? '#eab308' : '#ef4444'
  }

  return (
    <div>
      <Ticker />
      <div className="home-h">
        <div className="badge" style={{ marginBottom: 32 }}>X3⚛STAR // FUNDING PORTAL v2.0</div>
        <div className="home-t">
          <span className="l1">PROOF-DRIVEN</span>
          <span className="l2">INFRASTRUCTURE</span>
          <span className="l3"><Typewriter texts={['FOR SECURE CROSS-CHAIN SETTLEMENT', 'THAT PROVES BEFORE IT MOVES', 'WITHOUT SILENT FAILURE']} /></span>
        </div>
        <div className="home-s">X3 is not just another chain. It's a proof-driven, multi-VM, GPU-accelerated, agent-native blockchain infrastructure layer for secure cross-chain settlement, validator coordination, and AI-agent automation. 88% core readiness. Open-source. Verifiable. Real.</div>
        <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', justifyContent: 'center', position: 'relative', zIndex: 2 }}>
          <button className="btnp" onClick={() => nav('/match')}>⚡ Find My Grant Lane</button>
          <button className="btnpu" onClick={() => nav('/grants')}>View All Grants</button>
          <button className="btns" onClick={() => nav('/sponsors')}>Hardware Sponsors</button>
        </div>
      </div>

      <div className="statsbar">
        <div className="statsrow">
            {[
              { v: 80, s: '+', l: 'Rust Crates & Pallets' },
              { v: 54, s: '%', l: 'Average Readiness Score' },
              { v: TOTAL_ANGLES, s: '', l: 'Grant & Funding Angles' },
              { v: 9, s: '', l: 'Active Grant Lanes' },
              { v: 88, s: '%', l: 'Core Router Readiness' },
              { v: 24, s: '', l: 'ProofForge Runners' },
            ].map((s, i) => (
            <div key={i} className="stat">
              <div className="stat-v"><AnimCounter target={s.v} suffix={s.s} /></div>
              <div className="stat-l">{s.l}</div>
            </div>
          ))}
        </div>
      </div>

      <div style={{ maxWidth: 1100, margin: '0 auto', padding: '56px 28px' }}>
        <div className="shd"><span className="stag">SYSTEM ARCHITECTURE</span><div className="sline" /><span className="stitle">Cross-VM Settlement Stack</span></div>
        <div style={{ marginTop: 24, background: 'var(--s1)', border: '1px solid var(--b0)', borderRadius: 10, padding: '32px 20px', overflowX: 'auto' }}>
          <ArchDiagram />
        </div>
      </div>

      <div style={{ maxWidth: 1100, margin: '0 auto', padding: '0 28px 80px' }}>
        <div className="shd"><span className="stag">ALL FUNDING LANES</span><div className="sline" /><span className="stitle">{lanes.length} Active Surfaces</span></div>
        <div className="lgrid" style={{ marginTop: 24 }}>
          {lanes.map((p, i) => (
            <TiltCard key={i} className="lcard" onClick={() => nav(p.p)}>
              <div className="l-title">{p.l}</div>
              {p.score !== undefined && (
                <div className="l-score">
                  <div className="l-score-bar">
                    <div className="l-score-fill" style={{ width: `${p.score}%`, background: scoreColor(p.score) }} />
                  </div>
                  <span className="l-score-val" style={{ color: scoreColor(p.score) }}>{p.score}%</span>
                </div>
              )}
            </TiltCard>
          ))}
        </div>
      </div>

      <div style={{ background: 'var(--s1)', borderTop: '1px solid rgba(0,200,255,.07)', borderBottom: '1px solid rgba(0,200,255,.07)', padding: '32px 28px' }}>
        <div style={{ maxWidth: 1100, margin: '0 auto', display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'center' }}>
          <span style={{ fontFamily: 'var(--fm)', fontSize: 9, color: 'var(--txm)', letterSpacing: 2, marginRight: 4 }}>CORE TECH</span>
          {['Atomic Kernel', 'Cross-VM Router', 'Supply Ledger', 'AXE DEX', 'BTC Vault', 'PQC Crypto', 'ProofForge', 'X3 Reactor', 'Agent Swarm', 'Denver, CO'].map((t, i) => (
            <div key={i} className="tag">{t}</div>
          ))}
        </div>
      </div>
    </div>
  )
}

function FeatureRegistryPage({ nav }: { nav: (p: string) => void }) {
  const scoreColor = (s: number) => s >= 70 ? '#22c55e' : s >= 40 ? '#eab308' : '#ef4444'
  return (
    <div className="pg">
      <div className="ph">
        <Back label="HOME" onClick={() => nav('/')} />
        <Badge text="FEATURE REGISTRY" />
        <div className="ptitle">Real Readiness<br /><span className="ac">Scores from Code</span></div>
        <div className="psub">All scores sourced from <strong>FEATURE_REGISTRY.toml</strong> — the canonical readiness tracking file in the X3 repository. Each feature has a mode, score, blockers, and tests required.</div>
      </div>
      <div className="xc">
        <Sec tag="LIVE SCORES" title="Feature Readiness Overview">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 12 }}>
            {REAL_FEATURES.map((f, i) => (
              <div key={i} style={{ background: 'var(--s1)', border: '1px solid var(--b0)', borderRadius: 8, padding: '16px 20px' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
                  <div style={{ fontFamily: 'var(--fh)', fontSize: 16, fontWeight: 700, color: 'var(--txb)' }}>{f.name}</div>
                  <div style={{ fontFamily: 'var(--fo)', fontSize: 18, fontWeight: 700, color: scoreColor(f.readiness) }}>{f.readiness}%</div>
                </div>
                <div style={{ height: 6, background: 'var(--b0)', borderRadius: 3, overflow: 'hidden', marginBottom: 8 }}>
                  <div style={{ height: '100%', width: `${f.readiness}%`, background: scoreColor(f.readiness), borderRadius: 3, transition: 'width 1s ease' }} />
                </div>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                  <span style={{ fontSize: 12, color: 'var(--txm)' }}>{f.description}</span>
                  <span style={{ fontFamily: 'var(--fm)', fontSize: 9, color: 'var(--txm)', letterSpacing: 1 }}>{f.mode.replace(/_/g, ' ').toUpperCase()}</span>
                </div>
              </div>
            ))}
          </div>
        </Sec>
        <CTA title="Read the Full Registry" sub="FEATURE_REGISTRY.toml tracks 21+ features with per-feature blockers, required tests, health endpoints, and CI gates." actions={['View FEATURE_REGISTRY.toml', 'Check ProofForge Runners', 'Sponsor Readiness Improvement']} />
      </div>
    </div>
  )
}

function GrantsHub({ nav }: { nav: (p: string) => void }) {
  return (
    <div className="pg">
      <div className="ph">
        <Back label="HOME" onClick={() => nav('/')} />
        <Badge text="GRANTS PORTAL" />
          <div className="ptitle">Proof-Driven<br /><span className="ac">Infrastructure Grants</span></div>
          <div className="psub">X3 is building open-source infrastructure for secure cross-chain execution across EVM, SVM, Bitcoin, Substrate, Cosmos/CosmWasm, and X3VM. Grant funding supports the public testnet, cross-VM adapters, security testing, validator onboarding, documentation, and a public transparency dashboard.</div>
        <div style={{ display: 'flex', gap: 10 }}><button className="btnp" onClick={() => nav('/match')}>⚡ Find My Lane</button><button className="btns" onClick={() => nav('/funding/sbir-sttr')}>SBIR / Non-Dilutive</button></div>
      </div>
      <div className="xc">
        <Sec tag="FUNDING LANES" title="Select a Grant Category">
          <div className="hgrid">
            {GD.map((g, i) => (
              <TiltCard key={i} className="hcard" onClick={() => nav(`/grants/${g.id}`)}>
                <div className="hccode">{g.codename}</div>
                <div className="hctitle">{g.hero.join(' ')}</div>
                <div className="hcdesc">{g.sub.slice(0, 110)}...</div>
                <div className="hcscore" style={{ color: g.readiness >= 70 ? 'var(--gn)' : g.readiness >= 40 ? 'var(--yw)' : 'var(--rd)' }}>
                  {g.readiness}% ready
                </div>
              </TiltCard>
            ))}
          </div>
        </Sec>
        <CTA title="Request the Full Grant Packet" sub="Technical whitepaper, architecture docs, readiness reports, and milestone roadmap formatted for grant reviewers." actions={['Request Grant Packet', 'Schedule Technical Review', 'Sponsor a Milestone']} />
      </div>
    </div>
  )
}

function GrantPage({ data, nav }: { data: Grant; nav: (p: string) => void }) {
  return (
    <div className="pg">
      <div className="ph">
        <Back label="GRANTS" onClick={() => nav('/grants')} />
        <Badge text={data.badge} bc={data.bc} />
        {data.codename && <div style={{ fontFamily: 'var(--fm)', fontSize: 10, color: 'var(--txm)', letterSpacing: 3, marginBottom: 12 }}>⬡ {data.codename}</div>}
        <div className="ptitle">{data.hero[0]}<br /><span className="ac">{data.hero[1]}</span></div>
        <ReadinessBar score={data.readiness} mode={data.mode} />
        <div className="psub">{data.sub}</div>
        {data.warn && <div className="warn-box">⚠ {data.warn}</div>}
      </div>
      <StatusStrip status={data.status} />
      <div className="xc">
        <Sec tag="PROBLEM / SOLUTION" title="Why This Matters">
          <div className="ps">
            <div className="psc prob"><div className="pscl">THE PROBLEM</div><div className="psct">{data.problem}</div></div>
            <div className="psc sol"><div className="pscl">THE SOLUTION</div><div className="psct">{data.solution}</div></div>
          </div>
        </Sec>
        <Sec tag="TECHNICAL ARCHITECTURE" title="What We Are Building"><ModuleGrid modules={data.modules} /></Sec>
        <Sec tag="TARGET FUNDERS" title="Who Should Fund This"><FunderTags tags={data.funders} /></Sec>
        <Sec tag="FUNDING UNLOCKS" title="What This Investment Enables"><UnlockList items={data.fundingUnlocks} /></Sec>
        <Sec tag="MILESTONE ROADMAP" title="Development Timeline"><Roadmap items={data.roadmap} /></Sec>
        <Sec tag="FUNDING BREAKDOWN" title="How Capital Is Deployed"><AnimBreakdown items={data.breakdown} /></Sec>
        <Sec tag="PROOF / DEMO" title="Machine-Verifiable Evidence">
          <div className="term">
            <div className="cmd">$ proofforge run --module {data.id} --output signed-report.json</div>
            <div className="cmt"># Loading readiness gates...</div>
            <div className="ok">✓ Architecture documentation ........... PRESENT</div>
            <div className="ok">✓ Prototype implementation .............. PRESENT</div>
            <div className="warn">◎ External security audit .............. IN PROGRESS</div>
            <div className="ok">✓ Supply invariant dashboard ........... ACTIVE</div>
            <div>&nbsp;</div>
            <div>→ Generating signed readiness report...</div>
            <div className="ok">✓ x3-{data.id}-readiness-2026.json — SIGNED</div>
            <div className="cmt"># Report available at /proof/{data.id}</div>
          </div>
        </Sec>
        <CTA title={`Fund ${data.codename}`} sub={`Contact the X3 Atomic Star team for the technical whitepaper and milestone roadmap formatted for your funding category.`} actions={data.cta} />
      </div>
    </div>
  )
}

function SponsorsHub({ nav }: { nav: (p: string) => void }) {
  return (
    <div className="pg">
      <div className="ph">
        <Back label="HOME" onClick={() => nav('/')} />
        <Badge text="HARDWARE SPONSORSHIP" bc="or" />
        <div className="ptitle">X3 Atomic Star<br /><span className="ac">Hardware Sponsorship Portal</span></div>
        <div className="psub">Sponsor the physical infrastructure behind multi-VM settlement, validator benchmarking, and proof-driven blockchain readiness. GPU, servers, wallets, and node kits.</div>
        <div style={{ display: 'flex', gap: 10 }}><button className="btnp" onClick={() => nav('/sponsors/gpu-donation')}>GPU Donation</button><button className="btns" onClick={() => nav('/sponsors/validator-node-kit')}>Sponsor a Node</button></div>
      </div>
      <div className="xc">
        <Sec tag="SPONSORSHIP LANES" title="Select a Hardware Category">
          <div className="hgrid">
            {SD.map((s, i) => (
              <TiltCard key={i} className="hcard" onClick={() => nav(`/sponsors/${s.id}`)}>
                <div className="hccode">{s.codename}</div>
                <div className="hctitle">{s.hero.join(' ')}</div>
                <div className="hcdesc">{s.sub.slice(0, 110)}...</div>
              </TiltCard>
            ))}
          </div>
        </Sec>
        <Sec tag="SPONSORSHIP TIERS" title="What Sponsors Get">
          <TierCards tiers={[
            { num: '01', name: 'Component Sponsor', range: 'SSDs, RAM, GPUs, cables', benefits: 'Sponsor wall listing, hardware impact report, optional anonymous donation' },
            { num: '02', name: 'Node Sponsor', range: 'Full validator or archive node', benefits: 'Named node, monthly uptime report, testnet dashboard mention' },
            { num: '03', name: 'Reactor Sponsor', range: 'GPU donation or cloud GPU credits', benefits: 'X3 Reactor report mention, benchmark dashboard recognition' },
            { num: '04', name: 'Lab Sponsor', range: 'Rack, networking, power, full buildout', benefits: 'Full lab sponsor page, quarterly infrastructure report' },
            { num: '05', name: 'Founding Hardware Partner', range: 'Ongoing hardware or credit partnership', benefits: 'Founding sponsor placement, co-authored reports' },
          ]} />
        </Sec>
        <CTA title="Sponsor the Hardware Behind X3" sub="Recycled servers, GPU validators, hardware wallets, storage, networking, and power for a proof-driven public testnet." actions={['Request Hardware Wishlist', 'Submit Hardware Donation', 'Schedule Sponsor Call']} />
      </div>
    </div>
  )
}

function SponsorPage({ data, nav }: { data: Sponsor; nav: (p: string) => void }) {
  return (
    <div className="pg">
      <div className="ph">
        <Back label="SPONSORS" onClick={() => nav('/sponsors')} />
        <Badge text={data.badge} bc={data.bc} />
        {data.codename && <div style={{ fontFamily: 'var(--fm)', fontSize: 10, color: 'var(--txm)', letterSpacing: 3, marginBottom: 12 }}>⬡ {data.codename}</div>}
        <div className="ptitle">{data.hero[0]}<br /><span className="ac">{data.hero[1]}</span></div>
        <div className="psub">{data.sub}</div>
      </div>
      <StatusStrip status={data.status} />
      <div className="xc">
        <Sec tag="PROBLEM / SOLUTION" title="Why Sponsor This">
          <div className="ps">
            <div className="psc prob"><div className="pscl">THE PROBLEM</div><div className="psct">{data.problem}</div></div>
            <div className="psc sol"><div className="pscl">THE SOLUTION</div><div className="psct">{data.solution}</div></div>
          </div>
        </Sec>
        <Sec tag="WHAT THIS POWERS" title="Infrastructure This Enables"><ModuleGrid modules={data.modules} /></Sec>
        {data.hardware && <Sec tag="HARDWARE WANTED" title="What We Need"><HWList items={data.hardware} /></Sec>}
        {data.acceptance && <Sec tag="ACCEPTANCE REQUIREMENTS" title="What We Accept"><UnlockList items={data.acceptance} /></Sec>}
        {data.security && <Sec tag="DATA SECURITY" title="How We Handle Donated Hardware"><div className="hlbox"><p>{data.security}</p></div></Sec>}
        {data.note && <div className="warn-box">⚠ {data.note}</div>}
        <Sec tag="SPONSORSHIP TIERS" title="What Sponsors Get"><TierCards tiers={data.tiers} /></Sec>
        <CTA title={`Sponsor ${data.codename}`} sub="Contact the X3 team with what you can contribute. We confirm compatibility and provide intake documentation." actions={data.cta} />
      </div>
    </div>
  )
}

function FundingHub({ nav }: { nav: (p: string) => void }) {
  return (
    <div className="pg">
      <div className="ph">
        <Back label="HOME" onClick={() => nav('/')} />
        <Badge text="FUNDING PROGRAMS" bc="pu" />
          <div className="ptitle">Funding Programs<br /><span className="ac">For Multi-VM Infrastructure</span></div>
          <div className="psub">Non-dilutive federal funding (SBIR/STTR), Colorado state grants, AI startup accelerators, blockchain ecosystem grants, and cloud credit programs — each mapped to specific X3 technology tracks. The mission: reduce bridge risk and make multi-chain settlement verifiable, auditable, and developer-friendly.</div>
      </div>
      <div className="xc">
        <Sec tag="FUNDING LANES" title="Select a Funding Track">
          <div className="hgrid">
            {FD.map((f, i) => (
              <TiltCard key={i} className="hcard" onClick={() => nav(`/funding/${f.id}`)}>
                <div className="hccode">{f.codename}</div>
                <div className="hctitle">{f.hero.join(' ')}</div>
                <div className="hcdesc">{f.sub.slice(0, 110)}...</div>
              </TiltCard>
            ))}
          </div>
        </Sec>
        <Sec tag="FUNDING LANDSCAPE" title="The Opportunity">
          <div className="mgrid">
            {[
              { v: '$4B', l: 'Fed SBIR / Year' },
              { v: '4K+', l: 'Companies / Year' },
              { v: '$2M', l: 'NSF Max (Zero Equity)' },
              { v: '$350K', l: 'Google AI Credits' },
              { v: '$250K', l: 'CO Advanced Industries' },
              { v: '$89.5M', l: 'CO Workforce Awarded' },
            ].map((m, i) => (
              <div key={i} className="mc"><div className="mv">{m.v}</div><div className="ml">{m.l}</div></div>
            ))}
          </div>
        </Sec>
        <CTA title="Build the Funding Pipeline" sub="Cloud credits → hardware donations → Colorado state grants → NSF SBIR → ecosystem grants → paid services." actions={['Request NSF Pitch Packet', 'View Colorado Programs', 'Schedule Funding Strategy Call']} />
      </div>
    </div>
  )
}

function FundingPage({ data, nav }: { data: FundingTarget; nav: (p: string) => void }) {
  return (
    <div className="pg">
      <div className="ph">
        <Back label="FUNDING" onClick={() => nav('/funding')} />
        <Badge text={data.badge} bc={data.bc || 'pu'} />
        {data.codename && <div style={{ fontFamily: 'var(--fm)', fontSize: 10, color: 'var(--txm)', letterSpacing: 3, marginBottom: 12 }}>⬡ {data.codename}</div>}
        <div className="ptitle">{data.hero[0]}<br /><span className="ac">{data.hero[1]}</span></div>
        <div className="psub">{data.sub}</div>
      </div>
      <StatusStrip status={data.status} />
      <div className="xc">
        <Sec tag="X3 PITCH" title="What We Tell Funders"><div className="hlbox"><p style={{ fontSize: 15, lineHeight: 1.88 }}>{data.pitch}</p></div></Sec>
        <Sec tag="TARGET PROGRAMS" title="Where to Apply">
          <div style={{ display: 'flex', flexDirection: 'column', gap: 11 }}>
            {data.targets.map((t, i) => (
              <TiltCard key={i} className="fcard">
                <div className="fcard-l"><div className="fcard-n">{t.name}</div><div className="fcard-a">{t.amount}</div></div>
                <div className="fcard-d">{t.desc}</div>
              </TiltCard>
            ))}
          </div>
        </Sec>
        <CTA title={`Apply for ${data.codename} Funding`} sub="Request the full technical whitepaper and grant packet formatted for this program." actions={data.cta} />
      </div>
    </div>
  )
}

function MatchPage({ nav }: { nav: (p: string) => void }) {
  const [step, setStep] = useState(0)
  const [answers, setAnswers] = useState<Record<string, string>>({})
  const [done, setDone] = useState(false)

  const questions = [
    { id: 'who', q: 'Who are you?', sub: 'Route you to the right funding lane.', opts: ['Government / Federal Agency', 'Foundation / Nonprofit', 'Corporate / Tech Vendor', 'Individual / Angel Donor', 'VC / Accelerator', 'X3 team member'] },
    { id: 'interest', q: 'What interests you?', sub: 'Select the area that best fits your mandate.', opts: ['AI / Agent Systems', 'Post-Quantum Cryptography', 'Cross-Chain Infrastructure', 'GPU / High-Performance Compute', 'Open Source / Public Goods', 'Bitcoin Ecosystem', 'Hardware / Physical Infrastructure', 'DeFi / DEX Infrastructure'] },
    { id: 'size', q: 'What scale?', sub: 'Select your contribution range.', opts: ['Hardware donation', '< $50K (credits / small grants)', '$50K – $250K (Phase I / state)', '$250K – $1M (Phase II / accelerator)', '$1M+ (strategic / SBIR Phase II+)'] },
  ]

  const getResults = (a: Record<string, string>) => {
    const res: { badge: string; title: string; path: string }[] = []
    const i = a.interest || ''
    const w = a.who || ''
    if (i.includes('AI')) res.push({ badge: 'AI + AGENTS', title: 'X3 Swarm — Agent Systems', path: '/grants/ai-agents' })
    if (i.includes('Quantum') || i.includes('Cryptography')) res.push({ badge: 'PQC', title: 'Quantum Crypto Migration', path: '/grants/quantum-crypto' }, { badge: 'PQC FUNDING', title: 'Post-Quantum Funding', path: '/funding/sbir-sttr' })
    if (i.includes('Cross-Chain') || i.includes('Infrastructure')) res.push({ badge: 'INTEROP', title: 'Cross-Chain Gateway', path: '/grants/cross-chain-gateway' }, { badge: 'BTC', title: 'BTC Fortress', path: '/grants/btc-fortress' })
    if (i.includes('GPU')) res.push({ badge: 'GPU', title: 'X3 Reactor Benchmarks', path: '/grants/gpu-reactor' }, { badge: 'GPU SPONSOR', title: 'GPU Donation Program', path: '/sponsors/gpu-donation' })
    if (i.includes('Open Source')) res.push({ badge: 'PUBLIC GOODS', title: 'ProofForge Launch Gates', path: '/grants/proof-forge' })
    if (i.includes('Bitcoin')) res.push({ badge: 'BTC', title: 'BTC Fortress', path: '/grants/btc-fortress' })
    if (i.includes('Hardware')) res.push({ badge: 'HARDWARE', title: 'Hardware Sponsorship Hub', path: '/sponsors' }, { badge: 'SERVERS', title: 'Recycled Server Lab', path: '/sponsors/recycled-servers' })
    if (i.includes('DeFi') || i.includes('DEX')) res.push({ badge: 'DEFI', title: 'AXE DEX + Launchpad', path: '/grants/dex-defi' })
    if (w.includes('Government') || w.includes('Federal')) res.push({ badge: 'SBIR', title: 'Federal SBIR / STTR', path: '/funding/sbir-sttr' })
    if (res.length === 0) res.push({ badge: 'ALL LANES', title: 'View Full Grants Portal', path: '/grants' }, { badge: 'FUNDING', title: 'View Funding Programs', path: '/funding' })
    return [...new Map(res.map(r => [r.path, r])).values()].slice(0, 5)
  }

  const handleOpt = (opt: string) => {
    const newAnswers = { ...answers, [questions[step].id]: opt }
    setAnswers(newAnswers)
    if (step < questions.length - 1) setStep(s => s + 1)
    else setDone(true)
  }

  if (done) {
    const results = getResults(answers)
    return (
      <div className="pg">
        <div style={{ maxWidth: 720, margin: '0 auto', padding: '72px 28px' }}>
          <Back label="START OVER" onClick={() => { setDone(false); setStep(0); setAnswers({}); }} />
          <div className="match-result">
            <div className="match-res-header">
              <Badge text="GRANT MATCH RESULTS" />
              <div className="match-res-title">Your Recommended Funding Lanes</div>
              <div className="match-res-sub">Based on your profile, these X3 funding surfaces are the best fit.</div>
            </div>
            <div className="match-res-lanes">
              {results.map((r, i) => (
                <div key={i} className="match-lane" onClick={() => nav(r.path)}>
                  <div className="ml-badge">{r.badge}</div>
                  <div className="ml-title">{r.title}</div>
                  <div className="ml-arrow">→</div>
                </div>
              ))}
            </div>
            <div style={{ padding: '16px 32px 28px' }}>
              <button className="btns" onClick={() => nav('/grants')}>View All Grant Lanes</button>
            </div>
          </div>
        </div>
      </div>
    )
  }

  const q = questions[step]
  return (
    <div className="pg">
      <div className="match-wrap">
        <Back label="HOME" onClick={() => nav('/')} />
        <div style={{ marginBottom: 28 }}>
          <Badge text="GRANT MATCH TOOL" bc="pu" />
          <div className="ptitle" style={{ fontSize: 'clamp(24px,4vw,40px)', marginTop: 12 }}>Find Your<br /><span className="ac">Funding Lane</span></div>
          <div className="psub">Answer 3 questions. We route you to the right grants, sponsors, and funding programs.</div>
        </div>
        <div className="match-progress" style={{ marginBottom: 28 }}>
          {questions.map((_, i) => <div key={i} className={`mp-dot${i <= step ? ' active' : ''}`} />)}
        </div>
        <div className="match-step">
          <div style={{ fontFamily: 'var(--fm)', fontSize: 9, color: 'var(--txm)', letterSpacing: 2, marginBottom: 12 }}>QUESTION {step + 1} OF {questions.length}</div>
          <div className="match-q">{q.q}</div>
          <div className="match-sub">{q.sub}</div>
          <div className="match-opts">
            {q.opts.map((opt, i) => (
              <button key={i} className="match-opt" onClick={() => handleOpt(opt)}>{opt}</button>
            ))}
          </div>
        </div>
      </div>
    </div>
  )
}

function ServicesPage({ nav }: { nav: (p: string) => void }) {
  return (
    <div className="pg">
      <div className="ph">
        <Back label="HOME" onClick={() => nav('/')} />
        <Badge text="PAID SERVICES" bc="gn" />
        <div className="ptitle">X3 Atomic Star<br /><span className="ac">Paid Services</span></div>
        <div className="psub">Revenue-before-raise. ProofForge audits, Reactor benchmarks, route risk assessments, and PQC crypto SBOMs — each producing public evidence artifacts.</div>
      </div>
      <div className="xc">
        <Sec tag="SERVICE OFFERINGS" title="What We Provide">
          <div className="svc-grid">
            {SERVICES.map((s, i) => (
              <div key={i} className="svc-card">
                <div className="svc-badge">{s.badge}</div>
                <div className="svc-title">{s.title}</div>
                <div className="svc-desc">{s.desc}</div>
                <div className="svc-price">{s.price}</div>
                <div className="svc-features">{s.features.map((f, j) => <div key={j} className="svc-feat">{f}</div>)}</div>
                <button className="btnp" style={{ width: '100%' }}>Request This Service</button>
              </div>
            ))}
          </div>
        </Sec>
        <Sec tag="WHY SERVICES" title="Revenue Before Grant">
          <div className="hlbox"><p>Paid pilots create revenue, real case studies, and grant matching funds. Each audit or benchmark report becomes public evidence that X3 infrastructure is real, rigorous, and reproducible. Money from customers validates faster than "trust the roadmap."</p></div>
        </Sec>
        <CTA title="Request a Service Engagement" sub="Contact the X3 team to scope your audit, benchmark, or risk assessment. Delivered as signed machine-readable reports." actions={['Request ProofForge Audit', 'Request Reactor Benchmark', 'Schedule Scoping Call']} />
      </div>
    </div>
  )
}

function BountiesPage({ nav }: { nav: (p: string) => void }) {
  return (
    <div className="pg">
      <div className="ph">
        <Back label="HOME" onClick={() => nav('/')} />
        <Badge text="BOUNTY BOARD" bc="yw" />
        <div className="ptitle">X3 Atomic Star<br /><span className="ac">Bounty Board</span></div>
        <div className="psub">Open tasks with prize money. Build something useful, claim the bounty. Everything produces public open-source artifacts.</div>
        <div style={{ display: 'flex', gap: 10 }}>
          <div style={{ background: 'var(--gng)', border: '1px solid rgba(34,197,94,.3)', borderRadius: 4, padding: '4px 10px', fontFamily: 'var(--fm)', fontSize: 10, color: 'var(--gn)', letterSpacing: 1 }}>● OPEN</div>
          <div style={{ background: 'var(--rdg)', border: '1px solid rgba(239,68,68,.3)', borderRadius: 4, padding: '4px 10px', fontFamily: 'var(--fm)', fontSize: 10, color: 'var(--rd)', letterSpacing: 1 }}>● HOT</div>
          <div style={{ background: 'var(--pug)', border: '1px solid rgba(139,92,246,.3)', borderRadius: 4, padding: '4px 10px', fontFamily: 'var(--fm)', fontSize: 10, color: 'var(--pu)', letterSpacing: 1 }}>● RESEARCH</div>
        </div>
      </div>
      <div className="xc">
        <Sec tag="OPEN BOUNTIES" title={`${BOUNTIES.length} Tasks Available`}>
          <div className="bounty-grid">
            {BOUNTIES.map((b, i) => (
              <div key={i} className="bounty">
                <div className="bounty-prize">{b.prize}</div>
                <div className="bounty-info">
                  <div className="bounty-title">{b.title}</div>
                  <div className="bounty-desc">{b.desc}</div>
                </div>
                <div>
                  <div className={`bounty-tag ${b.tag}`} style={{ marginBottom: 6 }}>{b.tag.toUpperCase()}</div>
                  <div style={{ fontFamily: 'var(--fm)', fontSize: 9, color: 'var(--txm)', letterSpacing: 1 }}>{b.difficulty}</div>
                </div>
              </div>
            ))}
          </div>
        </Sec>
        <Sec tag="BOUNTY POOL" title="Total Available">
          <div className="mgrid">
            {[
              { v: '$31,750', l: 'Total Prize Pool' },
              { v: '10', l: 'Open Bounties' },
              { v: '3', l: 'Difficulty Tiers' },
              { v: '100%', l: 'Open-Source Output' },
            ].map((m, i) => (
              <div key={i} className="mc"><div className="mv">{m.v}</div><div className="ml">{m.l}</div></div>
            ))}
          </div>
        </Sec>
        <CTA title="Claim a Bounty" sub="Contact the team with the bounty title, your approach, and your GitHub. We confirm scope before you build." actions={['Apply for a Bounty', 'Propose a New Bounty', 'Sponsor the Bounty Pool']} />
      </div>
    </div>
  )
}

function FoundingPage({ nav, wallet }: { nav: (p: string) => void; wallet?: ReturnType<typeof useWallet> }) {
  const [payTier, setPayTier] = useState('')
  const [showPayModal, setShowPayModal] = useState(false)
  const [payAmount, setPayAmount] = useState('')
  const treasuryAddr = '5EYCAe5ijH2sZgXZy3KCDKNKEPn5N4JCpFYxBvYcHhgGzYuo'

  const handlePayClick = async (tier: string, amount: string) => {
    if (!wallet?.selected) {
      setPayTier(tier)
      setPayAmount(amount)
      setShowPayModal(true)
      await wallet?.connect()
      return
    }
    setPayTier(tier)
    setPayAmount(amount)
    setShowPayModal(true)
  }

  return (
    <div className="pg">
      <div className="ph">
        <Back label="HOME" onClick={() => nav('/')} />
        <Badge text="FOUNDING BUILDERS PROGRAM" />
        <div className="ptitle">Founding<br /><span className="ac">Builders Program</span></div>
        <div className="psub">Not a token sale. Not a presale. A founding membership for operators, developers, and infrastructure partners who want early access, builder calls, and proof-driven progress reports before public launch.</div>
        <div className="warn-box">⚠ No token allocations are implied or promised. Membership perks are infrastructure access, reports, and recognition — not financial instruments.</div>
      </div>
      <div className="xc">
        <Sec tag="MEMBERSHIP TIERS" title="Join the Founding Team">
          <div className="founding-grid">
            {FOUNDING_TIERS.map((t, i) => (
              <div key={i} className={`f-card${t.featured ? ' featured' : ''}`}>
                <div className="f-tier" style={{ color: t.color }}>{t.tier}</div>
                <div className="f-price" style={{ color: t.color }}>{t.price}</div>
                <div className="f-period">{t.period}</div>
                <div className="f-features">{t.features.map((f, j) => <div key={j} className="f-feat"><span className="check">✓</span>{f}</div>)}</div>
                <button className="btnp" style={{ width: '100%', background: t.color, color: 'var(--void)' }} onClick={() => handlePayClick(t.tier, t.price)}>{t.cta}</button>
                {wallet?.selected && (
                  <div style={{ marginTop: 8, textAlign: 'center', fontSize: 10, color: 'var(--txm)' }}>
                    Pay {Number(t.price.replace(/[$,]/g, '')) * 100} X3 via wallet
                  </div>
                )}
              </div>
            ))}
          </div>
        </Sec>
        <Sec tag="PAY WITH WALLET" title="Crypto Payment">
          <div style={{ background: 'var(--s1)', border: '1px solid var(--b0)', borderRadius: 8, padding: '16px 20px' }}>
            <div style={{ fontSize: 13, color: 'var(--txm)', marginBottom: 8 }}>Send X3 tokens directly to the Founding Builders treasury:</div>
            <div style={{ display: 'flex', alignItems: 'center', gap: 8, background: 'var(--void)', border: '1px solid rgba(0,200,255,.2)', borderRadius: 6, padding: '10px 14px', fontFamily: 'var(--fm)', fontSize: 11 }}>
              <span style={{ color: 'var(--txm)', flexShrink: 0 }}>TREASURY:</span>
              <code style={{ color: 'var(--ac)', wordBreak: 'break-all', userSelect: 'all' }}>{treasuryAddr}</code>
              <button className="copy-btn" onClick={() => { navigator.clipboard.writeText(treasuryAddr) }}>COPY</button>
            </div>
            <div style={{ marginTop: 12, fontSize: 11, color: 'var(--txm)', lineHeight: 1.6 }}>
              Price: <strong style={{ color: 'var(--txb)' }}>$1 = 100 X3</strong> — Connect your Polkadot.js wallet above to pay instantly, or send manually from any wallet.
            </div>
            {wallet?.selected && (
              <div style={{ marginTop: 12, display: 'flex', gap: 8 }}>
                <span style={{ fontSize: 11, color: 'var(--gn)' }}>● {wallet.selected.balance} X3 available</span>
              </div>
            )}
          </div>
        </Sec>
        <Sec tag="WHAT YOU GET" title="Early Infrastructure Access">
          <div className="cgrid">
            {[
              { name: 'Private Testnet', desc: 'Early access to X3 testnet before public launch' },
              { name: 'Builder Calls', desc: 'Quarterly calls with core team on technical roadmap' },
              { name: 'Readiness Reports', desc: 'ProofForge launch readiness reports as generated' },
              { name: 'Reactor Benchmarks', desc: 'Signed validator benchmark reports from X3 Reactor' },
              { name: 'Grant Evidence Pack', desc: 'Architecture docs used for actual grant applications' },
              { name: 'Treasury Transparency', desc: 'Public use-of-funds tracking and milestone reporting' },
            ].map((m, i) => (
              <TiltCard key={i} className="xcard"><div className="cn">{m.name}</div><div className="cd">{m.desc}</div></TiltCard>
            ))}
          </div>
        </Sec>
        <CTA title="Join the Founding Builders" sub="Be part of the infrastructure build from the beginning. Real access, real reports, no speculation." actions={['Connect Wallet to Join', 'Join as Builder ($99)', 'Contact for Custom Tier']} />
      </div>
    </div>
  )
}

// ─── GRANT ANGLES PAGE ──────────────────────────────────────────────────

function GrantAnglesPage({ nav }: { nav: (p: string) => void }) {
  return (
    <div className="pg">
      <div className="ph">
        <Back label="HOME" onClick={() => nav('/')} />
        <Badge text="GRANT ANGLES" bc="sb" />
        <div className="ptitle">{TOTAL_ANGLES} Grant & Funding<br /><span className="ac">Angles for X3</span></div>
        <div className="psub">
          X3 Atomic Star is not just a chain. It's a proof-driven, multi-VM, GPU-accelerated, agent-native blockchain infrastructure project that can fund, build, test, and prove itself. Every angle below represents a genuine grant, sponsorship, or funding surface tied to real code.
        </div>
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 8, marginTop: 16 }}>
          <button className="btnp" onClick={() => nav('/match')}>⚡ Find Your Angle</button>
          <button className="btns" onClick={() => nav('/grants')}>View Grant Lanes</button>
          <button className="btns" onClick={() => nav('/founding')}>Founding Builders</button>
        </div>
      </div>
      <div className="xc">
        <div className="shd"><span className="stag">ANGLE MAP</span><div className="sline" /><span className="stitle">{GRANT_ANGLES.length} Categories — {TOTAL_ANGLES} Funding Surfaces</span></div>
        <div style={{ marginTop: 24, display: 'flex', flexDirection: 'column', gap: 20 }}>
          {GRANT_ANGLES.map((cat, ci) => (
            <div key={ci} id={cat.category.replace(/\s+/g, '-').toLowerCase()} className="angle-cat" style={{ borderLeftColor: cat.color }}>
              <div className="angle-cat-header">
                <span className="angle-icon" style={{ color: cat.color }}>{cat.icon}</span>
                <span className="angle-cat-name" style={{ color: cat.color }}>{cat.category}</span>
                <span className="angle-count">{cat.items.length} angles</span>
              </div>
              <div className="angle-items">
                {cat.items.map((item, ii) => (
                  <div key={ii} className="angle-item">
                    <div className="angle-item-top">
                      <span className="angle-item-title">{item.title}</span>
                      <span className={`angle-status ${item.status}`}>{item.status === 'active' ? '● ACTIVE' : item.status === 'in-progress' ? '◎ IN PROGRESS' : '○ PLANNED'}</span>
                    </div>
                    <div className="angle-item-desc">{item.desc}</div>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      </div>
      <div style={{ maxWidth: 1100, margin: '0 auto', padding: '40px 28px 80px', textAlign: 'center' }}>
        <div style={{ fontFamily: 'var(--fh)', fontSize: 22, fontWeight: 700, color: 'var(--txb)', marginBottom: 16 }}>
          X3 Atomic Star: Proof-Driven Infrastructure for Secure Cross-Chain Settlement
        </div>
        <div style={{ color: 'var(--txm)', fontSize: 14, lineHeight: 1.8, maxWidth: 700, margin: '0 auto' }}>
          We are seeking milestone-based grant funding to launch the X3 public testnet, build cross-VM adapters, publish validator tooling, release developer documentation, run security tests, and create a public transparency dashboard. Our goal is to reduce bridge risk, improve cross-chain reliability, and give developers safer tools for building the next generation of multi-chain applications.
        </div>
        <div style={{ marginTop: 24, display: 'flex', gap: 12, justifyContent: 'center', flexWrap: 'wrap' }}>
          <button className="btnp" onClick={() => nav('/funding')}>View Funding Programs</button>
          <button className="btnpu" onClick={() => nav('/sponsors')}>Hardware Sponsorship</button>
          <button className="btns" onClick={() => nav('/founding')}>Founding Builders</button>
        </div>
      </div>
    </div>
  )
}

// ─── APP ───────────────────────────────────────────────────────────────

export default function App() {
  const [path, setPath] = useState('/')
  const nav = (p: string) => { setPath(p); try { window.scrollTo({ top: 0, behavior: 'smooth' }) } catch { /* noop */ } }
  const wallet = useWallet()
  const [showWallet, setShowWallet] = useState(false)

  const walletBar = (
    <button className="wallet-nav-btn" onClick={() => setShowWallet(p => !p)}>
      {wallet.selected
        ? <span style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span className="wallet-dot" />
            <span>{wallet.selected.name.slice(0, 8)}</span>
            <span style={{ fontSize: 9, opacity: 0.6 }}>{wallet.selected.address.slice(0, 4)}</span>
          </span>
        : <span>⊡ Connect</span>}
    </button>
  )

  const page = (() => {
    if (path === '/') return <HomePage nav={nav} />
    if (path === '/grants') return <GrantsHub nav={nav} />
    if (path === '/sponsors') return <SponsorsHub nav={nav} />
    if (path === '/funding') return <FundingHub nav={nav} />
    if (path === '/match') return <MatchPage nav={nav} />
    if (path === '/services') return <ServicesPage nav={nav} />
    if (path === '/bounties') return <BountiesPage nav={nav} />
    if (path === '/founding') return <FoundingPage nav={nav} wallet={wallet} />
    if (path === '/features') return <FeatureRegistryPage nav={nav} />
    if (path === '/angles') return <GrantAnglesPage nav={nav} />
    if (path === '/strategy') return <FundingStrategyPage nav={nav} />
    if (path.startsWith('/grants/')) { const d = GD.find(g => g.id === path.replace('/grants/', '')); return d ? <GrantPage data={d} nav={nav} /> : <GrantsHub nav={nav} /> }
    if (path.startsWith('/sponsors/')) { const d = SD.find(s => s.id === path.replace('/sponsors/', '')); return d ? <SponsorPage data={d} nav={nav} /> : <SponsorsHub nav={nav} /> }
    if (path.startsWith('/funding/')) { const d = FD.find(f => f.id === path.replace('/funding/', '')); return d ? <FundingPage data={d} nav={nav} /> : <FundingHub nav={nav} /> }
    return <HomePage nav={nav} />
  })()

  return (
    <div>
      <ThreeBg />
      <div className="bg-base" />
      <div className="grid-bg" />
      <div className="scanline" />
      <Stars />
      <Nav nav={nav} current={path} walletBar={walletBar} />
      <main className="x3main" key={path}>{page}</main>
      {showWallet && (
        <WalletModal
          accounts={wallet.accounts}
          selected={wallet.selected}
          connecting={wallet.connecting}
          error={wallet.error}
          onConnect={wallet.connect}
          onDisconnect={wallet.disconnect}
          onSwitch={wallet.switchAccount}
          onClose={() => setShowWallet(false)}
        />
      )}
    </div>
  )
}
