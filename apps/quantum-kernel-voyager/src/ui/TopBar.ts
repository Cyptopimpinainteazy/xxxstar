/**
 * @module ui/TopBar
 * Top HUD bar showing ship vitals, economy, and chain status.
 */
import type { GameState } from "../game/GameState";
import type { EconomyManager } from "../game/EconomyManager";

export class TopBar {
  private readonly el: HTMLElement;

  constructor(
    el: HTMLElement,
    private readonly gameState: GameState,
    private readonly economy: EconomyManager,
  ) {
    this.el = el;
    this.el.classList.add("top-bar");
    this.render();
  }

  update(): void {
    this.render();
  }

  private render(): void {
    const ship = this.gameState.getShip();
    const eco = this.economy.getState();
    const maxFuel = this.economy.getMaxFuel();
    const maxHealth = this.economy.getMaxHealth();

    const fuelPct = Math.round((ship.fuel / maxFuel) * 100);
    const healthPct = Math.round((ship.health / maxHealth) * 100);

    this.el.innerHTML = `
      <div class="hud-group">
        <div class="hud-item">
          <span class="hud-label">HULL</span>
          <div class="hud-bar">
            <div class="hud-bar-fill health-bar" style="width: ${healthPct}%"></div>
          </div>
          <span class="hud-value">${Math.round(ship.health)}/${maxHealth}</span>
        </div>
        <div class="hud-item">
          <span class="hud-label">FUEL</span>
          <div class="hud-bar">
            <div class="hud-bar-fill fuel-bar" style="width: ${fuelPct}%"></div>
          </div>
          <span class="hud-value">${Math.round(ship.fuel)}/${maxFuel}</span>
        </div>
      </div>
      <div class="hud-group hud-center">
        <span class="hud-title">QUANTUM KERNEL VOYAGER</span>
      </div>
      <div class="hud-group hud-right">
        <div class="hud-item">
          <span class="hud-label crystal-label">◇ CRYSTALS</span>
          <span class="hud-value crystal-value">${eco.quantumCrystals.toLocaleString()}</span>
        </div>
        <div class="hud-item">
          <span class="hud-label">WORLDS</span>
          <span class="hud-value">${this.gameState.getDiscoveredWorlds().length}</span>
        </div>
      </div>
    `;
  }
}
