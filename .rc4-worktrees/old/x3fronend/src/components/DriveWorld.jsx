import { useEffect, useRef, useState, useCallback } from 'react';
import * as THREE from 'three';
import { OBJLoader } from 'three/examples/jsm/loaders/OBJLoader.js';
import { MTLLoader } from 'three/examples/jsm/loaders/MTLLoader.js';
import { GLTFLoader } from 'three/examples/jsm/loaders/GLTFLoader.js';
import { Sky } from 'three/examples/jsm/objects/Sky.js';
import { useChainData } from '../hooks/useChainData.js';
import { useWallet } from '../hooks/useWallet.js';

/* ═══════════════════════════════════════════════════════════════════════
   X3 CHAIN DRIVE  ·  A Bruno Simon-inspired 3D portfolio experience
   Drive your X3 car through the blockchain world and discover secrets.
   Pure Three.js — no physics lib required.
   ═══════════════════════════════════════════════════════════════════════ */

/* ──────────────── Procedural Texture Factories ──────────────── */
function makeTex(w, h, draw) {
  const c = document.createElement('canvas');
  c.width = w; c.height = h;
  draw(c.getContext('2d'), w, h);
  const t = new THREE.CanvasTexture(c);
  t.wrapS = t.wrapT = THREE.RepeatWrapping;
  return t;
}

// Alien bioluminescent terrain — hexagonal crystal lattice + organic veins
function makeGroundTex() {
  return makeTex(512, 512, (ctx, W, H) => {
    // Deep void base — alien planet surface
    ctx.fillStyle = '#03030b';
    ctx.fillRect(0, 0, W, H);
    // Subtle volcanic rock cracks
    ctx.strokeStyle = 'rgba(60,20,100,0.22)';
    ctx.lineWidth = 0.6;
    for (let i = 0; i < 30; i++) {
      let vx = Math.random() * W, vy = Math.random() * H;
      ctx.beginPath(); ctx.moveTo(vx, vy);
      for (let s = 0; s < 6; s++) {
        vx = Math.max(0, Math.min(W, vx + (Math.random() - 0.5) * 55));
        vy = Math.max(0, Math.min(H, vy + (Math.random() - 0.5) * 55));
        ctx.lineTo(vx, vy);
      }
      ctx.stroke();
    }
    // Hexagonal crystal lattice (bioluminescent teal)
    const hexR = 26;
    const hexH = hexR * Math.sqrt(3);
    ctx.strokeStyle = 'rgba(0,255,180,0.11)';
    ctx.lineWidth = 0.9;
    for (let row = -1; row < H / hexH + 2; row++) {
      for (let col = -1; col < W / (hexR * 1.5) + 2; col++) {
        const hcx = col * hexR * 3 + (row % 2) * hexR * 1.5;
        const hcy = row * hexH;
        ctx.beginPath();
        for (let s = 0; s < 6; s++) {
          const a = (Math.PI / 3) * s - Math.PI / 6;
          const px = hcx + hexR * 0.86 * Math.cos(a);
          const py = hcy + hexR * 0.86 * Math.sin(a);
          s === 0 ? ctx.moveTo(px, py) : ctx.lineTo(px, py);
        }
        ctx.closePath();
        ctx.stroke();
      }
    }
    // Organic bioluminescent veins (violet/purple)
    ctx.strokeStyle = 'rgba(160,40,255,0.18)';
    ctx.lineWidth = 1.2;
    for (let i = 0; i < 22; i++) {
      let vx = Math.random() * W, vy = Math.random() * H;
      ctx.beginPath(); ctx.moveTo(vx, vy);
      for (let s = 0; s < 10; s++) {
        vx = Math.max(0, Math.min(W, vx + (Math.random() - 0.5) * 70));
        vy = Math.max(0, Math.min(H, vy + (Math.random() - 0.5) * 70));
        ctx.lineTo(vx, vy);
      }
      ctx.stroke();
    }
    // Bioluminescent glowing crystal nodes
    for (let i = 0; i < 100; i++) {
      const nx = Math.random() * W, ny = Math.random() * H;
      const nr = 0.8 + Math.random() * 2.5;
      const cols = ['rgba(0,255,180,0.9)','rgba(0,140,255,0.85)','rgba(200,60,255,0.85)','rgba(0,230,255,0.75)','rgba(255,120,40,0.6)'];
      const c = cols[Math.floor(Math.random() * cols.length)];
      const grad = ctx.createRadialGradient(nx, ny, 0, nx, ny, nr * 7);
      grad.addColorStop(0, c); grad.addColorStop(1, 'rgba(0,0,0,0)');
      ctx.fillStyle = grad;
      ctx.beginPath(); ctx.arc(nx, ny, nr * 7, 0, Math.PI * 2); ctx.fill();
    }
    // Faint energy grid overlay
    ctx.strokeStyle = 'rgba(0,60,160,0.06)';
    ctx.lineWidth = 0.4;
    for (let x = 0; x <= W; x += 32) { ctx.beginPath(); ctx.moveTo(x,0); ctx.lineTo(x,H); ctx.stroke(); }
    for (let y = 0; y <= H; y += 32) { ctx.beginPath(); ctx.moveTo(0,y); ctx.lineTo(W,y); ctx.stroke(); }
  });
}

// Asphalt with lane markings
function makeRoadTex() {
  return makeTex(128, 512, (ctx, W, H) => {
    ctx.fillStyle = '#08081c';
    ctx.fillRect(0, 0, W, H);
    // Asphalt grain
    for (let i = 0; i < 500; i++) {
      ctx.fillStyle = `rgba(255,255,255,${Math.random() * 0.022})`;
      ctx.fillRect(Math.random()*W, Math.random()*H, Math.random()*2+0.5, Math.random()*2+0.5);
    }
    // White edge stripes
    ctx.strokeStyle = 'rgba(255,255,255,0.5)';
    ctx.lineWidth = 2.5;
    ctx.beginPath(); ctx.moveTo(5,0); ctx.lineTo(5,H); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(W-5,0); ctx.lineTo(W-5,H); ctx.stroke();
    // Yellow center dashes
    ctx.strokeStyle = 'rgba(255,210,0,0.72)';
    ctx.lineWidth = 3;
    ctx.setLineDash([38, 26]);
    ctx.beginPath(); ctx.moveTo(W/2, 0); ctx.lineTo(W/2, H); ctx.stroke();
    ctx.setLineDash([]);
    // Neon edge glow
    const eg = ctx.createLinearGradient(0, 0, W, 0);
    eg.addColorStop(0, 'rgba(0,212,255,0.12)');
    eg.addColorStop(0.15, 'rgba(0,0,0,0)');
    eg.addColorStop(0.85, 'rgba(0,0,0,0)');
    eg.addColorStop(1, 'rgba(0,212,255,0.12)');
    ctx.fillStyle = eg;
    ctx.fillRect(0, 0, W, H);
  });
}

// Carbon fiber weave for car body
function makeCarbonTex() {
  return makeTex(128, 128, (ctx, W, H) => {
    const sz = 8;
    for (let row = 0; row < H/sz; row++) {
      for (let col = 0; col < W/sz; col++) {
        const even = (row + col) % 2 === 0;
        ctx.fillStyle = even ? '#0b0b26' : '#12122f';
        ctx.fillRect(col*sz, row*sz, sz, sz);
        // fiber highlight
        ctx.fillStyle = even ? 'rgba(0,212,255,0.055)' : 'rgba(168,85,247,0.04)';
        ctx.fillRect(col*sz, row*sz, sz/2, sz);
        ctx.fillStyle = 'rgba(255,255,255,0.025)';
        ctx.fillRect(col*sz, row*sz, sz, 1);
      }
    }
    // Lateral glow trim
    const g = ctx.createLinearGradient(0,0,W,0);
    g.addColorStop(0, 'rgba(0,212,255,0.14)');
    g.addColorStop(0.35, 'rgba(0,0,0,0)');
    g.addColorStop(0.65, 'rgba(0,0,0,0)');
    g.addColorStop(1, 'rgba(0,212,255,0.14)');
    ctx.fillStyle = g;
    ctx.fillRect(0,0,W,H);
  });
}

// Tech panel texture for platform slabs
function makePanelTex(hue) {
  return makeTex(256, 256, (ctx, W, H) => {
    ctx.fillStyle = '#0a0a1e';
    ctx.fillRect(0,0,W,H);
    // Panel lines
    ctx.strokeStyle = `${hue || 'rgba(0,180,255,0.25)'}`;
    ctx.lineWidth = 1;
    for (let i = 0; i < H; i += 20) { ctx.beginPath(); ctx.moveTo(0,i); ctx.lineTo(W,i); ctx.stroke(); }
    for (let i = 0; i < W; i += 20) { ctx.beginPath(); ctx.moveTo(i,0); ctx.lineTo(i,H); ctx.stroke(); }
    // Hex grid overlay
    const hr = 18, hh = hr * Math.sqrt(3)/2;
    ctx.strokeStyle = hue || 'rgba(0,212,255,0.15)';
    ctx.lineWidth = 0.8;
    for (let row = 0; row < 8; row++) {
      for (let col = 0; col < 8; col++) {
        const cx = col * hr * 1.5 + (row%2)*hr*0.75;
        const cy = row * hh;
        ctx.beginPath();
        for (let s = 0; s < 6; s++) {
          const a = (Math.PI/3)*s - Math.PI/6;
          const px = cx + hr*0.7*Math.cos(a), py = cy + hr*0.7*Math.sin(a);
          s===0 ? ctx.moveTo(px,py) : ctx.lineTo(px,py);
        }
        ctx.closePath(); ctx.stroke();
      }
    }
  });
}

/* ──────────────── Car Builder ──────────────── */
function buildUFO(scene) {
  const group = new THREE.Group();

  // ── Materials ──
  const _cTex   = makeCarbonTex();
  const hullM   = new THREE.MeshLambertMaterial({ map: _cTex, color: '#ccddff' });
  const domeM   = new THREE.MeshLambertMaterial({ color: '#0a2244', transparent: true, opacity: 0.72 });
  const ringM   = new THREE.MeshLambertMaterial({ color: '#00d4ff', emissive: '#00d4ff', emissiveIntensity: 0.9 });
  const glowM   = new THREE.MeshLambertMaterial({ color: '#44ffcc', emissive: '#44ffcc', emissiveIntensity: 1.6, transparent: true, opacity: 0.85 });
  const engineM = new THREE.MeshLambertMaterial({ color: '#7744ff', emissive: '#7744ff', emissiveIntensity: 2.0, transparent: true, opacity: 0.7 });
  const portM   = new THREE.MeshLambertMaterial({ color: '#ff44aa', emissive: '#ff44aa', emissiveIntensity: 1.4 });

  // ── Lower disc hull ──
  const discGeo = new THREE.CylinderGeometry(2.2, 1.4, 0.38, 28);
  const disc = new THREE.Mesh(discGeo, hullM);
  disc.castShadow = true;
  group.add(disc);

  // ── Upper dome ──
  const dome = new THREE.Mesh(new THREE.SphereGeometry(1.45, 20, 12, 0, Math.PI * 2, 0, Math.PI * 0.5), domeM);
  dome.position.y = 0.19;
  group.add(dome);

  // ── Dome ring (equator band) ──
  const domeRing = new THREE.Mesh(new THREE.TorusGeometry(1.44, 0.07, 8, 36), ringM);
  domeRing.position.y = 0.19;
  domeRing.rotation.x = Math.PI / 2;
  group.add(domeRing);

  // ── Main rotating outer ring ──
  const outerRing = new THREE.Mesh(new THREE.TorusGeometry(2.55, 0.14, 8, 48), ringM);
  outerRing.rotation.x = Math.PI / 2;
  outerRing.position.y = -0.05;
  group.add(outerRing);
  outerRing.userData.ufoRing = true;

  // ── Light pods around hull equator (8 evenly spaced) ──
  const podColors = ['#00ffff','#ff44cc','#44ffaa','#ff8800','#00aaff','#ff44cc','#44ffaa','#ff8800'];
  const pods = [];
  for (let i = 0; i < 8; i++) {
    const angle = (i / 8) * Math.PI * 2;
    const pod = new THREE.Mesh(new THREE.SphereGeometry(0.13, 6, 6), new THREE.MeshLambertMaterial({
      color: podColors[i], emissive: podColors[i], emissiveIntensity: 2.2
    }));
    pod.position.set(Math.cos(angle) * 2.1, -0.1, Math.sin(angle) * 2.1);
    group.add(pod);
    pods.push(pod);
  }

  // ── Engine thruster glow disc (underside) ──
  const thrust = new THREE.Mesh(new THREE.CylinderGeometry(0.9, 1.3, 0.12, 20), engineM);
  thrust.position.y = -0.22;
  group.add(thrust);

  // ── Tractor beam (cone pointing down) ──
  const beam = new THREE.Mesh(
    new THREE.ConeGeometry(1.8, 5.5, 20, 1, true),
    new THREE.MeshLambertMaterial({ color: '#44ffcc', emissive: '#44ffcc', emissiveIntensity: 0.5, transparent: true, opacity: 0.12, side: THREE.DoubleSide })
  );
  beam.position.y = -3.1;
  beam.rotation.x = Math.PI;
  group.add(beam);

  // ── Engine under-glow point light ──
  const engineLight = new THREE.PointLight('#7744ff', 3.5, 14);
  engineLight.position.y = -1.2;
  group.add(engineLight);

  // ── Forward spotlight ──
  const spotFwd = new THREE.PointLight('#00ffff', 2.0, 22);
  spotFwd.position.set(0, 0, 2.8);
  group.add(spotFwd);

  scene.add(group);
  return { group, pods, outerRing, engineLight, beam, disc, dome, domeRing };
}

/* ──────────────── World Builder ──────────────── */
function buildWorld(scene, chainRefs = {}) {
  const { genesisThreeRef, valPylonArr } = chainRefs;
  const hotspots = [];

  // Helper: create a glowing pylon
  function glowyPylon(color, emissive, height, x, z) {
    const g = new THREE.Group();
    const body = new THREE.Mesh(
      new THREE.CylinderGeometry(0.22, 0.35, height, 8),
      new THREE.MeshLambertMaterial({ color, emissive, emissiveIntensity: 0.4 })
    );
    body.position.y = height / 2;
    body.castShadow = true;
    g.add(body);
    const cap = new THREE.Mesh(
      new THREE.SphereGeometry(0.32, 8, 8),
      new THREE.MeshLambertMaterial({ color: emissive, emissive, emissiveIntensity: 1.5 })
    );
    cap.position.y = height + 0.15;
    g.add(cap);
    const ptLight = new THREE.PointLight(emissive, 1.0, 8);
    ptLight.position.y = height + 0.5;
    g.add(ptLight);
    g.position.set(x, 0, z);
    scene.add(g);
    return g;
  }

  // Helper: flat slab platform
  const _panelTex = makePanelTex();
  function platform(w, d, h, color, x, z) {
    const tex = _panelTex.clone();
    tex.wrapS = tex.wrapT = THREE.RepeatWrapping;
    tex.repeat.set(w / 3, d / 3);
    tex.needsUpdate = true;
    const m = new THREE.Mesh(
      new THREE.BoxGeometry(w, h, d),
      new THREE.MeshLambertMaterial({ map: tex, color: '#aaaacc' })
    );
    m.position.set(x, h / 2, z);
    m.castShadow = true;
    m.receiveShadow = true;
    scene.add(m);
    return m;
  }

  // Helper: sign billboard
  function sign(textColor, emissive, w, h, x, y, z, rotY) {
    const g = new THREE.Group();
    // Post
    const post = new THREE.Mesh(
      new THREE.CylinderGeometry(0.06, 0.06, y + h / 2, 6),
      new THREE.MeshLambertMaterial({ color: '#444466' })
    );
    post.position.y = (y + h / 2) / 2;
    g.add(post);
    // Board
    const board = new THREE.Mesh(
      new THREE.BoxGeometry(w, h, 0.12),
      new THREE.MeshLambertMaterial({ color: textColor, emissive, emissiveIntensity: 0.25 })
    );
    board.position.y = y + h / 2;
    g.add(board);
    g.position.set(x, 0, z);
    g.rotation.y = rotY || 0;
    scene.add(g);
    return g;
  }

  // ── Alien city cluster builder — creates a dense district of towers around a hotspot ──
  function buildClusterTowers(cx, cz, mainHex, altHex, count, spread) {
    const mainC = new THREE.Color(mainHex);
    const altC  = new THREE.Color(altHex);
    for (let i = 0; i < count; i++) {
      const angle  = (i / count) * Math.PI * 2 + Math.random() * 0.5;
      const r      = spread * (0.28 + Math.random() * 0.72);
      const tx     = cx + Math.cos(angle) * r;
      const tz     = cz + Math.sin(angle) * r;
      const h      = 20 + Math.random() * 42;  // 20–62 units tall — way above ship at y=8
      const bw     = 1.5 + Math.random() * 2.8;
      const bd     = 1.0 + Math.random() * 2.4;
      const tColor = Math.random() > 0.5 ? mainC : altC;

      // Main tower body — alien dark material with emissive accent
      const tower = new THREE.Mesh(
        new THREE.BoxGeometry(bw, h, bd),
        new THREE.MeshStandardMaterial({
          color: 0x05050e, emissive: tColor,
          emissiveIntensity: 0.06 + Math.random() * 0.12,
          metalness: 0.88, roughness: 0.20,
        })
      );
      tower.position.set(tx, h / 2, tz);
      tower.castShadow = true;
      scene.add(tower);

      // Glowing window bands
      const bandCount = Math.max(2, Math.floor(h / 7));
      for (let b = 0; b < bandCount; b++) {
        if (Math.random() > 0.40) {
          const band = new THREE.Mesh(
            new THREE.BoxGeometry(bw + 0.06, 0.22, bd + 0.06),
            new THREE.MeshStandardMaterial({
              color: tColor.getHex(), emissive: tColor,
              emissiveIntensity: 1.6 + Math.random() * 1.2,
            })
          );
          band.position.set(tx, b * (h / bandCount) + 1.8, tz);
          scene.add(band);
        }
      }

      // Tapered crystal spire on top (half of towers)
      if (Math.random() > 0.45) {
        const spire = new THREE.Mesh(
          new THREE.CylinderGeometry(0.05, bw * 0.40, h * 0.30, 4),
          new THREE.MeshStandardMaterial({ color: tColor.getHex(), emissive: tColor, emissiveIntensity: 0.95 })
        );
        spire.position.set(tx, h + h * 0.15, tz);
        scene.add(spire);
      }

      // Top point light (every 3rd tower)
      if (i % 3 === 0) {
        const ptl = new THREE.PointLight(tColor.getHex(), 0.75, 30);
        ptl.position.set(tx, h + 1.5, tz);
        scene.add(ptl);
      }
    }

    // Hexagonal ground plaza pads
    for (let p = 0; p < 8; p++) {
      const pa = (p / 8) * Math.PI * 2;
      const pr = spread * 0.28;
      const pad = new THREE.Mesh(
        new THREE.CylinderGeometry(1.8, 1.8, 0.20, 6),
        new THREE.MeshStandardMaterial({
          color: 0x07070d, emissive: new THREE.Color(mainHex),
          emissiveIntensity: 0.50, metalness: 0.7, roughness: 0.3,
        })
      );
      pad.position.set(cx + Math.cos(pa) * pr, 0.10, cz + Math.sin(pa) * pr);
      scene.add(pad);
    }

    // Central atmospheric city light
    const cl = new THREE.PointLight(mainHex, 2.5, spread * 3.0);
    cl.position.set(cx, 16, cz);
    scene.add(cl);
  }

  // ── Ground patches ──
  const patchColors = ['#151530', '#12122e', '#1a1a3a', '#0f0f28', '#161636'];
  for (let i = 0; i < 40; i++) {
    const angle = Math.random() * Math.PI * 2;
    const r = 10 + Math.random() * 65;
    const w = 6 + Math.random() * 14;
    const d = 6 + Math.random() * 14;
    const patch = new THREE.Mesh(
      new THREE.BoxGeometry(w, 0.02, d),
      new THREE.MeshLambertMaterial({ color: patchColors[i % patchColors.length] })
    );
    patch.position.set(Math.cos(angle) * r, 0.01, Math.sin(angle) * r);
    patch.rotation.y = Math.random() * Math.PI;
    scene.add(patch);
  }

  // ── Road network ──
  const _rTex = makeRoadTex();
  const roads = [
    [80, 3.5, 0, 0, 0, 0,  18, 1],   // E-W main road
    [3.5, 80, 0, 0, 0, 0,  1, 18],   // N-S main road
    [50, 3, 25, 0, -18, Math.PI/4,  13, 1],
    [50, 3, -22, 0, 18, Math.PI/4,  13, 1],
  ];
  for (const [rw, rd, rx, ry, rz, rot, repU, repV] of roads) {
    const tex = _rTex.clone();
    tex.wrapS = tex.wrapT = THREE.RepeatWrapping;
    tex.repeat.set(repU, repV);
    tex.needsUpdate = true;
    const roadMat = new THREE.MeshLambertMaterial({ map: tex, color: '#ffffff' });
    const road = new THREE.Mesh(new THREE.BoxGeometry(rw, 0.04, rd), roadMat);
    road.position.set(rx, 0.02, rz);
    road.rotation.y = rot;
    road.receiveShadow = true;
    scene.add(road);
  }

  // ── Road markings (dashed) ──
  const dashMat = new THREE.MeshLambertMaterial({ color: '#3a3a6a' });
  for (let i = -8; i <= 8; i++) {
    const d = new THREE.Mesh(new THREE.BoxGeometry(0.15, 0.04, 3), dashMat);
    d.position.set(0, 0.03, i * 6);
    scene.add(d);
    const d2 = new THREE.Mesh(new THREE.BoxGeometry(3, 0.04, 0.15), dashMat);
    d2.position.set(i * 6, 0.03, 0);
    scene.add(d2);
  }

  // ══════════════════════════════════════════
  // HOTSPOT 1: GENESIS MONUMENT  (0, 0, -18)
  // ══════════════════════════════════════════
  {
    const g = new THREE.Group();
    // Obelisk
    const ob = new THREE.Mesh(
      new THREE.CylinderGeometry(0.1, 0.8, 7, 4),
      new THREE.MeshLambertMaterial({ color: '#0a0a28', emissive: '#00d4ff', emissiveIntensity: 0.15 })
    );
    ob.position.y = 3.5;
    ob.castShadow = true;
    g.add(ob);
    // Tip glow
    const tip = new THREE.Mesh(
      new THREE.TetrahedronGeometry(0.4),
      new THREE.MeshLambertMaterial({ color: '#00ffff', emissive: '#00ffff', emissiveIntensity: 2.0 })
    );
    tip.position.y = 7.4;
    g.add(tip);
    genesisThreeRef.current.tip = tip; // chain-reactive
    const ptLight = new THREE.PointLight('#00ffff', 2.0, 18);
    ptLight.position.y = 8;
    g.add(ptLight);
    genesisThreeRef.current.ptLight = ptLight; // chain-reactive
    // Base platform steps
    for (let step = 0; step < 3; step++) {
      const s = new THREE.Mesh(
        new THREE.CylinderGeometry(2 - step * 0.5, 2 - step * 0.5, 0.35, 8),
        new THREE.MeshLambertMaterial({ color: '#0d0d28' })
      );
      s.position.y = step * 0.35;
      s.castShadow = true;
      s.receiveShadow = true;
      g.add(s);
    }
    // Orbiting rings
    for (let r = 0; r < 2; r++) {
      const ring = new THREE.Mesh(
        new THREE.TorusGeometry(2.5 + r * 1.2, 0.06, 8, 40),
        new THREE.MeshLambertMaterial({ color: '#00d4ff', emissive: '#00d4ff', emissiveIntensity: 0.6 })
      );
      ring.rotation.x = 0.6 + r * 0.4;
      ring.rotation.z = r * 0.3;
      ring.position.y = 3.5;
      g.add(ring);
      // Animate rings in user data
      ring.userData.ring = true;
      ring.userData.rotSpeedX = 0.004 + r * 0.002;
      ring.userData.rotSpeedY = 0.007 + r * 0.003;
    }
    g.position.set(0, 0, -18);
    scene.add(g);
    hotspots.push({
      mesh: g,
      pos: new THREE.Vector3(0, 0, -18),
      radius: 9,
      title: 'Genesis Block',
      emoji: '⛩',
      body: 'X3 Chain launched with 1,847 genesis validators across 6 continents. The first block was sealed with 4,200 TPS — a new standard for Layer-1 throughput.',
      color: '#00d4ff',
      link: '/?section=dashboard',
    });
  }

  // ══════════════════════════════════════════
  // HOTSPOT 2: VALIDATOR FOREST  (130, 0, 0)
  // ══════════════════════════════════════════
  {
    const valColors = ['#00d4ff', '#a855f7', '#22d3ee', '#818cf8', '#00ffaa'];
    for (let i = 0; i < 7; i++) {
      const angle = (i / 7) * Math.PI * 2;
      const r = 4.5;
      const h = 3.5 + Math.random() * 3.5;
      const c = valColors[i % valColors.length];
      const pylon = glowyPylon('#0f0f30', c, h, 130 + Math.cos(angle) * r, Math.sin(angle) * r);
      // Mark one for animation
      pylon.userData.valTower = true;
      pylon.userData.pulsePhase = i * 0.9;
      pylon.userData.valIndex = i;
      valPylonArr.current.push(pylon); // chain-reactive
    }
    // Center console
    const console1 = platform(2.5, 2.5, 0.8, '#0d0d2e', 130, 0);
    const screen1 = new THREE.Mesh(
      new THREE.BoxGeometry(1.8, 1.2, 0.12),
      new THREE.MeshLambertMaterial({ color: '#00d4ff', emissive: '#00d4ff', emissiveIntensity: 0.4 })
    );
    screen1.position.set(130, 1.4, 0);
    screen1.castShadow = false;
    scene.add(screen1);

    hotspots.push({
      pos: new THREE.Vector3(130, 0, 0),
      radius: 10,
      title: 'Validator Forest',
      emoji: '🛡',
      body: '1,847 genesis validators operate X3 consensus. Each node stakes X3 tokens and earns 12–18% APR. BFT + PoS hybrid — no slashable conditions in normal operation.',
      color: '#a855f7',
      link: '/x3star-validator-presale.html?from=drive',
      linkText: 'Join Validators →',
    });
  }

  // ══════════════════════════════════════════
  // HOTSPOT 3: DEX ARENA  (-130, 0, 0)
  // ══════════════════════════════════════════
  {
    // Ring structure
    const arena = new THREE.Mesh(
      new THREE.TorusGeometry(5.5, 0.5, 8, 32),
      new THREE.MeshLambertMaterial({ color: '#1a0a2e', emissive: '#ff6600', emissiveIntensity: 0.3 })
    );
    arena.rotation.x = Math.PI / 2;
    arena.position.set(-130, 0.5, 0);
    arena.castShadow = true;
    scene.add(arena);
    arena.userData.spinY = 0.003;

    // Inner columns
    for (let i = 0; i < 8; i++) {
      const angle = (i / 8) * Math.PI * 2;
      const col = new THREE.Mesh(
        new THREE.CylinderGeometry(0.18, 0.25, 2.5, 8),
        new THREE.MeshLambertMaterial({ color: '#200a2e' })
      );
      col.position.set(-130 + Math.cos(angle) * 3.5, 1.25, Math.sin(angle) * 3.5);
      col.castShadow = true;
      scene.add(col);
    }

    // Central swap orb
    const orb = new THREE.Mesh(
      new THREE.IcosahedronGeometry(1.2, 1),
      new THREE.MeshLambertMaterial({ color: '#ff6600', emissive: '#ff6600', emissiveIntensity: 0.8 })
    );
    orb.position.set(-130, 2, 0);
    scene.add(orb);
    orb.userData.spinY = 0.015;
    orb.userData.bouncePhase = 0;
    const orbLight = new THREE.PointLight('#ff6600', 2, 15);
    orbLight.position.set(-130, 2, 0);
    scene.add(orbLight);

    hotspots.push({
      pos: new THREE.Vector3(-130, 0, 0),
      radius: 9,
      title: 'DEX Atomic Swap',
      emoji: '⬡',
      body: 'Cross-VM trades settle in a single atomic transaction — zero bridge risk. Swap EVM ↔ SVM ↔ X3-native tokens at $0.0001 per tx with sub-second finality.',
      color: '#ff6600',
      link: '/x3star-landing.html?section=dex&from=drive',
      linkText: 'Open DEX →',
    });
  }

  // ══════════════════════════════════════════
  // HOTSPOT 4: TREASURY VAULT  (0, 0, 140)
  // ══════════════════════════════════════════
  {
    // Main vault building
    const vaultBase = platform(8, 10, 2, '#0a0a20', 0, 140);
    const vault = new THREE.Mesh(
      new THREE.BoxGeometry(6, 5, 8),
      new THREE.MeshLambertMaterial({ color: '#0d0d28', emissive: '#ffd700', emissiveIntensity: 0.06 })
    );
    vault.position.set(0, 4.5, 140);
    vault.castShadow = true;
    scene.add(vault);
    // Columns
    for (const cx of [-3.5, 3.5]) {
      const col = new THREE.Mesh(
        new THREE.CylinderGeometry(0.3, 0.3, 5, 8),
        new THREE.MeshLambertMaterial({ color: '#161636' })
      );
      col.position.set(cx, 4.5, 27);
      col.castShadow = true;
      scene.add(col);
    }
    // Gold bars on top
    for (let i = -1; i <= 1; i++) {
      const bar = new THREE.Mesh(
        new THREE.BoxGeometry(0.6, 0.35, 1.2),
        new THREE.MeshLambertMaterial({ color: '#ffd700', emissive: '#ffd700', emissiveIntensity: 0.6 })
      );
      bar.position.set(i * 0.8, 7.3, 30);
      scene.add(bar);
    }
    const vaultLight = new THREE.PointLight('#ffd700', 1.5, 14);
    vaultLight.position.set(0, 8, 30);
    scene.add(vaultLight);

    hotspots.push({
      pos: new THREE.Vector3(0, 0, 140),
      radius: 10,
      title: 'Treasury Vault',
      emoji: '🏦',
      body: '$48.2M TVL locked in X3 protocol contracts. DAO-controlled multi-sig treasury. All protocol fees flow here and are distributed via governance vote.',
      color: '#ffd700',
      link: '/x3star-governance.html?from=drive',
      linkText: 'View Treasury →',
    });
  }

  // ══════════════════════════════════════════
  // HOTSPOT 5: GRANT TOWER  (-110, 0, -110)
  // ══════════════════════════════════════════
  {
    // Tower
    for (let f = 0; f < 5; f++) {
      const floor = new THREE.Mesh(
        new THREE.BoxGeometry(4.5 - f * 0.25, 1.8, 4.5 - f * 0.25),
        new THREE.MeshLambertMaterial({ color: '#0a1428' })
      );
      floor.position.set(-110, 0.9 + f * 1.8, -110);
      floor.castShadow = true;
      floor.receiveShadow = true;
      scene.add(floor);
      // Window lights per floor
      for (const wx of [-1.5, 0, 1.5]) {
        const win = new THREE.Mesh(
          new THREE.BoxGeometry(0.6, 0.5, 0.08),
          new THREE.MeshLambertMaterial({ color: '#00ff88', emissive: '#00ff88', emissiveIntensity: 0.5 })
        );
        win.position.set(-110 + wx, 0.9 + f * 1.8, -107.7);
        scene.add(win);
      }
    }
    const grantLight = new THREE.PointLight('#00ff88', 1.2, 14);
    grantLight.position.set(-110, 9, -110);
    scene.add(grantLight);

    hotspots.push({
      pos: new THREE.Vector3(-110, 0, -110),
      radius: 9,
      title: 'Developer Grants',
      emoji: '🏗',
      body: '$5M+ deployed across 48 active grants. Projects range from AI inference, post-quantum cryptography, DeFi composability, and zkVM infrastructure.',
      color: '#00ff88',
      link: '/x3star-grant-hub.html?from=drive',
      linkText: 'Apply for Grant →',
    });
  }

  // ══════════════════════════════════════════
  // HOTSPOT 6: COMPUTE CLUSTER  (110, 0, 110)
  // ══════════════════════════════════════════
  {
    // Server racks
    for (let row = 0; row < 3; row++) {
      for (let col = 0; col < 4; col++) {
        const rack = new THREE.Mesh(
          new THREE.BoxGeometry(0.8, 2.5, 0.4),
          new THREE.MeshLambertMaterial({ color: '#080820' })
        );
        rack.position.set(108 + col * 1.2, 1.25, 109 + row * 1.2);
        rack.castShadow = true;
        scene.add(rack);
        // LEDs
        for (let led = 0; led < 5; led++) {
          const ledMesh = new THREE.Mesh(
            new THREE.BoxGeometry(0.08, 0.06, 0.1),
            new THREE.MeshLambertMaterial({
              color: ['#00ffff', '#00ff88', '#a855f7'][Math.floor(Math.random() * 3)],
              emissive: '#00ffff',
              emissiveIntensity: 0.8,
            })
          );
          ledMesh.position.set(108 + col * 1.2 + 0.32, 0.5 + led * 0.38, 109 + row * 1.2 + 0.24);
          scene.add(ledMesh);
        }
      }
    }
    const compLight = new THREE.PointLight('#a855f7', 1.2, 14);
    compLight.position.set(110, 4, 110);
    scene.add(compLight);

    hotspots.push({
      pos: new THREE.Vector3(110, 0, 110),
      radius: 9,
      title: 'Compute Marketplace',
      emoji: '🖥',
      body: 'Decentralised GPU/CPU orchestration for ZK proving and AI inference. Bid on compute in real-time. No KYC, no middlemen — pay in X3 tokens.',
      color: '#a855f7',
      link: '/x3star-compute-marketplace.html?from=drive',
      linkText: 'Access Compute →',
    });
  }

  // ══════════════════════════════════════════
  // HOTSPOT 7: FLASHLOAN ENGINE  (0, 0, -150)
  // ══════════════════════════════════════════
  {
    // Cylindrical reactor
    const reactor = new THREE.Mesh(
      new THREE.CylinderGeometry(2.2, 2.8, 4, 12),
      new THREE.MeshLambertMaterial({ color: '#200006', emissive: '#ff0055', emissiveIntensity: 0.15 })
    );
    reactor.position.set(0, 2, -150);
    reactor.castShadow = true;
    scene.add(reactor);
    // Rings
    for (let r = 0; r < 3; r++) {
      const ring = new THREE.Mesh(
        new THREE.TorusGeometry(2.5 + r * 0.5, 0.12, 6, 30),
        new THREE.MeshLambertMaterial({ color: '#ff0055', emissive: '#ff0055', emissiveIntensity: 0.7 })
      );
      ring.position.set(0, 2, -150);
      ring.userData.spinY = 0.012 + r * 0.005;
      scene.add(ring);
    }
    const flashLight = new THREE.PointLight('#ff0055', 2, 16);
    flashLight.position.set(0, 4, -150);
    scene.add(flashLight);

    hotspots.push({
      pos: new THREE.Vector3(0, 0, -150),
      radius: 9,
      title: 'Flashloan Engine',
      emoji: '🔥',
      body: 'Institutional-grade uncollateralised liquidity — borrow any amount, execute strategy, repay — in a single atomic block. 0.05% flat fee, no credit check.',
      color: '#ff0055',
      link: '/x3star-tech-deep-dive.html?section=flashloans&from=drive',
      linkText: 'Read Docs →',
    });
  }

  // ══════════════════════════════════════════
  // HOTSPOT 8: WHITEPAPER LIBRARY  (120, 0, -110)
  // ══════════════════════════════════════════
  {
    const lib = new THREE.Mesh(
      new THREE.BoxGeometry(7, 4.5, 6),
      new THREE.MeshLambertMaterial({ color: '#0a0a1e' })
    );
    lib.position.set(120, 2.25, -110);
    lib.castShadow = true;
    scene.add(lib);
    // Roof triangle
    const roof = new THREE.Mesh(
      new THREE.CylinderGeometry(0, 4.5, 2, 4),
      new THREE.MeshLambertMaterial({ color: '#151538' })
    );
    roof.rotation.y = Math.PI / 4;
    roof.position.set(120, 5.5, -110);
    scene.add(roof);
    // Book stacks (colorful)
    const bookColors = ['#00d4ff', '#a855f7', '#ffd700', '#00ff88', '#ff6600'];
    for (let i = 0; i < 5; i++) {
      const book = new THREE.Mesh(
        new THREE.BoxGeometry(0.25, 0.7 + Math.random() * 0.4, 0.4),
        new THREE.MeshLambertMaterial({ color: bookColors[i], emissive: bookColors[i], emissiveIntensity: 0.2 })
      );
      book.position.set(118.5 + i * 0.35, 2.2, -107.5);
      scene.add(book);
    }
    sign('#e8e0c8', '#ffd700', 3.5, 0.8, 120, 1.5, -107.4, 0);

    hotspots.push({
      pos: new THREE.Vector3(120, 0, -110),
      radius: 9,
      title: 'Whitepaper Library',
      emoji: '📜',
      body: 'X3 technical whitepaper, governance proposals, audit reports, and developer documentation — all on-chain IPFS linked, immutable, and open source.',
      color: '#e8e0c8',
      link: '/x3star-whitepaper.html?from=drive',
      linkText: 'Read Whitepaper →',
    });
  }

  // ── Scattered decorative objects ──
  // Crystals / spires
  const crystalColors = ['#00d4ff', '#a855f7', '#00ff88', '#ffd700', '#ff6600'];
  for (let i = 0; i < 60; i++) {
    const angle = Math.random() * Math.PI * 2;
    const r = 20 + Math.random() * 140;
    const cx = Math.cos(angle) * r;
    const cz = Math.sin(angle) * r;
    // Avoid hotspot areas
    const tooClose = hotspots.some(h => Math.hypot(cx - h.pos.x, cz - h.pos.z) < 12);
    if (tooClose) continue;
    const h = 1.5 + Math.random() * 4;
    const col = crystalColors[i % crystalColors.length];
    const cryst = new THREE.Mesh(
      new THREE.ConeGeometry(0.15 + Math.random() * 0.2, h, 4 + Math.floor(Math.random() * 3)),
      new THREE.MeshLambertMaterial({ color: col, emissive: col, emissiveIntensity: 0.25 })
    );
    cryst.position.set(cx, h / 2, cz);
    cryst.rotation.y = Math.random() * Math.PI;
    cryst.castShadow = true;
    scene.add(cryst);
  }

  // Street lights along main roads
  for (let i = -6; i <= 6; i += 3) {
    for (const side of [-2.8, 2.8]) {
      const sl = new THREE.Group();
      const pole = new THREE.Mesh(new THREE.CylinderGeometry(0.05, 0.05, 3, 6), new THREE.MeshLambertMaterial({ color: '#222244' }));
      pole.position.y = 1.5;
      sl.add(pole);
      const arm = new THREE.Mesh(new THREE.BoxGeometry(0.7, 0.05, 0.05), new THREE.MeshLambertMaterial({ color: '#222244' }));
      arm.position.set(-0.3, 3.05, 0);
      sl.add(arm);
      const bulb = new THREE.Mesh(new THREE.SphereGeometry(0.12, 6, 6), new THREE.MeshLambertMaterial({ color: '#a0c0ff', emissive: '#a0c0ff', emissiveIntensity: 1.0 }));
      bulb.position.set(-0.6, 3.05, 0);
      sl.add(bulb);
      const ptl = new THREE.PointLight('#6080cc', 0.5, 6);
      ptl.position.set(-0.6, 3.2, 0);
      sl.add(ptl);
      sl.position.set(i * 5, 0, side);
      scene.add(sl);
    }
  }

  // ── ALIEN CITY DISTRICTS — dense building clusters around each blockchain landmark ──
  // Each district has its own color identity and towers 20–60 units tall
  buildClusterTowers(   0,  -18, '#00d4ff', '#a855f7', 14, 26); // GENESIS CORE — teal/violet
  buildClusterTowers( 130,    0, '#4466ff', '#7799ee', 12, 22); // VALIDATOR DISTRICT — blue/indigo
  buildClusterTowers(-130,    0, '#ff7700', '#ff4400', 12, 24); // ATOMIC DEX PLAZA — orange/red
  buildClusterTowers(   0,  140, '#ffd700', '#ff9900', 12, 22); // TREASURY CITADEL — gold/amber
  buildClusterTowers(-110, -110, '#00ff88', '#00cc66', 10, 20); // GRANT ACADEMY — emerald/green
  buildClusterTowers( 110,  110, '#aa44ff', '#7711dd', 10, 20); // COMPUTE NEXUS — purple/violet
  buildClusterTowers(   0, -150, '#ff0044', '#cc0033',  8, 18); // FLASHLOAN REACTOR — crimson/red
  buildClusterTowers( 120, -110, '#ffeeaa', '#ddcc77',  8, 16); // ARCHIVE QUARTER — warm gold

  return hotspots;
}

/* ══════════════════════════════════════════════════════════
   MAIN COMPONENT
   ══════════════════════════════════════════════════════════ */
export default function DriveWorld() {
  const mountRef = useRef(null);
  const minimapRef = useRef(null);
  const mobileKeysRef = useRef({ w: false, a: false, s: false, d: false });
  const [started, setStarted] = useState(false);
  const [infoCard, setInfoCard] = useState(null);
  const [speedPct, setSpeedPct] = useState(0);
  const [nearLabel, setNearLabel] = useState('');
  const [enterZone, setEnterZone] = useState(null); // { title, emoji, link, color } when near navigable zone
  const enterZoneRef = useRef(null); // keep ref in sync for use inside RAF

  // ── Live chain data ──
  const chainData = useChainData();
  const wallet    = useWallet();
  const chainDataRef    = useRef(chainData);       // mutable ref readable in RAF without re-render
  const blockPulseRef   = useRef({ time: -999, blockNum: 0 }); // set on each new block
  const genesisThreeRef = useRef({});              // { ptLight, tip } — populated in scene setup
  const valPylonArr     = useRef([]);              // validator pylon meshes — populated in scene setup

  // Keep chainDataRef in sync
  useEffect(() => { chainDataRef.current = chainData; }, [chainData]);

  // Trigger Genesis pulse on each new block
  useEffect(() => {
    if (chainData.newBlockEvent > 0) {
      blockPulseRef.current = { time: performance.now(), blockNum: chainData.blockNumber };
    }
  }, [chainData.newBlockEvent]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (!started) return;
    const container = mountRef.current;
    if (!container) return;

    let animId;
    // Reset per-scene refs so HMR / strict-mode double-invoke doesn't leave stale refs
    genesisThreeRef.current = {};
    valPylonArr.current = [];
    const W = container.clientWidth;
    const H = container.clientHeight;

    // ── Renderer ──
    const renderer = new THREE.WebGLRenderer({ antialias: true });
    renderer.setSize(W, H);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 0.42;
    container.appendChild(renderer.domElement);

    // ── Scene ──
    const scene = new THREE.Scene();

    // Procedural sky — dramatic alien sunset replacing solid background
    const skydome = new Sky();
    skydome.scale.setScalar(900000);
    scene.add(skydome);
    const skyUni = skydome.material.uniforms;
    skyUni['turbidity'].value        = 9;
    skyUni['rayleigh'].value         = 2.0;
    skyUni['mieCoefficient'].value   = 0.004;
    skyUni['mieDirectionalG'].value  = 0.9;
    const sunDir = new THREE.Vector3();
    const _phi   = THREE.MathUtils.degToRad(88);   // sun near horizon
    const _theta = THREE.MathUtils.degToRad(215);  // southwest direction
    sunDir.setFromSphericalCoords(1, _phi, _theta);
    skyUni['sunPosition'].value.copy(sunDir);
    // Warm amber haze at horizon
    scene.fog = new THREE.FogExp2('#a05520', 0.0025);

    // ── Camera ──
    const camera = new THREE.PerspectiveCamera(65, W / H, 0.1, 2000);
    camera.position.set(0, 7, 14);
    camera.lookAt(0, 0, 0);

    // ── Lights ──
    scene.add(new THREE.AmbientLight('#3344aa', 1.8)); // boosted so GLB buildings are visible
    // Deep-space cold fill light from above
    const fillLight = new THREE.DirectionalLight('#6688cc', 0.4);
    fillLight.position.set(0, 80, 0);
    scene.add(fillLight);

    const sun = new THREE.DirectionalLight('#ffb060', 1.6);
    // Position to match sky sunDir
    sun.position.copy(sunDir).multiplyScalar(400);
    sun.castShadow = true;
    sun.shadow.mapSize.set(2048, 2048);
    sun.shadow.camera.left = -280;
    sun.shadow.camera.right = 280;
    sun.shadow.camera.top = 280;
    sun.shadow.camera.bottom = -280;
    sun.shadow.camera.near = 1;
    sun.shadow.camera.far = 900;
    sun.shadow.bias = -0.0002;
    scene.add(sun);

    // Rim light from below (blue fill)
    const rimLight = new THREE.DirectionalLight('#0033ff', 0.25);
    rimLight.position.set(-20, -5, -20);
    scene.add(rimLight);

    // ── Ground plane ──
    const geoGround = new THREE.PlaneGeometry(520, 520, 70, 70);
    // Slight vertex jitter
    const gpos = geoGround.attributes.position;
    for (let i = 0; i < gpos.count; i++) {
      if (Math.abs(gpos.getX(i)) > 3 || Math.abs(gpos.getZ(i)) > 3) {
        gpos.setY(i, (Math.random() - 0.5) * 0.4);
      }
    }
    geoGround.computeVertexNormals();
    const _gTex = makeGroundTex();
    _gTex.repeat.set(48, 48);
    const ground = new THREE.Mesh(
      geoGround,
      new THREE.MeshLambertMaterial({ map: _gTex, color: '#334466' })
    );
    ground.rotation.x = -Math.PI / 2;
    ground.receiveShadow = true;
    scene.add(ground);

    // Sparse glowing grid — space station deck
    const grid = new THREE.GridHelper(520, 80, '#001a30', '#000d1a');
    grid.position.y = 0.03;
    scene.add(grid);
    // Bright accent lines at origin cross
    const axisH = new THREE.Mesh(new THREE.BoxGeometry(520, 0.02, 0.08),
      new THREE.MeshLambertMaterial({ color: '#00aaff', emissive: '#00aaff', emissiveIntensity: 0.6, transparent: true, opacity: 0.5 }));
    axisH.position.y = 0.04;
    scene.add(axisH);
    const axisV = axisH.clone();
    axisV.rotation.y = Math.PI / 2;
    scene.add(axisV);

    // ── Stars — dense interstellar field ──
    const starGeo = new THREE.BufferGeometry();
    const starCount = 8000;
    const starPos = new Float32Array(starCount * 3);
    const starCol = new Float32Array(starCount * 3);
    for (let i = 0; i < starCount; i++) {
      const theta = Math.random() * Math.PI * 2;
      const phi = Math.acos(Math.random() * 2 - 1);
      const r = 120 + Math.random() * 160;
      starPos[i*3]   = r * Math.sin(phi) * Math.cos(theta);
      starPos[i*3+1] = Math.abs(r * Math.cos(phi)) * 0.6 + 12;
      starPos[i*3+2] = r * Math.sin(phi) * Math.sin(theta);
      const t = Math.random();
      if (t < 0.55)      { starCol[i*3]=1;    starCol[i*3+1]=1;    starCol[i*3+2]=1;    }
      else if (t < 0.72) { starCol[i*3]=0.4;  starCol[i*3+1]=0.7;  starCol[i*3+2]=1;    }  // blue
      else if (t < 0.85) { starCol[i*3]=0.7;  starCol[i*3+1]=0.3;  starCol[i*3+2]=1;    }  // purple
      else               { starCol[i*3]=1;    starCol[i*3+1]=0.85; starCol[i*3+2]=0.4;  }  // warm yellow
    }
    starGeo.setAttribute('position', new THREE.BufferAttribute(starPos, 3));
    starGeo.setAttribute('color', new THREE.BufferAttribute(starCol, 3));
    const starMat = new THREE.PointsMaterial({ size: 0.5, vertexColors: true, transparent: true, opacity: 0.95, sizeAttenuation: true });
    const starField = scene.add(new THREE.Points(starGeo, starMat));

    // ── Distant galaxy smear (large dim particles in a flat disc) ──
    const galGeo = new THREE.BufferGeometry();
    const galPos = new Float32Array(2400 * 3);
    const galCol = new Float32Array(2400 * 3);
    for (let i = 0; i < 2400; i++) {
      const a = Math.random() * Math.PI * 2;
      const d = 140 + Math.random() * 100;
      galPos[i*3]   = Math.cos(a) * d;
      galPos[i*3+1] = 60 + (Math.random() - 0.5) * 30;
      galPos[i*3+2] = Math.sin(a) * d * 0.3; // squashed
      galCol[i*3]=0.55; galCol[i*3+1]=0.35; galCol[i*3+2]=0.9;
    }
    galGeo.setAttribute('position', new THREE.BufferAttribute(galPos, 3));
    galGeo.setAttribute('color', new THREE.BufferAttribute(galCol, 3));
    scene.add(new THREE.Points(galGeo, new THREE.PointsMaterial({ size: 0.28, vertexColors: true, transparent: true, opacity: 0.5, sizeAttenuation: true })));

    // ── Nebula atmosphere sphere ──
    const nebulaGeo = new THREE.SphereGeometry(230, 20, 20);
    const nebulaCanvas = document.createElement('canvas');
    nebulaCanvas.width = 512; nebulaCanvas.height = 512;
    const nc = nebulaCanvas.getContext('2d');
    // Deep space gradient
    const ng = nc.createRadialGradient(256, 256, 20, 256, 256, 256);
    ng.addColorStop(0,    'rgba(2,1,10,0)');
    ng.addColorStop(0.40, 'rgba(4,2,22,0.3)');
    ng.addColorStop(0.65, 'rgba(18,4,40,0.65)');
    ng.addColorStop(0.82, 'rgba(30,8,55,0.82)');
    ng.addColorStop(1,    'rgba(2,1,8,1)');
    nc.fillStyle = ng;
    nc.fillRect(0, 0, 512, 512);
    // Add nebula colour blobs
    const blobs = [
      ['rgba(60,0,120,0.18)', 380, 180, 90],
      ['rgba(0,60,140,0.14)', 120, 350, 70],
      ['rgba(100,10,80,0.12)', 300, 420, 60],
    ];
    blobs.forEach(([c, bx, by, br]) => {
      const bg = nc.createRadialGradient(bx, by, 0, bx, by, br);
      bg.addColorStop(0, c); bg.addColorStop(1, 'rgba(0,0,0,0)');
      nc.fillStyle = bg; nc.fillRect(0, 0, 512, 512);
    });
    const nebulaTex = new THREE.CanvasTexture(nebulaCanvas);
    const nebulaMat = new THREE.MeshBasicMaterial({ map: nebulaTex, side: THREE.BackSide, transparent: true, opacity: 0.9, depthWrite: false });
    scene.add(new THREE.Mesh(nebulaGeo, nebulaMat));

    // Boundary walls (invisible but visual cue: faint border emitters)
    for (const [x, z, rY] of [[0, 190, 0],[0,-190,Math.PI],[190,0,Math.PI/2],[-190,0,-Math.PI/2]]) {
      const wall = new THREE.Mesh(
        new THREE.BoxGeometry(380, 4, 0.5),
        new THREE.MeshLambertMaterial({ color: '#00d4ff', emissive: '#00d4ff', emissiveIntensity: 0.12, transparent: true, opacity: 0.3 })
      );
      wall.position.set(x, 2, z);
      wall.rotation.y = rY;
      scene.add(wall);
    }

    // ── Build UFO shell (lights/glow/effects — visible immediately) ──
    const { group: carGroup, pods, outerRing, engineLight, beam, disc, dome, domeRing } = buildUFO(scene);
    carGroup.position.set(0, 2.8, 0);

    // ── Async: load the real Vulcan D'Kyr OBJ model and swap it into carGroup ──
    const mtlLoader = new MTLLoader();
    mtlLoader.setPath('/models/vulcan/');
    mtlLoader.load('VulcanDKyrClass.mtl', (materials) => {
      materials.preload();
      const objLoader = new OBJLoader();
      objLoader.setMaterials(materials);
      objLoader.setPath('/models/vulcan/');
      objLoader.load('VulcanDKyrClass.obj', (obj) => {
        // OBJ bounding: X ±1.5 (width=3), Y ±1.2, Z -4.4→+2.0 (center=-1.2)
        // Scale so ring (3 units) → ~5 scene units, matching the UFO placeholder
        obj.scale.setScalar(1.6);
        // Center the model: OBJ Z-center is -1.2 → shift +1.2 so pivot is at origin
        obj.position.set(0, 0, 1.2);
        // The D'Kyr flies nose-forward — nose is at Z=-4.4 (OBJ), rotate PI to face +Z
        obj.rotation.set(0, Math.PI, 0);
        obj.traverse((child) => {
          if (child.isMesh) {
            child.castShadow = true;
            child.receiveShadow = false;
          }
        });
        // Hide the placeholder hull/dome — keep the glow ring, pods, lights, beam
        disc.visible = false;
        dome.visible = false;
        domeRing.visible = false;
        carGroup.add(obj);
      }, undefined, (err) => console.warn('OBJ load error', err));
    }, undefined, (err) => console.warn('MTL load error', err));

    // ── Build world ──
    const hotspots = buildWorld(scene, { genesisThreeRef, valPylonArr });

    // ── World Props: RefractionJet (NPC ship, full MTL+textures) ──
    const refjetMtl = new MTLLoader();
    refjetMtl.setPath('/models/refjet/');
    refjetMtl.load('RefractionJet_by_Dommk.mtl', (mats) => {
      mats.preload();
      const refjetObj = new OBJLoader();
      refjetObj.setMaterials(mats);
      refjetObj.setPath('/models/refjet/');
      refjetObj.load('RefractionJet_by_Dommk.obj', (obj) => {
        // OBJ is ~6 wide — scale to ~9 scene units
        obj.scale.setScalar(1.5);
        obj.position.set(60, 10, -60);  // hovering between Genesis and Whitepaper Library
        obj.rotation.set(0.15, -0.8, 0.05);
        obj.traverse(c => { if (c.isMesh) c.castShadow = true; });
        scene.add(obj);
        // gentle bob + yaw in animate via userData
        obj.userData.propFloat = { t: 0, baseY: 8 };
        propObjects.push(obj);
      }, undefined, (e) => console.warn('refjet obj', e));
    }, undefined, (e) => console.warn('refjet mtl', e));

    // ── World Props: Portal gate (emissive cyan/purple, no MTL) ──
    const portalObjLoader = new OBJLoader();
    portalObjLoader.setPath('/models/portal/');
    portalObjLoader.load('portal.obj', (obj) => {
      const mat = new THREE.MeshStandardMaterial({
        color: '#2a0060', emissive: '#aa00ff', emissiveIntensity: 1.8,
        metalness: 0.9, roughness: 0.2,
      });
      // portal OBJ is ~22 wide — scale to ~14 scene units
      obj.scale.setScalar(0.65);
      obj.position.set(-70, 0, -70);    // NW quadrant between Genesis and Grant Tower
      obj.rotation.set(0, Math.PI * 0.25, 0);
      obj.traverse(c => { if (c.isMesh) { c.material = mat; c.castShadow = true; } });
      scene.add(obj);
      // pulsing glow light inside
      const portalGlow = new THREE.PointLight('#aa00ff', 4, 22);
      portalGlow.position.set(-70, 5, -70);
      scene.add(portalGlow);
      propObjects.push(obj);
      obj.userData.portalGlow = portalGlow;
    }, undefined, (e) => console.warn('portal obj', e));

    // ── World Props: Cave structure (dark rock, no MTL) ──
    const caveObjLoader = new OBJLoader();
    caveObjLoader.setPath('/models/cave/');
    caveObjLoader.load('Cave.obj', (obj) => {
      const rockMat = new THREE.MeshStandardMaterial({
        color: '#1a1228', emissive: '#330066', emissiveIntensity: 0.3,
        metalness: 0.1, roughness: 0.95,
      });
      // cave is ~7 wide, 30 deep — scale to about 18 wide
      obj.scale.setScalar(2.5);
      obj.position.set(85, 0, -30);     // between Validator Forest and Whitepaper Library
      obj.rotation.set(0, Math.PI * -0.15, 0);
      obj.traverse(c => { if (c.isMesh) { c.material = rockMat; c.castShadow = true; c.receiveShadow = true; } });
      scene.add(obj);
    }, undefined, (e) => console.warn('cave obj', e));

    // ── World Props: Spaceship hulk (metallic NPC wreck, no MTL) ──
    const hullObjLoader = new OBJLoader();
    hullObjLoader.setPath('/models/spaceship/');
    hullObjLoader.load('Spaceship.obj', (obj) => {
      const hullMat = new THREE.MeshStandardMaterial({
        color: '#334455', emissive: '#002244', emissiveIntensity: 0.5,
        metalness: 0.85, roughness: 0.3,
      });
      // mesh is ~1 unit but far from origin — scale up & re-center
      obj.scale.setScalar(18);
      // mesh center is ~(-29, 0.1, 9.6) — offset to bring to scene origin
      obj.position.set(-80, 3, 60);     // SW quadrant near Compute/Treasury side
      obj.traverse(c => { if (c.isMesh) { c.material = hullMat; c.castShadow = true; } });
      scene.add(obj);
    }, undefined, (e) => console.warn('spaceship obj', e));

    // prop objects array for animate loop
    const propObjects = [];

    // ── GLB Props: Sci-fi buildings pack (low-poly city fill) ──
    const gltfLoader = new GLTFLoader();
    gltfLoader.load('/models/scifi-buildings/buildings.glb', (gltf) => {
      const obj = gltf.scene;
      obj.scale.setScalar(2);
      obj.position.set(42, 0, 78);
      obj.rotation.y = Math.PI * 0.25;
      obj.traverse(c => {
        if (c.isMesh) {
          c.castShadow = true;
          c.receiveShadow = true;
          if (c.material) {
            const m = Array.isArray(c.material) ? c.material : [c.material];
            m.forEach(mat => {
              // Force standard material response
              if (mat.color) mat.color.multiplyScalar(1.8);
              if (mat.emissive) {
                mat.emissive.set(0x224466);
                mat.emissiveIntensity = 0.7;
              }
            });
          }
        }
      });
      scene.add(obj);
      // accent lights for building #1 (right-ahead cluster)
      const bld1a = new THREE.PointLight(0x4488ff, 4.0, 90);
      bld1a.position.set(42, 8, 78);
      scene.add(bld1a);
      const bld1b = new THREE.PointLight(0x00ffcc, 2.5, 60);
      bld1b.position.set(42, 3, 78);
      scene.add(bld1b);
      // add second instance as a mirrored city block on the other side
      const gltfLoader2 = new GLTFLoader();
      gltfLoader2.load('/models/scifi-buildings/buildings.glb', (gltf2) => {
        const obj2 = gltf2.scene;
        obj2.scale.setScalar(2);
        obj2.position.set(-48, 0, 92);
        obj2.rotation.y = Math.PI * 0.85;
        obj2.traverse(c => {
          if (c.isMesh) { c.castShadow = true; c.receiveShadow = true; }
        });
        scene.add(obj2);
        // accent lights for building #2 (left-ahead cluster)
        const bld2a = new THREE.PointLight(0xff6600, 3.5, 90);
        bld2a.position.set(-48, 8, 92);
        scene.add(bld2a);
        const bld2b = new THREE.PointLight(0xffcc00, 2.0, 60);
        bld2b.position.set(-48, 3, 92);
        scene.add(bld2b);
      }, undefined, (e) => console.warn('scifi-buildings 2nd instance', e));
    }, undefined, (e) => console.warn('scifi-buildings GLB', e));

    // ── GLB Props: Sci-fi modular terminal (near Exchange/DEX hotspot) ──
    gltfLoader.load('/models/terminal/terminal.glb', (gltf) => {
      const obj = gltf.scene;
      obj.scale.setScalar(1.2);
      obj.position.set(38, 0, 48);
      obj.rotation.y = Math.PI * -0.15;
      obj.traverse(c => {
        if (c.isMesh) {
          c.castShadow = true;
          c.receiveShadow = true;
          if (c.material && c.material.emissive) {
            c.material.emissiveIntensity = Math.max(c.material.emissiveIntensity || 0, 0.6);
          }
        }
      });
      scene.add(obj);
      // cyan point light to make the terminal glow
      const termLight = new THREE.PointLight(0x00eeff, 2.5, 40);
      termLight.position.set(38, 3, 48);
      scene.add(termLight);
      obj.userData.propFloat = { t: Math.random() * Math.PI * 2, baseY: 0 };
      propObjects.push(obj);
    }, undefined, (e) => console.warn('terminal GLB', e));

    // ── GLB Props: Sci-fi portal gate (SE corner landmark) ──
    gltfLoader.load('/models/portal-glb/portal.glb', (gltf) => {
      const obj = gltf.scene;
      obj.scale.setScalar(1.0);
      obj.position.set(65, 0, 62);
      obj.rotation.y = Math.PI * 0.5;
      obj.traverse(c => {
        if (c.isMesh) {
          c.castShadow = true;
          c.receiveShadow = true;
          if (c.material && c.material.emissive) {
            c.material.emissiveIntensity = Math.max(c.material.emissiveIntensity || 0, 0.8);
          }
        }
      });
      scene.add(obj);
      // magenta glow light inside the portal ring
      const portalGlow = new THREE.PointLight(0xff00cc, 2.8, 55);
      portalGlow.position.set(65, 3, 62);
      scene.add(portalGlow);
    }, undefined, (e) => console.warn('portal-glb', e));

    // ── Car physics state ──
    const car = {
      x: 0, z: 5,
      speed: 0, steer: 0, rotY: 0,
      bounce: 0, tilt: 0, pitch: 0,
    };
    const MAX_SPEED = 120;
    const ACCEL = 90;
    const BRAKE = 45;
    const FRICTION = 0.88;
    const MAX_STEER = 0.92;
    const STEER_LERP = 9;
    const BOUNDS = 185;

    // Camera lag state
    const camRotY = { val: 0 };

    // Keys
    const keys = { w: false, a: false, s: false, d: false, up: false, left: false, down: false, right: false };
    const onKey = (e, down) => {
      const k = e.key.toLowerCase();
      if (k === 'w' || k === 'arrowup')    keys.w    = down;
      if (k === 'a' || k === 'arrowleft')  keys.a    = down;
      if (k === 's' || k === 'arrowdown')  keys.s    = down;
      if (k === 'd' || k === 'arrowright') keys.d    = down;
    };
    // Enter / Space → navigate to active hotspot
    const onEnterKey = (e) => {
      if ((e.key === 'Enter' || e.key === ' ') && enterZoneRef.current?.link) {
        e.preventDefault();
        window.location.href = enterZoneRef.current.link;
      }
    };
    window.addEventListener('keydown', e => onKey(e, true));
    window.addEventListener('keyup',   e => onKey(e, false));
    window.addEventListener('keydown', onEnterKey);

    // ── Clock ──
    const clock = new THREE.Clock();
    let activeHotspot = null;

    // ── Animate ──
    function animate() {
      animId = requestAnimationFrame(animate);
      const dt = Math.min(clock.getDelta(), 0.05);
      const elapsed = clock.elapsedTime;

      // ── Car physics ──
      const mk = mobileKeysRef.current;
      const fwd = (keys.w || mk.w) ? 1 : ((keys.s || mk.s) ? -0.55 : 0);
      car.speed += fwd * (fwd > 0 ? ACCEL : BRAKE) * dt;
      car.speed *= Math.pow(FRICTION, dt * 60);
      car.speed = Math.max(-MAX_SPEED * 0.55, Math.min(MAX_SPEED, car.speed));

      const steerTarget = ((keys.a || mk.a) ? 1 : ((keys.d || mk.d) ? -1 : 0)) * MAX_STEER;
      car.steer += (steerTarget - car.steer) * STEER_LERP * dt;

      if (Math.abs(car.speed) > 0.05) {
        car.rotY += car.steer * (car.speed / MAX_SPEED) * dt * 60 * 0.068;
      }

      const nx = car.x + Math.sin(car.rotY) * car.speed * dt;
      const nz = car.z + Math.cos(car.rotY) * car.speed * dt;
      car.x = Math.max(-BOUNDS, Math.min(BOUNDS, nx));
      car.z = Math.max(-BOUNDS, Math.min(BOUNDS, nz));

      // ── Update UFO mesh ──
      // Cruise altitude: skim low over the city — buildings tower above
      const hoverY = 8 + Math.sin(elapsed * 1.5) * 0.5 + Math.abs(car.speed) * 0.04;
      car.bounce = hoverY;
      carGroup.position.set(car.x, hoverY, car.z);
      carGroup.rotation.y = car.rotY;
      // Bank into turns (roll) + pitch on acceleration
      // More dramatic banking and nose-pitch for aerial feel
      car.tilt += (-car.steer * (car.speed / MAX_SPEED) * 0.72 - car.tilt) * 6 * dt;
      const pitchTarget = -(car.speed / MAX_SPEED) * 0.30;
      car.pitch = (car.pitch || 0) + (pitchTarget - (car.pitch || 0)) * 5 * dt;
      carGroup.rotation.z = car.tilt;
      carGroup.rotation.x = car.pitch;

      // Outer ring spin (faster when moving)
      outerRing.rotation.z += (0.012 + Math.abs(car.speed) / MAX_SPEED * 0.08) * dt * 60;

      // Pod lights pulse in sequence
      pods.forEach((pod, i) => {
        const phase = (elapsed * 3 + i * (Math.PI * 2 / 8)) % (Math.PI * 2);
        pod.material.emissiveIntensity = 1.5 + Math.sin(phase) * 1.2;
      });

      // Engine glow throbs with speed
      engineLight.intensity = 2.5 + (car.speed / MAX_SPEED) * 4.5 + Math.sin(elapsed * 8) * 0.4;

      // Tractor beam opacity pulses
      beam.material.opacity = 0.06 + Math.sin(elapsed * 1.8) * 0.05;

      // ── World prop animations ──
      propObjects.forEach(obj => {
        // RefractionJet: slow bob + lazy yaw
        if (obj.userData.propFloat) {
          obj.userData.propFloat.t += dt;
          const t = obj.userData.propFloat.t;
          obj.position.y = obj.userData.propFloat.baseY + Math.sin(t * 0.7) * 1.2;
          obj.rotation.y += dt * 0.18;
        }
        // Portal: pulse its glow light
        if (obj.userData.portalGlow) {
          obj.userData.portalGlow.intensity = 3 + Math.sin(elapsed * 2.4) * 2;
        }
      });

      // ── Camera follow — aerial chase cam ──
      camRotY.val += (car.rotY - camRotY.val) * 5 * dt;
      const camDist = 16 + Math.abs(car.speed) * 0.22;
      // Camera 6 units above ship — tight chase cam
      const camH = hoverY + 6 + Math.abs(car.speed) * 0.06;
      const camTX = car.x - Math.sin(camRotY.val) * camDist;
      const camTZ = car.z - Math.cos(camRotY.val) * camDist;
      camera.position.x += (camTX - camera.position.x) * 8 * dt;
      camera.position.y += (camH - camera.position.y) * 6 * dt;
      camera.position.z += (camTZ - camera.position.z) * 8 * dt;
      // Look at ship-level ahead — see ground rushing past below
      camera.lookAt(
        car.x + Math.sin(car.rotY) * 8,
        hoverY - 2,
        car.z + Math.cos(car.rotY) * 8
      );

      // FOV rush — wide aerial view, extra spread at speed
      camera.fov = 72 + Math.abs(car.speed / MAX_SPEED) * 20;
      camera.updateProjectionMatrix();

      // ── Animated world objects ──
      scene.children.forEach(obj => {
        if (obj.userData.spinY)    obj.rotation.y += obj.userData.spinY;
        if (obj.userData.ring) {
          obj.rotation.x += obj.userData.rotSpeedX;
          obj.rotation.y += obj.userData.rotSpeedY;
        }
        if (obj.userData.valTower) {
          const ch = obj.children;
          if (ch[1]) {
            const pulse = (Math.sin(elapsed * 3 + obj.userData.pulsePhase) + 1) / 2;
            ch[1].material.emissiveIntensity = 0.8 + pulse * 1.2;
          }
        }
      });

      // ── CHAIN: Genesis Block pulse on new block ──
      {
        const ageMs = performance.now() - blockPulseRef.current.time;
        const { ptLight, tip } = genesisThreeRef.current;
        if (ageMs < 1400 && ptLight) {
          const t   = ageMs / 1400;                  // 0 → 1
          const amp = Math.sin(t * Math.PI);          // bell curve
          ptLight.intensity = 2.0 + amp * 7.0;
          if (tip) tip.material.emissiveIntensity = 2.0 + amp * 3.5;
        } else if (ptLight) {
          ptLight.intensity = 2.0;
          if (tip) tip.material.emissiveIntensity = 2.0;
        }
      }

      // ── CHAIN: Validator Forest live telemetry ──
      {
        const validators = chainDataRef.current.validators;
        valPylonArr.current.forEach((pylon, i) => {
          const val = validators?.[i];
          if (!val || !pylon) return;
          const topMesh = pylon.children?.[1];
          if (topMesh?.material) {
            const uptime = (val.uptime ?? 100) / 100;  // 0..1
            let ei = 0.4 + uptime * 1.6;
            // Flicker for validators with many missed blocks
            if (val.missed > 5) ei *= 0.7 + Math.sin(elapsed * 22 + i * 1.3) * 0.3;
            topMesh.material.emissiveIntensity = ei;
          }
        });
      }

      // ── Hotspot detection ──
      let nearest = null;
      let nearestDist = Infinity;
      hotspots.forEach(h => {
        const d = Math.hypot(car.x - h.pos.x, car.z - h.pos.z);
        if (d < h.radius && d < nearestDist) {
          nearest = h;
          nearestDist = d;
        }
      });

      if (nearest !== activeHotspot) {
        activeHotspot = nearest;
        if (nearest) {
          // Inject live chain stats into card body
          const cd = chainDataRef.current;
          const liveTag = cd.online ? '🟢 LIVE' : '🟡 MOCK';
          if (nearest.title === 'Genesis Block') {
            nearest.body = `Block #${cd.blockNumber.toLocaleString()} · ${cd.tps.toLocaleString()} TPS · ${liveTag}. X3 Chain launched with 1,847 genesis validators across 6 continents — the first block sealed a new Layer-1 throughput record.`;
          } else if (nearest.title === 'Validator Forest') {
            nearest.body = `${cd.validatorCount} active validators · ${liveTag}. BFT + PoS hybrid consensus. Each node stakes X3 tokens and earns 12–18% APR. No slashable conditions in normal operation.`;
          }
          setInfoCard({ ...nearest });
          setNearLabel(nearest.title);
          // Show ENTER prompt only for zones with a real navigation link
          const zone = nearest.link ? nearest : null;
          enterZoneRef.current = zone;
          setEnterZone(zone);
        } else {
          setInfoCard(null);
          setNearLabel('');
          enterZoneRef.current = null;
          setEnterZone(null);
        }
      }

      // ── Speed to state (throttled) ──
      setSpeedPct(Math.round(Math.abs(car.speed) / MAX_SPEED * 100));

      // ── Minimap draw ──
      if (minimapRef.current) {
        const mc  = minimapRef.current;
        const ctx = mc.getContext('2d');
        const MW = mc.width, MH = mc.height;
        const S  = MW / (BOUNDS * 2.1); // scale: pixels per world unit
        const ox = MW / 2, oz = MH / 2;
        ctx.clearRect(0, 0, MW, MH);
        // Background
        ctx.fillStyle = 'rgba(2,5,16,0.88)';
        ctx.fillRect(0, 0, MW, MH);
        // Faint grid lines
        ctx.strokeStyle = 'rgba(0,55,110,0.35)';
        ctx.lineWidth = 0.5;
        for (let g = -160; g <= 160; g += 40) {
          const px = ox + g * S, pz = oz + g * S;
          ctx.beginPath(); ctx.moveTo(px, 0); ctx.lineTo(px, MH); ctx.stroke();
          ctx.beginPath(); ctx.moveTo(0, pz); ctx.lineTo(MW, pz); ctx.stroke();
        }
        // Hotspot dots + emoji labels
        hotspots.forEach(h => {
          const px = ox + h.pos.x * S;
          const pz = oz + h.pos.z * S;
          ctx.shadowColor = h.color;
          ctx.shadowBlur = 8;
          ctx.beginPath();
          ctx.arc(px, pz, 5, 0, Math.PI * 2);
          ctx.fillStyle = h.color + 'cc';
          ctx.fill();
          ctx.strokeStyle = h.color;
          ctx.lineWidth = 1;
          ctx.stroke();
          ctx.shadowBlur = 0;
          ctx.font = '9px sans-serif';
          ctx.textAlign = 'center';
          ctx.fillStyle = 'rgba(255,255,255,0.75)';
          ctx.fillText(h.emoji, px, pz - 8);
        });
        // Player triangle (points in direction of travel)
        const ppx = ox + car.x * S;
        const ppz = oz + car.z * S;
        ctx.save();
        ctx.translate(ppx, ppz);
        ctx.rotate(-car.rotY);
        ctx.beginPath();
        ctx.moveTo(0, -8);
        ctx.lineTo(-5, 6);
        ctx.lineTo(5, 6);
        ctx.closePath();
        ctx.fillStyle = '#00d4ff';
        ctx.shadowColor = '#00d4ff';
        ctx.shadowBlur = 10;
        ctx.fill();
        ctx.shadowBlur = 0;
        ctx.restore();
      }

      renderer.render(scene, camera);
    }

    animate();

    // ── Resize ──
    const onResize = () => {
      const W2 = container.clientWidth;
      const H2 = container.clientHeight;
      camera.aspect = W2 / H2;
      camera.updateProjectionMatrix();
      renderer.setSize(W2, H2);
    };
    window.addEventListener('resize', onResize);

    return () => {
      cancelAnimationFrame(animId);
      window.removeEventListener('keydown', onEnterKey);
      window.removeEventListener('resize', onResize);
      renderer.dispose();
      if (container.contains(renderer.domElement)) {
        container.removeChild(renderer.domElement);
      }
    };
  }, [started]);

  return (
    <div style={{ width: '100vw', height: '100vh', position: 'relative', overflow: 'hidden', background: '#06060f' }}>

      {/* 3D Canvas mount */}
      <div ref={mountRef} style={{ width: '100%', height: '100%' }} />

      {/* ── Top Navigation Bar ── */}
      {started && (
        <nav style={{
          position: 'absolute', top: 0, left: 0, right: 0, height: 44,
          display: 'flex', alignItems: 'center', justifyContent: 'space-between',
          padding: '0 12px',
          background: 'rgba(3,3,12,0.80)',
          backdropFilter: 'blur(14px)',
          borderBottom: '1px solid rgba(0,212,255,0.10)',
          zIndex: 9, fontFamily: 'monospace', boxSizing: 'border-box', gap: 8,
        }}>
          {/* Logo */}
          <div style={{ color: '#00d4ff', fontWeight: 800, fontSize: 14, letterSpacing: 2, whiteSpace: 'nowrap', flexShrink: 0 }}>
            ⛓ X3
          </div>
          {/* Nav links — scrollable on mobile */}
          <div style={{ display: 'flex', gap: 2, overflowX: 'auto', scrollbarWidth: 'none', msOverflowStyle: 'none' }}>
            {[
              { label: 'DEX',        href: '/x3star-landing.html?section=dex',   color: '#ff7700' },
              { label: 'VALIDATORS', href: '/x3star-validator-presale.html',      color: '#4488ff' },
              { label: 'TREASURY',   href: '/x3star-governance.html',             color: '#ffd700' },
              { label: 'EXPLORER',   href: '/?section=explorer',                  color: '#00d4ff' },
              { label: 'COMPUTE',    href: '/x3star-compute-marketplace.html',    color: '#aa44ff' },
              { label: 'GRANTS',     href: '/x3star-grant-hub.html',              color: '#00ff88' },
              { label: 'DOCS',       href: '/x3star-whitepaper.html',             color: '#ffee88' },
            ].map(({ label, href, color }) => (
              <a key={label} href={href}
                style={{
                  color, padding: '4px 8px', fontSize: 11, textDecoration: 'none',
                  borderRadius: 4, letterSpacing: 1, whiteSpace: 'nowrap',
                  border: '1px solid transparent', transition: 'all 0.15s', opacity: 0.8,
                }}
                onMouseEnter={e => { e.currentTarget.style.borderColor = color + '55'; e.currentTarget.style.background = color + '18'; e.currentTarget.style.opacity = '1'; }}
                onMouseLeave={e => { e.currentTarget.style.borderColor = 'transparent'; e.currentTarget.style.background = 'transparent'; e.currentTarget.style.opacity = '0.8'; }}
              >{label}</a>
            ))}
          </div>
          {/* Compact chain status */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 5, flexShrink: 0 }}>
            <span style={{
              width: 6, height: 6, borderRadius: '50%', display: 'inline-block',
              background: chainData.online ? '#00d4ff' : '#ffaa00',
              boxShadow: chainData.online ? '0 0 5px #00d4ff' : '0 0 5px #ffaa00',
            }} />
            <span style={{ color: '#445566', fontSize: 10, letterSpacing: 0.5 }}>
              #{chainData.blockNumber.toLocaleString()}
            </span>
          </div>
        </nav>
      )}

      {/* ── Chain HUD — top-right (below nav) ── */}
      {started && (
        <div style={{
          position: 'absolute', top: 52, right: 16,
          zIndex: 5, pointerEvents: 'none', userSelect: 'none',
          fontFamily: 'monospace',
          display: 'flex', flexDirection: 'column', alignItems: 'flex-end', gap: 6,
        }}>
          {/* Block ticker */}
          <div style={{
            background: 'rgba(0,8,22,0.82)', backdropFilter: 'blur(8px)',
            border: `1px solid ${chainData.online ? '#00d4ff44' : '#ffaa0044'}`,
            borderRadius: 8, padding: '5px 12px',
            display: 'flex', alignItems: 'center', gap: 8,
          }}>
            <span style={{
              width: 8, height: 8, borderRadius: '50%', flexShrink: 0,
              background: chainData.online ? '#00d4ff' : '#ffaa00',
              boxShadow: chainData.online ? '0 0 6px #00d4ff' : '0 0 6px #ffaa00',
              animation: chainData.online ? 'chainPulse 1s infinite' : 'none',
              display: 'inline-block',
            }} />
            <span style={{ color: chainData.online ? '#00d4ff' : '#ffaa00', fontSize: 12, letterSpacing: 0.5 }}>
              {chainData.online ? 'LIVE' : 'MOCK'}
            </span>
            <span style={{ color: '#445566', fontSize: 11 }}>|</span>
            <span style={{ color: '#aabbcc', fontSize: 12 }}>
              #{chainData.blockNumber.toLocaleString()}
            </span>
            <span style={{ color: '#445566', fontSize: 11 }}>|</span>
            <span style={{ color: '#aabbcc', fontSize: 12 }}>
              {chainData.tps.toLocaleString()} TPS
            </span>
          </div>

          {/* Wallet connect / account */}
          <WalletButton wallet={wallet} />
        </div>
      )}

      {/* ── World Map — bottom-left ── */}
      {started && (
        <div style={{
          position: 'absolute', bottom: 16, left: 16,
          zIndex: 4, pointerEvents: 'none', userSelect: 'none',
          display: 'flex', flexDirection: 'column', alignItems: 'flex-start',
        }}>
          <div style={{
            fontSize: 8, letterSpacing: 3, color: 'rgba(0,212,255,0.45)',
            fontFamily: 'monospace', marginBottom: 3, textTransform: 'uppercase',
          }}>X3 MAP</div>
          <canvas
            ref={minimapRef}
            width={200}
            height={120}
            style={{
              borderRadius: 6,
              border: '1px solid rgba(0,212,255,0.22)',
              boxShadow: '0 2px 20px rgba(0,0,0,0.7), 0 0 12px rgba(0,212,255,0.08)',
            }}
          />
        </div>
      )}

      {/* ── Start screen ── */}
      {!started && (
        <div style={{
          position: 'absolute', inset: 0,
          display: 'flex', flexDirection: 'column',
          alignItems: 'center', justifyContent: 'center',
          background: 'rgba(6,6,15,0.96)',
          zIndex: 10,
          fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
        }}>
          {/* Logo / Title */}
          <div style={{ textAlign: 'center', marginBottom: 40 }}>
            <div style={{
              fontSize: 80, marginBottom: 8,
              filter: 'drop-shadow(0 0 24px #00d4ff)',
            }}>⛓</div>
            <h1 style={{
              fontSize: 54, fontWeight: 900,
              letterSpacing: '-2px', margin: 0,
              background: 'linear-gradient(135deg, #00d4ff 0%, #a855f7 50%, #ff6600 100%)',
              WebkitBackgroundClip: 'text', WebkitTextFillColor: 'transparent',
              textShadow: 'none',
            }}>X3 CHAIN</h1>
            <p style={{ color: '#6688aa', fontSize: 16, marginTop: 8, letterSpacing: 2 }}>
              DRIVE THE BLOCKCHAIN
            </p>
          </div>

          {/* Tagline */}
          <p style={{ color: '#8899bb', fontSize: 18, maxWidth: 440, textAlign: 'center', lineHeight: 1.6, marginBottom: 40 }}>
            Explore the X3 ecosystem behind the wheel.<br />
            Drive to each landmark — then press <strong style={{ color: '#00d4ff' }}>ENTER</strong> or <strong style={{ color: '#00d4ff' }}>SPACE</strong> to navigate there.
          </p>

          {/* Controls cheatsheet */}
          <div style={{
            display: 'flex', gap: 24, marginBottom: 48,
            background: 'rgba(255,255,255,0.04)',
            border: '1px solid rgba(0,212,255,0.15)',
            borderRadius: 12, padding: '14px 28px',
          }}>
            {[
              { key: 'W / ↑', label: 'Accelerate' },
              { key: 'S / ↓', label: 'Reverse' },
              { key: 'A / ←', label: 'Steer Left' },
              { key: 'D / →', label: 'Steer Right' },
              { key: 'ENTER / SPACE', label: 'Navigate In' },
            ].map(({ key, label }) => (
              <div key={key} style={{ textAlign: 'center' }}>
                <div style={{
                  background: 'rgba(0,212,255,0.1)', border: '1px solid #00d4ff',
                  borderRadius: 6, padding: '4px 10px', marginBottom: 4,
                  color: '#00d4ff', fontSize: 13, fontWeight: 700, letterSpacing: 1,
                }}>{key}</div>
                <div style={{ color: '#556688', fontSize: 12 }}>{label}</div>
              </div>
            ))}
          </div>

          {/* CTA */}
          <button
            onClick={() => setStarted(true)}
            style={{
              background: 'linear-gradient(135deg, #00d4ff, #a855f7)',
              border: 'none', borderRadius: 50,
              padding: '16px 52px', fontSize: 18, fontWeight: 800,
              color: '#06060f', cursor: 'pointer',
              letterSpacing: 1,
              boxShadow: '0 0 30px rgba(0,212,255,0.4)',
              transition: 'transform 0.15s, box-shadow 0.15s',
            }}
            onMouseEnter={e => { e.target.style.transform = 'scale(1.05)'; e.target.style.boxShadow = '0 0 50px rgba(0,212,255,0.6)'; }}
            onMouseLeave={e => { e.target.style.transform = 'scale(1)'; e.target.style.boxShadow = '0 0 30px rgba(0,212,255,0.4)'; }}
          >
            START DRIVING
          </button>

          <p style={{ color: '#334455', fontSize: 12, marginTop: 20 }}>
            8 landmarks to explore · Drive up to any zone and press ENTER to visit
          </p>
        </div>
      )}

      {/* ── HUD (top-left, below nav) ── */}
      {started && (
        <div style={{
          position: 'absolute', top: 52, left: 20,
          fontFamily: "'Inter', monospace, sans-serif",
          userSelect: 'none', pointerEvents: 'none',
        }}>
          {/* Speed bar */}
          <div style={{ display: 'flex', alignItems: 'center', gap: 10, marginBottom: 8 }}>
            <span style={{ color: '#334466', fontSize: 11, letterSpacing: 2 }}>SPD</span>
            <div style={{ width: 90, height: 6, background: 'rgba(255,255,255,0.08)', borderRadius: 3, overflow: 'hidden' }}>
              <div style={{
                height: '100%', borderRadius: 3,
                width: `${speedPct}%`,
                background: speedPct > 75 ? '#ff4444' : speedPct > 40 ? '#ffaa00' : '#00d4ff',
                transition: 'width 0.1s, background 0.2s',
              }} />
            </div>
            <span style={{ color: '#556688', fontSize: 11, minWidth: 28, textAlign: 'right' }}>
              {speedPct}%
            </span>
          </div>
          {/* Landmark proximity */}
          {nearLabel && (
            <div style={{
              background: 'rgba(0,212,255,0.1)', border: '1px solid rgba(0,212,255,0.3)',
              borderRadius: 6, padding: '4px 10px', fontSize: 12, color: '#00d4ff',
              letterSpacing: 0.5, animation: 'fadein 0.3s',
            }}>
              📍 {nearLabel}
            </div>
          )}
        </div>
      )}

      {/* ── Controls hint (bottom-center) ── */}
      {started && !infoCard && (
        <div style={{
          position: 'absolute', bottom: 20, left: '50%', transform: 'translateX(-50%)',
          display: 'flex', gap: 12, pointerEvents: 'none', userSelect: 'none',
          fontFamily: "'Inter', sans-serif",
        }}>
          {['W', 'A', 'S', 'D'].map(k => (
            <div key={k} style={{
              width: 32, height: 32, borderRadius: 6,
              background: 'rgba(255,255,255,0.05)', border: '1px solid rgba(255,255,255,0.12)',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              color: '#556688', fontSize: 13, fontWeight: 700,
            }}>{k}</div>
          ))}
        </div>
      )}

      {/* ── ENTER ZONE prompt (bottom-center, appears when near a navigable hotspot) ── */}
      {enterZone && (
        <div style={{
          position: 'absolute', bottom: 80, left: '50%', transform: 'translateX(-50%)',
          display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 10,
          zIndex: 6,
          animation: 'slideUp 0.3s cubic-bezier(0.23, 1, 0.32, 1)',
          fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
        }}>
          {/* Zone name badge */}
          <div style={{
            display: 'flex', alignItems: 'center', gap: 8,
            background: `${enterZone.color}18`,
            border: `1px solid ${enterZone.color}66`,
            borderRadius: 50,
            padding: '6px 18px',
            color: enterZone.color,
            fontSize: 13, fontWeight: 700, letterSpacing: 1,
          }}>
            <span>{enterZone.emoji}</span>
            <span>{enterZone.title.toUpperCase()}</span>
          </div>

          {/* Big ENTER button */}
          <a
            href={enterZone.link}
            style={{
              display: 'flex', alignItems: 'center', gap: 10,
              background: `linear-gradient(135deg, ${enterZone.color}dd, ${enterZone.color}88)`,
              border: `2px solid ${enterZone.color}`,
              borderRadius: 50,
              padding: '12px 36px',
              color: '#06060f',
              fontSize: 16, fontWeight: 900,
              textDecoration: 'none',
              letterSpacing: 2,
              boxShadow: `0 0 30px ${enterZone.color}55, 0 4px 20px rgba(0,0,0,0.5)`,
              transition: 'transform 0.12s, box-shadow 0.12s',
              cursor: 'pointer',
            }}
            onMouseEnter={e => { e.currentTarget.style.transform = 'scale(1.06)'; e.currentTarget.style.boxShadow = `0 0 50px ${enterZone.color}88, 0 4px 24px rgba(0,0,0,0.6)`; }}
            onMouseLeave={e => { e.currentTarget.style.transform = 'scale(1)'; e.currentTarget.style.boxShadow = `0 0 30px ${enterZone.color}55, 0 4px 20px rgba(0,0,0,0.5)`; }}
          >
            <span>ENTER</span>
            <span style={{ fontSize: 18 }}>→</span>
          </a>

          {/* Keyboard hint */}
          <div style={{
            display: 'flex', alignItems: 'center', gap: 6,
            color: '#445566', fontSize: 11, letterSpacing: 1,
          }}>
            <span style={{
              background: 'rgba(255,255,255,0.07)', border: '1px solid rgba(255,255,255,0.15)',
              borderRadius: 4, padding: '2px 8px', fontSize: 11, fontWeight: 700, color: '#667799',
            }}>ENTER</span>
            <span>or</span>
            <span style={{
              background: 'rgba(255,255,255,0.07)', border: '1px solid rgba(255,255,255,0.15)',
              borderRadius: 4, padding: '2px 8px', fontSize: 11, fontWeight: 700, color: '#667799',
            }}>SPACE</span>
            <span>to navigate</span>
          </div>
        </div>
      )}

      {/* ── Info Card ── */}
      {infoCard && (
        <InfoCard card={infoCard} onClose={() => { setInfoCard(null); setEnterZone(null); }} />
      )}

      {/* ── Mobile D-pad ── shown on touch devices / always shown when started */}
      {started && (
        <div style={{
          position: 'absolute', bottom: 24, right: 20, zIndex: 8,
          userSelect: 'none', touchAction: 'none',
          display: 'flex', flexDirection: 'column', alignItems: 'center', gap: 4,
        }}>
          {/* Up */}
          <button
            onPointerDown={() => { mobileKeysRef.current.w = true; }}
            onPointerUp={() => { mobileKeysRef.current.w = false; }}
            onPointerLeave={() => { mobileKeysRef.current.w = false; }}
            style={{
              width: 46, height: 46, borderRadius: 10,
              background: 'rgba(0,212,255,0.10)', border: '1px solid rgba(0,212,255,0.30)',
              color: '#00d4ff', fontSize: 18, cursor: 'pointer',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              touchAction: 'none', WebkitTapHighlightColor: 'transparent',
            }}>▲</button>
          {/* Middle row */}
          <div style={{ display: 'flex', gap: 4 }}>
            <button
              onPointerDown={() => { mobileKeysRef.current.a = true; }}
              onPointerUp={() => { mobileKeysRef.current.a = false; }}
              onPointerLeave={() => { mobileKeysRef.current.a = false; }}
              style={{
                width: 46, height: 46, borderRadius: 10,
                background: 'rgba(0,212,255,0.10)', border: '1px solid rgba(0,212,255,0.30)',
                color: '#00d4ff', fontSize: 18, cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                touchAction: 'none', WebkitTapHighlightColor: 'transparent',
              }}>◄</button>
            <div style={{ width: 46, height: 46 }} />
            <button
              onPointerDown={() => { mobileKeysRef.current.d = true; }}
              onPointerUp={() => { mobileKeysRef.current.d = false; }}
              onPointerLeave={() => { mobileKeysRef.current.d = false; }}
              style={{
                width: 46, height: 46, borderRadius: 10,
                background: 'rgba(0,212,255,0.10)', border: '1px solid rgba(0,212,255,0.30)',
                color: '#00d4ff', fontSize: 18, cursor: 'pointer',
                display: 'flex', alignItems: 'center', justifyContent: 'center',
                touchAction: 'none', WebkitTapHighlightColor: 'transparent',
              }}>►</button>
          </div>
          {/* Down */}
          <button
            onPointerDown={() => { mobileKeysRef.current.s = true; }}
            onPointerUp={() => { mobileKeysRef.current.s = false; }}
            onPointerLeave={() => { mobileKeysRef.current.s = false; }}
            style={{
              width: 46, height: 46, borderRadius: 10,
              background: 'rgba(168,85,247,0.10)', border: '1px solid rgba(168,85,247,0.30)',
              color: '#a855f7', fontSize: 18, cursor: 'pointer',
              display: 'flex', alignItems: 'center', justifyContent: 'center',
              touchAction: 'none', WebkitTapHighlightColor: 'transparent',
            }}>▼</button>
        </div>
      )}

      <style>{`
        @keyframes fadein { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: none; } }
        @keyframes slideUp { from { opacity: 0; transform: translateY(24px); } to { opacity: 1; transform: none; } }
        @keyframes chainPulse { 0%,100% { opacity: 1; } 50% { opacity: 0.3; } }
      `}</style>
    </div>
  );
}

/* ── Wallet Connect Button ── */
function WalletButton({ wallet }) {
  const { isConnected, connecting, error, connect, disconnect, shortAddress, account } = wallet;
  const pointerEvents = 'auto'; // re-enable for this element only

  if (isConnected) {
    return (
      <button
        onClick={disconnect}
        style={{
          pointerEvents,
          background: 'rgba(0,212,255,0.08)', border: '1px solid #00d4ff55',
          borderRadius: 8, padding: '5px 12px', cursor: 'pointer',
          display: 'flex', alignItems: 'center', gap: 7,
          fontFamily: 'monospace', fontSize: 12,
        }}
      >
        <span style={{ width: 8, height: 8, borderRadius: '50%', background: '#00d4ff', boxShadow: '0 0 6px #00d4ff', display: 'inline-block' }} />
        <span style={{ color: '#00d4ff' }}>{account.meta?.name ?? shortAddress}</span>
        <span style={{ color: '#334455', fontSize: 10 }}>({shortAddress})</span>
        <span style={{ color: '#334455', fontSize: 10, marginLeft: 4 }}>✕</span>
      </button>
    );
  }

  return (
    <button
      onClick={connect}
      disabled={connecting}
      title={error ?? undefined}
      style={{
        pointerEvents,
        background: connecting ? 'rgba(0,8,22,0.82)' : 'rgba(168,85,247,0.12)',
        border: `1px solid ${error ? '#ff444455' : '#a855f744'}`,
        borderRadius: 8, padding: '5px 14px', cursor: connecting ? 'default' : 'pointer',
        color: error ? '#ff6666' : '#a855f7', fontFamily: 'monospace', fontSize: 12,
        letterSpacing: 0.5, transition: 'background 0.15s',
      }}
    >
      {connecting ? 'Connecting…' : error ? '⚠ No Wallet' : '⚡ Connect Wallet'}
    </button>
  );
}

/* ── Info Card Component ── */
function InfoCard({ card, onClose }) {
  return (
    <div
      style={{
        position: 'absolute', bottom: 28, right: 28,
        width: 360, maxWidth: 'calc(100vw - 40px)',
        background: 'rgba(8,8,22,0.92)',
        border: `1px solid ${card.color}44`,
        borderRadius: 14,
        padding: '22px 24px',
        fontFamily: "'Inter', 'Segoe UI', system-ui, sans-serif",
        animation: 'slideUp 0.35s cubic-bezier(0.23, 1, 0.32, 1)',
        backdropFilter: 'blur(16px)',
        boxShadow: `0 0 40px ${card.color}22, 0 8px 32px rgba(0,0,0,0.5)`,
        zIndex: 5,
      }}
    >
      {/* Close */}
      <button
        onClick={onClose}
        style={{
          position: 'absolute', top: 12, right: 14,
          background: 'none', border: 'none', cursor: 'pointer',
          color: '#445566', fontSize: 18, lineHeight: 1, padding: 0,
        }}
      >×</button>

      {/* Header */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 12, marginBottom: 12 }}>
        <span style={{ fontSize: 32 }}>{card.emoji}</span>
        <div>
          <h3 style={{
            margin: 0, fontSize: 17, fontWeight: 800,
            color: card.color, letterSpacing: '-0.3px',
          }}>{card.title}</h3>
          <div style={{
            height: 2, width: 40, marginTop: 4,
            background: `linear-gradient(90deg, ${card.color}, transparent)`,
            borderRadius: 1,
          }} />
        </div>
      </div>

      {/* Body */}
      <p style={{
        margin: '0 0 16px',
        color: '#8899bb', fontSize: 14, lineHeight: 1.65,
      }}>{card.body}</p>

      {/* CTA */}
      {card.link && (
        <a
          href={card.link}
          style={{
            display: 'inline-block',
            background: `${card.color}18`,
            border: `1px solid ${card.color}66`,
            color: card.color,
            borderRadius: 6,
            padding: '7px 16px',
            fontSize: 13, fontWeight: 700,
            textDecoration: 'none',
            letterSpacing: 0.3,
            transition: 'background 0.15s',
          }}
          onMouseEnter={e => e.target.style.background = `${card.color}30`}
          onMouseLeave={e => e.target.style.background = `${card.color}18`}
        >
          {card.linkText || 'Learn More →'}
        </a>
      )}

      {/* Proximity indicator */}
      <div style={{
        position: 'absolute', top: 12, left: 16,
        width: 8, height: 8, borderRadius: '50%',
        background: card.color,
        boxShadow: `0 0 8px ${card.color}`,
        animation: 'fadein 0.5s',
      }} />
    </div>
  );
}
