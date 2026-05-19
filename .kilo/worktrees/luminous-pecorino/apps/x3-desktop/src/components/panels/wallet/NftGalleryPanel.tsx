import React, { useState } from 'react';
import { Image, TrendingUp, Settings2, Grid3x3, Search, Send } from 'lucide-react';
import clsx from 'clsx';

interface NFT {
  id: string;
  name: string;
  collection: string;
  image: string;
  floorPrice: number;
  rarity: 'common' | 'uncommon' | 'rare' | 'legendary';
  chain: string;
  contractAddress: string;
}

interface Collection {
  name: string;
  count: number;
  floorPrice: number;
  volume24h: number;
}

const MOCK_NFTS: NFT[] = [
  {
    id: '1',
    name: 'X3 Mainnet Validator #42',
    collection: 'X3 Validators',
    image: '🎖️',
    floorPrice: 150,
    rarity: 'rare',
    chain: 'X3',
    contractAddress: '0x123...abc',
  },
  {
    id: '2',
    name: 'Bold Profile #1024',
    collection: 'Bold NFT Collection',
    image: '👤',
    floorPrice: 2.5,
    rarity: 'uncommon',
    chain: 'Ethereum',
    contractAddress: '0x456...def',
  },
  {
    id: '3',
    name: 'Pudgy Penguins #5421',
    collection: 'Pudgy Penguins',
    image: '🐧',
    floorPrice: 8.5,
    rarity: 'common',
    chain: 'Ethereum',
    contractAddress: '0x789...xyz',
  },
  {
    id: '4',
    name: 'Azuki #8234',
    collection: 'Azuki',
    image: '🤖',
    floorPrice: 12.3,
    rarity: 'legendary',
    chain: 'Ethereum',
    contractAddress: '0xabc...123',
  },
  {
    id: '5',
    name: 'Magic Eden DeGods #2341',
    collection: 'DeGods',
    image: '☠️',
    floorPrice: 45,
    rarity: 'rare',
    chain: 'Solana',
    contractAddress: 'DeGods123...',
  },
  {
    id: '6',
    name: 'X Marks #999',
    collection: 'X Marks The Spot',
    image: '✖️',
    floorPrice: 3.2,
    rarity: 'uncommon',
    chain: 'X3',
    contractAddress: '0xdef...456',
  },
];

const COLLECTIONS: Collection[] = [
  { name: 'X3 Validators', count: 2, floorPrice: 150, volume24h: 8500 },
  { name: 'Azuki', count: 1, floorPrice: 12.3, volume24h: 2500000 },
  { name: 'DeGods', count: 1, floorPrice: 45, volume24h: 890000 },
  { name: 'Bold NFT Collection', count: 1, floorPrice: 2.5, volume24h: 125000 },
];

const RARITY_COLORS: Record<NFT['rarity'], string> = {
  common: 'text-gray-400 bg-gray-500/20 border-gray-500/40',
  uncommon: 'text-blue-400 bg-blue-500/20 border-blue-500/40',
  rare: 'text-purple-400 bg-purple-500/20 border-purple-500/40',
  legendary: 'text-yellow-400 bg-yellow-500/20 border-yellow-500/40',
};

const NftGalleryPanel: React.FC = () => {
  const [nfts] = useState<NFT[]>(MOCK_NFTS);
  const [collections] = useState<Collection[]>(COLLECTIONS);
  const [selectedNft, setSelectedNft] = useState<NFT | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [filterChain, setFilterChain] = useState<string>('all');
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');

  const filteredNfts = nfts.filter((nft) => {
    const matchesSearch = !searchQuery || 
      nft.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      nft.collection.toLowerCase().includes(searchQuery.toLowerCase());
    const matchesChain = filterChain === 'all' || nft.chain === filterChain;
    return matchesSearch && matchesChain;
  });

  const totalValue = nfts.reduce((sum, n) => sum + n.floorPrice, 0);
  const chains = [...new Set(nfts.map(n => n.chain))];

  return (
    <div className="h-full flex flex-col bg-[#0a0a0f] text-white overflow-auto">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[#1a1a1a]">
        <div className="flex items-center gap-3">
          <Image size={18} className="text-blue-400" />
          <h1 className="text-lg font-bold">NFT Gallery</h1>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setViewMode('grid')}
            className={clsx(
              'p-2 rounded-lg transition-colors',
              viewMode === 'grid' ? 'bg-blue-500/20 text-blue-400' : 'text-gray-500 hover:text-white'
            )}
          >
            <Grid3x3 size={16} />
          </button>
          <button
            onClick={() => setViewMode('list')}
            className={clsx(
              'p-2 rounded-lg transition-colors',
              viewMode === 'list' ? 'bg-blue-500/20 text-blue-400' : 'text-gray-500 hover:text-white'
            )}
          >
            <Settings2 size={16} />
          </button>
        </div>
      </div>

      {/* Stats */}
      <div className="grid grid-cols-4 gap-3 px-5 py-4 border-b border-[#1a1a1a]">
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Total NFTs</div>
          <div className="text-lg font-bold text-white">{nfts.length}</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Collections</div>
          <div className="text-lg font-bold text-blue-400">{collections.length}</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Floor Value</div>
          <div className="text-lg font-bold text-green-400">${totalValue.toFixed(1)}K</div>
        </div>
        <div className="bg-[#111111] rounded-xl p-3 border border-[#1a1a1a]">
          <div className="text-xs text-gray-500">Chains</div>
          <div className="text-lg font-bold text-purple-400">{chains.length}</div>
        </div>
      </div>

      {/* Filters */}
      <div className="flex gap-3 px-5 py-3 border-b border-[#1a1a1a]">
        <div className="relative flex-1">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500" />
          <input
            type="text"
            placeholder="Search NFT or collection..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full bg-[#111111] border border-[#1a1a1a] rounded-lg pl-9 pr-3 py-2 text-xs text-white placeholder-gray-600 outline-none focus:border-blue-500/40"
          />
        </div>
        <select
          value={filterChain}
          onChange={(e) => setFilterChain(e.target.value)}
          className="bg-[#111111] border border-[#1a1a1a] rounded-lg px-3 py-2 text-xs text-white outline-none focus:border-blue-500/40"
        >
          <option value="all">All Chains</option>
          {chains.map((chain) => (
            <option key={chain} value={chain}>
              {chain}
            </option>
          ))}
        </select>
      </div>

      {/* Gallery Grid */}
      <div className="flex-1 overflow-auto px-5 py-4">
        {viewMode === 'grid' ? (
          <div className="grid grid-cols-4 gap-4">
            {filteredNfts.map((nft) => (
              <div
                key={nft.id}
                onClick={() => setSelectedNft(nft)}
                className="bg-[#111111] border border-[#1a1a1a] rounded-lg overflow-hidden hover:border-[#2a2a2a] transition-colors cursor-pointer group"
              >
                <div className="aspect-square bg-gradient-to-br from-[#1a1a1a] to-[#0a0a0f] flex items-center justify-center text-4xl group-hover:scale-105 transition-transform">
                  {nft.image}
                </div>
                <div className="p-3 space-y-2">
                  <div>
                    <div className="font-semibold text-sm text-white group-hover:text-blue-400 transition-colors">{nft.name}</div>
                    <div className="text-xs text-gray-500">{nft.collection}</div>
                  </div>
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-mono text-gray-400">{nft.chain}</span>
                    <span className="text-sm font-bold text-green-400">${nft.floorPrice}</span>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ) : (
          <div className="space-y-2">
            {filteredNfts.map((nft) => (
              <div
                key={nft.id}
                onClick={() => setSelectedNft(nft)}
                className="bg-[#111111] border border-[#1a1a1a] rounded-lg p-4 hover:border-[#2a2a2a] transition-colors cursor-pointer flex items-center gap-4"
              >
                <div className="text-3xl">{nft.image}</div>
                <div className="flex-1 min-w-0">
                  <div className="font-semibold text-white truncate">{nft.name}</div>
                  <div className="text-xs text-gray-500">{nft.collection}</div>
                </div>
                <div className={clsx('px-2 py-1 rounded text-xs font-semibold border capitalize', RARITY_COLORS[nft.rarity])}>
                  {nft.rarity}
                </div>
                <div className="text-right">
                  <div className="font-bold text-green-400">${nft.floorPrice}</div>
                  <div className="text-xs text-gray-500">{nft.chain}</div>
                </div>
              </div>
            ))}
          </div>
        )}

        {filteredNfts.length === 0 && (
          <div className="text-center py-12 text-gray-500">
            <Image size={32} className="mx-auto mb-2 opacity-20" />
            <p>No NFTs match your search.</p>
          </div>
        )}
      </div>

      {/* NFT Detail Modal */}
      {selectedNft && (
        <div className="fixed inset-0 bg-black/60 flex items-center justify-center z-50">
          <div className="bg-[#111111] border border-[#2a2a2a] rounded-lg p-6 w-96 shadow-2xl max-h-96 overflow-auto">
            <div className="text-5xl mb-4 text-center">{selectedNft.image}</div>
            
            <h3 className="font-bold text-white mb-2">{selectedNft.name}</h3>
            <div className="text-sm text-gray-500 mb-4">{selectedNft.collection}</div>

            <div className="space-y-3 mb-6">
              <div className="bg-[#0a0a0f] rounded-lg p-3">
                <div className="text-xs text-gray-500 mb-1">Floor Price</div>
                <div className="text-lg font-bold text-green-400">${selectedNft.floorPrice}</div>
              </div>

              <div className="grid grid-cols-2 gap-2">
                <div className="bg-[#0a0a0f] rounded-lg p-3">
                  <div className="text-xs text-gray-500 mb-1">Rarity</div>
                  <div className={clsx('text-sm font-bold capitalize', RARITY_COLORS[selectedNft.rarity].split(' ')[0])}>
                    {selectedNft.rarity}
                  </div>
                </div>
                <div className="bg-[#0a0a0f] rounded-lg p-3">
                  <div className="text-xs text-gray-500 mb-1">Chain</div>
                  <div className="text-sm font-bold text-white">{selectedNft.chain}</div>
                </div>
              </div>

              <div className="bg-[#0a0a0f] rounded-lg p-3">
                <div className="text-xs text-gray-500 mb-1 font-mono">Contract</div>
                <div className="text-xs text-gray-400 font-mono break-all">{selectedNft.contractAddress}</div>
              </div>
            </div>

            <div className="flex gap-2 justify-between">
              <button
                onClick={() => setSelectedNft(null)}
                className="flex-1 px-4 py-2 rounded-lg bg-[#0a0a0f] border border-[#1a1a1a] text-gray-400 hover:text-white transition-colors"
              >
                Close
              </button>
              <button className="flex items-center gap-2 flex-1 px-4 py-2 rounded-lg bg-gradient-to-r from-blue-500 to-blue-600 text-white font-semibold hover:from-blue-400 hover:to-blue-500 transition-all">
                <Send size={14} /> List for Sale
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default NftGalleryPanel;

