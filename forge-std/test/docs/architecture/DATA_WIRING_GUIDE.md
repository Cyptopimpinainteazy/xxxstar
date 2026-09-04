# Frontend Data Wiring Architecture Guide
**X3 Dashboard Panels — Real Data Integration**
**Version**: 1.0
**Status**: READY FOR IMPLEMENTATION

---

## 🏗️ Architecture Overview

### Current State (Sprint 12 Phase 2)
- ✅ 15 dashboard panels created with mock data
- ✅ Panels registered in panelRegistry.tsx with 75+ keywords
- ✅ Dark theme + Lucide icons + responsive layouts working
- ⏳ **NO live data** — all values hardcoded for UI validation

### Target State (Sprint 13 Phase 2)
- ✅ All panels connected to real data sources
- ✅ WebSocket subscriptions for real-time updates
- ✅ Resilient error handling with graceful fallbacks
- ✅ Performance optimized (caching, memoization, pagination)

---

## 📡 Data Source Mappings

### Tier 1: Blockchain RPC (X3 Native)
**Endpoint**: `process.env.REACT_APP_X3_RPC_URL`

```typescript
// Example RPC calls needed
const getRpcClients = () => ({
  // Validator Metrics
  'system_networkState': () => ws.send({...}),  // ValidatorHealthPanel
  
  // Portfolio/Balance
  'system_accountNext': (address) => ws.send({...}),  // PortfolioAnalysisPanel
  
  // Blockchain Data
  'chain_getBlockHash': (blockNumber) => ws.send({...}),  // BlockExplorerPanel
  
  // Governance
  'pallet_dao_proposals': () => ws.send({...}),  // GovernanceProposalsPanel
  
  // Storage
  'state_getStorage': (storageKey) => ws.send({...}),  // DatastoreManagementPanel
  
  // Contract Calls
  atomic_trade_engine_routes: () => ws.send({...}),  // DEX routing
});
```

### Tier 2: External Price Feeds
**Primary**: CoinGecko (free, no auth required)
**Fallback**: DexScreener (Uniswap + DEX data)

```typescript
// services/priceFeeds.ts
export const fetchPriceData = async (tokenIds: string[]) => {
  try {
    const response = await fetch(
      `https://api.coingecko.com/api/v3/simple/price?ids=${tokenIds.join(',')}&vs_currencies=usd&include_market_cap=true&include_24hr_vol=true&include_24hr_change=true`
    );
    return response.json();
  } catch (error) {
    // Fallback to cached data or DexScreener
    return fetchDexScreenerPrices(tokenIds);
  }
};
```

### Tier 3: NFT & Token Discovery
**OpenNFT API** or **Magic Eden API** (requires auth key in .env)

```typescript
export const fetchNftCollections = async () => {
  const headers = {
    'Authorization': `Bearer ${process.env.REACT_APP_NFT_API_KEY}`,
  };
  
  const response = await fetch(
    'https://api.nftapi.io/collections?sort=floor_price_desc&limit=50',
    { headers }
  );
  return response.json();
};
```

### Tier 4: The Graph Subgraphs
**DEX Data**: `https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3`
**Governance**: `https://api.thegraph.com/subgraphs/name/<x3-governance-subgraph>`

```typescript
export const graphqlQuery = (query: string, variables?: any) => {
  return fetch('https://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3', {
    method: 'POST',
    body: JSON.stringify({ query, variables }),
    headers: { 'Content-Type': 'application/json' },
  }).then(r => r.json());
};
```

### Tier 5: WebSocket Real-Time (X3 RPC)
**Protocol**: JSON-RPC 2.0 over WebSocket
**Connection**: `process.env.REACT_APP_X3_WS_URL`

```typescript
// services/websocket.ts
export class X3WebSocketManager {
  private ws: WebSocket;
  private subscriptions: Map<number, (data: any) => void> = new Map();
  private messageId = 0;

  constructor(endpoint: string) {
    this.ws = new WebSocket(endpoint);
    this.ws.onmessage = (event) => {
      const response = JSON.parse(event.data);
      if (response.method === 'subscription') {
        this.subscriptions.get(response.params.subscription)?.(response.params.result);
      }
    };
  }

  subscribe(method: string, params: any[], callback: (data: any) => void) {
    const id = ++this.messageId;
    this.subscriptions.set(id, callback);
    
    this.ws.send(JSON.stringify({
      jsonrpc: '2.0',
      id,
      method: `${method}_subscribe`,
      params,
    }));
  }

  unsubscribe(subscriptionId: number) {
    this.subscriptions.delete(subscriptionId);
  }
}
```

---

## 🎣 Hook-Based Data Fetching Pattern

### Base Hook Template
```typescript
// hooks/useBlockchainData.ts
import { useEffect, useState, useCallback } from 'react';

interface UseDataHookOptions {
  refetchInterval?: number;  // ms to refetch
  enabled?: boolean;         // enable/disable fetching
  cacheKey?: string;         // localStorage cache key
  cacheTTL?: number;         // cache time-to-live in ms
}

export const useBlockchainData = <T,>(
  fetchFn: () => Promise<T>,
  options: UseDataHookOptions = {}
): { data: T | null; loading: boolean; error: Error | null; refetch: () => void } => {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<Error | null>(null);
  const [lastFetch, setLastFetch] = useState(0);

  const refetch = useCallback(async () => {
    if (!options.enabled && options.enabled !== undefined) return;

    setLoading(true);
    try {
      // Check cache first
      if (options.cacheKey) {
        const cached = localStorage.getItem(options.cacheKey);
        if (cached) {
          const { data: cachedData, timestamp } = JSON.parse(cached);
          if (Date.now() - timestamp < (options.cacheTTL || 60000)) {
            setData(cachedData);
            setLoading(false);
            return;
          }
        }
      }

      const result = await fetchFn();
      setData(result);
      setError(null);

      // Update cache
      if (options.cacheKey) {
        localStorage.setItem(
          options.cacheKey,
          JSON.stringify({ data: result, timestamp: Date.now() })
        );
      }
    } catch (err) {
      setError(err as Error);
      // Try fallback from cache
      if (options.cacheKey) {
        const cached = localStorage.getItem(options.cacheKey);
        if (cached) {
          setData(JSON.parse(cached).data);
        }
      }
    } finally {
      setLoading(false);
      setLastFetch(Date.now());
    }
  }, [fetchFn, options]);

  useEffect(() => {
    refetch();

    if (options.refetchInterval) {
      const interval = setInterval(refetch, options.refetchInterval);
      return () => clearInterval(interval);
    }
  }, [refetch, options.refetchInterval]);

  return { data, loading, error, refetch };
};
```

### Panel-Specific Hooks

```typescript
// hooks/index.ts

// ValidatorHealthPanel
export const useValidatorMetrics = (validatorAddress: string) => {
  return useBlockchainData(
    async () => {
      const response = await fetch(
        `${process.env.REACT_APP_X3_RPC_URL}`,
        // ... RPC call for validator stats
      );
      return response.json();
    },
    {
      refetchInterval: 3000,  // 3 seconds
      cacheKey: `validator_${validatorAddress}`,
      cacheTTL: 3000,
    }
  );
};

// PortfolioAnalysisPanel
export const usePortfolioData = (walletAddress: string) => {
  return useBlockchainData(
    async () => {
      const [balance, prices, stakes] = await Promise.all([
        fetch(`...`).then(r => r.json()),  // Get wallet balances
        fetchPriceData(['x3', 'ethereum', 'bitcoin', 'solana']),  // Get prices
        fetch(`...`).then(r => r.json()),  // Get staking positions
      ]);
      
      // Compute portfolio metrics
      return computePortfolioMetrics({ balance, prices, stakes });
    },
    {
      refetchInterval: 10000,  // 10 seconds
      cacheKey: `portfolio_${walletAddress}`,
      cacheTTL: 5000,
    }
  );
};

// GovernanceProposalsPanel
export const useGovernanceProposals = () => {
  return useBlockchainData(
    () => graphqlQuery(GET_PROPOSALS_QUERY),
    {
      refetchInterval: 30000,  // 30 seconds (block-based updates)
      cacheKey: 'governance_proposals',
      cacheTTL: 30000,
    }
  );
};

// NftMarketplacePanel
export const useNftCollections = (sortBy: 'floor_price' | 'volume' | 'trending') => {
  return useBlockchainData(
    async () => {
      const collections = await fetchNftCollections();
      return sortCollections(collections, sortBy);
    },
    {
      refetchInterval: 60000,  // 1 minute
      cacheKey: `nft_collections_${sortBy}`,
      cacheTTL: 60000,
    }
  );
};

// AnalyticsReportingPanel
export const useOnChainAnalytics = (timeframe: '24h' | '7d' | '30d') => {
  return useBlockchainData(
    async () => {
      const data = await graphqlQuery(GET_ANALYTICS_QUERY, { timeframe });
      return processAnalyticsData(data);
    },
    {
      refetchInterval: 120000,  // 2 minutes
      cacheKey: `analytics_${timeframe}`,
      cacheTTL: 120000,
    }
  );
};
```

---

## 🔄 Real-Time Updates (WebSocket)

### Strategy for Each Panel

**High-Frequency (Real-time, < 1 second updates)**
- PerformanceMonitor (CPU, Memory, Disk)
- AudioVisualizer (FFT data from microphone)

**Medium-Frequency (1-10 seconds)**
- ValidatorHealth (validator metrics)
- TokenMarketplace (price tickers)
- PortfolioAnalysis (balance + price updates)

**Low-Frequency (30+ seconds)**
- GovernanceProposals (block-based, ~12s blocks)
- TreasuryManagement (state changes only)
- NftMarketplace (new listings, sales)

```typescript
// hooks/useRealtimeUpdates.ts
export const useRealtimeValidator = (validatorAddress: string) => {
  const [metrics, setMetrics] = useState(null);

  useEffect(() => {
    const wsManager = new X3WebSocketManager(process.env.REACT_APP_X3_WS_URL);
    
    wsManager.subscribe('validator_updates', [validatorAddress], (data) => {
      setMetrics(data);
    });

    return () => wsManager.unsubscribe(subscriptionId);
  }, [validatorAddress]);

  return metrics;
};
```

---

## 🛡️ Error Handling & Resilience

### Cache-First Pattern
```typescript
// When API fails, use cached data
const getCachedOrFetch = async (key: string, fetch: () => Promise<any>) => {
  try {
    return await fetch();
  } catch (error) {
    const cached = localStorage.getItem(key);
    if (cached) {
      console.warn(`Using cached data for ${key}`, error);
      return JSON.parse(cached).data;
    }
    throw error;
  }
};
```

### Retry Logic with Exponential Backoff
```typescript
export const retryWithBackoff = async (
  fn: () => Promise<any>,
  maxRetries = 3,
  delayMs = 1000
) => {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      await new Promise(r => setTimeout(r, delayMs * Math.pow(2, i)));
    }
  }
};
```

---

## 📊 Example: Wiring PortfolioAnalysisPanel

```typescript
// components/panels/PortfolioAnalysisPanel.tsx
const PortfolioAnalysisPanel = () => {
  const { user } = useAuth();  // Get wallet address
  
  // Fetch portfolio data with all features
  const { data: portfolio, loading, error, refetch } = usePortfolioData(
    user.walletAddress
  );

  // Real-time price updates
  const realtimePrices = useRealtimeValidator(user.walletAddress);

  // Merge data: portfolio + real-time prices
  const displayData = useMemo(() => ({
    ...portfolio,
    prices: realtimePrices || portfolio?.prices,
  }), [portfolio, realtimePrices]);

  if (loading) return <LoadingSpinner />;
  if (error) return <ErrorBoundary error={error} onRetry={refetch} />;

  return (
    <div className="space-y-4">
      <PortfolioCards data={displayData} />
      <AssetAllocationChart data={displayData.assets} />
      <HoldingsTable data={displayData.holdings} />
    </div>
  );
};
```

---

## 📦 Environment Variables Required

Create or update `.env.local`:

```bash
# X3 Blockchain
REACT_APP_X3_RPC_URL=https://rpc.x3chain.io
REACT_APP_X3_WS_URL=wss://ws.x3chain.io

# Testnet (for development)
REACT_APP_X3_TESTNET_RPC=https://rpc.testnet.x3chain.io
REACT_APP_X3_TESTNET_WS=wss://ws.testnet.x3chain.io

# External APIs
REACT_APP_COINGECKO_API_KEY=free  # CoinGecko free tier
REACT_APP_NFT_API_KEY=<your-key>
REACT_APP_DEXSCREENER_API_KEY=free

# The Graph
REACT_APP_GRAPH_ENDPOINT=https://api.thegraph.com/subgraphs/name/

# Feature flags
REACT_APP_ENABLE_REALTIME_WS=true
REACT_APP_ENABLE_CACHING=true
REACT_APP_CACHE_TTL=60000  # 1 minute default
```

---

## ✅ Pre-Implementation Checklist

- [ ] All environment variables configured in CI/CD
- [ ] RPC endpoints validated (test network connectivity)
- [ ] API keys stored securely (GitHub Secrets)
- [ ] Cache strategy documented per panel
- [ ] Error scenarios tested (network failure, timeout, invalid data)
- [ ] WebSocket reconnection logic implemented
- [ ] Rate limiting handled (respect API limits)
- [ ] Performance profiled (no N+1 queries, bundle size)
- [ ] Documentation updated in README
- [ ] Monitoring/alerting set up for API failures

---

## 🚀 Launch Checklist (Week 10)

- [ ] Deploy 10 new panel files
- [ ] Wire all data sources per mapping
- [ ] Enable WebSocket for real-time panels
- [ ] Run E2E tests with real data
- [ ] Performance profile + optimize
- [ ] Security audit (no hardcoded keys)
- [ ] Staging validation (24h run)
- [ ] Production deployment
- [ ] Monitor for errors (first 24h)

---

**Next**: Start implementing hooks for Sprint 13 Phase 2 panels
