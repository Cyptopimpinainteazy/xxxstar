/**
 * @module ui/BottomNav
 * Bottom navigation orrery strip — a semi-transparent panel at the bottom
 * of the viewport showing a top-down mini solar system map, ship position,
 * network pulse, event ripple timeline, and quick nav controls.
 */
import type { PlanetEntity, SunMetrics, EventRipple } from "../types/solar-system";
import type { PlanetRegistry } from "../services/PlanetRegistry";

export class BottomNav {
  private el: HTMLElement;
  private canvas: HTMLCanvasElement;
  private ctx: CanvasRenderingContext2D;
  private infoPanel: HTMLElement;
  private modeIndicator: HTMLElement;
  private pulseMeter: HTMLElement;
  private eventBar: HTMLElement;
  private onPlanetClick: ((planetId: string) => void) | null = null;

  private shipX = 0;
  private shipZ = 0;
  private maxRadius = 200;

  /** Recent event ripples. */
  private ripples: EventRipple[] = [];

  constructor() {
    // Root container
    this.el = document.createElement("div");
    this.el.id = "bottom-nav";
    this.el.className = "bottom-nav";

    // Left: mini orrery canvas
    const orreryBox = document.createElement("div");
    orreryBox.className = "bn-orrery";
    this.canvas = document.createElement("canvas");
    this.canvas.className = "bn-orrery-canvas";
    this.canvas.width = 200;
    this.canvas.height = 140;
    orreryBox.appendChild(this.canvas);
    this.ctx = this.canvas.getContext("2d")!;

    // Center: info + event timeline
    const centerBox = document.createElement("div");
    centerBox.className = "bn-center";

    this.modeIndicator = document.createElement("div");
    this.modeIndicator.className = "bn-mode";
    this.modeIndicator.textContent = "SOLAR SYSTEM";

    this.eventBar = document.createElement("div");
    this.eventBar.className = "bn-events";

    this.infoPanel = document.createElement("div");
    this.infoPanel.className = "bn-info";

    centerBox.appendChild(this.modeIndicator);
    centerBox.appendChild(this.infoPanel);
    centerBox.appendChild(this.eventBar);

    // Right: network pulse meter
    const rightBox = document.createElement("div");
    rightBox.className = "bn-right";

    this.pulseMeter = document.createElement("div");
    this.pulseMeter.className = "bn-pulse";
    this.pulseMeter.innerHTML = `<span class="bn-pulse-label">NETWORK</span><span class="bn-pulse-value">0 TPS</span>`;

    rightBox.appendChild(this.pulseMeter);

    this.el.appendChild(orreryBox);
    this.el.appendChild(centerBox);
    this.el.appendChild(rightBox);

    // Click handler for orrery canvas
    this.canvas.addEventListener("click", (e) => this.handleOrreryClick(e));
  }

  /** Mount into the DOM. */
  mount(parent: HTMLElement = document.body): void {
    parent.appendChild(this.el);
  }

  /** Unmount from the DOM. */
  unmount(): void {
    this.el.remove();
  }

  /** Set callback for when user clicks a planet on the orrery. */
  setOnPlanetClick(cb: (planetId: string) => void): void {
    this.onPlanetClick = cb;
  }

  /** Set the current game mode label. */
  setMode(label: string): void {
    this.modeIndicator.textContent = label.toUpperCase();
  }

  /** Push an event ripple to the timeline. */
  pushRipple(ripple: EventRipple): void {
    this.ripples.push(ripple);
    if (this.ripples.length > 30) this.ripples.shift();
  }

  /** Full update each frame. */
  update(
    registry: PlanetRegistry,
    sunMetrics: SunMetrics,
    shipPos: { x: number; z: number },
  ): void {
    this.shipX = shipPos.x;
    this.shipZ = shipPos.z;

    // Render orrery mini-map
    this.renderOrrery(registry.getAll());

    // Update info panel
    this.infoPanel.innerHTML = [
      `<span class="bn-stat">Block <b>#${sunMetrics.blockHeight}</b></span>`,
      `<span class="bn-stat">Validators <b>${sunMetrics.validatorCount}</b></span>`,
      `<span class="bn-stat">Peers <b>${sunMetrics.peerCount}</b></span>`,
    ].join("");

    // Update pulse meter
    const tpsEl = this.pulseMeter.querySelector(".bn-pulse-value");
    if (tpsEl) tpsEl.textContent = `${sunMetrics.tps.toFixed(1)} TPS`;

    // Render event bar
    this.renderEvents();
  }

  // -----------------------------------------------------------------------
  // Private rendering
  // -----------------------------------------------------------------------

  private renderOrrery(planets: PlanetEntity[]): void {
    const ctx = this.ctx;
    const w = this.canvas.width;
    const h = this.canvas.height;
    const cx = w / 2;
    const cy = h / 2;

    ctx.clearRect(0, 0, w, h);

    // Scale: map MAX_ORBIT_RADIUS to canvas half-width minus margin
    const margin = 12;
    const scale = (Math.min(cx, cy) - margin) / this.maxRadius;

    // Orbit rings
    for (const p of planets) {
      ctx.beginPath();
      ctx.arc(cx, cy, p.orbitRadius * scale, 0, Math.PI * 2);
      ctx.strokeStyle = `rgba(${(p.color >> 16) & 0xff}, ${(p.color >> 8) & 0xff}, ${p.color & 0xff}, 0.12)`;
      ctx.lineWidth = 0.5;
      ctx.stroke();
    }

    // Sun
    ctx.beginPath();
    ctx.arc(cx, cy, 4, 0, Math.PI * 2);
    ctx.fillStyle = "#ffaa22";
    ctx.fill();

    // Planets
    for (const p of planets) {
      const px = cx + Math.cos(p.orbitAngle) * p.orbitRadius * scale;
      const py = cy + Math.sin(p.orbitAngle) * p.orbitRadius * scale;
      const r = Math.max(2, p.planetRadius * scale * 0.5);

      ctx.beginPath();
      ctx.arc(px, py, r, 0, Math.PI * 2);
      ctx.fillStyle = `#${p.color.toString(16).padStart(6, "0")}`;
      ctx.fill();

      // Label (tiny)
      ctx.fillStyle = "rgba(255,255,255,0.6)";
      ctx.font = "7px monospace";
      ctx.fillText(p.name.slice(0, 6), px + r + 2, py + 2);
    }

    // Ship position
    const sx = cx + this.shipX * scale;
    const sy = cy + this.shipZ * scale;
    ctx.beginPath();
    ctx.moveTo(sx, sy - 3);
    ctx.lineTo(sx + 2, sy + 2);
    ctx.lineTo(sx - 2, sy + 2);
    ctx.closePath();
    ctx.fillStyle = "#00f0ff";
    ctx.fill();
  }

  private renderEvents(): void {
    // Show last 8 events as small colored pips
    const now = Date.now();
    const recent = this.ripples.filter((r) => now - r.timestamp < 30_000).slice(-8);
    const colorMap: Record<string, string> = {
      block: "#00f0ff",
      transaction: "#44ff88",
      deploy: "#ffaa00",
      governance: "#ff00ff",
      swarm: "#00ff88",
      alert: "#ff4444",
    };

    this.eventBar.innerHTML = recent
      .map((r) => {
        const age = (now - r.timestamp) / 30_000; // 0..1
        const opacity = 1 - age * 0.7;
        const col = colorMap[r.type] || "#888";
        return `<span class="bn-event-pip" style="background:${col};opacity:${opacity.toFixed(2)}" title="${r.label}"></span>`;
      })
      .join("");
  }

  private handleOrreryClick(e: MouseEvent): void {
    if (!this.onPlanetClick) return;
    const rect = this.canvas.getBoundingClientRect();
    const mx = e.clientX - rect.left;
    const my = e.clientY - rect.top;
    const cx = this.canvas.width / 2;
    const cy = this.canvas.height / 2;
    const margin = 12;
    const scale = (Math.min(cx, cy) - margin) / this.maxRadius;

    // TODO: Hit-test planets (for now, find closest)
    // This is a placeholder — full hit testing can be added later
    void mx; void my; void scale;
  }
}
