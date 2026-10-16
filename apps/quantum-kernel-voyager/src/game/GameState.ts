/**
 * @module game/GameState
 * Discriminated-union state machine for game modes.
 *
 * Modes: loading → menu → exploring ↔ warping ↔ inspecting ↔ paused
 *
 * Enforces valid transitions. Each state carries its own context data.
 */
import type { GameMode, GameStateSnapshot, VoyageState, DiscoveredWorld, ShipState, EconomyState } from "../types/game";

// -------------------------------------------------------------------------
// State context per mode
// -------------------------------------------------------------------------

interface LoadingCtx {
  progress: number;
  message: string;
}

interface MenuCtx {
  selectedSlot: number;
}

interface ExploringCtx {
  currentWorldSeed: string;
}

interface WarpingCtx {
  fromSeed: string;
  toSeed: string;
  elapsed: number;
}

interface InspectingCtx {
  entityId: string;
}

interface PausedCtx {
  previousMode: GameMode;
}

interface SolarSystemCtx {
  /** Currently empty — reserved for future state. */
  _?: undefined;
}

interface LandingCtx {
  targetPlanetId: string;
  elapsed: number;
}

interface SurfaceCtx {
  planetId: string;
  landingTarget: string;
}

type ModeContext = {
  loading: LoadingCtx;
  menu: MenuCtx;
  exploring: ExploringCtx;
  warping: WarpingCtx;
  inspecting: InspectingCtx;
  paused: PausedCtx;
  solar_system: SolarSystemCtx;
  landing: LandingCtx;
  surface: SurfaceCtx;
};

// -------------------------------------------------------------------------
// Valid transitions
// -------------------------------------------------------------------------

const VALID_TRANSITIONS: Record<GameMode, GameMode[]> = {
  loading: ["menu"],
  menu: ["loading", "exploring", "solar_system"],
  exploring: ["warping", "inspecting", "paused", "menu", "solar_system"],
  warping: ["exploring"],
  inspecting: ["exploring", "solar_system"],
  paused: ["exploring", "menu", "solar_system"],
  solar_system: ["landing", "inspecting", "paused", "menu", "exploring"],
  landing: ["surface", "solar_system"],
  surface: ["solar_system", "paused"],
};

// -------------------------------------------------------------------------
// GameState class
// -------------------------------------------------------------------------

export type GameStateChangeCallback = (prev: GameMode, next: GameMode) => void;

export class GameState {
  private mode: GameMode = "loading";
  private contexts: Partial<ModeContext> = {
    loading: { progress: 0, message: "Initializing..." },
  };
  private listeners: GameStateChangeCallback[] = [];

  // Persistent voyage-level data
  private worlds: DiscoveredWorld[] = [];
  private ship: ShipState;
  private economy: EconomyState;

  constructor() {
    this.ship = {
      position: { x: 0, y: 0, z: 0 },
      rotation: { x: 0, y: 0, z: 0 },
      health: 100,
      fuel: 100,
      upgrades: [],
    };
    this.economy = {
      quantumCrystals: 0,
      totalMined: 0,
      totalSpent: 0,
    };
  }

  // -----------------------------------------------------------------------
  // Mode management
  // -----------------------------------------------------------------------

  getMode(): GameMode {
    return this.mode;
  }

  getContext<M extends GameMode>(mode: M): ModeContext[M] | undefined {
    return this.contexts[mode] as ModeContext[M] | undefined;
  }

  /**
   * Transition to a new game mode.
   * @throws if the transition is invalid.
   */
  transition<M extends GameMode>(to: M, context: ModeContext[M]): void {
    const valid = VALID_TRANSITIONS[this.mode];
    if (!valid.includes(to)) {
      throw new Error(`Invalid transition: ${this.mode} → ${to}`);
    }
    const prev = this.mode;
    this.mode = to;
    this.contexts[to] = context;
    for (const cb of this.listeners) {
      cb(prev, to);
    }
  }

  /** Register a mode-change listener. */
  onChange(callback: GameStateChangeCallback): void {
    this.listeners.push(callback);
  }

  /** Remove a listener. */
  offChange(callback: GameStateChangeCallback): void {
    const idx = this.listeners.indexOf(callback);
    if (idx >= 0) this.listeners.splice(idx, 1);
  }

  // -----------------------------------------------------------------------
  // World management
  // -----------------------------------------------------------------------

  addWorld(world: DiscoveredWorld): void {
    if (!this.worlds.find((w) => w.seed === world.seed)) {
      this.worlds.push(world);
    }
  }

  getWorld(seed: string): DiscoveredWorld | undefined {
    return this.worlds.find((w) => w.seed === seed);
  }

  getDiscoveredWorlds(): DiscoveredWorld[] {
    return [...this.worlds];
  }

  // -----------------------------------------------------------------------
  // Ship
  // -----------------------------------------------------------------------

  getShip(): ShipState {
    return this.ship;
  }

  updateShip(patch: Partial<ShipState>): void {
    Object.assign(this.ship, patch);
  }

  /** Consume fuel. Returns false if insufficient. */
  consumeFuel(amount: number): boolean {
    if (this.ship.fuel < amount) return false;
    this.ship.fuel -= amount;
    return true;
  }

  // -----------------------------------------------------------------------
  // Economy
  // -----------------------------------------------------------------------

  getEconomy(): EconomyState {
    return this.economy;
  }

  addCrystals(amount: number): void {
    this.economy.quantumCrystals += amount;
    this.economy.totalMined += amount;
  }

  spendCrystals(amount: number): boolean {
    if (this.economy.quantumCrystals < amount) return false;
    this.economy.quantumCrystals -= amount;
    this.economy.totalSpent += amount;
    return true;
  }

  // -----------------------------------------------------------------------
  // Serialization
  // -----------------------------------------------------------------------

  /** Create a serializable snapshot for save/load. */
  toSnapshot(): GameStateSnapshot {
    return {
      mode: this.mode,
      timestamp: Date.now(),
    };
  }

  /** Export full voyage state for persistence. */
  toVoyageState(): VoyageState {
    return {
      schemaVersion: 1,
      savedAt: Date.now(),
      ship: { ...this.ship },
      worlds: this.worlds.map((w) => ({ ...w })),
      economy: { ...this.economy },
    };
  }

  /** Restore from a saved voyage state. */
  loadVoyageState(state: VoyageState): void {
    if (state.schemaVersion !== 1) {
      throw new Error(`Unsupported schema version: ${state.schemaVersion}`);
    }
    this.ship = { ...state.ship };
    this.worlds = state.worlds.map((w) => ({ ...w }));
    this.economy = { ...state.economy };
  }
}
