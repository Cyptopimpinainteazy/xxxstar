import { useEffect, useRef } from 'react';
import * as THREE from 'three';
import { OrbitControls } from 'three/examples/jsm/controls/OrbitControls';
import HexRing from './HexRing';
import Button from './Button';
import ScrollIndicator from './ScrollIndicator';
import Eyebrow from './Eyebrow';
import ListItem from './ListItem';

export default function Landing() {
  const heroRef = useRef();
  const canvasRef = useRef();

  useEffect(() => {
    const ripple = () => {
      const t = window.scrollY / window.innerHeight;
      heroRef.current.style.setProperty('--ripple', Math.min(1, t));
    };
    window.addEventListener('scroll', ripple);
    return () => window.removeEventListener('scroll', ripple);
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    const scene = new THREE.Scene();
    scene.fog = new THREE.FogExp2(0x000008, 0.015);

    const camera = new THREE.PerspectiveCamera(
      35,
      canvas.clientWidth / canvas.clientHeight,
      0.1,
      1000
    );
    camera.position.set(0, 0, 8);

    const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
    renderer.setSize(canvas.clientWidth, canvas.clientHeight);
    renderer.setPixelRatio(window.devicePixelRatio);

    const geom = new THREE.BufferGeometry();
    const count = 15000;
    const positions = new Float32Array(count * 3);
    for (let i = 0; i < count; i++) {
      positions[i * 3 + 0] = (Math.random() - 0.5) * 40;
      positions[i * 3 + 1] = (Math.random() - 0.5) * 40;
      positions[i * 3 + 2] = (Math.random() - 0.5) * 40;
    }
    geom.setAttribute('position', new THREE.BufferAttribute(positions, 3));
    const mat = new THREE.PointsMaterial({
      size: 0.05,
      color: 0x00e5ff,
      transparent: true,
      opacity: 0.6,
      blending: THREE.AdditiveBlending
    });
    const points = new THREE.Points(geom, mat);
    scene.add(points);

    const logoGeom = new THREE.TorusKnotGeometry(1.2, 0.3, 200, 32);
    const logoMat = new THREE.MeshStandardMaterial({
      color: 0xffd700,
      metalness: 0.7,
      roughness: 0.1,
      emissive: 0xFF9500,
      emissiveIntensity: 0.6
    });
    const logo = new THREE.Mesh(logoGeom, logoMat);
    scene.add(logo);

    const ambient = new THREE.AmbientLight(0x222222);
    scene.add(ambient);
    const point = new THREE.PointLight(0x00e5ff, 2, 50);
    point.position.set(5, 5, 5);
    scene.add(point);

    const controls = new OrbitControls(camera, canvas);
    controls.enableZoom = false;
    controls.enablePan = false;
    controls.autoRotate = true;
    controls.autoRotateSpeed = 0.2;

    const resize = () => {
      const { clientWidth: w, clientHeight: h } = canvas;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h);
    };
    window.addEventListener('resize', resize);

    const tick = () => {
      logo.rotation.x += 0.002;
      logo.rotation.y += 0.003;
      points.rotation.y += 0.0005;
      controls.update();
      renderer.render(scene, camera);
      requestAnimationFrame(tick);
    };
    tick();

    return () => {
      window.removeEventListener('resize', resize);
      renderer.dispose();
    };
  }, []);

  return (
    <main className="bg-ink text-cream font-body overflow-x-hidden relative">
      <canvas ref={canvasRef} className="absolute inset-0 z-0" />
      <section
        ref={heroRef}
        className="relative min-h-screen flex flex-col justify-center items-center text-center px-6 z-10"
        style={{ '--ripple': 0 }}
      >
        <div className="absolute inset-0 pointer-events-none">
          <HexRing className="w-96 h-96 animate-spin-slow" />
        </div>
        <p className="badge mb-8">
          <span className="pulse" /> Live mainnet – 42 validators
        </p>
        <h1
          className="display text-6xl lg:text-9xl leading-tight gradient-text drop-shadow-xl"
          style={{
            transform: 'scale(calc(1 + var(--ripple) * .1))',
            filter: 'blur(calc(var(--ripple) * 1px))'
          }}
        >
          X3STAR
        </h1>
        <p className="max-w-xl mt-6 opacity-60 leading-relaxed">
          An ocean‑hardened, atomic‑swap middleware blockchain.{' '}
          <strong className="text-cyan">Valencia testnet live.</strong>
        </p>
        <div className="mt-12 flex flex-wrap justify-center gap-4">
          <Button primary>Get started</Button>
          <Button outline>Read the whitepaper</Button>
        </div>
        <ScrollIndicator />
      </section>
      <section id="about" className="section">
        <Eyebrow text="Problem" />
        <h2 className="section-title">Interoperability is a leak.</h2>
        <p className="section-sub">
          Projects force users to jump between incompatible VMs, losing atomicity and
          custody. X3 plugs between chains and <strong className="text-cyan">guarantees</strong> cross‑VM trades in a
          single transaction.
        </p>
        <div className="grid lg:grid-cols-2 gap-16 mt-12">
          <ListItem icon="🔥" title="Slippage" text="Users bleed value across hops." />
          <ListItem icon="⛓️" title="Complexity" text="Smart contracts become stitching messes." />
          <ListItem icon="👁️" title="Opacity" text="You never know which chain your assets took." />
          <ListItem icon="🐢" title="Latency" text="Blocks per hop stack to minutes." />
        </div>
      </section>
      <section id="tech" className="tech-section">
        <Eyebrow text="Tech specs" />
        <div className="grid lg:grid-cols-3 gap-2 bg-surface-dark rounded-3xl overflow-hidden mt-12">
          {[
            { n: '1M', label: 'TPS' },
            { n: '12s', label: 'Finality' },
            { n: '0.0001ꜩ', label: 'Tx cost' }
          ].map((d, i) => (
            <div key={i} className="tech-card group">
              <span className="tc-number">{d.n}</span>
              <span className="tc-label">{d.label}</span>
              <div className="tc-bar mt-4">
                <div className="tc-bar-fill group-hover:w-full transition-[width] duration-1500 ease-[var(--easing)]" />
              </div>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}
