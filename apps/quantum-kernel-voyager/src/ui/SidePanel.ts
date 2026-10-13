/**
 * @module ui/SidePanel
 * Collapsible side panel with tabbed sections:
 * - Navigator: world list, warp controls
 * - Inventory: artifact collection
 * - Chain: connected chains, block heights
 * - Ship: upgrades, status
 */
import type { GameState } from "../game/GameState";

type SidePanelTab = "navigator" | "inventory" | "chain" | "ship";

export class SidePanel {
  private readonly el: HTMLElement;
  private activeTab: SidePanelTab = "navigator";
  private collapsed = false;

  constructor(el: HTMLElement, private readonly gameState: GameState) {
    this.el = el;
    this.el.classList.add("side-panel");
    this.render();
    this.bindTabEvents();
  }

  update(): void {
    this.renderContent();
  }

  toggle(): void {
    this.collapsed = !this.collapsed;
    this.el.classList.toggle("collapsed", this.collapsed);
  }

  setTab(tab: SidePanelTab): void {
    this.activeTab = tab;
    this.render();
    this.bindTabEvents();
  }

  private render(): void {
    this.el.innerHTML = `
      <div class="sp-tabs">
        ${this.tabButton("navigator", "NAV")}
        ${this.tabButton("inventory", "INV")}
        ${this.tabButton("chain", "CHAIN")}
        ${this.tabButton("ship", "SHIP")}
        <button class="sp-tab-btn sp-collapse-btn" data-action="collapse">≡</button>
      </div>
      <div class="sp-content" id="sp-content"></div>
    `;
    this.renderContent();
  }

  private tabButton(tab: SidePanelTab, label: string): string {
    const active = this.activeTab === tab ? "active" : "";
    return `<button class="sp-tab-btn ${active}" data-tab="${tab}">${label}</button>`;
  }

  private renderContent(): void {
    const container = this.el.querySelector("#sp-content");
    if (!container) return;

    switch (this.activeTab) {
      case "navigator":
        container.innerHTML = this.renderNavigator();
        break;
      case "inventory":
        container.innerHTML = this.renderInventory();
        break;
      case "chain":
        container.innerHTML = this.renderChain();
        break;
      case "ship":
        container.innerHTML = this.renderShipTab();
        break;
    }
  }

  private renderNavigator(): string {
    const worlds = this.gameState.getDiscoveredWorlds();
    if (worlds.length === 0) {
      return `
        <div class="sp-section">
          <h3>Navigator</h3>
          <p class="sp-empty">No worlds discovered yet. Explore to find new worlds!</p>
        </div>
      `;
    }
    const worldItems = worlds
      .map(
        (w) => `
      <div class="sp-list-item" data-seed="${w.seed}">
        <span class="sp-world-name">${w.name}</span>
        <span class="sp-world-biome">${w.biome}</span>
        ${w.visited ? '<span class="sp-badge visited">VISITED</span>' : '<span class="sp-badge new">NEW</span>'}
      </div>
    `,
      )
      .join("");

    return `
      <div class="sp-section">
        <h3>Navigator <span class="sp-count">${worlds.length}</span></h3>
        <div class="sp-list">${worldItems}</div>
      </div>
    `;
  }

  private renderInventory(): string {
    return `
      <div class="sp-section">
        <h3>Inventory</h3>
        <p class="sp-empty">Claim artifacts on planet surfaces to add them here.</p>
        <div class="sp-rarity-grid">
          <div class="sp-rarity-item common">Common <span>0</span></div>
          <div class="sp-rarity-item uncommon">Uncommon <span>0</span></div>
          <div class="sp-rarity-item rare">Rare <span>0</span></div>
          <div class="sp-rarity-item epic">Epic <span>0</span></div>
          <div class="sp-rarity-item legendary">Legendary <span>0</span></div>
        </div>
      </div>
    `;
  }

  private renderChain(): string {
    return `
      <div class="sp-section">
        <h3>Chain Status</h3>
        <div class="sp-chain-item">
          <span class="sp-chain-name">Atlas Sphere</span>
          <span class="sp-chain-status connecting">CONNECTING</span>
        </div>
        <div class="sp-chain-item">
          <span class="sp-chain-name">Local Dev</span>
          <span class="sp-chain-status connected">CONNECTED</span>
        </div>
      </div>
    `;
  }

  private renderShipTab(): string {
    const ship = this.gameState.getShip();
    const upgrades = ship.upgrades;
    const upgradeList =
      upgrades.length > 0
        ? upgrades
            .map(
              (u) => `
          <div class="sp-list-item">
            <span class="sp-upgrade-name">${u.name}</span>
            <span class="sp-upgrade-tier">T${u.tier}</span>
          </div>
        `,
            )
            .join("")
        : '<p class="sp-empty">No upgrades installed.</p>';

    return `
      <div class="sp-section">
        <h3>Ship</h3>
        <div class="sp-ship-stats">
          <div>Hull: ${Math.round(ship.health)}</div>
          <div>Fuel: ${Math.round(ship.fuel)}</div>
          <div>Position: (${ship.position.x.toFixed(1)}, ${ship.position.y.toFixed(1)}, ${ship.position.z.toFixed(1)})</div>
        </div>
        <h4>Upgrades</h4>
        ${upgradeList}
      </div>
    `;
  }

  private bindTabEvents(): void {
    this.el.querySelectorAll<HTMLButtonElement>("[data-tab]").forEach((btn) => {
      btn.addEventListener("click", () => {
        this.activeTab = btn.dataset.tab as SidePanelTab;
        this.render();
        this.bindTabEvents();
      });
    });
    this.el.querySelector<HTMLButtonElement>("[data-action=collapse]")?.addEventListener("click", () => {
      this.toggle();
    });
  }
}
