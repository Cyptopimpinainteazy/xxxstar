# Sprint 13 Phase 2 — Advanced Features & Real Data Integration
**Target: 10 new panels + frontend data wiring strategy**
**Timeline: 4-6 weeks**
**Status: PREPARED & READY FOR LAUNCH**

---

## 📋 Sprint 13 Phase 2 — Panel Creation List (10 panels)

### Tier 1: Privacy & Security (2 panels)
1. **PrivacyVaultPanel**
   - E2E encrypted key vault (ChaCha20-Poly1305)
   - Argon2id KDF with configurable parameters
   - Stealth address generation
   - Key derivation paths (m/44'/0'/0'/0/*)
   - Biometric unlock integration (TouchID/FaceID/Fingerprint)
   - Hardware wallet backup tracking
   - **Data Sources**: Local keystore + Tauri secure storage
   - **Dependencies**: TweetNaCl.js, tweetnacl-util

2. **AdvancedPortfolioAnalyticsPanel**
   - Sharpe ratio calculation (risk-adjusted returns)
   - Maximum drawdown tracking
   - Volatility (annualized std dev)
   - Value at Risk (VaR) at 95%/99% confidence
   - Beta coefficient vs market
   - Correlation matrix between assets
   - Risk score dashboard (1-10)
   - **Data Sources**: Historical price data (CoinGecko API) + wallet holdings
   - **Dependencies**: chart-js, simple-statistics

### Tier 2: Marketplace & Discovery (3 panels)
3. **NftMarketplacePanel**
   - Collection discovery with trending sorting
   - Rarity ranking algorithm (trait score, rarity %), floor price tracking
   - Recent sales history with price trends
   - Buy/offer/list interface
   - Collection stats (volume, floor, holders)
   - Trait filters and floor price heatmaps
   - **Data Sources**: OpenNFT API or Magic Eden API
   - **Dependencies**: wagmi, viem (for wallet signing)

4. **TokenMarketplacePanel**
   - Token listings with market cap ranking
   - 24h volume, 7d returns, price charts
   - Launch tracking (ICO, launch date)
   - Swap pair discovery (X3/USDC, X3/ETH, etc.)
   - Integration with DEX routing (best execution)
   - **Data Sources**: CoinGecko, DexScreener, Uniswap Subgraph
   - **Dependencies**: recharts, axios

5. **GovernanceProposalsPanel**
   - DAO proposal submission form
   - Voting interface with vote tracking
   - Vote breakdown (for/against/abstain percentages)
   - Quorum tracking (current vs required)
   - Proposal timeline visualization
   - Historical proposals archive
   - **Data Sources**: Governance contract RPC calls
   - **Dependencies**: viem, zustand (state management)

### Tier 3: Treasury & Operations (2 panels)
6. **TreasuryManagementPanel**
   - Multi-sig wallet control (setup, approve, execute)
   - Budget allocation by category (engineering, marketing, ops)
   - Spending history with timeline
   - Approval workflows (threshold signatures)
   - Fund tracking and allocation percentage
   - Recipient whitelisting
   - **Data Sources**: Multi-sig contract state + transaction history
   - **Dependencies**: wagmi, zustand

7. **IntegrationMarketplacePanel**
   - Third-party plugin discovery
   - Adoption stats (installs, active users)
   - Rating system (stars, reviews)
   - Category browsing (automation, analytics, trading)
   - Developer ecosystem metrics
   - One-click installation workflow
   - **Data Sources**: Plugin registry contract + github stars API
   - **Dependencies**: axios, lucide-react

### Tier 4: Content & Streaming (2 panels)
8. **MediaStreamingPanel**
   - Decentralized music/video streaming
   - Creator micropayment tracking (per-stream)
   - Stream analytics (listener count, play duration)
   - Creator profiles with earnings
   - Playlist creation and sharing
   - **Data Sources**: IPFS/Arweave for media + contract for micropayments
   - **Dependencies**: hls.js, react-joyride

9. **QuantumSecurityPanel**
   - Post-quantum crypto readiness assessment
   - Lattice algorithm migration status (ML-KEM/Kyber)
   - Key size comparison (classical vs quantum-safe)
   - Security audit results and scores
   - Migration timeline and checklist
   - **Data Sources**: Security audit reports + algorithm status page
   - **Dependencies**: zustand, framer-motion

### Tier 5: Advanced Analytics (1 panel)
10. **OnChainAnalyticsPanel**
    - Real-time TVL (Total Value Locked) tracking per protocol
    - Transaction volume and velocity
    - Gas fee trends (gwei/transaction)
    - Smart contract call monitoring
    - Token holder distribution (top 10, gini coefficient)
    - Trade flow analysis (buy/sell volume)
    - **Data Sources**: Blockchain RPC + The Graph subgraph
    - **Dependencies**: recharts, axios, zustand

---

## 🔌 Frontend Data Wiring Strategy

### Phase 1: API Integration Layer (1-2 weeks)
**Goal**: Centralize all external data source connections

```typescript
// services/dataProviders.ts
export interface DataProvider {
  name: string;
  endpoint: string;
  headers: Record<string, string>;
  retryPolicy: { maxRetries: number; delayMs: number };
  cache: { ttl: number; enabled: boolean };
}

export const dataSources = {
  // Blockchain Data
  rpc: {
    mainnet: 'https://rpc.x3chain.io',
    testnet: 'https://rpc.testnet.x3chain.io',
  },
  
  // Price & Market Data
  coingecko: {
    endpoint: 'https://api.coingecko.com/api/v3',
    cache: { ttl: 60000 }, // 1 minute
  },
  dexscreener: {
    endpoint: 'https://api.dexscreener.com/latest',
    cache: { ttl: 30000 }, // 30 seconds (market-critical)
  },
  
  // NFT Data
  nftApi: {
    endpoint: 'https://api.nftapi.io', // or Magic Eden
    auth: process.env.REACT_APP_NFT_API_KEY,
  },
  
  // Graph Data
  theGraph: {
    endpoint: 'https://api.thegraph.com/subgraphs/name/...',
    cache: { ttl: 120000 }, // 2 minutes
  },
};
```

### Phase 2: React Hooks for Data Fetching (1-2 weeks)
**Goal**: Encapsulate async data fetching with loading/error states

```typescript
// hooks/useBlockchainData.ts
export const usePriceData = (tokenId: string) => {
  const [price, setPrice] = useState<number | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let isMounted = true;
    const fetchPrice = async () => {
      try {
        setLoading(true);
        const res = await fetch(
          `${dataSources.coingecko.endpoint}/simple/price?ids=${tokenId}`
        );
        if (!isMounted) return;
        const data = await res.json();
        setPrice(data[tokenId]?.usd);
      } catch (err) {
        if (isMounted) setError(err.message);
      } finally {
        if (isMounted) setLoading(false);
      }
    };
    fetchPrice();
    return () => { isMounted = false; };
  }, [tokenId]);

  return { price, loading, error };
};

// Similar hooks for:
// - useValidatorMetrics()
// - usePortfolioData()
// - useGovernanceProposals()
// - useNftCollections()
// - useTreasuryBalance()
```

### Phase 3: WebSocket Real-Time Updates (2-3 weeks)
**Goal**: Enable live metric updates for critical panels

```typescript
// services/websocketManager.ts
export class WebSocketManager {
  private ws: WebSocket | null = null;
  private subscriptions = new Map<string, Set<(data: any) => void>>();

  connect() {
    this.ws = new WebSocket(process.env.REACT_APP_WS_ENDPOINT);
    this.ws.onmessage = (event) => {
      const { channel, data } = JSON.parse(event.data);
      this.subscriptions.get(channel)?.forEach(cb => cb(data));
    };
  }

  subscribe(channel: string, callback: (data: any) => void) {
    if (!this.subscriptions.has(channel)) {
      this.subscriptions.set(channel, new Set());
    }
    this.subscriptions.get(channel)!.add(callback);
    this.ws?.send(JSON.stringify({ action: 'subscribe', channel }));
  }

  unsubscribe(channel: string, callback: (data: any) => void) {
    this.subscriptions.get(channel)?.delete(callback);
  }
}

// Usage in panels:
const useRealTimeMetrics = (metricName: string) => {
  const [metrics, setMetrics] = useState(null);
  
  useEffect(() => {
    const wsManager = new WebSocketManager();
    wsManager.connect();
    wsManager.subscribe(`metrics:${metricName}`, setMetrics);
    
    return () => wsManager.unsubscribe(`metrics:${metricName}`, setMetrics);
  }, [metricName]);

  return metrics;
};
```

### Phase 4: State Management (Zustand) (1 week)
**Goal**: Centralize panel state and prevent prop drilling

```typescript
// store/panelStore.ts
import { create } from 'zustand';

export const usePanelStore = create((set) => ({
  // Portfolio Data
  portfolio: { assets: [], totalValue: 0 },
  setPortfolio: (portfolio) => set({ portfolio }),

  // Validator Data
  validators: [],
  setValidators: (validators) => set({ validators }),

  // NFT Collections
  nftCollections: [],
  setNftCollections: (collections) => set({ nftCollections: collections }),

  // Governance Proposals
  proposals: [],
  setProposals: (proposals) => set({ proposals }),

  // Cache TTLs
  cacheTimestamps: {},
  updateCache: (key, timestamp) => set((state) => ({
    cacheTimestamps: { ...state.cacheTimestamps, [key]: timestamp },
  })),
}));

// Usage in panels:
const PortfolioAnalysisPanel = () => {
  const { portfolio, setPortfolio } = usePanelStore();
  
  useEffect(() => {
    // Fetch and update
    fetchPortfolioData().then(setPortfolio);
  }, []);
};
```

### Phase 5: Error Handling & Resilience (1 week)
**Goal**: Graceful degradation, caching, fallback patterns

```typescript
// services/errorHandler.ts
export const withErrorBoundary = async (
  fetchFn: () => Promise<any>,
  options: {
    cacheKey?: string;
    fallback?: any;
    retry?: { maxAttempts: number; delayMs: number };
    timeout?: number;
  }
) => {
  // Try fetching with timeout
  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), options.timeout || 10000);
    
    const result = await fetchFn();
    clearTimeout(timeoutId);
    
    // Update cache
    if (options.cacheKey) {
      localStorage.setItem(options.cacheKey, JSON.stringify(result));
    }
    return result;
  } catch (error) {
    // Retry logic
    if (options.retry?.maxAttempts > 0) {
      await new Promise(r => setTimeout(r, options.retry!.delayMs));
      return withErrorBoundary(fetchFn, {
        ...options,
        retry: { ...options.retry, maxAttempts: options.retry.maxAttempts - 1 },
      });
    }
    
    // Fallback to cache or provided fallback
    if (options.cacheKey) {
      const cached = localStorage.getItem(options.cacheKey);
      if (cached) return JSON.parse(cached);
    }
    
    return options.fallback || null;
  }
};
```

### Phase 6: Performance Optimization (1 week)
**Goal**: Memoization, pagination, lazy loading

```typescript
// hooks/useLazyLoad.ts
export const useLazyLoadTable = (data: any[], pageSize: number = 50) => {
  const [displayedCount, setDisplayedCount] = useState(pageSize);

  const handleLoadMore = () => {
    setDisplayedCount(prev => prev + pageSize);
  };

  return {
    displayed: data.slice(0, displayedCount),
    hasMore: displayedCount < data.length,
    loadMore: handleLoadMore,
  };
};

// Memoization for expensive components
export const PortfolioChart = memo(({ data }) => {
  return <ResponsiveChart data={data} />;
}, (prev, next) => {
  return JSON.stringify(prev.data) === JSON.stringify(next.data);
});
```

---

## 🎯 Implementation Roadmap

| Week | Phase | Deliverables |
|------|-------|--------------|
| W1-2 | API Integration Layer | DataProvider abstraction, centralized config, error handling |
| W3-4 | Data Fetching Hooks | `usePriceData`, `usePortfolioData`, `useGovernanceProposals`, etc. |
| W5-6 | WebSocket Integration | Real-time updates for metrics, prices, votes |
| W7 | Zustand State Store | Global state for all 10 panels + cache management |
| W8-9 | Panel Implementation | Implement all 10 panels with real data sources |
| W10 | Testing & Optimization | E2E tests, performance profiling, load testing |

---

## 🔗 Data Source Matrix

| Panel | Primary Source | Secondary | Cache TTL | Update Freq |
|-------|---|---|---|---|
| PrivacyVault | Local keystore (Tauri) | — | N/A | On-demand |
| AdvancedPortfolioAnalytics | CoinGecko API | Wallet RPC | 60s | 60s intervals |
| NftMarketplace | OpenNFT/Magic Eden | IPFS | 120s | Real-time WS |
| TokenMarketplace | CoinGecko + DexScreener | Uniswap Graph | 30s | Real-time WS |
| GovernanceProposals | Governance contract RPC | The Graph | 300s | Block-based |
| TreasuryManagement | Multi-sig contract RPC | Tx history | 600s | On-demand |
| IntegrationMarketplace | Plugin registry contract | GitHub API | 3600s | Daily |
| MediaStreaming | IPFS/Arweave | Micropay contract | 180s | Listener-based |
| QuantumSecurity | Local config + audit reports | — | N/A | On-demand |
| OnChainAnalytics | X3 RPC + The Graph | Dune Analytics | 120s | Real-time WS |

---

## 🧪 Testing Strategy

### Unit Tests
- Test each data fetching hook independently
- Mock API responses with jest-mock-fetch
- Test error scenarios (timeout, 4xx, 5xx)

### Integration Tests
- Test panel + data source combinations
- Verify cache invalidation logic
- Test WebSocket reconnection

### E2E Tests
- Load panels, verify data renders
- Trigger real API calls in testnet
- Simulate network failures

---

## 📝 Dependencies to Add

```json
{
  "depend-on": [
    "@tanstack/react-query",
    "zustand",
    "axios",
    "recharts",
    "framer-motion",
    "tweetnacl",
    "tweetnacl-util",
    "simple-statistics",
    "chart.js",
    "react-chartjs-2",
    "viem",
    "wagmi",
    "hls.js"
  ]
}
```

---

## Next Actions
1. ✅ Sprint 12 committed to git
2. ⏳ Create all 10 Sprint 13 Phase 2 panel files (5-7 days)
3. ⏳ Build data integration layer (services + hooks)
4. ⏳ Integrate WebSocket for real-time updates
5. ⏳ Test panels with real data sources
6. ⏳ Deploy to testnet for validation

---

**Prepared by**: GitHub Copilot (Claude Haiku 4.5)
**Creation Date**: March 1, 2026
**Status**: READY FOR DEVELOPMENT
