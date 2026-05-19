/**
 * EventEmitter — minimal typed event emitter for inter-component communication.
 */

type Handler<T = unknown> = (payload: T) => void;

export class EventEmitter {
  private listeners = new Map<string, Set<Handler>>();

  /**
   * Subscribe to an event.
   * @returns An unsubscribe function.
   */
  on<T = unknown>(event: string, handler: Handler<T>): () => void {
    if (!this.listeners.has(event)) {
      this.listeners.set(event, new Set());
    }
    const set = this.listeners.get(event)!;
    set.add(handler as Handler);

    return () => {
      set.delete(handler as Handler);
      if (set.size === 0) this.listeners.delete(event);
    };
  }

  /**
   * Subscribe to an event for a single invocation.
   */
  once<T = unknown>(event: string, handler: Handler<T>): () => void {
    const wrapper: Handler<T> = (payload) => {
      unsub();
      handler(payload);
    };
    const unsub = this.on(event, wrapper);
    return unsub;
  }

  /**
   * Emit an event to all subscribers.
   */
  emit<T = unknown>(event: string, payload: T): void {
    const set = this.listeners.get(event);
    if (!set) return;
    for (const handler of set) {
      try {
        (handler as Handler<T>)(payload);
      } catch (err) {
        console.error(`[EventEmitter] Error in handler for "${event}":`, err);
      }
    }
  }

  /**
   * Remove all listeners for an event (or all events if none specified).
   */
  clear(event?: string): void {
    if (event) {
      this.listeners.delete(event);
    } else {
      this.listeners.clear();
    }
  }
}

/** Global application event bus */
export const appBus = new EventEmitter();
