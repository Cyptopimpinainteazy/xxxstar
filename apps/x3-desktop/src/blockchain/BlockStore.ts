import { create } from 'zustand';
import { listen } from '../ipc/tauri';

export interface BlockEntry {
  id: string;
  height: number;
  agentId: string;
  status: 'confirmed' | 'pending' | 'failed';
  timestamp: number;
  position: { x: number; z: number };
}

interface BlockStore {
  recentBlocks: BlockEntry[];
  addBlock: (block: BlockEntry) => void;
  removeBlock: (id: string) => void;
  clearBlocks: () => void;
}

/** Maximum number of recent blocks retained by the store. */
const MAX_RECENT_BLOCKS = 30;

// Subscribe to real block:new events from the Tauri backend.
// The Rust backend (main.rs start_telemetry_stream) polls chain_getBlock
// on the node and emits 'block:new' events with { number, timestamp }.
function initBlockListener(): Promise<() => void> {
  return listen<{ number: number; timestamp: number }>(
    'block:new',
    (payload) => {
      const store = useBlockStore.getState();
      // Only push a falling block cube when we have a valid block number.
      if (!payload?.number || payload.number <= 0) return;

      const block: BlockEntry = {
        id: `block-${payload.number}`,
        height: payload.number,
        agentId: 'chain', // live chain block — not agent-generated
        status: 'confirmed',
        timestamp: payload.timestamp || Date.now(),
        position: {
          x: (Math.random() - 0.5) * 4,
          z: (Math.random() - 0.5) * 4,
        },
      };

      // Deduplicate: skip if this block number is already in recent history
      const alreadySeen = store.recentBlocks.some(
        (b) => b.height === block.height
      );
      if (!alreadySeen) {
        store.addBlock(block);
      }
    }
  );
}

// Start listening on module load — no cleanup needed for app lifetime.
if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
  void initBlockListener();
}

export const useBlockStore = create<BlockStore>((set) => ({
  recentBlocks: [],

  addBlock: (block) =>
    set((state) => {
      const next = [...state.recentBlocks, block];
      return {
        // Trim to the tail so we keep exactly MAX_RECENT_BLOCKS entries.
        // Append-then-trim avoids the slice(-30) + append off-by-one of 31.
        recentBlocks:
          next.length > MAX_RECENT_BLOCKS
            ? next.slice(next.length - MAX_RECENT_BLOCKS)
            : next,
      };
    }),

  removeBlock: (id) =>
    set((state) => ({
      recentBlocks: state.recentBlocks.filter((b) => b.id !== id),
    })),

  clearBlocks: () => set({ recentBlocks: [] }),
}));