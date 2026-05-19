/**
 * Panel registry — maps application IDs to their React panel components.
 *
 * When a window is opened for a registered app, WindowManager renders
 * the real panel instead of a placeholder letter.
 */
import React, { lazy, Suspense, type ComponentType } from "react";

/* Lazy-load panels to keep initial bundle lean */
const SwarmHealthPanel = lazy(() => import("@/components/panels/SwarmHealthPanel"));
const NetworkPanel     = lazy(() => import("@/components/panels/NetworkPanel"));
const StoragePanel     = lazy(() => import("@/components/panels/StoragePanel"));
const DevToolsPanel    = lazy(() => import("@/components/panels/DevToolsPanel"));
const SecurityPanel    = lazy(() => import("@/components/panels/SecurityPanel"));
const LiveTelemetryPanel = lazy(() => import("@/components/panels/LiveTelemetryPanel"));
const MonitoringDashboard = lazy(() => import("@/components/monitoring/MonitoringDashboard"));
const WorldMonitorPanel = lazy(() => import("@/components/panels/WorldMonitorPanel"));
const DocumentationPanel = lazy(() => import("@/components/documentation/Documentation"));

/* Blockchain Connector */
const BlockchainConnectorPanel = lazy(() => import("@/components/panels/BlockchainConnectorPanel"));

/* AGI Substrate panels */
const SelfModelViewer = lazy(() => import("@/components/monitoring/SelfModelViewer"));
const GoalGenomeViewer = lazy(() => import("@/components/monitoring/GoalGenomeViewer"));
const WorldSimViewer = lazy(() => import("@/components/monitoring/WorldSimViewer"));
const SelfImprovementViewer = lazy(() => import("@/components/monitoring/SelfImprovementViewer"));
const TripwireMonitor = lazy(() => import("@/components/monitoring/TripwireMonitor"));

/* ── Explorer Sub-App Panels (ported from apps/explorer) ──── */
const AISwarmPanel       = lazy(() => import("@/components/panels/explorer/AISwarmPanel"));
const BlogPanel          = lazy(() => import("@/components/panels/explorer/BlogPanel"));
const BridgePanel        = lazy(() => import("@/components/panels/explorer/BridgePanel"));
const CommunityPanel     = lazy(() => import("@/components/panels/explorer/CommunityPanel"));
const EarnPanel          = lazy(() => import("@/components/panels/explorer/EarnPanel"));
const EcosystemPanel     = lazy(() => import("@/components/panels/explorer/EcosystemPanel"));
const BlockExplorerPanel = lazy(() => import("@/components/panels/explorer/BlockExplorerPanel"));
const LearnPanel         = lazy(() => import("@/components/panels/explorer/LearnPanel"));
const MetricsPanel       = lazy(() => import("@/components/panels/explorer/MetricsPanel"));
const NetworkPanel2      = lazy(() => import("@/components/panels/explorer/NetworkPanel2"));
const PortfolioPanel     = lazy(() => import("@/components/panels/explorer/PortfolioPanel"));
const QuantumPanel       = lazy(() => import("@/components/panels/explorer/QuantumPanel"));
const SecurityPanel2     = lazy(() => import("@/components/panels/explorer/SecurityPanel2"));
const StakePanel         = lazy(() => import("@/components/panels/explorer/StakePanel"));
const SwapPanel          = lazy(() => import("@/components/panels/explorer/SwapPanel"));
const TreasuryPanel      = lazy(() => import("@/components/panels/explorer/TreasuryPanel"));
const X3ChainPanel       = lazy(() => import("@/components/panels/explorer/X3ChainPanel"));
const X3OSPanel          = lazy(() => import("@/components/panels/explorer/X3OSPanel"));
const X3StarPanel        = lazy(() => import("@/components/panels/explorer/X3StarPanel"));
const PrivacyPanel       = lazy(() => import("@/components/panels/explorer/PrivacyPanel"));
const TermsPanel         = lazy(() => import("@/components/panels/explorer/TermsPanel"));

/* ── Explorer Sub-Pages (deeper routes) ──── */
const DevDocsPanel          = lazy(() => import("@/components/panels/explorer/DevDocsPanel"));
const SolutionsDetailPanel  = lazy(() => import("@/components/panels/explorer/SolutionsDetailPanel"));
const NetworkValidatorsPanel = lazy(() => import("@/components/panels/explorer/NetworkValidatorsPanel"));
const LearnArchitecturePanel = lazy(() => import("@/components/panels/explorer/LearnArchitecturePanel"));
const X3SubPagesPanel       = lazy(() => import("@/components/panels/explorer/X3SubPagesPanel"));
const CommunitySubPanel     = lazy(() => import("@/components/panels/explorer/CommunitySubPanel"));
const QuantumEnhancedPanel  = lazy(() => import("@/components/panels/explorer/QuantumEnhancedPanel"));
const ExplorerHomePanel     = lazy(() => import("@/components/panels/explorer/ExplorerHomePanel"));
const ExplorerDetailPanel   = lazy(() => import("@/components/panels/explorer/ExplorerDetailPanel"));

/* ── Wallet (ported from apps/wallet) ──── */
const WalletPanel = lazy(() => import("@/components/panels/wallet/WalletPanel"));
const NftGalleryPanel = lazy(() => import("@/components/panels/wallet/NftGalleryPanel"));
const TokenChartsPanel = lazy(() => import("@/components/panels/wallet/TokenChartsPanel"));
const PrivacyModePanel = lazy(() => import("@/components/panels/wallet/PrivacyModePanel"));

/* ── X3 Intelligence (ported from apps/x3-intelligence) ──── */
const X3FloorDashboardPanel = lazy(() => import("@/components/panels/x3intel/X3FloorDashboardPanel"));
const X3AgentsPanel         = lazy(() => import("@/components/panels/x3intel/X3AgentsPanel"));
const X3BondsPanel          = lazy(() => import("@/components/panels/x3intel/X3BondsPanel"));
const X3GuidePanel          = lazy(() => import("@/components/panels/x3intel/X3GuidePanel"));
const X3IntentsPanel        = lazy(() => import("@/components/panels/x3intel/X3IntentsPanel"));
const X3SlashingPanel       = lazy(() => import("@/components/panels/x3intel/X3SlashingPanel"));
const X3WhyPanel            = lazy(() => import("@/components/panels/x3intel/X3WhyPanel"));

/* ── DEX (ported from apps/dex) ──── */
const DexPanel           = lazy(() => import("@/components/panels/dex/DexPanel"));
const DexPoolsPanel      = lazy(() => import("@/components/panels/dex/DexPoolsPanel"));
const DexOrderbookPanel  = lazy(() => import("@/components/panels/dex/DexOrderbookPanel"));
const ConcentratedLiquidityPanel = lazy(() => import("@/components/panels/dex/ConcentratedLiquidityPanel"));
const DexAdvancedOrdersPanel = lazy(() => import("@/components/panels/dex/DexAdvancedOrdersPanel"));
const TransactionSimulatorPanel = lazy(() => import("@/components/panels/dex/TransactionSimulatorPanel"));
const LpNftMarketplacePanel = lazy(() => import("@/components/panels/dex/LpNftMarketplacePanel"));

/* ── DeFi (Vote-Escrow & Liquidity Mining) ──── */
const VeX3Panel = lazy(() => import("@/components/panels/defi/VeX3Panel"));
const LiquidityMiningPanel = lazy(() => import("@/components/panels/defi/LiquidityMiningPanel"));
const TokenLaunchpadPanel = lazy(() => import("@/components/panels/defi/TokenLaunchpadPanel"));

/* ── Social (Social Network & Creator Economy) ──── */
const SocialPanel = lazy(() => import("@/components/panels/social/SocialPanel"));
const CreatorMonetizationPanel = lazy(() => import("@/components/panels/social/CreatorMonetizationPanel"));

/* ── Trading & Bots ──── */
const StrategyBuilderPanel = lazy(() => import("@/components/panels/trading/StrategyBuilderPanel"));
const BacktestingPanel = lazy(() => import("@/components/panels/trading/BacktestingPanel"));
const BotMarketplacePanel = lazy(() => import("@/components/panels/trading/BotMarketplacePanel"));
const MevBotPanel = lazy(() => import("@/components/panels/trading/MevBotPanel"));

/* ── Swarm Dashboard (ported from apps/swarm-dashboard) ──── */
const SwarmDashboardPanel = lazy(() => import("@/components/panels/swarm/SwarmDashboardPanel"));

/* ── Infrastructure Dashboard (ported from apps/inferstructor-dashboard) ──── */
const InfrastructurePanel = lazy(() => import("@/components/panels/infrastructure/InfrastructurePanel"));
const RpcStatsPanel = lazy(() => import("@/components/panels/infrastructure/RpcStatsPanel"));
const AirdropsPanel = lazy(() => import("@/components/panels/infrastructure/AirdropsPanel"));
const WhaleTrackerPanel = lazy(() => import("@/components/panels/infrastructure/WhaleTrackerPanel"));
const BridgeStatusPanel = lazy(() => import("@/components/panels/infrastructure/BridgeStatusPanel"));

/* ── Desktop Updates & Settings ──── */
const DesktopUpdatesPanel = lazy(() => import("@/components/panels/desktop/DesktopUpdatesPanel"));
const WidgetLayerPanel = lazy(() => import("@/components/panels/desktop/WidgetLayerPanel"));
const WindowLayoutsPanel = lazy(() => import("@/components/panels/desktop/WindowLayoutsPanel"));

/* ── Validators Globe (ported from apps/validators) ──── */
const ValidatorsPanel = lazy(() => import("@/components/panels/validators/ValidatorsPanel"));
const ValidatorSetupWizardPanel = lazy(() => import("@/components/panels/validators/ValidatorSetupWizardPanel"));
const ValidatorLeaderboardPanel = lazy(() => import("@/components/panels/validators/ValidatorLeaderboardPanel"));

/* ── Health Dashboard (ported from apps/health-dashboard) ──── */
const HealthDashboardPanel = lazy(() => import("@/components/panels/health/HealthDashboardPanel"));

/* ── Admin Dashboard ──── */
const AdminPanel = lazy(() => import("@/components/panels/admin/AdminPanel"));

/* ── Analytics (Risk, Heatmap, etc) ──── */
const CryptoHeatmapPanel = lazy(() => import("@/components/panels/analytics/CryptoHeatmapPanel"));
const PortfolioRiskPanel = lazy(() => import("@/components/panels/analytics/PortfolioRiskPanel"));

/* ── Documentation & API ──── */
const ApiReferencePanel = lazy(() => import("@/components/panels/documentation/ApiReferencePanel"));

/* ── Sprint 9: App Store, Multi-Monitor, KYC, Whitelist, Anti-Sniper, Token Audit, Liquidity Lock, Social Recovery, Governance, Analytics Audit ──── */
const AppStorePanel = lazy(() => import("@/components/panels/desktop/AppStorePanel"));
const MultiMonitorPanel = lazy(() => import("@/components/panels/desktop/MultiMonitorPanel"));
const KycGatingPanel = lazy(() => import("@/components/panels/defi/KycGatingPanel"));
const WhitelistPresalePanel = lazy(() => import("@/components/panels/defi/WhitelistPresalePanel"));
const AntisniperPanel = lazy(() => import("@/components/panels/defi/AntisniperPanel"));
const TokenAuditPanel = lazy(() => import("@/components/panels/defi/TokenAuditPanel"));
const LiquidityLockPanel = lazy(() => import("@/components/panels/defi/LiquidityLockPanel"));
const SocialRecoveryPanel = lazy(() => import("@/components/panels/global/SocialRecoveryPanel"));
const GovernancePanel = lazy(() => import("@/components/panels/global/GovernancePanel"));
const AnalyticsAuditPanel = lazy(() => import("@/components/panels/global/AnalyticsAuditPanel"));

/* ── Sprint 10: Content Moderation, Agent Marketplace, Advanced DEX, Infrastructure Automation, Enterprise Security, Cross-Chain Bridge, Compliance Report, Token Vesting, API Gateway, Disaster Recovery ──── */
const ContentModerationPanel = lazy(() => import("@/components/panels/social/ContentModerationPanel"));
const AgentMarketplacePanel = lazy(() => import("@/components/panels/trading/AgentMarketplacePanel"));
const AdvancedDexPanel = lazy(() => import("@/components/panels/dex/AdvancedDexPanel"));
const InfrastructureAutomationPanel = lazy(() => import("@/components/panels/infrastructure/InfrastructureAutomationPanel"));
const EnterpriseSecurityPanel = lazy(() => import("@/components/panels/admin/EnterpriseSecurityPanel"));
const CrossChainBridgePanel = lazy(() => import("@/components/panels/infrastructure/CrossChainBridgePanel"));
const ComplianceReportPanel = lazy(() => import("@/components/panels/admin/ComplianceReportPanel"));
const TokenVestingPanel = lazy(() => import("@/components/panels/trading/TokenVestingPanel"));
const APIGatewayPanel = lazy(() => import("@/components/panels/infrastructure/APIGatewayPanel"));
const DisasterRecoveryPanel = lazy(() => import("@/components/panels/infrastructure/DisasterRecoveryPanel"));

/* ── Sprint 11: Wallet Security, CRM Backend, Social Infrastructure, Developer Experience ──── */
// Wallet - Real transaction signing, hardware wallets, multi-signature
const RealTransactionSigningPanel = lazy(() => import("@/components/panels/wallet/RealTransactionSigningPanel"));
const HardwareWalletPanel = lazy(() => import("@/components/panels/wallet/HardwareWalletPanel"));
const MultiSignaturePanel = lazy(() => import("@/components/panels/wallet/MultiSignaturePanel"));

// Social - CRM, E2E messaging, notifications, communities, creator monetization
const RealCrmBackendPanel = lazy(() => import("@/components/panels/social/RealCrmBackendPanel"));
const E2eMessagesPanel = lazy(() => import("@/components/panels/social/E2eMessagesPanel"));
const RealTimeNotificationsPanel = lazy(() => import("@/components/panels/social/RealTimeNotificationsPanel"));
const CommunitiesPanel = lazy(() => import("@/components/panels/social/CommunitiesPanel"));
const CreatorMonetizationPremiumPanel = lazy(() => import("@/components/panels/social/CreatorMonetizationPremiumPanel"));

// Documentation - Interactive code playground, SDK generator
const InteractiveCodePlaygroundPanel = lazy(() => import("@/components/panels/documentation/InteractiveCodePlaygroundPanel"));
const SdkCodeGeneratorPanel = lazy(() => import("@/components/panels/documentation/SdkCodeGeneratorPanel"));

/* ── Sprint 12: Privacy, Analytics, Marketplace, Governance, Infrastructure ──── */
// Security - Privacy Vault
const PrivacyVaultPanel = lazy(() => import("@/components/panels/security/PrivacyVaultPanel"));

// Analytics - Advanced Portfolio & OnChain
const AdvancedPortfolioAnalyticsPanel = lazy(() => import("@/components/panels/analytics/AdvancedPortfolioAnalyticsPanel"));
const OnChainAnalyticsPanel = lazy(() => import("@/components/panels/analytics/OnChainAnalyticsPanel"));

// Marketplace - NFT, Token
const NftMarketplacePanel = lazy(() => import("@/components/panels/marketplace/NftMarketplacePanel"));
const TokenMarketplacePanel = lazy(() => import("@/components/panels/marketplace/TokenMarketplacePanel"));

// Governance - Proposals & Treasury
const GovernanceProposalsPanel = lazy(() => import("@/components/panels/governance/GovernanceProposalsPanel"));
const TreasuryManagementPanel = lazy(() => import("@/components/panels/governance/TreasuryManagementPanel"));

// Infrastructure - Integration & Quantum
const IntegrationMarketplacePanel = lazy(() => import("@/components/panels/infrastructure/IntegrationMarketplacePanel"));
const QuantumSecurityPanel = lazy(() => import("@/components/panels/infrastructure/QuantumSecurityPanel"));

// Media - Streaming
const MediaStreamingPanel = lazy(() => import("@/components/panels/media/MediaStreamingPanel"));

/* ── Sprint 13 Phase 2: Performance, Terminal, Infrastructure, CRM, Social, Security, Growth ──── */
// Performance - Virtualization, WebWorkers, GPU, Startup, Memory
const VirtualizedPanelPanel = lazy(() => import("@/components/panels/performance/VirtualizedPanelPanel"));

// Terminal - Shell, CLI, REPL
const X3TerminalPanel = lazy(() => import("@/components/panels/terminal/X3TerminalPanel"));

// Infrastructure - Validator Alerts, Geo Distribution, RPC Keys
const ValidatorAlertsPanel = lazy(() => import("@/components/panels/infrastructure/ValidatorAlertsPanel"));
const GeoDistributionPanel = lazy(() => import("@/components/panels/infrastructure/GeoDistributionPanel"));
const RpcKeysPanel = lazy(() => import("@/components/panels/infrastructure/RpcKeysPanel"));

// CRM - Deal Pipeline, Hardware Acquisition, Task Management, Call Logging, Email Templates, Wallet-Linked Contacts
const DealPipelinePanel = lazy(() => import("@/components/panels/crm/DealPipelinePanel"));
const HardwareAcquisitionPanel = lazy(() => import("@/components/panels/crm/HardwareAcquisitionPanel"));
const HardwareSourcesPanel = lazy(() => import("@/components/panels/crm/HardwareSourcesPanel"));

// Social - E2E Messaging (enhanced), Communities (enhanced), Media Upload, Content Moderation (enhanced)
const E2EMessagingPanel = lazy(() => import("@/components/panels/social/E2EMessagingPanel"));

// Security - Compliance Checklist, Audit Tracking
const ComplianceChecklistPanel = lazy(() => import("@/components/panels/security/ComplianceChecklistPanel"));

// Growth - DAO Governance, Grants Program, Airdrop Campaign, Mainnet Genesis, Partnerships
const DAOGovernancePanel = lazy(() => import("@/components/panels/growth/DAOGovernancePanel"));

/* ── Global Search ──── */
const GlobalSearchPanel = lazy(() => import("@/components/panels/global/GlobalSearchPanel"));
const CrashReporterPanel = lazy(() => import("@/components/panels/global/CrashReporterPanel"));

/* ── Additional missing panels (not yet imported above) ──── */
// Analytics - Advanced
const AdvancedAnalyticsPanel = lazy(() => import("@/components/panels/analytics/AdvancedAnalyticsPanel"));
const MarketHeatmapPanel = lazy(() => import("@/components/panels/analytics/MarketHeatmapPanel"));

// Wallet - Privacy
const PrivacyWalletPanel = lazy(() => import("@/components/panels/wallet/PrivacyWalletPanel"));

// Validators - Globe
const ValidatorGlobe = lazy(() => import("@/components/panels/validators/ValidatorGlobe"));

// Auction
const NftAuctionPanel = lazy(() => import("@/components/panels/auctions/NftAuctionPanel"));

// Marketplace - Real Assets
const RealMarketplacePanel = lazy(() => import("@/components/panels/marketplace/RealMarketplacePanel"));

// Root level panels
const IframePanel = lazy(() => import("@/components/panels/IframePanel"));

// Governance - Voting
const GovernanceVotingPanel = lazy(() => import("@/components/panels/governance/GovernanceVotingPanel"));

// Governance - CRM Swarm Governance
const CrmGovernancePanel = lazy(() => import("@/components/panels/governance/CrmGovernancePanel"));

/* ── Embedded External Apps (full standalone apps via IframePanel) ──── */
const InfestructorDashboardPanel = lazy(() => import("@/components/panels/embeds/InfestructorDashboardPanel"));
const InfraDashboardEmbedPanel   = lazy(() => import("@/components/panels/embeds/InfraDashboardPanel"));
const X3FrontendPanel            = lazy(() => import("@/components/panels/embeds/X3FrontendPanel"));
const ModularDashboardPanel      = lazy(() => import("@/components/panels/embeds/ModularDashboardPanel"));
const ValidatorsGlobeEmbedPanel  = lazy(() => import("@/components/panels/embeds/ValidatorsGlobePanel"));
const WalletAppPanel             = lazy(() => import("@/components/panels/embeds/WalletAppPanel"));
const DexAppPanel                = lazy(() => import("@/components/panels/embeds/DexAppPanel"));
const TpsMonitorEmbedPanel       = lazy(() => import("@/components/panels/embeds/TpsMonitorPanel"));
const MainnetProgressEmbedPanel  = lazy(() => import("@/components/panels/embeds/MainnetProgressPanel"));
const SwarmAutonomicEmbedPanel   = lazy(() => import("@/components/panels/embeds/SwarmAutonomicPanel"));
const X3IntelligenceFullPanel    = lazy(() => import("@/components/panels/embeds/X3IntelligenceFullPanel"));

/**
 * Map of appId → lazy-loaded panel component.
 * Add new panels here as they are created.
 */
const PANEL_MAP: Record<string, ComponentType> = {
  "swarm-health":   SwarmHealthPanel,
  "network-control": NetworkPanel,
  "storage-manager": StoragePanel,
  "dev-tools":       DevToolsPanel,
  "live-telemetry":  LiveTelemetryPanel,
  "system-monitoring": MonitoringDashboard,
  "world-monitor":    WorldMonitorPanel,
  "documentation":   DocumentationPanel,

  // Aliases: existing apps can also route to panels
  "admin-command-center":   NetworkPanel,
  "htlc-manager":           StoragePanel,
  "dev-dashboard":          DevToolsPanel,

  // AGI Substrate panels
  "self-model":             SelfModelViewer,
  "goal-genome":            GoalGenomeViewer,
  "world-sim":              WorldSimViewer,
  "self-improvement":       SelfImprovementViewer,
  "tripwire-monitor":       TripwireMonitor,

  // Enterprise Blockchain Connector
  "blockchain-connector":   BlockchainConnectorPanel,

  /* ── Explorer Sub-Apps (native panels, no iframe needed) ── */
  "block-explorer":         BlockExplorerPanel,
  "defi-metrics":           MetricsPanel,
  "developers-portal":      DevDocsPanel,       // full developer docs portal
  "prometheus-metrics":     MetricsPanel,       // shares the metrics panel

  /* ── Explorer Sub-Pages (deeper routes) ── */
  "dev-docs":               DevDocsPanel,
  "community-hub":          CommunitySubPanel,

  /* ── Wallet (ported from apps/wallet) ── */
  "wallet":                 WalletPanel,
  "wallet-dashboard":       WalletPanel,
  "wallet-send":            WalletPanel,
  "wallet-receive":         WalletPanel,
  "wallet-swap":            WalletPanel,

  /* ── X3 Intelligence (ported from apps/x3-intelligence) ── */
  "x3-intelligence":        X3FloorDashboardPanel, // override old alias

  /* ── DEX (ported from apps/dex) ── */

  /* ── DeFi (Vote-Escrow & Liquidity Mining) ── */

  /* ── Social (Social Network & Creator Economy) ── */
  "social":                 SocialPanel,
  "social-feed":            SocialPanel,
  "creator-economy":        SocialPanel,

  /* ── Validators (ported from apps/validators) ── */
  "validators":             ValidatorsPanel,

  /* ── Swarm Dashboard (ported from apps/swarm-dashboard) ── */
  "swarm-dashboard":        SwarmDashboardPanel,   // override old alias
  "gpu-swarm-dashboard":    SwarmDashboardPanel,

  /* ── Infrastructure Dashboard (ported from apps/inferstructor-dashboard) ── */
  "infrastructure":         InfrastructurePanel,
  "infra-dashboard":        InfrastructurePanel,
  "rpc-stats":              RpcStatsPanel,
  "rpc-pool":               RpcStatsPanel,
  "airdrops":               AirdropsPanel,
  "airdrops-faucets":       AirdropsPanel,
  "whale-tracker":          WhaleTrackerPanel,
  "whale-alerts":           WhaleTrackerPanel,

  /* ── Desktop Updates & Settings ── */
  "desktop-updates":        DesktopUpdatesPanel,
  "updates":                DesktopUpdatesPanel,
  "changelog":              DesktopUpdatesPanel,

  /* ── Health Dashboard (ported from apps/health-dashboard) ── */
  "health-dashboard":       HealthDashboardPanel,
  "system-health":          HealthDashboardPanel,

  /* ── Admin Dashboard ── */
  "admin-dashboard":        AdminPanel,
  "admin-panel":            AdminPanel,

  /* ── Sprint 6: Validator Setup, NFT Gallery, Concentrated Liquidity, Widgets ── */
  "validator-setup":        ValidatorSetupWizardPanel,
  "setup-wizard":           ValidatorSetupWizardPanel,
  "nft-gallery":            NftGalleryPanel,
  "nft-explorer":           NftGalleryPanel,
  "v3":                     ConcentratedLiquidityPanel,
  "widget-layer":           WidgetLayerPanel,
  "widgets":                WidgetLayerPanel,
  "floating-widgets":       WidgetLayerPanel,

  /* ── Sprint 7: Advanced Orders, Token Charts, Bridge, Creator Monetization, Leaderboard, API, Heatmap, Simulator, Risk, Search ── */
  "price-charts":           TokenChartsPanel,
  "bridge-status":          BridgeStatusPanel,
  "validator-leaderboard":  ValidatorLeaderboardPanel,
  "validator-ranking":      ValidatorLeaderboardPanel,
  "api-reference":          ApiReferencePanel,
  "api-docs":               ApiReferencePanel,
  "swap-preview":           TransactionSimulatorPanel,
  "risk-analysis":          PortfolioRiskPanel,
  "global-search":          GlobalSearchPanel,
  "search":                 GlobalSearchPanel,
  "command-palette":        GlobalSearchPanel,

  /* ── Sprint 8: Liquidity Mining, Strategy Builder, Backtesting, Bot Marketplace, Token Launchpad, Privacy Mode, Window Layouts, LP NFT Marketplace, MEV Bot, Crash Reporter ── */
  "tokenomics-designer":    TokenLaunchpadPanel,
  "presale":                TokenLaunchpadPanel,
  "transaction-mixer":      PrivacyModePanel,
  "window-layouts":         WindowLayoutsPanel,
  "snap-layouts":           WindowLayoutsPanel,
  "layout-manager":         WindowLayoutsPanel,
  "lp-position-nfts":       LpNftMarketplacePanel,
  "liquidity-nft-market":   LpNftMarketplacePanel,
  "mev-bot":                MevBotPanel,
  "mev-capture":            MevBotPanel,
  "sandwich-protection":    MevBotPanel,
  "crash-reporter":         CrashReporterPanel,
  "bug-reporter":           CrashReporterPanel,
  "error-report":           CrashReporterPanel,

  /* ── Sprint 9: App Store, Multi-Monitor, KYC, Whitelist Presales, Anti-Sniper, Token Audit, Liquidity Lock, Social Recovery, Governance, Analytics Audit ── */
  // Desktop
  "app-store":              AppStorePanel,
  "plugin-marketplace":     AppStorePanel,
  "extensions-store":       AppStorePanel,
  "desktop-apps":           AppStorePanel,
  "multi-monitor":          MultiMonitorPanel,
  "multi-display":          MultiMonitorPanel,
  "display-settings":       MultiMonitorPanel,
  "screen-arrangement":     MultiMonitorPanel,

  // DeFi - KYC & Compliance
  "kyc-gating":             KycGatingPanel,
  "kyc-verification":       KycGatingPanel,
  "aml-compliance":         KycGatingPanel,
  "identity-verification":  KycGatingPanel,
  "tier-limits":            KycGatingPanel,

  // DeFi - Whitelist Presales
  "whitelist-presale":      WhitelistPresalePanel,
  "presale-whitelist":      WhitelistPresalePanel,
  "whitelist-tiers":        WhitelistPresalePanel,
  "presale-rounds":         WhitelistPresalePanel,
  "presale-claims":         WhitelistPresalePanel,
  "token-allocation":       WhitelistPresalePanel,

  // DeFi - Anti-Sniper
  "anti-sniper":            AntisniperPanel,
  "antidump":               AntisniperPanel,
  "bot-protection":         AntisniperPanel,
  "sniper-defense":         AntisniperPanel,
  "launch-protection":      AntisniperPanel,

  // DeFi - Token Audit
  "token-audit":            TokenAuditPanel,
  "audit-badge":            TokenAuditPanel,
  "smart-contract-audit":   TokenAuditPanel,
  "certik-audit":           TokenAuditPanel,

  // DeFi - Liquidity Lock
  "liquidity-lock":         LiquidityLockPanel,
  "lp-lock":                LiquidityLockPanel,
  "lock-lp":                LiquidityLockPanel,
  "lock-schedule":          LiquidityLockPanel,

  // Global - Social Recovery
  "social-recovery":        SocialRecoveryPanel,
  "guardian-recovery":      SocialRecoveryPanel,
  "wallet-recovery":        SocialRecoveryPanel,
  "key-recovery":           SocialRecoveryPanel,
  "guardians":              SocialRecoveryPanel,

  // Global - Governance
  "governance":             GovernancePanel,
  "dao-voting":             GovernancePanel,
  "quorum":                 GovernancePanel,
  "crm-governance":         CrmGovernancePanel,
  "agent-voting":           CrmGovernancePanel,

  // Global - Analytics Audit
  "analytics-audit":        AnalyticsAuditPanel,
  "audit-analytics":        AnalyticsAuditPanel,
  "audit-history":          AnalyticsAuditPanel,
  "security-score":         AnalyticsAuditPanel,
  "vulnerability-timeline": AnalyticsAuditPanel,
  "audit-report":           AnalyticsAuditPanel,

  /* ── Sprint 10: Content Moderation, Agent Marketplace, Advanced DEX, Infrastructure Automation, Enterprise Security, Cross-Chain Bridge, Compliance Report, Token Vesting, API Gateway, Disaster Recovery ── */
  
  // Social - Content Moderation
  "content-moderation":     ContentModerationPanel,
  "moderation":             ContentModerationPanel,
  "content-flags":          ContentModerationPanel,
  "content-voting":         ContentModerationPanel,
  "audit-logs":             ContentModerationPanel,
  "moderation-dashboard":   ContentModerationPanel,

  // Trading - Agent Marketplace
  "agent-marketplace":      AgentMarketplacePanel,
  "trading-bots":           AgentMarketplacePanel,
  "copy-trading":           AgentMarketplacePanel,
  "bot-subscription":       AgentMarketplacePanel,
  "ai-agents":              AgentMarketplacePanel,

  // DEX - Advanced Routing
  "advanced-dex":           AdvancedDexPanel,
  "dex-routing":            AdvancedDexPanel,
  "amm-routing":            AdvancedDexPanel,
  "intelligent-routing":    AdvancedDexPanel,
  "mev-protection":         AdvancedDexPanel,
  "route-optimization":     AdvancedDexPanel,
  "slippage-control":       AdvancedDexPanel,

  // Infrastructure - Automation
  "infrastructure-automation": InfrastructureAutomationPanel,
  "validator-automation":   InfrastructureAutomationPanel,
  "node-deployment":        InfrastructureAutomationPanel,
  "automation-tasks":       InfrastructureAutomationPanel,
  "validator-nodes":        InfrastructureAutomationPanel,
  "geo-nodes":              InfrastructureAutomationPanel,
  "deployment-manager":     InfrastructureAutomationPanel,

  // Admin - Enterprise Security
  "enterprise-security":    EnterpriseSecurityPanel,
  "access-control":         EnterpriseSecurityPanel,
  "rbac":                   EnterpriseSecurityPanel,
  "hsm-keys":               EnterpriseSecurityPanel,
  "access-logs":            EnterpriseSecurityPanel,

  // Infrastructure - Cross-Chain Bridge
  "cross-chain-bridge":     CrossChainBridgePanel,
  "bridge-liquidity":       CrossChainBridgePanel,
  "bridge-transfers":       CrossChainBridgePanel,
  "bridging":               CrossChainBridgePanel,
  "liquidity-pools-bridge": CrossChainBridgePanel,

  // Admin - Compliance Report
  "compliance-report":      ComplianceReportPanel,
  "compliance-dashboard":   ComplianceReportPanel,
  "gdpr-compliance":        ComplianceReportPanel,
  "iso-compliance":         ComplianceReportPanel,
  "regulatory-dashboard":   ComplianceReportPanel,
  "audit-compliance":       ComplianceReportPanel,

  // Trading - Token Vesting
  "token-vesting":          TokenVestingPanel,
  "vesting-schedule":       TokenVestingPanel,
  "unlock-schedule":        TokenVestingPanel,
  "vesting-timeline":       TokenVestingPanel,
  "cliff-release":          TokenVestingPanel,
  "token-release":          TokenVestingPanel,
  "vesting-dashboard":      TokenVestingPanel,

  // Infrastructure - API Gateway
  "api-gateway":            APIGatewayPanel,
  "quota-management":       APIGatewayPanel,
  "api-quota":              APIGatewayPanel,
  "rate-limits":            APIGatewayPanel,
  "api-analytics":          APIGatewayPanel,

  // Infrastructure - Disaster Recovery
  "disaster-recovery":      DisasterRecoveryPanel,
  "backup-recovery":        DisasterRecoveryPanel,
  "backup-snapshots":       DisasterRecoveryPanel,
  "restore-points":         DisasterRecoveryPanel,
  "recovery-testing":       DisasterRecoveryPanel,
  "backup-management":      DisasterRecoveryPanel,
  "rto-rpo":                DisasterRecoveryPanel,

  /* ── Sprint 11: Wallet Security, CRM Backend, Social Infrastructure, Developer Experience ── */
  
  // Wallet - Real transaction signing
  "real-transaction-signing": RealTransactionSigningPanel,
  "transaction-signing":      RealTransactionSigningPanel,
  "sign-transactions":        RealTransactionSigningPanel,
  "tx-approval":              RealTransactionSigningPanel,
  "signed-history":           RealTransactionSigningPanel,

  // Wallet - Hardware wallets
  "hardware-wallet":          HardwareWalletPanel,
  "ledger":                   HardwareWalletPanel,
  "trezor":                   HardwareWalletPanel,
  "hardware-device":          HardwareWalletPanel,
  "device-connection":        HardwareWalletPanel,
  "bip44-paths":              HardwareWalletPanel,
  "hardware-security":        HardwareWalletPanel,

  // Wallet - Multi-signature
  "multi-signature":          MultiSignaturePanel,
  "multisig":                 MultiSignaturePanel,
  "multisig-wallet":          MultiSignaturePanel,
  "msig-approval":            MultiSignaturePanel,
  "co-signers":               MultiSignaturePanel,
  "threshold-approval":       MultiSignaturePanel,

  // Social - CRM Backend
  "crm-backend":              RealCrmBackendPanel,
  "contact-manager":          RealCrmBackendPanel,
  "crm-contacts":             RealCrmBackendPanel,
  "sqlite-sync":              RealCrmBackendPanel,
  "websocket-sync":           RealCrmBackendPanel,
  "contact-tags":             RealCrmBackendPanel,
  "contact-database":         RealCrmBackendPanel,

  // Social - E2E Encrypted Messages
  "e2e-messages":             E2eMessagesPanel,
  "encrypted-messages":       E2eMessagesPanel,
  "signal-protocol":          E2eMessagesPanel,
  "x3dh":                     E2eMessagesPanel,
  "encrypted-conversations":  E2eMessagesPanel,
  "key-exchange":             E2eMessagesPanel,

  // Social - Real-time Notifications
  "real-time-notifications":  RealTimeNotificationsPanel,
  "notifications":            RealTimeNotificationsPanel,
  "websocket-push":           RealTimeNotificationsPanel,
  "notification-queue":       RealTimeNotificationsPanel,
  "push-delivery":            RealTimeNotificationsPanel,
  "notification-settings":    RealTimeNotificationsPanel,
  "notification-types":       RealTimeNotificationsPanel,

  // Social - Communities
  "communities":              CommunitiesPanel,
  "community-feed":           CommunitiesPanel,
  "reddit-equivalent":        CommunitiesPanel,
  "topic-communities":        CommunitiesPanel,
  "community-moderation":     CommunitiesPanel,
  "community-posts":          CommunitiesPanel,
  "community-mods":           CommunitiesPanel,

  // Social - Creator Monetization Premium
  "creator-monetization-premium": CreatorMonetizationPremiumPanel,
  "subscriptions":            CreatorMonetizationPremiumPanel,
  "subscription-tiers":       CreatorMonetizationPremiumPanel,
  "donation-pool":            CreatorMonetizationPremiumPanel,
  "revenue-splitting":        CreatorMonetizationPremiumPanel,
  "creator-revenue":          CreatorMonetizationPremiumPanel,
  "tipping-pool":             CreatorMonetizationPremiumPanel,

  // Documentation - Interactive Code Playground
  "code-playground":          InteractiveCodePlaygroundPanel,
  "interactive-playground":   InteractiveCodePlaygroundPanel,
  "x3-lang-ide":              InteractiveCodePlaygroundPanel,
  "browser-ide":              InteractiveCodePlaygroundPanel,
  "compile-deploy":           InteractiveCodePlaygroundPanel,
  "smart-contract-compiler":  InteractiveCodePlaygroundPanel,
  "testnet-deploy":           InteractiveCodePlaygroundPanel,

  // Documentation - SDK Code Generator
  "sdk-generator":            SdkCodeGeneratorPanel,
  "code-generator":           SdkCodeGeneratorPanel,
  "abi-codegen":              SdkCodeGeneratorPanel,
  "typescript-sdk":           SdkCodeGeneratorPanel,
  "python-sdk":               SdkCodeGeneratorPanel,
  "go-sdk":                   SdkCodeGeneratorPanel,
  "sdk-codegen":              SdkCodeGeneratorPanel,

  /* ── Sprint 12: Privacy, Analytics, Marketplace, Governance, Infrastructure ── */
  
  // Security - Privacy Vault
  "privacy-vault":            PrivacyVaultPanel,
  "encrypted-keys":           PrivacyVaultPanel,
  "stealth-addresses":        PrivacyVaultPanel,
  "private-key-vault":        PrivacyVaultPanel,
  "key-encryption":           PrivacyVaultPanel,
  "biometric-unlock":         PrivacyVaultPanel,

  // Analytics - Advanced Portfolio
  "advanced-portfolio-analytics": AdvancedPortfolioAnalyticsPanel,
  "portfolio-analytics":       AdvancedPortfolioAnalyticsPanel,
  "sharpe-ratio":             AdvancedPortfolioAnalyticsPanel,
  "volatility-analysis":      AdvancedPortfolioAnalyticsPanel,
  "correlation-matrix":       AdvancedPortfolioAnalyticsPanel,
  "asset-correlation":        AdvancedPortfolioAnalyticsPanel,
  "risk-metrics":             AdvancedPortfolioAnalyticsPanel,

  // Analytics - OnChain Analytics
  "onchain-analytics":        OnChainAnalyticsPanel,
  "token-flows":              OnChainAnalyticsPanel,
  "transaction-analysis":     OnChainAnalyticsPanel,
  "tvl-metrics":              OnChainAnalyticsPanel,
  "trading-volume":           OnChainAnalyticsPanel,
  "smart-contract-calls":     OnChainAnalyticsPanel,
  "holder-distribution":      OnChainAnalyticsPanel,

  // Marketplace - NFT
  "nft-marketplace":          NftMarketplacePanel,
  "nft-collections":          NftMarketplacePanel,
  "nft-trading":              NftMarketplacePanel,
  "rarity-ranking":           NftMarketplacePanel,
  "floor-price":              NftMarketplacePanel,
  "nft-offers":               NftMarketplacePanel,
  "nft-sales":                NftMarketplacePanel,

  // Marketplace - Token
  "token-marketplace":        TokenMarketplacePanel,
  "token-listings":           TokenMarketplacePanel,
  "token-launches":           TokenMarketplacePanel,
  "launchpad":                TokenMarketplacePanel,
  "token-discovery":          TokenMarketplacePanel,
  "trading-pairs":            TokenMarketplacePanel,

  // Governance - Proposals
  "governance-proposals":     GovernanceProposalsPanel,
  "dao-proposals":            GovernanceProposalsPanel,
  "voting-interface":         GovernanceProposalsPanel,
  "proposal-details":         GovernanceProposalsPanel,
  "quorum-tracker":           GovernanceProposalsPanel,
  "voting-power":             GovernanceProposalsPanel,
  "proposal-timeline":        GovernanceProposalsPanel,

  // Governance - Treasury
  "treasury-management":      TreasuryManagementPanel,
  "treasury-allocation":      TreasuryManagementPanel,
  "multisig-wallets":         TreasuryManagementPanel,
  "spending-history":         TreasuryManagementPanel,
  "budget-tracking":          TreasuryManagementPanel,
  "fund-allocation":          TreasuryManagementPanel,
  "approval-workflows":       TreasuryManagementPanel,

  // Infrastructure - Integration Marketplace
  "integration-marketplace":  IntegrationMarketplacePanel,
  "plugin-discovery":         IntegrationMarketplacePanel,
  "dex-integrations":         IntegrationMarketplacePanel,
  "oracle-integrations":      IntegrationMarketplacePanel,
  "lending-integrations":     IntegrationMarketplacePanel,
  "plugin-stats":             IntegrationMarketplacePanel,
  "developer-ecosystem":      IntegrationMarketplacePanel,

  // Infrastructure - Quantum Security
  "quantum-security":         QuantumSecurityPanel,
  "post-quantum-crypto":      QuantumSecurityPanel,
  "lattice-algorithms":       QuantumSecurityPanel,
  "key-migration":            QuantumSecurityPanel,
  "quantum-readiness":        QuantumSecurityPanel,
  "security-audits":          QuantumSecurityPanel,
  "migration-timeline":       QuantumSecurityPanel,

  // Media - Streaming
  "media-streaming":          MediaStreamingPanel,
  "music-streaming":          MediaStreamingPanel,
  "micropayments":            MediaStreamingPanel,
  "stream-analytics":         MediaStreamingPanel,
  "creator-profile":          MediaStreamingPanel,
  "content-monetization":     MediaStreamingPanel,

  /* ── Sprint 13 Phase 2: Performance, Terminal, Infrastructure, CRM, Social, Security, Growth ── */
  
  // Performance - Virtualization
  "virtualized-panels":       VirtualizedPanelPanel,
  "panel-virtualization":     VirtualizedPanelPanel,
  "webworker-offloading":     VirtualizedPanelPanel,
  "gpu-compositing":          VirtualizedPanelPanel,
  "startup-optimization":     VirtualizedPanelPanel,
  "memory-leak-audit":        VirtualizedPanelPanel,
  "performance-metrics":      VirtualizedPanelPanel,

  // Terminal - Shell & CLI
  "x3-terminal":              X3TerminalPanel,
  "terminal-shell":           X3TerminalPanel,
  "x3-cli":                   X3TerminalPanel,
  "command-line":             X3TerminalPanel,
  "x3-repl":                  X3TerminalPanel,
  "shell-emulation":          X3TerminalPanel,
  "cli-reference":            X3TerminalPanel,

  // Infrastructure - Validator Alerts
  "validator-alerts":         ValidatorAlertsPanel,
  "alert-monitoring":         ValidatorAlertsPanel,
  "validator-uptime":         ValidatorAlertsPanel,
  "block-production":         ValidatorAlertsPanel,
  "slashing-alerts":          ValidatorAlertsPanel,
  "validator-status":         ValidatorAlertsPanel,
  "threshold-monitoring":     ValidatorAlertsPanel,

  // Infrastructure - Geographic Distribution
  "geo-distribution":         GeoDistributionPanel,
  "validator-map":            GeoDistributionPanel,
  "world-map-validators":     GeoDistributionPanel,
  "regional-stats":           GeoDistributionPanel,
  "validator-regions":        GeoDistributionPanel,
  "geographic-clustering":    GeoDistributionPanel,
  "network-coverage":         GeoDistributionPanel,

  // Infrastructure - RPC Keys
  "rpc-keys":                 RpcKeysPanel,
  "api-keys":                 RpcKeysPanel,
  "access-keys":              RpcKeysPanel,
  "rate-limiting":            RpcKeysPanel,
  "key-management":           RpcKeysPanel,
  "api-permissions":          RpcKeysPanel,
  "usage-analytics":          RpcKeysPanel,

  // CRM - Deal Pipeline
  "deal-pipeline":            DealPipelinePanel,
  "sales-kanban":             DealPipelinePanel,
  "deal-stages":              DealPipelinePanel,
  "win-probability":          DealPipelinePanel,
  "sales-forecast":           DealPipelinePanel,
  "deal-analytics":           DealPipelinePanel,
  "crm-pipeline":             DealPipelinePanel,

  // CRM - Hardware Acquisition
  "hardware-acquisition":     HardwareAcquisitionPanel,
  "gpu-sourcing":             HardwareAcquisitionPanel,
  "hardware-contacts":        HardwareSourcesPanel,
  "hardware-sources":         HardwareSourcesPanel,
  "acquisition-campaigns":    HardwareAcquisitionPanel,
  "hardware-roi":             HardwareAcquisitionPanel,
  "supplier-management":      HardwareAcquisitionPanel,
  "contact-browser":          HardwareSourcesPanel,
  "supplier-contacts":        HardwareSourcesPanel,
  "200-contacts":             HardwareSourcesPanel,

  // Social - E2E Messaging
  "e2e-messaging":            E2EMessagingPanel,
  "direct-messages":          E2EMessagingPanel,
  "encrypted-chat":           E2EMessagingPanel,
  "x3dh-protocol":            E2EMessagingPanel,
  "double-ratchet":           E2EMessagingPanel,
  "forward-secrecy":          E2EMessagingPanel,
  "message-encryption":       E2EMessagingPanel,

  // Security - Compliance Checklist
  "compliance-checklist":     ComplianceChecklistPanel,
  "audit-tracking":           ComplianceChecklistPanel,
  "soc2-compliance":          ComplianceChecklistPanel,
  "security-audit":           ComplianceChecklistPanel,
  "regulatory-compliance":    ComplianceChecklistPanel,
  "audit-trail":              ComplianceChecklistPanel,
  "compliance-reports":       ComplianceChecklistPanel,

  // Growth - DAO Governance
  "dao-governance":           DAOGovernancePanel,
  "governance-dashboard":     DAOGovernancePanel,
  "treasury-dao":             DAOGovernancePanel,
  "voting-power-distribution": DAOGovernancePanel,
  "proposal-voting":          DAOGovernancePanel,
  "dao-treasury":             DAOGovernancePanel,

  /* ── ALL MISSING 55+ PANELS: Now Registered ──── */

  // Explorer - Community & Ecosystem
  "ai-swarm":                 AISwarmPanel,
  "swarm-intelligence":       AISwarmPanel,
  "blog":                     BlogPanel,
  "news":                     BlogPanel,
  "bridge":                   BridgePanel,
  "atomic-swap":              BridgePanel,
  "community":                CommunityPanel,
  "community-sub":            CommunitySubPanel,
  "earn":                     EarnPanel,
  "earning-programs":         EarnPanel,
  "ecosystem":                EcosystemPanel,
  "partners":                 EcosystemPanel,
  "explorer-home":            ExplorerHomePanel,
  "explorer-detail":          ExplorerDetailPanel,
  "learn":                    LearnPanel,
  "learn-architecture":       LearnArchitecturePanel,
  "developer-docs":           DevDocsPanel,
  "api-documentation":        DevDocsPanel,
  "network-metrics":          MetricsPanel,
  "network-status":           NetworkPanel2,
  "network-validators":       NetworkValidatorsPanel,
  "portfolio":                PortfolioPanel,
  "portfolio-dashboard":      PortfolioPanel,
  "privacy-policy":           PrivacyPanel,
  "privacy-agreement":        PrivacyPanel,
  "quantum":                  QuantumPanel,
  "quantum-landing":          QuantumPanel,
  "quantum-enhanced":         QuantumEnhancedPanel,
  "security-page":            SecurityPanel2,
  "security-policy":          SecurityPanel2,
  "solutions":                SolutionsDetailPanel,
  "solutions-detail":         SolutionsDetailPanel,
  "stake":                    StakePanel,
  "staking":                  StakePanel,
  "swap":                     SwapPanel,
  "trading-swap":             SwapPanel,
  "treasury-dashboard":       TreasuryPanel,
  "terms":                    TermsPanel,
  "terms-of-service":         TermsPanel,
  "x3-chain":                 X3ChainPanel,
  "x3-blockchain":            X3ChainPanel,
  "x3os":                     X3OSPanel,
  "x3-operating-system":      X3OSPanel,
  "x3star":                   X3StarPanel,
  "x3-star":                  X3StarPanel,
  "x3-pages":                 X3SubPagesPanel,
  "x3-sub-pages":             X3SubPagesPanel,

  // DEX - Complete Coverage
  "dex":                      DexPanel,
  "dex-main":                 DexPanel,
  "dex-swap":                 DexPanel,
  "dex-pools":                DexPoolsPanel,
  "liquidity-pools":          DexPoolsPanel,
  "dex-orderbook":            DexOrderbookPanel,
  "orderbook":                DexOrderbookPanel,
  "advanced-orders":          DexAdvancedOrdersPanel,
  "limit-orders":             DexAdvancedOrdersPanel,
  "stop-loss":                DexAdvancedOrdersPanel,
  "transaction-simulator":    TransactionSimulatorPanel,
  "swap-simulator":           TransactionSimulatorPanel,
  "concentrated-liquidity":   ConcentratedLiquidityPanel,
  "v3-pools":                 ConcentratedLiquidityPanel,
  "lp-nft-marketplace":       LpNftMarketplacePanel,
  "position-nfts":            LpNftMarketplacePanel,

  // DeFi - Vote Escrow & Liquidity
  "vex3":                     VeX3Panel,
  "ve-tokenomics":            VeX3Panel,
  "liquidity-mining":         LiquidityMiningPanel,
  "lm-rewards":               LiquidityMiningPanel,
  "token-launchpad":          TokenLaunchpadPanel,
  "launchpad-tokens":         TokenLaunchpadPanel,

  // Wallet - Privacy & Management
  "privacy-wallet":           PrivacyWalletPanel,
  "stealth-wallet":           PrivacyWalletPanel,
  "token-charts":             TokenChartsPanel,
  "token-price-charts":       TokenChartsPanel,
  "privacy-mode":             PrivacyModePanel,
  "stealth-mode":             PrivacyModePanel,

  // X3 Intelligence - ALL
  "x3-floor-dashboard":       X3FloorDashboardPanel,
  "x3-floor-price":           X3FloorDashboardPanel,
  "x3-agents":                X3AgentsPanel,
  "x3-agent-management":      X3AgentsPanel,
  "x3-bonds":                 X3BondsPanel,
  "x3-bond-marketplace":      X3BondsPanel,
  "x3-guide":                 X3GuidePanel,
  "x3-tutorial":              X3GuidePanel,
  "x3-intents":               X3IntentsPanel,
  "x3-intent-system":         X3IntentsPanel,
  "x3-slashing":              X3SlashingPanel,
  "x3-slash-penalties":       X3SlashingPanel,
  "x3-why":                   X3WhyPanel,
  "x3-why-us":                X3WhyPanel,

  // Social - Creator Economy
  "creator-monetization":     CreatorMonetizationPanel,
  "creator-earnings":         CreatorMonetizationPanel,
  "tipping":                  CreatorMonetizationPanel,
  "creator-tips":             CreatorMonetizationPanel,

  // Trading - Strategy & Bots
  "strategy-builder":         StrategyBuilderPanel,
  "bot-strategy-composer":    StrategyBuilderPanel,
  "backtesting":              BacktestingPanel,
  "backtest-engine":          BacktestingPanel,
  "bot-marketplace":          BotMarketplacePanel,
  "strategy-marketplace":     BotMarketplacePanel,

  // Analytics - Risk & Heatmaps
  "advanced-analytics":       AdvancedAnalyticsPanel,
  "analytics-dashboard":      AdvancedAnalyticsPanel,
  "market-heatmap":           MarketHeatmapPanel,
  "market-heat":              MarketHeatmapPanel,
  "crypto-heatmap":           CryptoHeatmapPanel,
  "token-heatmap":            CryptoHeatmapPanel,

  // Validators
  "validator-globe":          ValidatorGlobe,
  "validator-world-map":      ValidatorGlobe,
  "validators-globe":         ValidatorGlobe,

  // Auctions
  "nft-auction":              NftAuctionPanel,
  "auction-marketplace":      NftAuctionPanel,

  // Marketplace - Real Assets
  "real-marketplace":         RealMarketplacePanel,
  "real-asset-marketplace":   RealMarketplacePanel,

  // Global & Root Panels
  "iframe":                   IframePanel as unknown as ComponentType,
  "embedded":                 IframePanel as unknown as ComponentType,
  "security":                 SecurityPanel,
  "security-vault":           SecurityPanel,
  "governance-voting":        GovernanceVotingPanel,
  "voting":                   GovernanceVotingPanel,

  // Aliases for Discovery (Common Search Terms)
  "apps":                     AppStorePanel,
  "store":                    AppStorePanel,
  "plugins":                  AppStorePanel,
  "monitor":                  MultiMonitorPanel,
  "displays":                 MultiMonitorPanel,
  "analytics":                AdvancedAnalyticsPanel,
  "risk":                     PortfolioRiskPanel,
  "portfolio-risk":           PortfolioRiskPanel,
  "onchain":                  OnChainAnalyticsPanel,
  "nft-market":               NftMarketplacePanel,
  "token-market":             TokenMarketplacePanel,
  "dao":                      DAOGovernancePanel,
  "proposals":                GovernanceProposalsPanel,
  "treasury":                 TreasuryManagementPanel,
  "terminal":                 X3TerminalPanel,
  "cli":                      X3TerminalPanel,
  "shell":                    X3TerminalPanel,

  /* ── Embedded External Apps (full standalone app iframes) ── */
  "inferstructor-dashboard":  InfestructorDashboardPanel,
  "infra-dashboard-embed":    InfraDashboardEmbedPanel,
  "x3-frontend":              X3FrontendPanel,
  "x3-fronend":               X3FrontendPanel,
  "x3-landing":               X3FrontendPanel,
  "modular-dashboard":        ModularDashboardPanel,
  "validators-globe-embed":   ValidatorsGlobeEmbedPanel,
  "wallet-app":               WalletAppPanel,
  "dex-app":                  DexAppPanel,
  "tps-monitor":              TpsMonitorEmbedPanel,
  "mainnet-progress":         MainnetProgressEmbedPanel,
  "swarm-autonomic":          SwarmAutonomicEmbedPanel,
  "x3-intelligence-full":     X3IntelligenceFullPanel,
};

/**
 * Loading spinner shown while a panel chunk is fetched.
 */
const PanelLoader: React.FC = () => (
  <div className="flex items-center justify-center h-full bg-[#0a0a0f]">
    <div className="text-center">
      <div className="inline-block w-5 h-5 border-2 border-[#1a9fb5]/30 border-t-[#1a9fb5] rounded-full animate-spin mb-2" />
      <div className="text-[10px] font-mono text-[#666]">Loading panel...</div>
    </div>
  </div>
);

/**
 * Look up the panel component for a given app ID.
 * Returns null if no dedicated panel exists (WindowManager will show its default placeholder).
 */
export function getPanelForApp(appId: string): React.ReactNode | null {
  const Panel = PANEL_MAP[appId];
  if (!Panel) return null;

  return (
    <Suspense fallback={<PanelLoader />}>
      <Panel />
    </Suspense>
  );
}

/**
 * Check if an app has a dedicated panel registered.
 */
export function hasPanel(appId: string): boolean {
  return appId in PANEL_MAP;
}
