/**
 * @module ui/Overlay
 * Root overlay controller that owns all UI panels.
 *
 * Manages DOM mounting, theme, and coordinates sub-panels.
 * All UI is pure DOM manipulation — no framework dependency.
 */
import { TopBar } from "./TopBar";
import { SidePanel } from "./SidePanel";
import { ModalSystem } from "./ModalSystem";
import { ContextMenu } from "./ContextMenu";
import type { GameState } from "../game/GameState";
import type { EconomyManager } from "../game/EconomyManager";
import type { SceneManager } from "../scene/SceneManager";

export class Overlay {
  readonly topBar: TopBar;
  readonly sidePanel: SidePanel;
  readonly modal: ModalSystem;
  readonly contextMenu: ContextMenu;

  private readonly root: HTMLElement;
  private readonly bottomBar: HTMLElement;
  private updateTimer: ReturnType<typeof setInterval> | null = null;

  constructor(
    private readonly gameState: GameState,
    economy: EconomyManager,
    private readonly sceneManager: SceneManager,
  ) {
    this.root = document.getElementById("overlay-container")!;

    // Top bar
    const topBarEl = document.getElementById("top-bar")!;
    this.topBar = new TopBar(topBarEl, gameState, economy);

    // Side panel
    const sidePanelEl = document.getElementById("side-panel")!;
    this.sidePanel = new SidePanel(sidePanelEl, gameState);

    // Bottom bar
    this.bottomBar = document.getElementById("bottom-bar")!;
    this.setupBottomBar();

    // Modal
    const modalRoot = document.getElementById("modal-root")!;
    this.modal = new ModalSystem(modalRoot);

    // Context menu
    const ctxRoot = document.getElementById("context-menu-root")!;
    this.contextMenu = new ContextMenu(ctxRoot);
  }

  /** Start periodic UI updates. */
  start(): void {
    if (this.updateTimer) return;
    this.updateTimer = setInterval(() => this.refresh(), 250);
    this.refresh();
  }

  /** Stop updates. */
  stop(): void {
    if (this.updateTimer) {
      clearInterval(this.updateTimer);
      this.updateTimer = null;
    }
  }

  /** Force a full refresh of all panels. */
  refresh(): void {
    this.topBar.update();
    this.sidePanel.update();
    this.updateBottomBar();
  }

  /** Show or hide the entire overlay. */
  setVisible(visible: boolean): void {
    this.root.style.display = visible ? "" : "none";
  }

  dispose(): void {
    this.stop();
    this.contextMenu.dispose();
    this.modal.dispose();
  }

  // -----------------------------------------------------------------------
  // Bottom bar — render stats + controls
  // -----------------------------------------------------------------------

  private setupBottomBar(): void {
    this.bottomBar.innerHTML = `
      <div class="bottom-bar-section" id="bb-stats">
        <span id="bb-fps">-- FPS</span>
        <span id="bb-draws">0 draws</span>
        <span id="bb-tris">0 tris</span>
        <span id="bb-entities">0 entities</span>
      </div>
      <div class="bottom-bar-section" id="bb-mode">
        <span id="bb-gamemode">LOADING</span>
      </div>
      <div class="bottom-bar-section" id="bb-controls">
        <span class="key-hint">WASD</span> Move
        <span class="key-hint">F</span> Camera
        <span class="key-hint">Shift</span> Boost
        <span class="key-hint">Esc</span> Pause
      </div>
    `;
  }

  private updateBottomBar(): void {
    const stats = this.sceneManager.getRenderStats();
    const fps = document.getElementById("bb-fps");
    const draws = document.getElementById("bb-draws");
    const tris = document.getElementById("bb-tris");
    const entities = document.getElementById("bb-entities");
    const mode = document.getElementById("bb-gamemode");

    if (fps) fps.textContent = `${stats.fps} FPS`;
    if (draws) draws.textContent = `${stats.drawCalls} draws`;
    if (tris) tris.textContent = `${(stats.triangles / 1000).toFixed(1)}k tris`;
    if (entities) entities.textContent = `${stats.entities} entities`;
    if (mode) mode.textContent = this.gameState.getMode().toUpperCase();
  }
}
