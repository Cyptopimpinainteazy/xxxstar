/**
 * @module ui/LandingPrompt
 * Shows "Press F to Land on <Planet>" when the ship is within proximity of a planet.
 * Fades in/out smoothly. Shows planet type icon and basic stats.
 */
import type { PlanetEntity } from "../types/solar-system";

export class LandingPrompt {
  private el: HTMLElement;
  private visible = false;
  private currentPlanet: PlanetEntity | null = null;

  constructor() {
    this.el = document.createElement("div");
    this.el.id = "landing-prompt";
    this.el.className = "landing-prompt hidden";
    this.el.innerHTML = `
      <div class="lp-icon"></div>
      <div class="lp-text">
        <div class="lp-action">Press <kbd>F</kbd> to Land</div>
        <div class="lp-name"></div>
        <div class="lp-type"></div>
      </div>
    `;
  }

  mount(parent: HTMLElement = document.body): void {
    parent.appendChild(this.el);
  }

  unmount(): void {
    this.el.remove();
  }

  /** Show the prompt for a specific planet. */
  show(planet: PlanetEntity): void {
    if (this.visible && this.currentPlanet?.id === planet.id) return;
    this.currentPlanet = planet;
    this.visible = true;

    const nameEl = this.el.querySelector(".lp-name") as HTMLElement;
    const typeEl = this.el.querySelector(".lp-type") as HTMLElement;
    const iconEl = this.el.querySelector(".lp-icon") as HTMLElement;

    nameEl.textContent = planet.name;
    typeEl.textContent = `${planet.category} · ${capitalize(planet.type)}`;

    // Icon color
    const hex = "#" + planet.color.toString(16).padStart(6, "0");
    iconEl.style.background = hex;
    iconEl.style.boxShadow = `0 0 16px ${hex}80`;

    this.el.classList.remove("hidden");
  }

  /** Hide the prompt. */
  hide(): void {
    if (!this.visible) return;
    this.visible = false;
    this.currentPlanet = null;
    this.el.classList.add("hidden");
  }

  isVisible(): boolean {
    return this.visible;
  }

  getCurrentPlanet(): PlanetEntity | null {
    return this.currentPlanet;
  }
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
