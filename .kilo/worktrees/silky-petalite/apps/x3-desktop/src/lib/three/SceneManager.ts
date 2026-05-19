/**
 * SceneManager.ts — Epic dark background with 7 animated layers.
 *
 *   1. Twinkling starfield  (4 000 stars, custom twinkle shader)
 *   2. Nebula particle clouds  (1 200 particles, additive blend)
 *   3. Ambient dust  (600 tiny drifting particles)
 *   4. Perspective grid floor  (orange lines, dual density)
 *   5. Sonar pulse rings  (expanding rings on the grid)
 *   6. Network constellation  (35 orbiting nodes + dynamic connections)
 *   7. Data rain + shooting stars
 *
 * Dark #050508 base · orange / blue / purple accents · UnrealBloom glow
 */
import * as THREE from 'three';
import { EffectComposer } from 'three/examples/jsm/postprocessing/EffectComposer.js';
import { RenderPass } from 'three/examples/jsm/postprocessing/RenderPass.js';
import { UnrealBloomPass } from 'three/examples/jsm/postprocessing/UnrealBloomPass.js';

/* ── Palette ──────────────────────────────────────────────── */
const C = {
  bg:     0x050508,
  orange: 0xff6b35,
  amber:  0xff8c42,
  blue:   0x00b4ff,
  purple: 0x8b5cf6,
  pink:   0xff4488,
  white:  0xffffff,
};

interface NodeOrbit {
  mesh: THREE.Mesh;
  baseY: number;
  radius: number;
  speed: number;
  phase: number;
  yAmp: number;
  ySpeed: number;
  yPhase: number;
}

export class SceneManager {
  private scene!:    THREE.Scene;
  private camera!:   THREE.PerspectiveCamera;
  private renderer!: THREE.WebGLRenderer;
  private container: HTMLElement;
  private composer!: EffectComposer;

  private mouse     = new THREE.Vector2();
  private targetPos!: THREE.Vector3;
  private time      = 0;
  private lineFrame = 0;

  private starfield!:     THREE.Points;
  private nebula!:        THREE.Points;
  private ambientDust!:   THREE.Points;
  private gridFloor!:     THREE.Group;
  private pulseRings!:    THREE.Group;
  private networkNodes!:  THREE.Group;
  private networkLines!:  THREE.Group;
  private dataStreams!:    THREE.Group;
  private shootingStars!: THREE.Group;
  private nodeOrbits:     NodeOrbit[] = [];

  constructor(container: HTMLElement) {
    this.container = container;
    this.scene = new THREE.Scene();
    this.scene.background = new THREE.Color(C.bg);
    this.scene.fog = new THREE.FogExp2(C.bg, 0.006);

    this.initCamera();
    this.initRenderer();
    this.initLights();
    this.initStarfield();
    this.initNebula();
    this.initAmbientDust();
    this.initGrid();
    this.initPulseRings();
    this.initNetwork();
    this.initDataStreams();
    this.initShootingStars();
    this.initPostProcessing();

    this.onResize();
    window.addEventListener('resize', () => this.onResize());
    window.addEventListener('mousemove', (e) => {
      this.mouse.x = (e.clientX / window.innerWidth) * 2 - 1;
      this.mouse.y = -(e.clientY / window.innerHeight) * 2 + 1;
    });
  }

  /* ── Camera ─────────────────────────────────────────────── */
  private initCamera() {
    this.camera = new THREE.PerspectiveCamera(
      60, window.innerWidth / window.innerHeight, 0.1, 500,
    );
    this.camera.position.set(0, 2, 22);
    this.targetPos = this.camera.position.clone();
  }

  /* ── Renderer ───────────────────────────────────────────── */
  private initRenderer() {
    this.renderer = new THREE.WebGLRenderer({ antialias: true, alpha: false });
    this.renderer.setSize(window.innerWidth, window.innerHeight);
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.toneMapping = THREE.ACESFilmicToneMapping;
    this.renderer.toneMappingExposure = 1.0;
    this.container.appendChild(this.renderer.domElement);
  }

  /* ── Lights ─────────────────────────────────────────────── */
  private initLights() {
    this.scene.add(new THREE.AmbientLight(C.white, 0.12));

    const pt1 = new THREE.PointLight(C.orange, 18, 90);
    pt1.position.set(12, 10, 5);
    this.scene.add(pt1);

    const pt2 = new THREE.PointLight(C.blue, 12, 80);
    pt2.position.set(-12, -4, 4);
    this.scene.add(pt2);

    const pt3 = new THREE.PointLight(C.purple, 10, 70);
    pt3.position.set(0, 14, -25);
    this.scene.add(pt3);
  }

  /* ── LAYER 1: Twinkling Starfield (custom shader) ─────── */
  private initStarfield() {
    const N = 4000;
    const pos = new Float32Array(N * 3);
    const sz  = new Float32Array(N);
    const spd = new Float32Array(N);
    const off = new Float32Array(N);
    const col = new Float32Array(N * 3);

    for (let i = 0; i < N; i++) {
      const r     = 35 + Math.random() * 165;
      const theta = Math.random() * Math.PI * 2;
      const phi   = Math.acos(2 * Math.random() - 1);
      pos[i * 3]     = r * Math.sin(phi) * Math.cos(theta);
      pos[i * 3 + 1] = r * Math.sin(phi) * Math.sin(theta);
      pos[i * 3 + 2] = r * Math.cos(phi);

      sz[i]  = Math.random() * 2.5 + 0.4;
      spd[i] = Math.random() * 3 + 0.4;
      off[i] = Math.random() * 6.2832;

      const c = Math.random();
      if (c < 0.60)      { col[i*3]=1;    col[i*3+1]=1;    col[i*3+2]=1;    }
      else if (c < 0.75) { col[i*3]=1;    col[i*3+1]=0.72; col[i*3+2]=0.33; }
      else if (c < 0.88) { col[i*3]=0.33; col[i*3+1]=0.72; col[i*3+2]=1;    }
      else               { col[i*3]=0.68; col[i*3+1]=0.42; col[i*3+2]=1;    }
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
    geo.setAttribute('aSize',    new THREE.BufferAttribute(sz,  1));
    geo.setAttribute('aSpeed',   new THREE.BufferAttribute(spd, 1));
    geo.setAttribute('aOffset',  new THREE.BufferAttribute(off, 1));
    geo.setAttribute('aColor',   new THREE.BufferAttribute(col, 3));

    const mat = new THREE.ShaderMaterial({
      uniforms: {
        uTime:  { value: 0 },
        uScale: { value: Math.min(window.devicePixelRatio, 2) },
      },
      vertexShader: `
        attribute float aSize;
        attribute float aSpeed;
        attribute float aOffset;
        attribute vec3  aColor;
        uniform float uTime;
        uniform float uScale;
        varying float vAlpha;
        varying vec3  vColor;
        void main() {
          vColor = aColor;
          vAlpha = 0.3 + 0.7 * (0.5 + 0.5 * sin(uTime * aSpeed + aOffset));
          vec4 mv = modelViewMatrix * vec4(position, 1.0);
          gl_PointSize = clamp(aSize * uScale * (200.0 / -mv.z), 0.4, 7.0);
          gl_Position  = projectionMatrix * mv;
        }
      `,
      fragmentShader: `
        varying float vAlpha;
        varying vec3  vColor;
        void main() {
          float d = length(gl_PointCoord - 0.5);
          if (d > 0.5) discard;
          float a = smoothstep(0.5, 0.0, d) * vAlpha;
          gl_FragColor = vec4(vColor, a);
        }
      `,
      transparent: true,
      depthWrite: false,
      blending: THREE.AdditiveBlending,
    });

    this.starfield = new THREE.Points(geo, mat);
    this.scene.add(this.starfield);
  }

  /* ── LAYER 2: Nebula Clouds ───────────────────────────── */
  private initNebula() {
    const N = 1200;
    const pos = new Float32Array(N * 3);
    const col = new Float32Array(N * 3);
    const clusters = [
      { cx: -30, cy:  8, cz: -50, r: 22, color: new THREE.Color(C.orange) },
      { cx:  28, cy: -5, cz: -55, r: 18, color: new THREE.Color(C.purple) },
      { cx:   5, cy: 15, cz: -65, r: 20, color: new THREE.Color(C.blue)   },
      { cx: -12, cy: -8, cz: -40, r: 14, color: new THREE.Color(C.amber)  },
      { cx:  38, cy: 12, cz: -45, r: 16, color: new THREE.Color(C.pink)   },
    ];

    for (let i = 0; i < N; i++) {
      const cl = clusters[Math.floor(Math.random() * clusters.length)];
      pos[i * 3]     = cl.cx + (Math.random() - 0.5) * cl.r * 2;
      pos[i * 3 + 1] = cl.cy + (Math.random() - 0.5) * cl.r * 2;
      pos[i * 3 + 2] = cl.cz + (Math.random() - 0.5) * cl.r * 2;

      const c = cl.color.clone().offsetHSL(
        Math.random() * 0.08 - 0.04, 0, Math.random() * 0.1 - 0.05,
      );
      col[i * 3] = c.r; col[i * 3 + 1] = c.g; col[i * 3 + 2] = c.b;
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
    geo.setAttribute('color',    new THREE.BufferAttribute(col, 3));

    const mat = new THREE.PointsMaterial({
      size: 3.5, sizeAttenuation: true,
      transparent: true, opacity: 0.06,
      vertexColors: true,
      blending: THREE.AdditiveBlending,
      depthWrite: false,
    });

    this.nebula = new THREE.Points(geo, mat);
    this.scene.add(this.nebula);
  }

  /* ── LAYER 3: Ambient Dust ────────────────────────────── */
  private initAmbientDust() {
    const N = 600;
    const pos = new Float32Array(N * 3);
    for (let i = 0; i < N; i++) {
      pos[i * 3]     = (Math.random() - 0.5) * 80;
      pos[i * 3 + 1] = (Math.random() - 0.5) * 40;
      pos[i * 3 + 2] = (Math.random() - 0.5) * 80 - 10;
    }

    const geo = new THREE.BufferGeometry();
    geo.setAttribute('position', new THREE.BufferAttribute(pos, 3));

    const mat = new THREE.PointsMaterial({
      color: C.white, size: 0.15, sizeAttenuation: true,
      transparent: true, opacity: 0.25,
      blending: THREE.AdditiveBlending, depthWrite: false,
    });

    this.ambientDust = new THREE.Points(geo, mat);
    this.scene.add(this.ambientDust);
  }

  /* ── LAYER 4: Perspective Grid Floor ──────────────────── */
  private initGrid() {
    this.gridFloor = new THREE.Group();

    const main = new THREE.GridHelper(200, 80, C.orange, C.orange);
    main.position.y = -8;
    this.setGridOpacity(main, 0.07);
    this.gridFloor.add(main);

    const fine = new THREE.GridHelper(200, 200, C.orange, C.orange);
    fine.position.y = -8;
    this.setGridOpacity(fine, 0.025);
    this.gridFloor.add(fine);

    this.scene.add(this.gridFloor);
  }

  private setGridOpacity(grid: THREE.GridHelper, opacity: number) {
    const mats = Array.isArray(grid.material) ? grid.material : [grid.material];
    mats.forEach((m) => { m.transparent = true; m.opacity = opacity; });
  }

  /* ── LAYER 5: Sonar Pulse Rings ───────────────────────── */
  private initPulseRings() {
    this.pulseRings = new THREE.Group();
    this.scene.add(this.pulseRings);
  }

  private spawnPulse() {
    const geo = new THREE.RingGeometry(0.2, 0.5, 64);
    const mat = new THREE.MeshBasicMaterial({
      color: C.orange, transparent: true, opacity: 0.4,
      side: THREE.DoubleSide, blending: THREE.AdditiveBlending,
    });
    const ring = new THREE.Mesh(geo, mat);
    ring.rotation.x = -Math.PI / 2;
    ring.position.set(
      (Math.random() - 0.5) * 30, -7.95, (Math.random() - 0.5) * 30 - 5,
    );
    (ring as any).__pd = {
      s: 0.1, max: 30 + Math.random() * 20, spd: 0.12 + Math.random() * 0.14,
    };
    this.pulseRings.add(ring);
  }

  /* ── LAYER 6: Network Constellation ───────────────────── */
  private initNetwork() {
    this.networkNodes = new THREE.Group();
    this.networkLines = new THREE.Group();

    for (let i = 0; i < 35; i++) {
      const r     = 5 + Math.random() * 30;
      const angle = Math.random() * Math.PI * 2;
      const y     = (Math.random() - 0.5) * 18;
      const primary = Math.random() > 0.35;
      const c = primary ? C.orange : C.blue;

      const geo = new THREE.IcosahedronGeometry(0.05 + Math.random() * 0.13, 2);
      const mat = new THREE.MeshStandardMaterial({
        color: c, emissive: c, emissiveIntensity: 2.8,
        metalness: 0.9, roughness: 0.15,
      });
      const mesh = new THREE.Mesh(geo, mat);
      mesh.position.set(Math.cos(angle) * r, y, Math.sin(angle) * r - 10);
      this.networkNodes.add(mesh);

      this.nodeOrbits.push({
        mesh, baseY: y, radius: r,
        speed:  0.0001 + Math.random() * 0.0005,
        phase:  angle,
        yAmp:   0.25 + Math.random() * 0.5,
        ySpeed: 0.35 + Math.random() * 0.65,
        yPhase: Math.random() * 6.28,
      });
    }

    this.scene.add(this.networkNodes);
    this.scene.add(this.networkLines);
  }

  private rebuildLines() {
    for (const c of this.networkLines.children) {
      (c as any).geometry?.dispose();
      (c as any).material?.dispose();
    }
    this.networkLines.clear();

    const MAX = 16;
    const meshes = this.networkNodes.children as THREE.Mesh[];
    for (let i = 0; i < meshes.length; i++) {
      for (let j = i + 1; j < meshes.length; j++) {
        const d = meshes[i].position.distanceTo(meshes[j].position);
        if (d < MAX) {
          const g = new THREE.BufferGeometry().setFromPoints([
            meshes[i].position, meshes[j].position,
          ]);
          const m = new THREE.LineBasicMaterial({
            color: C.orange, transparent: true,
            opacity: (1 - d / MAX) * 0.12,
            blending: THREE.AdditiveBlending,
          });
          this.networkLines.add(new THREE.Line(g, m));
        }
      }
    }
  }

  /* ── LAYER 7a: Data Rain Streams ──────────────────────── */
  private initDataStreams() {
    this.dataStreams = new THREE.Group();

    for (let i = 0; i < 50; i++) {
      const x  = (Math.random() - 0.5) * 80;
      const z  = -10 - Math.random() * 50;
      const h  = 1 + Math.random() * 4;
      const y0 = Math.random() * 40;

      const g = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(x, y0, z),
        new THREE.Vector3(x, y0 - h, z),
      ]);
      const m = new THREE.LineBasicMaterial({
        color: C.orange, transparent: true,
        opacity: 0.06 + Math.random() * 0.16,
        blending: THREE.AdditiveBlending,
      });
      const line = new THREE.Line(g, m);
      (line as any).__sd = { spd: 0.04 + Math.random() * 0.08, top: 38, bot: -14 };
      this.dataStreams.add(line);
    }

    this.scene.add(this.dataStreams);
  }

  /* ── LAYER 7b: Shooting Stars ─────────────────────────── */
  private initShootingStars() {
    this.shootingStars = new THREE.Group();
    this.scene.add(this.shootingStars);
  }

  private spawnComet() {
    const sx  = (Math.random() - 0.5) * 120;
    const sy  = 15 + Math.random() * 30;
    const sz  = -20 - Math.random() * 40;
    const len = 4 + Math.random() * 7;
    const dir = new THREE.Vector3(
      -0.6 - Math.random() * 0.4,
      -0.4 - Math.random() * 0.4, 0,
    ).normalize();

    const g = new THREE.BufferGeometry().setFromPoints([
      new THREE.Vector3(sx, sy, sz),
      new THREE.Vector3(sx + dir.x * len, sy + dir.y * len, sz),
    ]);
    const m = new THREE.LineBasicMaterial({
      color: C.white, transparent: true, opacity: 0.75,
      blending: THREE.AdditiveBlending,
    });
    const line = new THREE.Line(g, m);
    (line as any).__cd = {
      dx: dir.x * 0.9, dy: dir.y * 0.9,
      life: 1, decay: 0.007 + Math.random() * 0.012,
    };
    this.shootingStars.add(line);
  }

  /* ── Post-processing (bloom glow) ───────────────────────── */
  private initPostProcessing() {
    const bloom = new UnrealBloomPass(
      new THREE.Vector2(window.innerWidth, window.innerHeight),
      1.6,  // strength
      0.28, // threshold
      0.7,  // radius
    );

    this.composer = new EffectComposer(this.renderer);
    this.composer.addPass(new RenderPass(this.scene, this.camera));
    this.composer.addPass(bloom);
  }

  /* ── Resize ─────────────────────────────────────────────── */
  private onResize() {
    this.camera.aspect = window.innerWidth / window.innerHeight;
    this.camera.updateProjectionMatrix();
    this.renderer.setSize(window.innerWidth, window.innerHeight);
    this.composer.setSize(window.innerWidth, window.innerHeight);
  }

  /* ═══════════════════════════════════════════════════════════
     UPDATE LOOP — called every animation frame
     ═══════════════════════════════════════════════════════════ */
  update(_scrollY: number = 0) {
    this.time += 0.016;
    this.lineFrame++;

    /* stars */
    (this.starfield.material as THREE.ShaderMaterial).uniforms.uTime.value = this.time;
    this.starfield.rotation.y += 0.00003;

    /* nebula */
    this.nebula.rotation.y += 0.00007;
    this.nebula.rotation.x  = Math.sin(this.time * 0.07) * 0.012;

    /* ambient dust */
    this.ambientDust.rotation.y -= 0.00004;
    this.ambientDust.rotation.x  = Math.sin(this.time * 0.05) * 0.008;

    /* network orbits */
    for (const n of this.nodeOrbits) {
      n.phase += n.speed;
      n.mesh.position.x = Math.cos(n.phase) * n.radius;
      n.mesh.position.z = Math.sin(n.phase) * n.radius - 10;
      n.mesh.position.y = n.baseY + Math.sin(this.time * n.ySpeed + n.yPhase) * n.yAmp;
    }
    if (this.lineFrame % 40 === 0) this.rebuildLines();

    /* data streams */
    for (const c of this.dataStreams.children) {
      const d = (c as any).__sd;
      if (!d) continue;
      c.position.y -= d.spd;
      if (c.position.y < d.bot) c.position.y = d.top;
    }

    /* pulse rings */
    if (Math.random() < 0.007) this.spawnPulse();
    for (let i = this.pulseRings.children.length - 1; i >= 0; i--) {
      const r = this.pulseRings.children[i];
      const p = (r as any).__pd;
      if (!p) continue;
      p.s += p.spd;
      r.scale.set(p.s, p.s, 1);
      ((r as any).material as THREE.Material).opacity = 0.3 * (1 - p.s / p.max);
      if (p.s >= p.max) {
        (r as any).geometry.dispose();
        (r as any).material.dispose();
        this.pulseRings.remove(r);
      }
    }

    /* shooting stars */
    if (Math.random() < 0.004) this.spawnComet();
    for (let i = this.shootingStars.children.length - 1; i >= 0; i--) {
      const s = this.shootingStars.children[i];
      const d = (s as any).__cd;
      if (!d) continue;
      s.position.x += d.dx;
      s.position.y += d.dy;
      d.life -= d.decay;
      ((s as any).material as THREE.Material).opacity = d.life * 0.7;
      if (d.life <= 0) {
        (s as any).geometry.dispose();
        (s as any).material.dispose();
        this.shootingStars.remove(s);
      }
    }

    /* camera parallax on mouse */
    this.targetPos.x = this.mouse.x * 3;
    this.targetPos.y = 2 + this.mouse.y * 1.5;
    this.targetPos.z = 22;
    this.camera.position.lerp(this.targetPos, 0.025);
    this.camera.lookAt(this.mouse.x * 4, this.mouse.y * 2, 0);

    this.composer.render();
  }

  /* ── Cleanup ────────────────────────────────────────────── */
  dispose() {
    this.renderer.dispose();
    this.composer.dispose();
  }
}
