/**
 * @module ui/PlanetDetail
 * Panel that shows detailed information about a selected/hovered planet.
 * Appears as a floating card on the right side of the viewport.
 * Shows: name, type, price, volume, swarm nodes, factories, landing button.
 */
import type { PlanetEntity } from "../types/solar-system";

export class PlanetDetail {
  private el: HTMLElement;
  private visible = false;
  private onLand: ((planetId: string) => void) | null = null;

  constructor() {
    this.el = document.createElement("div");
    this.el.id = "planet-detail";
    this.el.className = "planet-detail hidden";
    this.el.innerHTML = `
      <button class="pd-close" title="Close">&times;</button>
      <div class="pd-header">
        <div class="pd-icon"></div>
        <div class="pd-title">
          <div class="pd-name"></div>
          <div class="pd-category"></div>
        </div>
      </div>
      <div class="pd-stats"></div>
      <div class="pd-swarm"></div>
      <div class="pd-factories"></div>
      <button class="pd-land-btn">Land on Planet</button>
    `;

    // Close button
    this.el.querySelector(".pd-close")!.addEventListener("click", () => this.hide());
    // Land button
    this.el.querySelector(".pd-land-btn")!.addEventListener("click", () => {
      const name = this.el.querySelector(".pd-name")?.textContent;
      if (this.onLand && name) {
        // Find by current planet data attr
        const id = this.el.dataset.planetId;
        if (id) this.onLand(id);
      }
    });
  }

  mount(parent: HTMLElement = document.body): void {
    parent.appendChild(this.el);
  }

  unmount(): void {
    this.el.remove();
  }

  setOnLand(cb: (planetId: string) => void): void {
    this.onLand = cb;
  }

  show(planet: PlanetEntity): void {
    this.visible = true;
    this.el.dataset.planetId = planet.id;

    // Header
    const nameEl = this.el.querySelector(".pd-name") as HTMLElement;
    const catEl = this.el.querySelector(".pd-category") as HTMLElement;
    const iconEl = this.el.querySelector(".pd-icon") as HTMLElement;

    nameEl.textContent = planet.name;
    catEl.textContent = `${planet.category} · ${capitalize(planet.type)}`;
    const hex = "#" + planet.color.toString(16).padStart(6, "0");
    iconEl.style.background = hex;
    iconEl.style.boxShadow = `0 0 20px ${hex}60`;

    // Stats
    const statsEl = this.el.querySelector(".pd-stats") as HTMLElement;
    statsEl.innerHTML = `
      <div class="pd-row"><span>Price</span><span>$${formatNumber(planet.price)}</span></div>
      <div class="pd-row"><span>Market Cap</span><span>$${formatNumber(planet.marketCap)}</span></div>
      <div class="pd-row"><span>24h Volume</span><span>$${formatNumber(planet.volume24h)}</span></div>
      <div class="pd-row"><span>24h Change</span><span class="${planet.priceChange24h >= 0 ? "pd-green" : "pd-red"}">${planet.priceChange24h >= 0 ? "+" : ""}${planet.priceChange24h.toFixed(2)}%</span></div>
      <div class="pd-row"><span>Orbit Radius</span><span>${planet.orbitRadius.toFixed(1)} AU</span></div>
    `;

    // Swarm nodes
    const swarmEl = this.el.querySelector(".pd-swarm") as HTMLElement;
    if (planet.swarmNodes.length > 0) {
      swarmEl.innerHTML = `
        <div class="pd-section-title">Swarm Nodes (${planet.swarmNodes.length})</div>
        ${planet.swarmNodes.slice(0, 5).map((n) => `
          <div class="pd-row pd-swarm-row">
            <span class="pd-agent-type">${n.agentType}</span>
            <span class="pd-badge pd-badge-${n.status}">${n.status}</span>
          </div>
        `).join("")}
      `;
    } else {
      swarmEl.innerHTML = `<div class="pd-section-title pd-empty">No swarm nodes assigned</div>`;
    }

    // Factories
    const factEl = this.el.querySelector(".pd-factories") as HTMLElement;
    if (planet.factories.length > 0) {
      factEl.innerHTML = `
        <div class="pd-section-title">dApp Factories (${planet.factories.length})</div>
        ${planet.factories.slice(0, 5).map((f) => `
          <div class="pd-row">
            <span>${f.name}</span>
            <span class="pd-dim">${f.activity} tx/hr</span>
          </div>
        `).join("")}
      `;
    } else {
      factEl.innerHTML = `<div class="pd-section-title pd-empty">No factories deployed</div>`;
    }

    // Landing target info
    const landBtn = this.el.querySelector(".pd-land-btn") as HTMLButtonElement;
    landBtn.textContent = `Land → ${planet.landingTarget}`;

    this.el.classList.remove("hidden");
  }

  hide(): void {
    this.visible = false;
    this.el.classList.add("hidden");
  }

  isVisible(): boolean {
    return this.visible;
  }
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}

function formatNumber(n: number): string {
  if (n >= 1_000_000_000) return (n / 1_000_000_000).toFixed(2) + "B";
  if (n >= 1_000_000) return (n / 1_000_000).toFixed(2) + "M";
  if (n >= 1_000) return (n / 1_000).toFixed(2) + "K";
  return n.toFixed(2);
}
