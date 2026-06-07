import React, { useState } from 'react';
import { Wallet, Shield, Users, BarChart3, Lock, TrendingUp } from 'lucide-react';

interface WalletLink {
  id: string;
  walletAddress: string;
  linkedContact: string;
  nftBalance: number;
  linkedDate: string;
  verificationStatus: 'verified' | 'pending' | 'failed';
  linkedNfts: number;
}

interface OnChainDeal {
  id: string;
  dealName: string;
  nftContractAddress: string;
  tokenRequirement: number;
  dealStatus: 'active' | 'completed' | 'paused';
  dealValue: number;
  nftHolders: number;
  claimable: boolean;
}

interface TokenGatedGroup {
  id: string;
  groupName: string;
  nftCollection: string;
  minimumTokens: number;
  members: number;
  memberNfts: number;
  visibility: 'public' | 'private';
  adminWallet: string;
  totalValue: number;
}

interface NftMetadata {
  tokenId: string;
  contractAddress: string;
  collectionName: string;
  rarity: string;
  floorPrice: number;
  ownerCount: number;
  royaltyBps: number;
}

interface CrmNftMetrics {
  totalContactsLinked: number;
  totalNftValue: number;
  activeGatedGroups: number;
  onChainDealsCount: number;
  avgNftPerContact: number;
}

export const NFTCRMIntegrationPanel: React.FC = () => {
  const [activeTab, setActiveTab] = useState<'wallets' | 'onchain' | 'groups' | 'metrics'>(
    'wallets'
  );

  const [walletLinks] = useState<WalletLink[]>([
    {
      id: 'link-1',
      walletAddress: '0x742d35Cc6634C0532925a3b844Bc9e7595f45e5',
      linkedContact: 'Alice Johnson (TechCorp)',
      nftBalance: 8,
      linkedDate: '2024-01-15',
      verificationStatus: 'verified',
      linkedNfts: 8,
    },
    {
      id: 'link-2',
      walletAddress: '0xE3a5B7D5F8C1B3d2E4a9C5F2D8e0A1b3C5D7E9',
      linkedContact: 'Bob Smith (FinServices)',
      nftBalance: 3,
      linkedDate: '2024-02-08',
      verificationStatus: 'verified',
      linkedNfts: 3,
    },
    {
      id: 'link-3',
      walletAddress: '0x8F2S9K4J2L5P8Q1R4T7U0V3W6X9Y2Z5A8B1C4D7',
      linkedContact: 'Carol Davis (DevStudio)',
      nftBalance: 12,
      linkedDate: '2024-02-20',
      verificationStatus: 'verified',
      linkedNfts: 12,
    },
    {
      id: 'link-4',
      walletAddress: '0x5M8N2P0Q3R6S9T2U5V8W1X4Y7Z0A3B6C9D2E5F',
      linkedContact: 'Enterprise Partner Alpha',
      nftBalance: 0,
      linkedDate: '2024-02-25',
      verificationStatus: 'pending',
      linkedNfts: 0,
    },
  ]);

  const [onChainDeals] = useState<OnChainDeal[]>>[
    {
      id: 'deal-1',
      dealName: 'Enterprise Holder Benefits Program',
      nftContractAddress: '0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D',
      tokenRequirement: 5,
      dealStatus: 'active',
      dealValue: 450000,
      nftHolders: 245,
      claimable: true,
    },
    {
      id: 'deal-2',
      dealName: 'Exclusive Early Access Token Drop',
      nftContractAddress: '0x60E4d786d1Ad0075BCace08D4ca0418Ff6362456',
      tokenRequirement: 2,
      dealStatus: 'active',
      dealValue: 85000,
      nftHolders: 128,
      claimable: true,
    },
    {
      id: 'deal-3',
      dealName: 'Premium Validator Rewards',
      nftContractAddress: '0x49Cf6f5d44e70224e2E23fDcdd2C053f30313270',
      tokenRequirement: 10,
      dealStatus: 'completed',
      dealValue: 250000,
      nftHolders: 89,
      claimable: false,
    },
  ]);

  const [tokenGatedGroups] = useState<TokenGatedGroup[]>([
    {
      id: 'group-1',
      groupName: 'Whale Traders Club',
      nftCollection: 'X3 Founders Pass',
      minimumTokens: 5,
      members: 87,
      memberNfts: 432,
      visibility: 'private',
      adminWallet: '0x742d35Cc6634C0532925a3b844Bc9e7595f45e5',
      totalValue: 2840000,
    },
    {
      id: 'group-2',
      groupName: 'Enterprise Partners',
      nftCollection: 'Corporate Badges',
      minimumTokens: 1,
      members: 34,
      memberNfts: 78,
      visibility: 'private',
      adminWallet: '0xE3a5B7D5F8C1B3d2E4a9C5F2D8e0A1b3C5D7E9',
      totalValue: 450000,
    },
    {
      id: 'group-3',
      groupName: 'Community Builders',
      nftCollection: 'Contributor Badges',
      minimumTokens: 1,
      members: 256,
      memberNfts: 512,
      visibility: 'public',
      adminWallet: '0x8F2S9K4J2L5P8Q1R4T7U0V3W6X9Y2Z5A8B1C4D7',
      totalValue: 1250000,
    },
  ]);

  const [nftMetadata] = useState<NftMetadata[]>([
    {
      tokenId: '#1234',
      contractAddress: '0xBC4CA0EdA7647A8aB7C2061c2E118A18a936f13D',
      collectionName: 'X3 Founders Pass',
      rarity: 'Rare',
      floorPrice: 85.5,
      ownerCount: 245,
      royaltyBps: 500,
    },
    {
      tokenId: '#5678',
      contractAddress: '0x60E4d786d1Ad0075BCace08D4ca0418Ff6362456',
      collectionName: 'Corporate Badges',
      rarity: 'Uncommon',
      floorPrice: 12.3,
      ownerCount: 128,
      royaltyBps: 250,
    },
  ]);

  const metrics: CrmNftMetrics = {
    totalContactsLinked: 4,
    totalNftValue: 4540000,
    activeGatedGroups: 3,
    onChainDealsCount: 3,
    avgNftPerContact: 5.75,
  };

  const totalLinkedNfts = walletLinks.reduce((sum, w) => sum + w.linkedNfts, 0);

  return (
    <div className="min-h-screen bg-gradient-to-br from-[#0a0a0f] via-[#1a1a2e] to-[#0a0a0f] p-6">
      <div className="max-w-7xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-4xl font-bold text-transparent bg-clip-text bg-gradient-to-r from-purple-400 to-pink-500 mb-2">
              NFT-CRM Integration
            </h1>
            <p className="text-gray-400">Wallet Linking • On-Chain Deals • Token-Gated Groups</p>
          </div>
          <Wallet className="w-12 h-12 text-purple-400" />
        </div>

        {/* KPI Grid */}
        <div className="grid grid-cols-4 gap-4 mb-8">
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Contacts Linked</div>
            <div className="text-2xl font-bold text-purple-400">{metrics.totalContactsLinked}</div>
            <div className="text-xs text-gray-500 mt-2">Verified wallet connections</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Portfolio Value</div>
            <div className="text-2xl font-bold text-pink-400">${(metrics.totalNftValue / 1000000).toFixed(1)}M</div>
            <div className="text-xs text-gray-500 mt-2">Total NFT value holdings</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">Gated Groups</div>
            <div className="text-2xl font-bold text-blue-400">{metrics.activeGatedGroups}</div>
            <div className="text-xs text-gray-500 mt-2">Token-gated communities</div>
          </div>
          <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-4">
            <div className="text-gray-400 text-sm mb-2">NFTs Linked</div>
            <div className="text-2xl font-bold text-teal-400">{totalLinkedNfts}</div>
            <div className="text-xs text-gray-500 mt-2">Across all wallets</div>
          </div>
        </div>

        {/* Tabs */}
        <div className="flex gap-2 mb-6 border-b border-[#2a2a35]">
          {(['wallets', 'onchain', 'groups', 'metrics'] as const).map((tab) => (
            <button
              key={tab}
              onClick={() => setActiveTab(tab)}
              className={`px-4 py-3 font-semibold transition-colors ${
                activeTab === tab
                  ? 'text-purple-400 border-b-2 border-purple-400'
                  : 'text-gray-400 hover:text-gray-300'
              }`}
            >
              {tab === 'wallets' && 'Wallet Linking'}
              {tab === 'onchain' && 'On-Chain Deals'}
              {tab === 'groups' && 'Token-Gated Groups'}
              {tab === 'metrics' && 'NFT Metrics'}
            </button>
          ))}
        </div>

        {/* Content */}
        <div className="bg-[#1a1a2e] border border-[#2a2a35] rounded-lg p-6">
          {activeTab === 'wallets' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between mb-6">
                <h3 className="text-lg font-semibold text-white">Linked Wallets</h3>
                <button className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg font-semibold transition">
                  + Link Wallet
                </button>
              </div>
              {walletLinks.map((wallet) => (
                <div key={wallet.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h4 className="text-white font-semibold">{wallet.linkedContact}</h4>
                      <p className="text-xs text-gray-500 font-mono mt-1">{wallet.walletAddress}</p>
                    </div>
                    <div
                      className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        wallet.verificationStatus === 'verified'
                          ? 'bg-green-500/20 text-green-400'
                          : wallet.verificationStatus === 'pending'
                          ? 'bg-yellow-500/20 text-yellow-400'
                          : 'bg-red-500/20 text-red-400'
                      }`}
                    >
                      {wallet.verificationStatus.toUpperCase()}
                    </div>
                  </div>
                  <div className="grid grid-cols-5 gap-4 text-sm">
                    <div>
                      <div className="text-gray-400 text-xs">NFT Balance</div>
                      <div className="text-white font-semibold">{wallet.nftBalance}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Linked NFTs</div>
                      <div className="text-purple-400 font-semibold">{wallet.linkedNfts}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Linked Date</div>
                      <div className="text-white font-semibold">{wallet.linkedDate}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Status</div>
                      <div className="w-2 h-2 rounded-full bg-green-400 mt-2"></div>
                    </div>
                    <div className="flex gap-2">
                      <button className="px-2 py-1 text-xs bg-[#2a2a35] hover:bg-[#3a3a45] text-gray-400 rounded transition">
                        View NFTs
                      </button>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'onchain' && (
            <div className="space-y-4">
              <h3 className="text-lg font-semibold text-white mb-6">On-Chain Deals</h3>
              {onChainDeals.map((deal) => (
                <div key={deal.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h4 className="text-white font-semibold">{deal.dealName}</h4>
                      <p className="text-xs text-gray-500 font-mono mt-1">{deal.nftContractAddress}</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <span
                        className={`px-3 py-1 rounded-full text-xs font-semibold ${
                          deal.dealStatus === 'active'
                            ? 'bg-green-500/20 text-green-400'
                            : deal.dealStatus === 'completed'
                            ? 'bg-blue-500/20 text-blue-400'
                            : 'bg-gray-500/20 text-gray-400'
                        }`}
                      >
                        {deal.dealStatus.toUpperCase()}
                      </span>
                      {deal.claimable && (
                        <button className="px-3 py-1 bg-teal-600 hover:bg-teal-700 text-white text-xs rounded font-semibold transition">
                          Claim
                        </button>
                      )}
                    </div>
                  </div>
                  <div className="grid grid-cols-5 gap-4 text-sm">
                    <div>
                      <div className="text-gray-400 text-xs">Min. Required</div>
                      <div className="text-white font-semibold">{deal.tokenRequirement} NFT</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Deal Value</div>
                      <div className="text-pink-400 font-semibold">${(deal.dealValue / 1000).toFixed(0)}K</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Eligible Holders</div>
                      <div className="text-white font-semibold">{deal.nftHolders}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Claimable</div>
                      <div className={deal.claimable ? 'text-green-400 font-semibold' : 'text-gray-500 font-semibold'}>
                        {deal.claimable ? 'Yes' : 'No'}
                      </div>
                    </div>
                    <div>
                      <Shield className="w-5 h-5 text-blue-400" />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'groups' && (
            <div className="space-y-4">
              <div className="flex items-center justify-between mb-6">
                <h3 className="text-lg font-semibold text-white">Token-Gated Groups</h3>
                <button className="px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white rounded-lg font-semibold transition">
                  + Create Group
                </button>
              </div>
              {tokenGatedGroups.map((group) => (
                <div key={group.id} className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="flex items-start justify-between mb-3">
                    <div>
                      <h4 className="text-white font-semibold">{group.groupName}</h4>
                      <p className="text-sm text-gray-400">Collection: {group.nftCollection}</p>
                    </div>
                    <span
                      className={`px-3 py-1 rounded-full text-xs font-semibold ${
                        group.visibility === 'public'
                          ? 'bg-blue-500/20 text-blue-400'
                          : 'bg-purple-500/20 text-purple-400'
                      }`}
                    >
                      {group.visibility.toUpperCase()}
                    </span>
                  </div>
                  <div className="grid grid-cols-6 gap-3 text-sm">
                    <div>
                      <div className="text-gray-400 text-xs">Min. Tokens</div>
                      <div className="text-white font-semibold">{group.minimumTokens}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Members</div>
                      <div className="text-teal-400 font-semibold">{group.members}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Member NFTs</div>
                      <div className="text-purple-400 font-semibold">{group.memberNfts}</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Total Value</div>
                      <div className="text-pink-400 font-semibold">${(group.totalValue / 1000000).toFixed(1)}M</div>
                    </div>
                    <div>
                      <div className="text-gray-400 text-xs">Admin</div>
                      <div className="text-white font-mono text-xs truncate">{group.adminWallet.slice(0, 8)}...</div>
                    </div>
                    <div>
                      <Users className="w-5 h-5 text-teal-400" />
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}

          {activeTab === 'metrics' && (
            <div>
              <h3 className="text-lg font-semibold text-white mb-6">NFT Portfolio Metrics</h3>
              <div className="grid grid-cols-2 gap-4 mb-6">
                <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                  <div className="text-gray-400 text-sm mb-3">Portfolio Composition</div>
                  <div className="space-y-2">
                    {nftMetadata.map((nft) => (
                      <div key={nft.tokenId} className="flex items-center justify-between p-2 bg-[#0a0a0f] rounded">
                        <div>
                          <div className="text-white text-sm font-semibold">{nft.collectionName}</div>
                          <div className="text-gray-500 text-xs">{nft.rarity} • {nft.ownerCount} owners</div>
                        </div>
                        <div className="text-right">
                          <div className="text-pink-400 font-semibold">${nft.floorPrice}</div>
                          <div className="text-gray-500 text-xs">{(nft.royaltyBps / 100).toFixed(1)}% royalty</div>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
                <div className="space-y-3">
                  <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="text-gray-400 text-xs mb-1">Total Portfolio Value</div>
                    <div className="text-3xl font-bold text-pink-400">${(metrics.totalNftValue / 1000000).toFixed(1)}M</div>
                  </div>
                  <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="text-gray-400 text-xs mb-1">Avg. NFTs per Contact</div>
                    <div className="text-3xl font-bold text-purple-400">{metrics.avgNftPerContact.toFixed(2)}</div>
                  </div>
                  <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-4">
                    <div className="text-gray-400 text-xs mb-1">On-Chain Deals Active</div>
                    <div className="text-3xl font-bold text-teal-400">{metrics.onChainDealsCount}</div>
                  </div>
                </div>
              </div>
              <div className="bg-[#0a0a0f] border border-[#2a2a35] rounded-lg p-6">
                <h4 className="text-white font-semibold mb-4">Group Membership Distribution</h4>
                <div className="space-y-3">
                  {tokenGatedGroups.map((group) => (
                    <div key={group.id}>
                      <div className="flex justify-between text-sm mb-2">
                        <span className="text-gray-400">{group.groupName}</span>
                        <span className="text-white font-semibold">{group.members} members</span>
                      </div>
                      <div className="w-full bg-[#2a2a35] rounded-full h-3">
                        <div
                          className="bg-gradient-to-r from-purple-500 to-pink-500 h-3 rounded-full"
                          style={{ width: `${(group.members / 400) * 100}%` }}
                        />
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export default NFTCRMIntegrationPanel;
