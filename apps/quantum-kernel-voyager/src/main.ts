/**
 * @module main
 * Application entry point for Quantum Kernel Voyager.
 *
 * Initializes all systems in order:
 * 1. Storage (IndexedDB)
 * 2. Three.js scene
 * 3. Chain adapters
 * 4. Game state (+ load saved voyage)
 * 5. Game managers
 * 6. UI overlay
 * 7. Input wiring
 * 8. Render loop start
 */
import "./styles/main.css";
import "./styles/overlay.css";
import "./styles/components.css";
import "./styles/bottom-nav.css";

import { SceneManager } from "./scene/SceneManager";
import { InteractionManager } from "./scene/InteractionManager";
import { ParticleSystem } from "./scene/ParticleSystem";
import { GameState } from "./game/GameState";
import { WorldManager } from "./game/WorldManager";
import { EconomyManager } from "./game/EconomyManager";
import { ArtifactManager } from "./game/ArtifactManager";
import { VoyagerShip } from "./game/VoyagerShip";
import { Overlay } from "./ui/Overlay";
import { BottomNav } from "./ui/BottomNav";
import { LandingPrompt } from "./ui/LandingPrompt";
import { PlanetDetail } from "./ui/PlanetDetail";
import { AtlasSphereAdapter } from "./adapters/AtlasSphereAdapter";
import { LocalDevAdapter } from "./adapters/LocalDevAdapter";
import { SyncQueue } from "./adapters/SyncQueue";
import { store } from "./storage/IndexedDBStore";
import { eventBus } from "./ipc/events";
import { loadVoyage, saveVoyage } from "./ipc/tauri";
import { animateEntity } from "./scene/Animations";
import { createWarpState, updateWarp } from "./scene/Animations";
import { SolarSystemBuilder } from "./scene/SolarSystemBuilder";
import { PlanetRegistry } from "./services/PlanetRegistry";
import { getPlanetPosition } from "./services/OrbitCalculator";
import { DEFAULT_SUN_METRICS } from "./config/solar-system-config";
import type { SunMetrics } from "./types/solar-system";
import * as THREE from "three";
import type { ChainEvent } from "./types/chain";

// -------------------------------------------------------------------------
// Bootstrap
// -------------------------------------------------------------------------

async function main(): Promise<void> {
  console.info("[Voyager] Initializing Quantum Kernel Voyager...");

  const splashStatus = document.getElementById("splash-status");
  const setSplash = (msg: string) => {
    if (splashStatus) splashStatus.textContent = msg;
    console.info(`[Voyager] ${msg}`);
  };

  // 1. Storage
  setSplash("Opening local storage...");
  await store.open();

  // 2. Three.js scene
  setSplash("Initializing 3D engine...");
  const canvasContainer = document.getElementById("canvas-container")!;
  const canvas = document.createElement("canvas");
  canvas.style.width = "100%";
  canvas.style.height = "100%";
  canvasContainer.appendChild(canvas);

  const sceneManager = new SceneManager(canvas);
  const particles = new ParticleSystem(sceneManager.scene);
  const interactions = new InteractionManager(sceneManager.camera, canvas);

  // Populate ambient scene so the 3D view isn't empty while in menu
  populateAmbientScene(sceneManager);

  // 3. Game state
  setSplash("Initializing game state...");
  const gameState = new GameState();
  const economy = new EconomyManager(gameState);
  const worldManager = new WorldManager(sceneManager);
  const artifacts = new ArtifactManager(economy);
  const ship = new VoyagerShip(sceneManager, particles);

  // 4. Chain adapters
  setSplash("Connecting to blockchain...");
  const atlasAdapter = new AtlasSphereAdapter();
  const localAdapter = new LocalDevAdapter();
  const syncQueue = new SyncQueue();
  syncQueue.setAdapter(localAdapter);
  await syncQueue.init();

  // Connect chains (non-blocking)
  localAdapter.connect().then(() => {
    console.info("[Voyager] Local dev chain connected");
    eventBus.emit("chain:status", { chainId: "local-dev", connected: true, latencyMs: 1 });
  });

  atlasAdapter.connect().then(() => {
    console.info("[Voyager] Atlas Sphere adapter connected");
    eventBus.emit("chain:status", { chainId: "atlas-sphere-devnet", connected: true, latencyMs: 12 });
  });

  // Subscribe to chain events
  localAdapter.subscribe({}, (event: ChainEvent) => {
    if (event.type === "block") {
      eventBus.emit("chain:block", {
        chainId: event.chainId,
        height: event.blockHeight,
        hash: typeof event.data === "object" && event.data !== null ? (event.data as Record<string, string>).hash ?? "" : "",
        txCount: typeof event.data === "object" && event.data !== null ? ((event.data as Record<string, unknown[]>).transactions?.length ?? 0) : 0,
      });
      worldManager.updateNodeBlockHeight(event.chainId, event.blockHeight);
      // Passive staking reward
      economy.onBlockReward(0); // No staking in dev mode
    }
  });

  // 5. Load saved voyage state (if any)
  setSplash("Loading saved data...");
  try {
    const savedVoyage = await loadVoyage();
    if (savedVoyage) {
      gameState.loadVoyageState(savedVoyage);
      console.info("[Voyager] Loaded saved voyage");
    }
  } catch (err) {
    console.warn("[Voyager] No saved voyage found:", err);
  }

  // 6. Solar system
  setSplash("Building solar system...");
  const planetRegistry = new PlanetRegistry();
  const solarSystem = new SolarSystemBuilder(sceneManager, planetRegistry);
  let sunMetrics: SunMetrics = { ...DEFAULT_SUN_METRICS };

  // 7. UI overlay + solar system UI
  setSplash("Building interface...");
  const overlay = new Overlay(gameState, economy, sceneManager);
  const bottomNav = new BottomNav();
  const landingPrompt = new LandingPrompt();
  const planetDetail = new PlanetDetail();

  bottomNav.mount();
  landingPrompt.mount();
  planetDetail.mount();

  // Wire planet detail "Land" button
  planetDetail.setOnLand((planetId) => {
    if (gameState.getMode() === "solar_system") {
      const planet = planetRegistry.get(planetId);
      if (planet) {
        gameState.transition("landing", { targetPlanetId: planetId, elapsed: 0 });
      }
    }
  });

  // 8. Transition to menu (then Enter → solar_system)
  gameState.transition("menu", { selectedSlot: 0 });

  // 8. Wire interactions
  interactions.on((event) => {
    if (event.type === "select" && event.entity) {
      eventBus.emit("scene:entity-click", {
        entityId: event.entity.id,
        kind: event.entity.kind,
        screenX: event.screenX,
        screenY: event.screenY,
      });
    }
    if (event.type === "double_click" && event.entity && event.worldPoint) {
      sceneManager.cameraController.focusOn(
        new THREE.Vector3(event.worldPoint.x, event.worldPoint.y, event.worldPoint.z),
      );
    }
    if (event.type === "context_menu" && event.entity) {
      overlay.contextMenu.show(event.screenX, event.screenY, [
        { label: "Inspect", icon: "🔍", action: () => console.info("Inspect:", event.entity?.id) },
        { label: "Focus", icon: "📷", action: () => {
          if (event.worldPoint) {
            sceneManager.cameraController.focusOn(
              new THREE.Vector3(event.worldPoint.x, event.worldPoint.y, event.worldPoint.z),
            );
          }
        }},
        { separator: true, label: "" },
        { label: "Warp Here", icon: "🚀", action: () => console.info("Warp to:", event.entity?.id) },
      ]);
    }
  });

  // 9. Game mode handlers
  gameState.onChange((prev, next) => {
    eventBus.emit("game:mode-change", { from: prev, to: next });
    console.info(`[Voyager] Mode: ${prev} → ${next}`);

    if (next === "solar_system") {
      // Build solar system if not already built
      solarSystem.build();
      ship.spawn({ x: 0, y: 20, z: 80 });
      bottomNav.setMode("SOLAR SYSTEM");

      // Position camera to see the sun + inner planets
      sceneManager.cameraController.focusOn(new THREE.Vector3(0, 30, 120));
    }

    if (next === "exploring") {
      // Teardown solar system, load world (backward compat)
      solarSystem.teardown();
      landingPrompt.hide();
      planetDetail.hide();

      const ctx = gameState.getContext("exploring");
      if (ctx) {
        const { world, artifacts: worldArtifacts } = worldManager.loadWorld(ctx.currentWorldSeed);
        gameState.addWorld(world);
        artifacts.setWorldArtifacts(ctx.currentWorldSeed, worldArtifacts);
        ship.spawn({ x: 0, y: world.visited ? 60 : 80, z: 100 });
      }
      bottomNav.setMode("EXPLORING");
    }

    if (next === "landing") {
      const ctx = gameState.getContext("landing");
      if (ctx) {
        const planet = planetRegistry.get(ctx.targetPlanetId);
        bottomNav.setMode(`LANDING → ${planet?.name ?? "Unknown"}`);
        landingPrompt.hide();
        planetDetail.hide();
      }
    }

    if (next === "surface") {
      const ctx = gameState.getContext("surface");
      if (ctx) {
        const planet = planetRegistry.get(ctx.planetId);
        bottomNav.setMode(`ON SURFACE: ${planet?.name ?? "Unknown"}`);
        console.info(`[Voyager] Landed on ${planet?.name} → ${ctx.landingTarget}`);
        // TODO: transition to planet's app interface (iframe, route, etc.)
      }
    }

    if (prev === "solar_system" && next === "paused") {
      bottomNav.setMode("PAUSED");
    }
    if (prev === "paused" && next === "solar_system") {
      bottomNav.setMode("SOLAR SYSTEM");
    }
  });

  // 10. Keyboard shortcuts
  window.addEventListener("keydown", (e) => {
    const mode = gameState.getMode();

    if (e.key === "Escape") {
      if (mode === "exploring") {
        gameState.transition("paused", { previousMode: mode });
      } else if (mode === "solar_system") {
        gameState.transition("paused", { previousMode: mode });
      } else if (mode === "paused") {
        const ctx = gameState.getContext("paused");
        const prevMode = ctx?.previousMode ?? "solar_system";
        if (prevMode === "exploring") {
          gameState.transition("exploring", {
            currentWorldSeed: worldManager.getCurrentSeed() ?? "genesis",
          });
        } else {
          gameState.transition("solar_system", {});
        }
      } else if (mode === "surface") {
        gameState.transition("solar_system", {});
      } else if (mode === "inspecting") {
        gameState.transition("solar_system", {});
        planetDetail.hide();
      }
    }

    if (e.key === "Enter" && mode === "menu") {
      // Enter the solar system
      gameState.transition("solar_system", {});
    }

    // F key: land on nearby planet
    if (e.key === "f" || e.key === "F") {
      if (mode === "solar_system") {
        const candidate = landingPrompt.getCurrentPlanet();
        if (candidate) {
          gameState.transition("landing", { targetPlanetId: candidate.id, elapsed: 0 });
        }
      }
    }

    // Tab key: toggle planet detail for nearest planet
    if (e.key === "Tab") {
      e.preventDefault();
      if (mode === "solar_system") {
        if (planetDetail.isVisible()) {
          planetDetail.hide();
        } else {
          const shipState = gameState.getShip();
          const candidate = planetRegistry.findLandingCandidate(shipState.position);
          if (candidate) planetDetail.show(candidate);
        }
      }
    }
  });

  // Ship input forwarding (works in exploring AND solar_system modes)
  window.addEventListener("keydown", (e) => {
    const mode = gameState.getMode();
    if (mode !== "exploring" && mode !== "solar_system") return;
    switch (e.code) {
      case "KeyW": ship.setInput("forward", true); break;
      case "KeyS": ship.setInput("backward", true); break;
      case "KeyA": ship.setInput("left", true); break;
      case "KeyD": ship.setInput("right", true); break;
      case "Space": ship.setInput("up", true); break;
      case "ShiftLeft": ship.setInput("boost", true); break;
    }
  });

  window.addEventListener("keyup", (e) => {
    switch (e.code) {
      case "KeyW": ship.setInput("forward", false); break;
      case "KeyS": ship.setInput("backward", false); break;
      case "KeyA": ship.setInput("left", false); break;
      case "KeyD": ship.setInput("right", false); break;
      case "Space": ship.setInput("up", false); break;
      case "ShiftLeft": ship.setInput("boost", false); break;
    }
  });

  // Auto-save every 60 seconds while exploring or in solar system
  setInterval(() => {
    const mode = gameState.getMode();
    if (mode === "exploring" || mode === "solar_system") {
      const voyageState = gameState.toVoyageState();
      saveVoyage(voyageState).catch((err) => console.warn("[Voyager] Auto-save failed:", err));
    }
  }, 60_000);

  // Warp state
  const warpState = createWarpState();

  // Landing animation duration (seconds)
  const LANDING_DURATION = 2.0;

  // 11. Extended game loop (runs alongside SceneManager's render loop)
  const clock = { last: performance.now() };

  function gameLoop(): void {
    requestAnimationFrame(gameLoop);
    const now = performance.now();
    const dt = (now - clock.last) / 1000;
    clock.last = now;

    const mode = gameState.getMode();

    // Update particles
    particles.update(dt);

    // Animate entities
    const time = now / 1000;
    for (const entity of sceneManager.getAllEntities()) {
      const obj = sceneManager.getObject(entity.id);
      if (obj) {
        animateEntity(obj, entity.kind, time, dt);
      }
    }

    // Ship physics (exploring or solar_system)
    if (mode === "exploring" || mode === "solar_system") {
      ship.update(dt, gameState.getShip());
      ship.setLimits(economy.getMaxFuel(), economy.getMaxHealth());
    }

    // Solar system updates
    if (mode === "solar_system" || mode === "landing") {
      solarSystem.update(dt, sunMetrics);

      // Update bottom nav
      const shipState = gameState.getShip();
      bottomNav.update(planetRegistry, sunMetrics, { x: shipState.position.x, z: shipState.position.z });

      // Proximity detection for landing prompt
      if (mode === "solar_system") {
        const candidate = planetRegistry.findLandingCandidate(shipState.position);
        if (candidate) {
          landingPrompt.show(candidate);
        } else {
          landingPrompt.hide();
        }
      }
    }

    // Landing animation
    if (mode === "landing") {
      const ctx = gameState.getContext("landing");
      if (ctx) {
        ctx.elapsed += dt;
        if (ctx.elapsed >= LANDING_DURATION) {
          const planet = planetRegistry.get(ctx.targetPlanetId);
          gameState.transition("surface", {
            planetId: ctx.targetPlanetId,
            landingTarget: planet?.landingTarget ?? "",
          });
        } else {
          // Animate camera zoom toward planet
          const planet = planetRegistry.get(ctx.targetPlanetId);
          if (planet) {
            const ppos = getPlanetPosition(planet);
            const t = ctx.elapsed / LANDING_DURATION;
            const eased = t * t * (3 - 2 * t); // smoothstep
            const target = new THREE.Vector3(ppos.x, ppos.y + planet.planetRadius * 2, ppos.z);
            sceneManager.camera.position.lerp(target, eased * 0.02);
            sceneManager.camera.lookAt(ppos.x, ppos.y, ppos.z);
          }
        }
      }
    }

    // Warp update
    if (warpState.active) {
      const pos = updateWarp(warpState, dt);
      if (pos) ship.teleport({ x: pos.x, y: pos.y, z: pos.z });
    }
  }

  // 12. Start everything
  sceneManager.start();
  overlay.start();
  gameLoop();

  // Dismiss boot splash
  setSplash("Ready — press Enter to explore the solar system");
  const splash = document.getElementById("boot-splash");
  if (splash) {
    // Brief delay so user can read the final status
    setTimeout(() => splash.classList.add("hidden"), 800);
    // Remove from DOM after transition
    setTimeout(() => splash.remove(), 1500);
  }

  console.info("[Voyager] ✦ Quantum Kernel Voyager initialized");
  console.info("[Voyager] Press Enter to begin exploring");
}

// -------------------------------------------------------------------------
// Ambient scene — visible in menu & loading, gives visual life to the 3D view
// -------------------------------------------------------------------------

function populateAmbientScene(sm: SceneManager): void {
  // Starfield
  const starCount = 2000;
  const starGeo = new THREE.BufferGeometry();
  const positions = new Float32Array(starCount * 3);
  const colors = new Float32Array(starCount * 3);
  for (let i = 0; i < starCount; i++) {
    const i3 = i * 3;
    const r = 200 + Math.random() * 800;
    const theta = Math.random() * Math.PI * 2;
    const phi = Math.acos(2 * Math.random() - 1);
    positions[i3] = r * Math.sin(phi) * Math.cos(theta);
    positions[i3 + 1] = r * Math.sin(phi) * Math.sin(theta);
    positions[i3 + 2] = r * Math.cos(phi);
    // Cyan / white / magenta tints
    const tint = Math.random();
    if (tint < 0.3) {
      colors[i3] = 0; colors[i3 + 1] = 0.94; colors[i3 + 2] = 1.0; // cyan
    } else if (tint < 0.5) {
      colors[i3] = 1.0; colors[i3 + 1] = 0; colors[i3 + 2] = 1.0; // magenta
    } else {
      colors[i3] = 0.9; colors[i3 + 1] = 0.9; colors[i3 + 2] = 1.0; // white-ish
    }
  }
  starGeo.setAttribute("position", new THREE.BufferAttribute(positions, 3));
  starGeo.setAttribute("color", new THREE.BufferAttribute(colors, 3));
  const starMat = new THREE.PointsMaterial({ size: 1.2, vertexColors: true, sizeAttenuation: true, transparent: true, opacity: 0.8 });
  const stars = new THREE.Points(starGeo, starMat);
  stars.name = "ambient-stars";
  sm.scene.add(stars);

  // Slowly rotating nebula ring
  const ringGeo = new THREE.TorusGeometry(60, 0.8, 16, 100);
  const ringMat = new THREE.MeshBasicMaterial({ color: 0x00f0ff, transparent: true, opacity: 0.12, wireframe: true });
  const ring = new THREE.Mesh(ringGeo, ringMat);
  ring.rotation.x = Math.PI / 3;
  ring.name = "ambient-ring";
  sm.scene.add(ring);

  // A second ring at different angle
  const ring2 = ring.clone();
  ring2.rotation.x = -Math.PI / 4;
  ring2.rotation.z = Math.PI / 5;
  (ring2.material as THREE.MeshBasicMaterial).color.setHex(0xff00ff);
  ring2.name = "ambient-ring2";
  sm.scene.add(ring2);

  // Central glowing orb (represents genesis block / home)
  const orbGeo = new THREE.IcosahedronGeometry(3, 2);
  const orbMat = new THREE.MeshStandardMaterial({
    color: 0x00f0ff,
    emissive: 0x00f0ff,
    emissiveIntensity: 0.8,
    metalness: 0.9,
    roughness: 0.1,
    wireframe: true,
  });
  const orb = new THREE.Mesh(orbGeo, orbMat);
  orb.name = "ambient-orb";
  sm.scene.add(orb);

  const orbLight = new THREE.PointLight(0x00f0ff, 4, 50);
  orbLight.name = "ambient-orb-light";
  sm.scene.add(orbLight);

  // Floating data nodes in a circle
  for (let i = 0; i < 6; i++) {
    const angle = (i / 6) * Math.PI * 2;
    const nodeGeo = new THREE.OctahedronGeometry(0.6, 0);
    const nodeMat = new THREE.MeshStandardMaterial({
      color: i % 2 === 0 ? 0x00f0ff : 0xff00ff,
      emissive: i % 2 === 0 ? 0x00f0ff : 0xff00ff,
      emissiveIntensity: 0.5,
      metalness: 0.8,
      roughness: 0.1,
    });
    const node = new THREE.Mesh(nodeGeo, nodeMat);
    node.position.set(Math.cos(angle) * 15, Math.sin(angle * 2) * 3, Math.sin(angle) * 15);
    node.name = `ambient-node-${i}`;
    sm.scene.add(node);
  }

  // Animate ambient scene objects
  function animateAmbient() {
    requestAnimationFrame(animateAmbient);
    const t = performance.now() * 0.001;

    // Rotate stars slowly
    stars.rotation.y = t * 0.01;

    // Rotate rings
    const r1 = sm.scene.getObjectByName("ambient-ring");
    if (r1) r1.rotation.z = t * 0.05;
    const r2 = sm.scene.getObjectByName("ambient-ring2");
    if (r2) r2.rotation.y = t * 0.03;

    // Pulse the central orb
    const o = sm.scene.getObjectByName("ambient-orb");
    if (o) {
      const s = 1 + Math.sin(t * 2) * 0.08;
      o.scale.setScalar(s);
      o.rotation.y = t * 0.3;
      o.rotation.x = t * 0.15;
    }

    // Orbit the data nodes
    for (let i = 0; i < 6; i++) {
      const n = sm.scene.getObjectByName(`ambient-node-${i}`);
      if (n) {
        const angle = (i / 6) * Math.PI * 2 + t * 0.2;
        n.position.x = Math.cos(angle) * 15;
        n.position.z = Math.sin(angle) * 15;
        n.position.y = Math.sin(t + i) * 3;
        n.rotation.x = t * 0.5;
        n.rotation.z = t * 0.3;
      }
    }
  }
  animateAmbient();
}

// -------------------------------------------------------------------------
// Launch
// -------------------------------------------------------------------------

main().catch((err) => {
  console.error("[Voyager] Fatal initialization error:", err);
  document.body.innerHTML = `
    <div style="padding: 40px; color: #ff4444; font-family: monospace; background: #0a0a1a; height: 100vh;">
      <h1>⚠ Quantum Kernel Voyager — Initialization Failed</h1>
      <pre>${err instanceof Error ? err.stack : String(err)}</pre>
    </div>
  `;
});
