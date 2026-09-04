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

      // Deduplicate: skip if this block number is already in the last 30
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
    set((state) => ({
      recentBlocks: [...state.recentBlocks.slice(-30), block], // keep last 30
    })),

  removeBlock: (id) =>
    set((state) => ({
      recentBlocks: state.recentBlocks.filter((b) => b.id !== id),
    })),

  clearBlocks: () => set({ recentBlocks: [] }),
}));