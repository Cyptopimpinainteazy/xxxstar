import { create } from 'zustand';

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