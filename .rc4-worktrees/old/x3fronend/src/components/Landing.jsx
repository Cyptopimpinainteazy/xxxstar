import { useEffect, useRef, useState, useCallback } from 'react';
import * as THREE from 'three';
import useLiveStats from '../hooks/useLiveStats';

/* ────────────────────────────────────────────────────────────────────────
   X3STAR — Cinematic Three.js Landing  ·  The Coolest Page Ever Built
   ──────────────────────────────────────────────────────────────────────── */

const NAV_LINKS = [
  { label: 'Technology', href: '#technology' },
  { label: 'Ecosystem',  href: '#ecosystem'  },
  { label: 'Validators', href: '#validators'  },
  { label: 'Whitepaper', href: '/x3star-whitepaper.html' },
];

const CHAIN_FEATURES = [
  { icon: '⚡', stat: '4,200', unit: ' TPS',   label: 'Throughput',  sub: 'Sustained peak on mainnet benchmarks' },
  { icon: '🔮', stat: '0.4s',  unit: '',        label: 'Finality',    sub: 'Sub-second guaranteed settlement'   },
  { icon: '💎', stat: '$0.0001', unit: '/tx',   label: 'Fee',         sub: 'Fixed atomic swap execution cost'   },
  { icon: '🌐', stat: '1,847', unit: '',        label: 'Validators',  sub: 'Genesis-set nodes globally'         },
  { icon: '🏦', stat: '$48.2M', unit: '',       label: 'TVL',         sub: 'Total value locked in protocol'     },
  { icon: '📜', stat: '48',    unit: '',        label: 'Grants',      sub: 'Active developer grants live'       },
];

const ECOSYSTEM = [
  { title: 'DeFi Atomic Swaps',   icon: '⬡', href: '/x3-nexus.html',                  desc: 'Cross-VM trades in a single tx — no wrapping, no bridges, no slippage.'  },
  { title: 'Validator Network',   icon: '🛡', href: '/x3star-validator-presale.html',   desc: 'Join 1,847 genesis validators. Stake, earn, govern.'                     },
  { title: 'On-chain Governance', icon: '🗳', href: '/x3star-governance.html',          desc: 'Fully decentralised protocol upgrades via weighted validator quorum.'    },
  { title: 'Flashloan Engine',    icon: '🔥', href: '/x3-flashloans.html',              desc: 'Institutional-grade uncollateralised liquidity in one block.'            },
  { title: 'Developer Grants',    icon: '🏗', href: '/x3star-grant-hub.html',           desc: '$5M+ in deployed grants across AI, PQC, DeFi and zkVM projects.'        },
  { title: 'Compute Marketplace', icon: '🖥', href: '/x3star-compute-marketplace.html', desc: 'Decentralised GPU/CPU orchestration for ZK proving and inference.'      },
];

function useAnimatedNumber(target, duration = 1800) {
  const [display, setDisplay] = useState(0);
  useEffect(() => {
    let start = null;
    const step = (ts) => {
      if (!start) start = ts;
      const p = Math.min((ts - start) / duration, 1);
      const ease = 1 - Math.pow(1 - p, 3);
      setDisplay(Math.round(target * ease));
      if (p < 1) requestAnimationFrame(step);
    };
    requestAnimationFrame(step);
  }, [target, duration]);
  return display;
}

export default function Landing() {
  const canvasRef = useRef(null);
  const [navScrolled, setNavScrolled] = useState(false);
  const { validators, tps, blockHeight, loading } = useLiveStats(6000);

  const animatedValidators  = useAnimatedNumber(validators  || 1847);
  const animatedTps         = useAnimatedNumber(tps         || 4200);
  const animatedBlockHeight = useAnimatedNumber(blockHeight || 2341789);

  // nav shadow on scroll
  useEffect(() => {
    const fn = () => setNavScrolled(window.scrollY > 40);
    window.addEventListener('scroll', fn, { passive: true });
    return () => window.removeEventListener('scroll', fn);
  }, []);

  // ── THREE.JS: Galaxy + Crystal + Orbital Nodes ──
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const W = canvas.clientWidth  || window.innerWidth;
    const H = canvas.clientHeight || window.innerHeight;

    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
    renderer.setSize(W, H);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor(0x000000, 0);

    const scene  = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(60, W / H, 0.1, 2000);
    camera.position.set(0, 1.5, 5.5);

    // Galaxy spiral
    const GALAXY_N = 80000;
    const gPos = new Float32Array(GALAXY_N * 3);
    const gCol = new Float32Array(GALAXY_N * 3);
    for (let i = 0; i < GALAXY_N; i++) {
      const r    = Math.pow(Math.random(), 1.6) * 10;
      const arm  = (i % 5) * (Math.PI * 2 / 5);
      const spin = r * 0.8;
      const ang  = arm + spin + (Math.random() - 0.5) * 0.55;
      const sp   = Math.random() * 0.3 * r;
      gPos[i*3]   = Math.cos(ang)*r + (Math.random()-.5)*sp;
      gPos[i*3+1] = (Math.random()-.5) * 0.45;
      gPos[i*3+2] = Math.sin(ang)*r + (Math.random()-.5)*sp;
      const t = r / 10;
      gCol[i*3]   = THREE.MathUtils.lerp(0.7, 0.0, t);
      gCol[i*3+1] = THREE.MathUtils.lerp(0.9, 0.55, t);
      gCol[i*3+2] = 1.0;
    }
    const galaxyGeo = new THREE.BufferGeometry();
    galaxyGeo.setAttribute('position', new THREE.BufferAttribute(gPos, 3));
    galaxyGeo.setAttribute('color',    new THREE.BufferAttribute(gCol, 3));
    const galaxyMat = new THREE.PointsMaterial({ size: 0.022, vertexColors: true, transparent: true, opacity: 0.85, blending: THREE.AdditiveBlending, depthWrite: false });
    const galaxy = new THREE.Points(galaxyGeo, galaxyMat);
    scene.add(galaxy);

    // Central crystal
    const crystalGeo = new THREE.OctahedronGeometry(0.65, 2);
    const crystalMat = new THREE.MeshStandardMaterial({ color: 0xffd700, metalness: 0.98, roughness: 0.04, emissive: 0xff8800, emissiveIntensity: 0.6 });
    const crystal = new THREE.Mesh(crystalGeo, crystalMat);
    scene.add(crystal);

    // Outer wireframe shell
    const wireGeo = new THREE.IcosahedronGeometry(1.1, 1);
    const wireMat = new THREE.MeshBasicMaterial({ color: 0x00e5ff, wireframe: true, transparent: true, opacity: 0.14 });
    const wireShell = new THREE.Mesh(wireGeo, wireMat);
    scene.add(wireShell);

    // Orbit rings
    for (let i = 1; i <= 3; i++) {
      const rg = new THREE.TorusGeometry(i * 0.55, 0.0025, 8, 128);
      const rm = new THREE.MeshBasicMaterial({ color: i === 1 ? 0x00e5ff : 0xffd700, transparent: true, opacity: 0.22 / i });
      const rr = new THREE.Mesh(rg, rm);
      rr.rotation.x = Math.PI / 2 + i * 0.2;
      rr.rotation.y = i * 0.3;
      scene.add(rr);
    }

    // Orbiting validator nodes
    const nodes = [];
    for (let i = 0; i < 14; i++) {
      const a  = (i / 14) * Math.PI * 2;
      const r  = 1.75 + Math.random() * 0.45;
      const ng = new THREE.OctahedronGeometry(0.038, 0);
      const nm = new THREE.MeshBasicMaterial({ color: Math.random() > 0.5 ? 0x00e5ff : 0xffd700 });
      const n  = new THREE.Mesh(ng, nm);
      n.position.set(Math.cos(a)*r, (Math.random()-.5)*0.55, Math.sin(a)*r);
      scene.add(n);
      nodes.push({ mesh: n, a, r, spd: 0.004 + Math.random()*0.004, ph: Math.random()*Math.PI*2 });
    }

    // Lights
    scene.add(new THREE.AmbientLight(0x120820, 2.2));
    const lGold = new THREE.PointLight(0xffd700, 5, 16); lGold.position.set(2.5, 2, 2);    scene.add(lGold);
    const lCyan = new THREE.PointLight(0x00e5ff, 5, 16); lCyan.position.set(-2.5,-1,-2);   scene.add(lCyan);

    // Mouse parallax
    let mx = 0, my = 0;
    const onMouse = e => { mx = (e.clientX/window.innerWidth - 0.5)*2; my = (e.clientY/window.innerHeight - 0.5)*2; };
    window.addEventListener('mousemove', onMouse);

    // Resize
    const onResize = () => {
      const w = canvas.clientWidth, h = canvas.clientHeight;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h);
    };
    window.addEventListener('resize', onResize);

    // Animate
    let raf;
    const clock = new THREE.Clock();
    const tick = () => {
      raf = requestAnimationFrame(tick);
      const t = clock.getElapsedTime();
      galaxy.rotation.y    = t * 0.022;
      crystal.rotation.x   = t * 0.38;
      crystal.rotation.y   = t * 0.58;
      wireShell.rotation.x = -t * 0.13;
      wireShell.rotation.y =  t * 0.19;
      camera.position.x += (mx * 0.55 - camera.position.x) * 0.028;
      camera.position.y += (-my * 0.38 - camera.position.y + 1.5) * 0.028;
      camera.lookAt(0, 0, 0);
      for (const n of nodes) {
        n.a += n.spd;
        n.mesh.position.x = Math.cos(n.a) * n.r;
        n.mesh.position.z = Math.sin(n.a) * n.r;
        n.mesh.position.y = Math.sin(t + n.ph) * 0.22;
        n.mesh.rotation.y += 0.03;
      }
      lGold.position.x = Math.sin(t * 0.45) * 3.5;
      lCyan.position.x = Math.cos(t * 0.45) * 3.5;
      renderer.render(scene, camera);
    };
    tick();

    return () => {
      cancelAnimationFrame(raf);
      window.removeEventListener('mousemove', onMouse);
      window.removeEventListener('resize', onResize);
      renderer.dispose();
    };
  }, []);

  const scrollTo = useCallback(id => {
    document.getElementById(id)?.scrollIntoView({ behavior: 'smooth' });
  }, []);

  return (
    <div style={{ background: 'radial-gradient(ellipse 130% 65% at 50% 0%, #0a0018 0%, #000008 55%, #000000 100%)', minHeight: '100vh', color: '#f0f4ff', fontFamily: "'JetBrains Mono','Fira Mono',monospace", overflowX: 'hidden' }}>

      {/* ── NAV ── */}
      <nav style={{
        position: 'fixed', top: 0, left: 0, right: 0, zIndex: 100,
        display: 'flex', alignItems: 'center', justifyContent: 'space-between',
        padding: '0 clamp(1rem, 4vw, 2.5rem)', height: 64,
        background:     navScrolled ? 'rgba(2,2,18,0.96)' : 'transparent',
        backdropFilter: navScrolled ? 'blur(22px)'         : 'none',
        borderBottom:   navScrolled ? '1px solid rgba(255,215,0,0.14)' : 'none',
        transition: 'all 0.35s',
      }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.7rem', flexShrink: 0 }}>
          <div style={{ width: 30, height: 30, background: 'linear-gradient(135deg,#ffd700,#00e5ff)', clipPath: 'polygon(50% 0%,100% 25%,100% 75%,50% 100%,0% 75%,0% 25%)' }} />
          <span style={{ fontWeight: 700, fontSize: '1.05rem', letterSpacing: '0.22em', background: 'linear-gradient(90deg,#ffd700,#00e5ff)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>X3STAR</span>
        </div>
        <div style={{ display: 'flex', gap: '2rem' }}>
          {NAV_LINKS.map(l =>
            l.href.startsWith('#')
              ? <button key={l.label} onClick={() => scrollTo(l.href.slice(1))} style={{ background: 'none', border: 'none', cursor: 'pointer', color: 'rgba(240,244,255,0.65)', fontSize: '0.75rem', letterSpacing: '0.12em', textTransform: 'uppercase', padding: 0, transition: 'color 0.2s' }} onMouseOver={e => e.target.style.color='#ffd700'} onMouseOut={e => e.target.style.color='rgba(240,244,255,0.65)'}>{l.label}</button>
              : <a key={l.label} href={l.href} style={{ color: 'rgba(240,244,255,0.65)', textDecoration: 'none', fontSize: '0.75rem', letterSpacing: '0.12em', textTransform: 'uppercase', transition: 'color 0.2s' }} onMouseOver={e => e.target.style.color='#ffd700'} onMouseOut={e => e.target.style.color='rgba(240,244,255,0.65)'}>{l.label}</a>
          )}
        </div>
        <div style={{ display: 'flex', gap: '0.7rem', flexShrink: 0 }}>
          <a href="/x3star-whitepaper.html" style={{ padding: '0.45rem 1.1rem', border: '1px solid rgba(240,244,255,0.25)', borderRadius: 8, color: '#f0f4ff', textDecoration: 'none', fontSize: '0.75rem', letterSpacing: '0.1em', transition: 'all 0.2s' }} onMouseOver={e => { e.currentTarget.style.borderColor='#ffd700'; e.currentTarget.style.background='rgba(255,215,0,0.07)'; }} onMouseOut={e => { e.currentTarget.style.borderColor='rgba(240,244,255,0.25)'; e.currentTarget.style.background='transparent'; }}>Whitepaper</a>
          <a href="/x3star-token-presale.html" style={{ padding: '0.45rem 1.1rem', background: 'linear-gradient(135deg,#ffd700,#ff9500)', borderRadius: 8, color: '#000', fontWeight: 800, textDecoration: 'none', fontSize: '0.75rem', letterSpacing: '0.1em', boxShadow: '0 0 22px rgba(255,215,0,0.4)', transition: 'all 0.2s' }} onMouseOver={e => { e.currentTarget.style.transform='scale(1.05)'; e.currentTarget.style.boxShadow='0 0 38px rgba(255,215,0,0.7)'; }} onMouseOut={e => { e.currentTarget.style.transform='scale(1)'; e.currentTarget.style.boxShadow='0 0 22px rgba(255,215,0,0.4)'; }}>Buy X3S →</a>
        </div>
      </nav>

      {/* ── HERO ── */}
      <section style={{ position: 'relative', minHeight: '100vh', display: 'flex', flexDirection: 'column', alignItems: 'center', justifyContent: 'center', textAlign: 'center', padding: '64px 1.5rem 0' }}>
        <canvas ref={canvasRef} style={{ position: 'absolute', inset: 0, width: '100%', height: '100%', zIndex: 0, pointerEvents: 'none' }} />
        <div style={{ position: 'absolute', inset: 0, background: 'radial-gradient(ellipse 65% 75% at 50% 50%, transparent 25%, #000008 100%)', zIndex: 1, pointerEvents: 'none' }} />
        <div style={{ position: 'relative', zIndex: 2, maxWidth: 920, width: '100%' }}>

          {/* Live badge */}
          <div style={{ display: 'inline-flex', alignItems: 'center', gap: '0.55rem', padding: '0.38rem 1rem', border: '1px solid rgba(0,229,255,0.38)', borderRadius: 999, fontSize: '0.68rem', letterSpacing: '0.2em', textTransform: 'uppercase', marginBottom: '2rem', background: 'rgba(0,229,255,0.055)', color: '#00e5ff', fontFamily: 'sans-serif' }}>
            <span style={{ width: 7, height: 7, background: '#00ff88', borderRadius: '50%', boxShadow: '0 0 8px #00ff88', display: 'inline-block', animation: 'pulseGreen 1.8s infinite' }} />
            {loading ? 'Connecting to network…' : `Block #${animatedBlockHeight.toLocaleString()} · ${animatedValidators.toLocaleString()} Validators · ${animatedTps.toLocaleString()} TPS`}
          </div>

          {/* Headline */}
          <h1 style={{ fontSize: 'clamp(3.8rem, 12vw, 9rem)', fontWeight: 900, lineHeight: 0.92, margin: '0 0 0.9rem', letterSpacing: '-0.025em', fontFamily: 'inherit' }}>
            <span style={{ display: 'block', background: 'linear-gradient(180deg,#ffffff 10%,rgba(200,210,255,0.55) 100%)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>X3STAR</span>
          </h1>
          <p style={{ fontSize: 'clamp(0.78rem, 2vw, 1rem)', fontWeight: 400, letterSpacing: '0.45em', textTransform: 'uppercase', background: 'linear-gradient(90deg,#ffd700,#00e5ff)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent', marginBottom: '2rem', fontFamily: 'sans-serif' }}>Atomic Blockchain Protocol</p>
          <p style={{ fontSize: 'clamp(0.95rem, 2.2vw, 1.2rem)', lineHeight: 1.75, color: 'rgba(240,244,255,0.6)', maxWidth: 640, margin: '0 auto 2.5rem', fontFamily: 'sans-serif' }}>
            The <strong style={{ color: '#ffd700' }}>high-throughput, cross-VM blockchain</strong> built for real-world DeFi. Atomic swaps between any chain, sub-second finality, and $5M+ in developer grants.
          </p>

          {/* CTAs */}
          <div style={{ display: 'flex', gap: '0.9rem', justifyContent: 'center', flexWrap: 'wrap', marginBottom: '3.5rem' }}>
            <a href="/x3star-token-presale.html" style={{ padding: '0.9rem 2.2rem', background: 'linear-gradient(135deg,#ffd700,#ff9500)', borderRadius: 12, color: '#000', fontWeight: 800, textDecoration: 'none', fontSize: '0.92rem', letterSpacing: '0.1em', boxShadow: '0 0 44px rgba(255,215,0,0.52)', transition: 'all 0.2s' }} onMouseOver={e => { e.currentTarget.style.transform='scale(1.06)'; e.currentTarget.style.boxShadow='0 0 66px rgba(255,215,0,0.78)'; }} onMouseOut={e => { e.currentTarget.style.transform='scale(1)'; e.currentTarget.style.boxShadow='0 0 44px rgba(255,215,0,0.52)'; }}>⬡ Buy X3S Tokens</a>
            <a href="/x3star-validator-presale.html" style={{ padding: '0.9rem 2.2rem', border: '1px solid rgba(0,229,255,0.48)', borderRadius: 12, color: '#00e5ff', textDecoration: 'none', fontSize: '0.92rem', letterSpacing: '0.08em', background: 'rgba(0,229,255,0.055)', transition: 'all 0.2s' }} onMouseOver={e => { e.currentTarget.style.background='rgba(0,229,255,0.14)'; e.currentTarget.style.borderColor='#00e5ff'; }} onMouseOut={e => { e.currentTarget.style.background='rgba(0,229,255,0.055)'; e.currentTarget.style.borderColor='rgba(0,229,255,0.48)'; }}>Run a Validator Node</a>
            <a href="/x3star-whitepaper.html" style={{ padding: '0.9rem 2.2rem', border: '1px solid rgba(255,255,255,0.14)', borderRadius: 12, color: 'rgba(240,244,255,0.65)', textDecoration: 'none', fontSize: '0.92rem', letterSpacing: '0.08em', transition: 'all 0.2s' }} onMouseOver={e => { e.currentTarget.style.borderColor='rgba(255,215,0,0.48)'; e.currentTarget.style.color='#ffd700'; }} onMouseOut={e => { e.currentTarget.style.borderColor='rgba(255,255,255,0.14)'; e.currentTarget.style.color='rgba(240,244,255,0.65)'; }}>Read Whitepaper</a>
          </div>

          {/* Live ticker */}
          <div style={{ display: 'flex', justifyContent: 'center', flexWrap: 'wrap', borderTop: '1px solid rgba(255,255,255,0.07)', paddingTop: '1.5rem' }}>
            {[
              { val: `${animatedTps.toLocaleString()}`,         unit: ' TPS',    label: 'Live Throughput' },
              { val: '0.4s',                                    unit: '',        label: 'Finality'        },
              { val: '$0.0001',                                 unit: '/tx',     label: 'Fee'             },
              { val: `${animatedValidators.toLocaleString()}`,  unit: '',        label: 'Validators'      },
              { val: '$14.7M',                                  unit: '',        label: 'Raised'          },
              { val: '48',                                      unit: ' Grants', label: 'Active'          },
            ].map((item, i) => (
              <div key={i} style={{ padding: '0 1.6rem', borderRight: i < 5 ? '1px solid rgba(255,255,255,0.07)' : 'none', textAlign: 'center', minWidth: 90 }}>
                <div style={{ fontSize: 'clamp(1.15rem,2.6vw,1.5rem)', fontWeight: 700, fontFamily: 'inherit', background: 'linear-gradient(90deg,#ffd700,#00e5ff)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>{item.val}<span style={{ fontSize: '0.62em' }}>{item.unit}</span></div>
                <div style={{ fontSize: '0.6rem', letterSpacing: '0.17em', textTransform: 'uppercase', color: 'rgba(240,244,255,0.38)', marginTop: '0.18rem', fontFamily: 'sans-serif' }}>{item.label}</div>
              </div>
            ))}
          </div>
        </div>

        {/* Scroll cue */}
        <button onClick={() => scrollTo('technology')} style={{ position: 'absolute', bottom: '2.2rem', left: '50%', transform: 'translateX(-50%)', display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '0.4rem', cursor: 'pointer', color: 'rgba(255,255,255,0.28)', zIndex: 2, fontSize: '0.6rem', letterSpacing: '0.22em', textTransform: 'uppercase', background: 'none', border: 'none', padding: 0, fontFamily: 'sans-serif' }}>
          <div style={{ width: 1, height: 44, background: 'linear-gradient(180deg,rgba(0,229,255,0.7),transparent)', animation: 'scrollLine 2.2s ease-in-out infinite' }} />
          Scroll
        </button>
      </section>

      {/* ── TECH SPECS ── */}
      <section id="technology" style={{ padding: '7rem clamp(1rem,5vw,3rem)', maxWidth: 1240, margin: '0 auto' }}>
        <div style={{ marginBottom: '3.5rem', textAlign: 'center' }}>
          <div style={{ fontSize: '0.68rem', letterSpacing: '0.4em', textTransform: 'uppercase', color: '#ffd700', marginBottom: '0.7rem', fontFamily: 'sans-serif' }}>Tech Specs</div>
          <h2 style={{ fontSize: 'clamp(2rem,5.5vw,3.8rem)', fontWeight: 800, lineHeight: 1.1, margin: 0, fontFamily: 'inherit' }}>
            Built for the <span style={{ background: 'linear-gradient(90deg,#ffd700,#00e5ff)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>next billion users.</span>
          </h2>
          <p style={{ color: 'rgba(240,244,255,0.45)', fontFamily: 'sans-serif', fontSize: '1rem', maxWidth: 520, margin: '0.8rem auto 0' }}>Every metric audited on mainnet. No simulated benchmarks.</p>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(270px, 1fr))', gap: '1.2rem' }}>
          {CHAIN_FEATURES.map((f, i) => (
            <div key={i}
              style={{ padding: '2rem', border: '1px solid rgba(255,255,255,0.07)', borderRadius: 20, background: 'rgba(255,255,255,0.022)', backdropFilter: 'blur(10px)', position: 'relative', overflow: 'hidden', transition: 'border-color 0.3s, transform 0.25s, box-shadow 0.3s' }}
              onMouseOver={e => { e.currentTarget.style.borderColor='rgba(255,215,0,0.42)'; e.currentTarget.style.transform='translateY(-5px)'; e.currentTarget.style.boxShadow='0 14px 44px rgba(255,215,0,0.1)'; }}
              onMouseOut={e =>  { e.currentTarget.style.borderColor='rgba(255,255,255,0.07)'; e.currentTarget.style.transform='translateY(0)'; e.currentTarget.style.boxShadow='none'; }}>
              <div style={{ position: 'absolute', inset: 0, background: 'radial-gradient(circle at 15% 15%, rgba(255,215,0,0.035), transparent 65%)', pointerEvents: 'none' }} />
              <div style={{ fontSize: '2rem', marginBottom: '0.9rem' }}>{f.icon}</div>
              <div style={{ fontSize: 'clamp(1.8rem,3.5vw,2.6rem)', fontWeight: 800, lineHeight: 1, background: 'linear-gradient(150deg,#fff,rgba(255,255,255,0.55))', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent', fontFamily: 'inherit' }}>{f.stat}<span style={{ fontSize: '0.42em', opacity: 0.65 }}>{f.unit}</span></div>
              <div style={{ fontSize: '0.9rem', fontWeight: 600, color: '#ffd700', margin: '0.35rem 0 0.5rem', fontFamily: 'sans-serif' }}>{f.label}</div>
              <div style={{ fontSize: '0.78rem', color: 'rgba(240,244,255,0.42)', fontFamily: 'sans-serif', lineHeight: 1.55 }}>{f.sub}</div>
            </div>
          ))}
        </div>
      </section>

      {/* ── ECOSYSTEM ── */}
      <section id="ecosystem" style={{ padding: '7rem clamp(1rem,5vw,3rem)', maxWidth: 1240, margin: '0 auto', borderTop: '1px solid rgba(255,255,255,0.055)' }}>
        <div style={{ marginBottom: '3.5rem', textAlign: 'center' }}>
          <div style={{ fontSize: '0.68rem', letterSpacing: '0.4em', textTransform: 'uppercase', color: '#00e5ff', marginBottom: '0.7rem', fontFamily: 'sans-serif' }}>Ecosystem</div>
          <h2 style={{ fontSize: 'clamp(2rem,5.5vw,3.8rem)', fontWeight: 800, lineHeight: 1.1, margin: 0, fontFamily: 'inherit' }}>
            One protocol. <span style={{ background: 'linear-gradient(90deg,#00e5ff,#bf5fff)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>Infinite rails.</span>
          </h2>
        </div>
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(300px, 1fr))', gap: '1.2rem' }}>
          {ECOSYSTEM.map((item, i) => (
            <a key={i} href={item.href}
              style={{ display: 'block', padding: '2rem', border: '1px solid rgba(0,229,255,0.1)', borderRadius: 20, background: 'rgba(0,229,255,0.025)', textDecoration: 'none', color: 'inherit', transition: 'all 0.28s' }}
              onMouseOver={e => { e.currentTarget.style.borderColor='rgba(0,229,255,0.44)'; e.currentTarget.style.background='rgba(0,229,255,0.065)'; e.currentTarget.style.transform='translateY(-5px)'; }}
              onMouseOut={e =>  { e.currentTarget.style.borderColor='rgba(0,229,255,0.1)';  e.currentTarget.style.background='rgba(0,229,255,0.025)'; e.currentTarget.style.transform='translateY(0)'; }}>
              <div style={{ fontSize: '2rem', marginBottom: '0.9rem' }}>{item.icon}</div>
              <div style={{ fontSize: '1.05rem', fontWeight: 700, color: '#f0f4ff', marginBottom: '0.55rem', fontFamily: 'sans-serif' }}>{item.title}</div>
              <div style={{ fontSize: '0.82rem', color: 'rgba(240,244,255,0.45)', lineHeight: 1.62, fontFamily: 'sans-serif' }}>{item.desc}</div>
              <div style={{ marginTop: '1.2rem', fontSize: '0.72rem', color: '#00e5ff', letterSpacing: '0.12em', textTransform: 'uppercase', fontFamily: 'sans-serif' }}>Explore →</div>
            </a>
          ))}
        </div>
      </section>

      {/* ── VALIDATOR CTA ── */}
      <section id="validators" style={{ padding: '7rem clamp(1rem,5vw,3rem)', textAlign: 'center', borderTop: '1px solid rgba(255,255,255,0.055)', background: 'radial-gradient(ellipse 90% 65% at 50% 50%, rgba(255,215,0,0.038), transparent)' }}>
        <div style={{ fontSize: '0.68rem', letterSpacing: '0.4em', textTransform: 'uppercase', color: '#ffd700', marginBottom: '0.7rem', fontFamily: 'sans-serif' }}>Join the Network</div>
        <h2 style={{ fontSize: 'clamp(2rem,6.5vw,4.5rem)', fontWeight: 900, lineHeight: 1.08, maxWidth: 860, margin: '0 auto 1.4rem', fontFamily: 'inherit' }}>
          <span style={{ background: 'linear-gradient(135deg,#ffd700,#00e5ff)', WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent' }}>Become a Genesis Validator.</span>
          <br />Earn. Govern. Secure the future.
        </h2>
        <p style={{ color: 'rgba(240,244,255,0.5)', maxWidth: 560, margin: '0 auto 3rem', lineHeight: 1.8, fontFamily: 'sans-serif', fontSize: '1.02rem' }}>
          Only <strong style={{ color: '#ffd700' }}>153 genesis node slots remain.</strong> Validators earn block rewards, governance rights, and share in protocol fees from every atomic swap.
        </p>
        <div style={{ display: 'flex', gap: '1rem', justifyContent: 'center', flexWrap: 'wrap' }}>
          <a href="/x3star-validator-presale.html" style={{ padding: '1rem 2.5rem', background: 'linear-gradient(135deg,#ffd700,#ff9500)', borderRadius: 14, color: '#000', fontWeight: 800, textDecoration: 'none', fontSize: '0.98rem', letterSpacing: '0.1em', boxShadow: '0 0 55px rgba(255,215,0,0.55)', transition: 'all 0.2s' }} onMouseOver={e => { e.currentTarget.style.transform='scale(1.06)'; e.currentTarget.style.boxShadow='0 0 80px rgba(255,215,0,0.75)'; }} onMouseOut={e => { e.currentTarget.style.transform='scale(1)'; e.currentTarget.style.boxShadow='0 0 55px rgba(255,215,0,0.55)'; }}>Reserve My Node →</a>
          <a href="/x3star-roi-calculator.html" style={{ padding: '1rem 2.5rem', border: '1px solid rgba(255,215,0,0.32)', borderRadius: 14, color: '#ffd700', textDecoration: 'none', fontSize: '0.98rem', letterSpacing: '0.08em', transition: 'background 0.2s' }} onMouseOver={e => e.currentTarget.style.background='rgba(255,215,0,0.08)'} onMouseOut={e => e.currentTarget.style.background='transparent'}>Calculate ROI</a>
        </div>
      </section>

      {/* ── FOOTER ── */}
      <footer style={{ borderTop: '1px solid rgba(255,255,255,0.055)', padding: '3rem clamp(1rem,5vw,3rem)', textAlign: 'center', color: 'rgba(240,244,255,0.28)', fontSize: '0.75rem', letterSpacing: '0.08em', fontFamily: 'sans-serif' }}>
        <div style={{ display: 'flex', justifyContent: 'center', gap: '1.8rem', flexWrap: 'wrap', marginBottom: '1.4rem' }}>
          {[['Presale','/x3star-token-presale.html'],['Staking','/x3star-staking.html'],['Governance','/x3star-governance.html'],['Grant Hub','/x3star-grant-hub.html'],['Dashboard','/x3star-dashboard.html'],['Node Health','/x3star-node-health.html'],['Whitepaper','/x3star-whitepaper.html']].map(([label, href]) => (
            <a key={label} href={href} style={{ color: 'rgba(240,244,255,0.38)', textDecoration: 'none', transition: 'color 0.2s' }} onMouseOver={e => e.target.style.color='#ffd700'} onMouseOut={e => e.target.style.color='rgba(240,244,255,0.38)'}>{label}</a>
          ))}
        </div>
        <div>© 2026 X3STAR Protocol — All rights reserved</div>
      </footer>

      {/* ── GLOBAL KEYFRAMES ── */}
      <style>{`
        @keyframes pulseGreen {
          0%,100% { opacity:1; box-shadow:0 0 8px #00ff88; }
          50%      { opacity:0.32; box-shadow:0 0 3px #00ff88; }
        }
        @keyframes scrollLine {
          0%   { transform:scaleY(0); transform-origin:top; opacity:0; }
          35%  { transform:scaleY(1); opacity:1; }
          70%  { transform:scaleY(1); opacity:1; }
          100% { transform:scaleY(0); transform-origin:bottom; opacity:0; }
        }
        *, *::before, *::after { box-sizing:border-box; }
        html { scroll-behavior:smooth; }
        ::-webkit-scrollbar { width:5px; }
        ::-webkit-scrollbar-track { background:#000; }
        ::-webkit-scrollbar-thumb { background:rgba(255,215,0,0.28); border-radius:3px; }
      `}</style>
    </div>
  );
}
