/**
 * @module ipc/events
 * Typed event bus for cross-system communication.
 *
 * Event channels:
 * - chain:block        — New block produced
 * - chain:event        — Chain event (tx, slash, etc.)
 * - chain:status       — Chain connection status change
 * - sync:progress      — Sync queue progress
 * - game:mode-change   — Game mode transition
 * - game:crystal-earn  — Crystal economy event
 * - scene:entity-click — Entity selected in 3D scene
 * - notification:show  — Push notification to UI
 */

export type EventChannel =
  | "chain:block"
  | "chain:event"
  | "chain:status"
  | "sync:progress"
  | "game:mode-change"
  | "game:crystal-earn"
  | "scene:entity-click"
  | "notification:show";

export interface ChainBlockEvent {
  chainId: string;
  height: number;
  hash: string;
  txCount: number;
}

export interface ChainStatusEvent {
  chainId: string;
  connected: boolean;
  latencyMs: number;
}

export interface SyncProgressEvent {
  pending: number;
  submitted: number;
  failed: number;
}

export interface GameModeEvent {
  from: string;
  to: string;
}

export interface CrystalEarnEvent {
  amount: number;
  source: "mining" | "block_reward" | "sale";
}

export interface EntityClickEvent {
  entityId: string;
  kind: string;
  screenX: number;
  screenY: number;
}

export interface NotificationEvent {
  title: string;
  message: string;
  variant: "info" | "success" | "warning" | "error";
  duration?: number;
}

type EventDataMap = {
  "chain:block": ChainBlockEvent;
  "chain:event": Record<string, unknown>;
  "chain:status": ChainStatusEvent;
  "sync:progress": SyncProgressEvent;
  "game:mode-change": GameModeEvent;
  "game:crystal-earn": CrystalEarnEvent;
  "scene:entity-click": EntityClickEvent;
  "notification:show": NotificationEvent;
};

type EventCallback<C extends EventChannel> = (data: EventDataMap[C]) => void;

interface Subscription {
  channel: EventChannel;
  callback: EventCallback<EventChannel>;
  id: string;
}

/**
 * Typed pub/sub event bus.
 *
 * @example
 * ```ts
 * const bus = new EventBus();
 * bus.on("chain:block", (data) => console.log(data.height));
 * bus.emit("chain:block", { chainId: "local", height: 42, hash: "0x...", txCount: 5 });
 * ```
 */
export class EventBus {
  private readonly subscriptions: Subscription[] = [];

  /** Subscribe to an event channel. Returns an unsubscribe ID. */
  on<C extends EventChannel>(channel: C, callback: EventCallback<C>): string {
    const id = `evt-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.subscriptions.push({
      channel,
      callback: callback as EventCallback<EventChannel>,
      id,
    });
    return id;
  }

  /** Subscribe to an event channel for a single emission. */
  once<C extends EventChannel>(channel: C, callback: EventCallback<C>): string {
    const id = this.on(channel, (data) => {
      callback(data as EventDataMap[C]);
      this.off(id);
    });
    return id;
  }

  /** Unsubscribe by ID. */
  off(id: string): void {
    const idx = this.subscriptions.findIndex((s) => s.id === id);
    if (idx >= 0) this.subscriptions.splice(idx, 1);
  }

  /** Unsubscribe all listeners for a channel. */
  offAll(channel: EventChannel): void {
    for (let i = this.subscriptions.length - 1; i >= 0; i--) {
      if (this.subscriptions[i].channel === channel) {
        this.subscriptions.splice(i, 1);
      }
    }
  }

  /** Emit an event to all subscribers of the channel. */
  emit<C extends EventChannel>(channel: C, data: EventDataMap[C]): void {
    for (const sub of this.subscriptions) {
      if (sub.channel === channel) {
        try {
          sub.callback(data);
        } catch (err) {
          console.error(`[EventBus] Error in ${channel} handler:`, err);
        }
      }
    }
  }

  /** Remove all subscriptions. */
  dispose(): void {
    this.subscriptions.length = 0;
  }
}

/** Singleton event bus instance for the application. */
export const eventBus = new EventBus();
