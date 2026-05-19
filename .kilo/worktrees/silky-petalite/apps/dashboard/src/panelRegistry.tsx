import React, { lazy } from 'react';

// Sprint 13 Phase 1: Infrastructure & Optimization (Panels 1-12)
const ChainCoreOptimizationPanel = lazy(() => import('./panels/infrastructure/ChainCoreOptimizationPanel'));
const DynamicFeeMarketPanel = lazy(() => import('./panels/infrastructure/DynamicFeeMarketPanel'));
const CrossChainBridgePanel = lazy(() => import('./panels/infrastructure/CrossChainBridgePanel'));
const SolanaAdapterPanel = lazy(() => import('./panels/adapters/SolanaAdapterPanel'));
const CRMDatabasePanel = lazy(() => import('./panels/backend/CRMDatabasePanel'));
const SocialBackendPanel = lazy(() => import('./panels/backend/SocialBackendPanel'));
const AgentMarketplacePanel = lazy(() => import('./panels/agents/AgentMarketplacePanel'));
const ValidatorAutomationPanel = lazy(() => import('./panels/tools/ValidatorAutomationPanel'));
const TerminalShellPanel = lazy(() => import('./panels/tools/TerminalShellPanel'));
const PriceOraclePanel = lazy(() => import('./panels/integration/PriceOraclePanel'));
const WebWorkerOptimizationPanel = lazy(() => import('./panels/performance/WebWorkerOptimizationPanel'));
const NFTCRMIntegrationPanel = lazy(() => import('./panels/integration/NFTCRMIntegrationPanel'));

// Sprint 13 Phase 2: Developer Experience & Analytics (Panels 13-27)
const DeveloperPlaygroundPanel = lazy(() => import('./panels/docs/DeveloperPlaygroundPanel'));
const ValidatorHealthPanel = lazy(() => import('./panels/docs/ValidatorHealthPanel'));
const AudioVisualizerPanel = lazy(() => import('./panels/docs/AudioVisualizerPanel'));
const CryptoKeyManagementPanel = lazy(() => import('./panels/docs/CryptoKeyManagementPanel'));
const CommunicationCenterPanel = lazy(() => import('./panels/docs/CommunicationCenterPanel'));
const PortfolioAnalysisPanel = lazy(() => import('./panels/docs/PortfolioAnalysisPanel'));
const DatastoreManagementPanel = lazy(() => import('./panels/docs/DatastoreManagementPanel'));
const GeoLocationPanel = lazy(() => import('./panels/docs/GeoLocationPanel'));
const SessionSecurityPanel = lazy(() => import('./panels/docs/SessionSecurityPanel'));
const PerformanceMonitorPanel = lazy(() => import('./panels/docs/PerformanceMonitorPanel'));
const DocumentationLibraryPanel = lazy(() => import('./panels/docs/DocumentationLibraryPanel'));
const SettingsPanel = lazy(() => import('./panels/docs/SettingsPanel'));
const GamificationAndAchievementsPanel = lazy(() => import('./panels/docs/GamificationAndAchievementsPanel'));
const FAQSupportPanel = lazy(() => import('./panels/docs/FAQSupportPanel'));
const AnalyticsReportingPanel = lazy(() => import('./panels/docs/AnalyticsReportingPanel'));
const NftMarketplacePanel = lazy(() => import('./panels/docs/NftMarketplacePanel'));
const AdvancedPortfolioAnalyticsPanel = lazy(() => import('./panels/docs/AdvancedPortfolioAnalyticsPanel'));
const PrivacyVaultPanel = lazy(() => import('./panels/docs/PrivacyVaultPanel'));
const TokenMarketplacePanel = lazy(() => import('./panels/docs/TokenMarketplacePanel'));
const GovernanceProposalsPanel = lazy(() => import('./panels/docs/GovernanceProposalsPanel'));
const TreasuryManagementPanel = lazy(() => import('./panels/docs/TreasuryManagementPanel'));
const IntegrationMarketplacePanel = lazy(() => import('./panels/docs/IntegrationMarketplacePanel'));
const MediaStreamingPanel = lazy(() => import('./panels/docs/MediaStreamingPanel'));
const QuantumSecurityPanel = lazy(() => import('./panels/docs/QuantumSecurityPanel'));
const OnChainAnalyticsPanel = lazy(() => import('./panels/docs/OnChainAnalyticsPanel'));

export const panelRegistry = {
  // Infrastructure Panels (GPU, Fees, Bridges)
  'gpu-pooling': ChainCoreOptimizationPanel,
  'gpu': ChainCoreOptimizationPanel,
  'gpu-optimization': ChainCoreOptimizationPanel,
  'chain-core-optimization': ChainCoreOptimizationPanel,
  'multi-device-gpu': ChainCoreOptimizationPanel,
  'memory-pool': ChainCoreOptimizationPanel,
  'fallback-chain': ChainCoreOptimizationPanel,
  'kernel-versioning': ChainCoreOptimizationPanel,
  'benchmark-attestation': ChainCoreOptimizationPanel,

  'dynamic-fees': DynamicFeeMarketPanel,
  'eip-1559': DynamicFeeMarketPanel,
  'mev-protection': DynamicFeeMarketPanel,
  'slashing-insurance': DynamicFeeMarketPanel,
  'validator-commission': DynamicFeeMarketPanel,
  'fee-market': DynamicFeeMarketPanel,
  'commit-reveal': DynamicFeeMarketPanel,
  'threshold-encryption': DynamicFeeMarketPanel,
  'dark-pool': DynamicFeeMarketPanel,
  'commission-caps': DynamicFeeMarketPanel,

  'cross-chain': CrossChainBridgePanel,
  'bridges': CrossChainBridgePanel,
  'ethereum-bridge': CrossChainBridgePanel,
  'solana-wormhole': CrossChainBridgePanel,
  'cosmos-ibc': CrossChainBridgePanel,
  'bitcoin-htlc': CrossChainBridgePanel,
  'security-council': CrossChainBridgePanel,
  'multisig': CrossChainBridgePanel,
  'liquidity-bridge': CrossChainBridgePanel,

  // Adapter Panels (Solana)
  'solana': SolanaAdapterPanel,
  'solana-programs': SolanaAdapterPanel,
  'spl-token': SolanaAdapterPanel,
  'anchor': SolanaAdapterPanel,
  'token-program': SolanaAdapterPanel,
  'assoc-token': SolanaAdapterPanel,
  'uniswap-v3': SolanaAdapterPanel,
  'aave-v3': SolanaAdapterPanel,
  'pyth-oracle': SolanaAdapterPanel,

  // Backend Panels (CRM, Social)
  'crm': CRMDatabasePanel,
  'crm-database': CRMDatabasePanel,
  'contacts': CRMDatabasePanel,
  'deals': CRMDatabasePanel,
  'sales-pipeline': CRMDatabasePanel,
  'email-campaigns': CRMDatabasePanel,
  'import-export': CRMDatabasePanel,
  'smtp': CRMDatabasePanel,
  'pipeline-tracking': CRMDatabasePanel,

  'social': SocialBackendPanel,
  'activitypub': SocialBackendPanel,
  'federation': SocialBackendPanel,
  'e2e-encryption': SocialBackendPanel,
  'ipfs-media': SocialBackendPanel,
  'communities': SocialBackendPanel,
  'x3dh': SocialBackendPanel,
  'chacha20': SocialBackendPanel,
  'messaging': SocialBackendPanel,

  // Agent Panels (Marketplace)
  'agents': AgentMarketplacePanel,
  'agent-marketplace': AgentMarketplacePanel,
  'agent-trading': AgentMarketplacePanel,
  'sandboxing': AgentMarketplacePanel,
  'security-audits': AgentMarketplacePanel,
  'multi-agent': AgentMarketplacePanel,
  'agent-coordination': AgentMarketplacePanel,
  'hierarchical-agents': AgentMarketplacePanel,
  'sequential-agents': AgentMarketplacePanel,

  // Tools Panels (Validators, Terminal)
  'validators': ValidatorAutomationPanel,
  'validator-automation': ValidatorAutomationPanel,
  'one-click-setup': ValidatorAutomationPanel,
  'slashing-alerts': ValidatorAutomationPanel,
  'auto-compound': ValidatorAutomationPanel,
  'staking': ValidatorAutomationPanel,
  'network-health': ValidatorAutomationPanel,
  'validator-metrics': ValidatorAutomationPanel,
  'delegation': ValidatorAutomationPanel,

  'terminal': TerminalShellPanel,
  'shell': TerminalShellPanel,
  'pty-terminal': TerminalShellPanel,
  'x3-cli': TerminalShellPanel,
  'command-history': TerminalShellPanel,
  'repl': TerminalShellPanel,
  'x3-lang': TerminalShellPanel,
  'cli-reference': TerminalShellPanel,

  // Integration Panels (Oracle, NFT-CRM)
  'oracle': PriceOraclePanel,
  'price-oracle': PriceOraclePanel,
  'pyth': PriceOraclePanel,
  'chainlink': PriceOraclePanel,
  'twap': PriceOraclePanel,
  'band-protocol': PriceOraclePanel,
  'amm-liquidity': PriceOraclePanel,
  'price-feeds': PriceOraclePanel,

  'nft-crm': NFTCRMIntegrationPanel,
  'wallet-linking': NFTCRMIntegrationPanel,
  'on-chain-deals': NFTCRMIntegrationPanel,
  'token-gated': NFTCRMIntegrationPanel,
  'nft-portfolio': NFTCRMIntegrationPanel,
  'nft-collection': NFTCRMIntegrationPanel,
  'gated-groups': NFTCRMIntegrationPanel,
  'wallet-verification': NFTCRMIntegrationPanel,

  // Performance Panels (Web Workers)
  'web-workers': WebWorkerOptimizationPanel,
  'worker-threads': WebWorkerOptimizationPanel,
  'gpu-compositing': WebWorkerOptimizationPanel,
  'webgl': WebWorkerOptimizationPanel,
  'wgpu': WebWorkerOptimizationPanel,
  'startup-preload': WebWorkerOptimizationPanel,
  'frame-compositing': WebWorkerOptimizationPanel,
  'performance-optimization': WebWorkerOptimizationPanel,
  'page-load': WebWorkerOptimizationPanel,

  // Developer Experience & Analytics Panels (13-27)
  'playground': DeveloperPlaygroundPanel,
  'developer-playground': DeveloperPlaygroundPanel,
  'ide': DeveloperPlaygroundPanel,
  'code-editor': DeveloperPlaygroundPanel,
  'compile': DeveloperPlaygroundPanel,
  'deploy': DeveloperPlaygroundPanel,
  'smart-contract': DeveloperPlaygroundPanel,

  'validator-health': ValidatorHealthPanel,
  'health': ValidatorHealthPanel,
  'health-monitor': ValidatorHealthPanel,
  'uptime': ValidatorHealthPanel,
  'block-performance': ValidatorHealthPanel,
  'validator-status': ValidatorHealthPanel,

  'audio-visualizer': AudioVisualizerPanel,
  'visualizer': AudioVisualizerPanel,
  'spectrum': AudioVisualizerPanel,
  'frequency': AudioVisualizerPanel,
  'audio': AudioVisualizerPanel,

  'crypto-keys': CryptoKeyManagementPanel,
  'key-management': CryptoKeyManagementPanel,
  'signing-keys': CryptoKeyManagementPanel,
  'encryption-keys': CryptoKeyManagementPanel,
  'key-generation': CryptoKeyManagementPanel,
  'ed25519': CryptoKeyManagementPanel,

  'messages': CommunicationCenterPanel,
  'communication': CommunicationCenterPanel,
  'inbox': CommunicationCenterPanel,
  'compose': CommunicationCenterPanel,
  'notifications': CommunicationCenterPanel,
  'alerts': CommunicationCenterPanel,

  'portfolio': PortfolioAnalysisPanel,
  'portfolio-analysis': PortfolioAnalysisPanel,
  'asset-allocation': PortfolioAnalysisPanel,
  'holdings': PortfolioAnalysisPanel,
  'pnl': PortfolioAnalysisPanel,
  'rebalance': PortfolioAnalysisPanel,

  'datastore': DatastoreManagementPanel,
  'key-value': DatastoreManagementPanel,
  'storage': DatastoreManagementPanel,
  'database': DatastoreManagementPanel,
  'kv-storage': DatastoreManagementPanel,
  'config-storage': DatastoreManagementPanel,

  'geolocation': GeoLocationPanel,
  'location-tracking': GeoLocationPanel,
  'geo': GeoLocationPanel,
  'world-map': GeoLocationPanel,
  'network-map': GeoLocationPanel,
  'validator-locations': GeoLocationPanel,

  'sessions': SessionSecurityPanel,
  'session-security': SessionSecurityPanel,
  'active-sessions': SessionSecurityPanel,
  'device-management': SessionSecurityPanel,
  'security-status': SessionSecurityPanel,
  'ip-masking': SessionSecurityPanel,

  'performance': PerformanceMonitorPanel,
  'performance-monitor': PerformanceMonitorPanel,
  'system-monitor': PerformanceMonitorPanel,
  'cpu': PerformanceMonitorPanel,
  'memory': PerformanceMonitorPanel,
  'disk-usage': PerformanceMonitorPanel,
  'latency': PerformanceMonitorPanel,

  'docs': DocumentationLibraryPanel,
  'documentation': DocumentationLibraryPanel,
  'dev-docs': DocumentationLibraryPanel,
  'guides': DocumentationLibraryPanel,
  'tutorials': DocumentationLibraryPanel,
  'api-reference': DocumentationLibraryPanel,

  'settings': SettingsPanel,
  'preferences': SettingsPanel,
  'config': SettingsPanel,
  'user-settings': SettingsPanel,
  'theme': SettingsPanel,
  'privacy-settings': SettingsPanel,

  'achievements': GamificationAndAchievementsPanel,
  'gamification': GamificationAndAchievementsPanel,
  'leaderboard': GamificationAndAchievementsPanel,
  'xp': GamificationAndAchievementsPanel,
  'badges': GamificationAndAchievementsPanel,
  'quests': GamificationAndAchievementsPanel,

  'faq': FAQSupportPanel,
  'support': FAQSupportPanel,
  'help': FAQSupportPanel,
  'contact-support': FAQSupportPanel,
  'help-center': FAQSupportPanel,
  'troubleshooting': FAQSupportPanel,

  'analytics': AnalyticsReportingPanel,
  'analytics-reporting': AnalyticsReportingPanel,
  'reporting': AnalyticsReportingPanel,
  'reports': AnalyticsReportingPanel,
  'metrics': AnalyticsReportingPanel,
  'trending': AnalyticsReportingPanel,

  'nft-marketplace': NftMarketplacePanel,
  'nft-market': NftMarketplacePanel,
  'nft-collections': NftMarketplacePanel,
  'nft-discovery': NftMarketplacePanel,
  'nft-rarity': NftMarketplacePanel,
  'floor-price': NftMarketplacePanel,
  'collection-analytics': NftMarketplacePanel,
  'nft-trading': NftMarketplacePanel,
  'nft-sales': NftMarketplacePanel,

  'token-marketplace': TokenMarketplacePanel,
  'token-market': TokenMarketplacePanel,
  'token-listings': TokenMarketplacePanel,
  'market-cap': TokenMarketplacePanel,
  'token-volume': TokenMarketplacePanel,
  'token-launches': TokenMarketplacePanel,
  'token-pairs': TokenMarketplacePanel,
  'token-trading': TokenMarketplacePanel,

  'governance': GovernanceProposalsPanel,
  'proposals': GovernanceProposalsPanel,
  'voting': GovernanceProposalsPanel,
  'dao': GovernanceProposalsPanel,
  'snapshot': GovernanceProposalsPanel,
  'poll': GovernanceProposalsPanel,

  'treasury': TreasuryManagementPanel,
  'multisig': TreasuryManagementPanel,
  'budget': TreasuryManagementPanel,
  'spending': TreasuryManagementPanel,
  'fund-allocation': TreasuryManagementPanel,

  'integrations': IntegrationMarketplacePanel,
  'plugins': IntegrationMarketplacePanel,
  'marketplace': IntegrationMarketplacePanel,
  'extensions': IntegrationMarketplacePanel,
  'app-store': IntegrationMarketplacePanel,

  'streaming': MediaStreamingPanel,
  'music': MediaStreamingPanel,
  'media': MediaStreamingPanel,
  'audio': MediaStreamingPanel,
  'creator-monetization': MediaStreamingPanel,

  'quantum': QuantumSecurityPanel,
  'post-quantum': QuantumSecurityPanel,
  'quantum-security': QuantumSecurityPanel,
  'lattice': QuantumSecurityPanel,

  'on-chain': OnChainAnalyticsPanel,
  'chain-analytics': OnChainAnalyticsPanel,
  'tvl': OnChainAnalyticsPanel,
  'gas-fees': OnChainAnalyticsPanel,
  'smart-contracts': OnChainAnalyticsPanel,

};

export type PanelId = keyof typeof panelRegistry;

export function getPanelComponent(panelId: PanelId): React.ComponentType | undefined {
  return panelRegistry[panelId];
}

export function getAllPanels() {
  return Object.keys(panelRegistry) as PanelId[];
}

export const panelMetadata = {
  'gpu-pooling': {
    name: 'Chain Core Optimization',
    category: 'Infrastructure',
    description: 'GPU pooling, multi-device optimization, memory management, and fallback chains',
    tags: ['gpu', 'performance', 'hardware', 'optimization'],
  },
  'dynamic-fees': {
    name: 'Dynamic Fee Market',
    category: 'Infrastructure',
    description: 'EIP-1559 fee structure, MEV protection, slashing insurance, and validator commission caps',
    tags: ['fees', 'mev', 'validator', 'economics'],
  },
  'cross-chain': {
    name: 'Cross-Chain Bridges',
    category: 'Infrastructure',
    description: 'Multi-chain bridge infrastructure with security council and liquidity pools',
    tags: ['bridges', 'cross-chain', 'security', 'liquidity'],
  },
  'solana': {
    name: 'Solana Adapter',
    category: 'Adapters',
    description: '10 standard Solana programs with Anchor framework and SPL token integration',
    tags: ['solana', 'programs', 'spl', 'anchor'],
  },
  'crm': {
    name: 'CRM Database',
    category: 'Backend',
    description: 'Real contact database with sales pipeline, email campaigns, and import/export',
    tags: ['crm', 'sales', 'contacts', 'database'],
  },
  'social': {
    name: 'Social Backend',
    category: 'Backend',
    description: 'ActivityPub federation, E2E encrypted messaging, IPFS media, and communities',
    tags: ['social', 'federation', 'messaging', 'media'],
  },
  'agents': {
    name: 'Agent Marketplace',
    category: 'Agents',
    description: 'AI agent marketplace with sandboxing, security audits, and multi-agent coordination',
    tags: ['agents', 'marketplace', 'ai', 'automation'],
  },
  'validators': {
    name: 'Validator Automation',
    category: 'Tools',
    description: 'One-click validator setup, metrics, slashing alerts, and auto-compound staking',
    tags: ['validators', 'staking', 'automation', 'monitoring'],
  },
  'terminal': {
    name: 'Terminal Shell',
    category: 'Tools',
    description: 'Real PTY terminal with X3 CLI reference and REPL environment',
    tags: ['terminal', 'cli', 'repl', 'commands'],
  },
  'oracle': {
    name: 'Price Oracle',
    category: 'Integration',
    description: 'Pyth, Chainlink, TWAP aggregation, and AMM liquidity providers',
    tags: ['oracle', 'prices', 'feeds', 'liquidity'],
  },
  'nft-crm': {
    name: 'NFT-CRM Integration',
    category: 'Integration',
    description: 'Wallet linking, on-chain deals, token-gated groups, and NFT portfolio metrics',
    tags: ['nft', 'crm', 'wallet', 'blockchain'],
  },
  'web-workers': {
    name: 'Web Workers & GPU Compositing',
    category: 'Performance',
    description: 'Worker threads, WebGL 2.0, WGPU, and startup preload optimization',
    tags: ['performance', 'workers', 'gpu', 'optimization'],
  },

  // Developer Experience & Analytics Metadata
  'playground': {
    name: 'Developer Playground',
    category: 'Developer Tools',
    description: 'X3-Lang IDE with code editor, compiler, and smart contract deployment',
    tags: ['ide', 'development', 'smart-contract', 'compilation'],
  },
  'validator-health': {
    name: 'Validator Health Monitor',
    category: 'Monitoring',
    description: 'Real-time validator performance, uptime, and slashing risk tracking',
    tags: ['validator', 'monitoring', 'performance', 'health'],
  },
  'audio-visualizer': {
    name: 'Audio Visualizer',
    category: 'Tools',
    description: 'Real-time frequency spectrum analyzer with Web Audio API integration',
    tags: ['audio', 'visualization', 'spectrum', 'analytics'],
  },
  'crypto-keys': {
    name: 'Cryptographic Key Management',
    category: 'Security',
    description: 'Generate, store, and manage Ed25519 signing and encryption keys securely',
    tags: ['security', 'cryptography', 'keys', 'encryption'],
  },
  'messages': {
    name: 'Communication Center',
    category: 'Social',
    description: 'Messages inbox, notifications, and alert management system',
    tags: ['messaging', 'notifications', 'communication', 'alerts'],
  },
  'portfolio': {
    name: 'Portfolio Analysis',
    category: 'Analytics',
    description: 'Real-time asset allocation, P&L tracking, and rebalancing analytics',
    tags: ['portfolio', 'analytics', 'assets', 'allocation'],
  },
  'datastore': {
    name: 'Datastore Management',
    category: 'Backend',
    description: 'Key-value storage management with search, creation, and monitoring',
    tags: ['database', 'storage', 'key-value', 'configuration'],
  },
  'geolocation': {
    name: 'Geolocation Tracking',
    category: 'Monitoring',
    description: 'Monitor validator node locations, geographic distribution, and risk assessment',
    tags: ['geolocation', 'monitoring', 'security', 'validators'],
  },
  'sessions': {
    name: 'Session Security',
    category: 'Security',
    description: 'Manage active sessions, devices, and IP address security controls',
    tags: ['security', 'sessions', 'devices', 'authentication'],
  },
  'performance': {
    name: 'Performance Monitor',
    category: 'Monitoring',
    description: 'Real-time CPU, memory, disk, network latency, and throughput metrics',
    tags: ['performance', 'monitoring', 'metrics', 'system'],
  },
  'docs': {
    name: 'Documentation Library',
    category: 'Developer Tools',
    description: 'Searchable developer documentation with guides, API reference, and code examples',
    tags: ['documentation', 'guides', 'api', 'reference'],
  },
  'settings': {
    name: 'Settings',
    category: 'User',
    description: 'User preferences, security settings, data retention, and privacy controls',
    tags: ['settings', 'preferences', 'configuration', 'security'],
  },
  'achievements': {
    name: 'Gamification & Achievements',
    category: 'Engagement',
    description: 'XP tracking, achievement badges, leaderboards, and quest management',
    tags: ['gamification', 'achievements', 'leaderboard', 'engagement'],
  },
  'faq': {
    name: 'FAQ & Support',
    category: 'Support',
    description: 'FAQ database, contact support forms, and help center resources',
    tags: ['faq', 'support', 'help', 'troubleshooting'],
  },
  'analytics': {
    name: 'Analytics & Reporting',
    category: 'Analytics',
    description: 'Comprehensive metrics, trend analysis, and downloadable reports',
    tags: ['analytics', 'reporting', 'metrics', 'trends'],
  },
  'nft-marketplace': {
    name: 'NFT Marketplace',
    category: 'Web3 Trading',
    description: 'Collection discovery, rarity ranking, floor prices, and trading activity',
    tags: ['nft', 'marketplace', 'trading', 'rarity', 'analytics'],
  },
  'token-marketplace': {
    name: 'Token Marketplace',
    category: 'Web3 Trading',
    description: 'Token listings with market cap ranking, 24h volume, 7d returns, price charts, launch tracking',
    tags: ['tokens', 'marketplace', 'trading', 'market-cap', 'volume'],
  },
  'governance': {
    name: 'Governance Proposals',
    category: 'DAO',
    description: 'DAO proposal submission, voting interface, vote breakdown, quorum tracking, timeline visualization',
    tags: ['governance', 'voting', 'dao', 'proposals', 'consensus'],
  },
  'treasury': {
    name: 'Treasury Management',
    category: 'DAO',
    description: 'Multi-sig wallet control, budget allocation by category, spending history, approval workflows',
    tags: ['treasury', 'multisig', 'budget', 'wallet', 'dao'],
  },
  'integrations': {
    name: 'Integration Marketplace',
    category: 'Ecosystem',
    description: 'Plugin discovery with adoption stats, rating system, category browsing, developer ecosystem metrics',
    tags: ['integrations', 'plugins', 'marketplace', 'ecosystem', 'extensions'],
  },
  'streaming': {
    name: 'Media Streaming',
    category: 'Creator Economy',
    description: 'Decentralized music/video with creator micropayments per-stream, stream analytics, creator profiles',
    tags: ['streaming', 'music', 'media', 'creator', 'monetization'],
  },
  'quantum': {
    name: 'Quantum Security',
    category: 'Security',
    description: 'Post-quantum crypto readiness assessment, lattice algorithm migration status, security audit results',
    tags: ['quantum', 'security', 'cryptography', 'latice', 'fips'],
  },
  'on-chain': {
    name: 'On-Chain Analytics',
    category: 'Analytics',
    description: 'Real-time TVL tracking, transaction volume/velocity, gas fee trends, smart contract call monitoring',
    tags: ['analytics', 'tvl', 'volume', 'gas', 'smart-contracts'],
  },
};

export default panelRegistry;
