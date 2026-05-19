import React, { useEffect, useMemo, useState } from "react";
import { OrderBook, generateOrderBook, PriceSparkline, FlashloanPanel, PriceDifference } from "../components/ArbitrageComponents";
import { Badge, Button } from "../components/UIComponents";
import WalletConnect from "../components/WalletConnect";
import { getProviders, estimateFlashloan } from "../services/flashloans";

const DEFAULT_PROVIDERS = ["Aave", "DyDx", "Balancer", "UniswapV3", "BProtocol"];

export function ArbitragePage() {
  const [midA, setMidA] = useState(1842.12);
  const [midB, setMidB] = useState(1842.62);
  const [providers, setProviders] = useState<string[]>(DEFAULT_PROVIDERS);
  const [enabled, setEnabled] = useState<Record<string, boolean>>(() => {
    const o: Record<string, boolean> = {};
    DEFAULT_PROVIDERS.forEach((p) => (o[p] = true));
    return o;
  });
  const [historyA, setHistoryA] = useState<number[]>([midA]);
  const [historyB, setHistoryB] = useState<number[]>([midB]);

  // simulate price updates
  useEffect(() => {
    const t = setInterval(() => {
      setMidA((m) => +(m + (Math.random() - 0.5) * 0.6).toFixed(4));
      setMidB((m) => +(m + (Math.random() - 0.5) * 0.6).toFixed(4));
      setHistoryA((h) => [...h.slice(-20), +(h[h.length-1] + (Math.random() - 0.5) * 0.6).toFixed(4)]);
      setHistoryB((h) => [...h.slice(-20), +(h[h.length-1] + (Math.random() - 0.5) * 0.6).toFixed(4)]);
    }, 1500);
    return () => clearInterval(t);
  }, []);

  const { bids: bidsA, asks: asksA } = useMemo(() => generateOrderBook(midA, 10, 0.06), [midA]);
  const { bids: bidsB, asks: asksB } = useMemo(() => generateOrderBook(midB, 10, 0.06), [midB]);

  const bestBidA = bidsA[0]?.price ?? 0;
  const bestAskB = asksB[0]?.price ?? 0;

  const spread = useMemo(() => ({ diff: bestAskB - bestBidA, pct: ((bestAskB - bestBidA) / ((bestAskB + bestBidA)/2)) * 100 }), [bestAskB, bestBidA]);

  function toggleProvider(p: string) {
    setEnabled((e) => ({ ...e, [p]: !e[p] }));
  }

  const [estimates, setEstimates] = useState<Record<string, any>>({});

  async function refreshEstimates() {
    const active = Object.keys(enabled).filter((k) => enabled[k]);
    const res: Record<string, any> = {};
    for (const p of active) {
      try {
        const e = await estimateFlashloan(p, Math.abs(spread.diff) * 1000 || 1000);
        res[p] = e;
      } catch (err) {
        res[p] = { error: true };
      }
    }
    setEstimates(res);
  }

  function simulateArbitrage() {
    const active = Object.keys(enabled).filter((k) => enabled[k]);
    const est = Object.entries(estimates).map(([p, e]) => `${p}: ${e.feePct ? (e.feePct*100).toFixed(3)+'%' : '—'}`);
    alert(`Simulating arbitrage across providers: ${active.join(', ')}\nSpread: ${spread.diff.toFixed(4)} USDC (${spread.pct.toFixed(2)}%)\nEstimates: ${est.join('; ')}`);
  }

  return (
    <div className="page">
      <div className="page-header">
        <h1>Crypto Wizards — Arbitrage Desk</h1>
        <span className="subtitle">Order-book style arbitrage monitoring & flashloan options</span>
        <div style={{ marginLeft: 'auto' }}>
          <Button size="sm" variant="primary" onClick={simulateArbitrage}>Simulate Arbitrage</Button>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '2fr 1fr', gap: 16 }}>
        <div className="card">
          <div className="card-header">
            <h2>Order Books</h2>
            <div className="muted">Exchange A vs Exchange B — best prices and depth</div>
          </div>
          <div style={{ display: 'flex', gap: 12, alignItems: 'center' }}>
            <div style={{ flex: 1 }}>
              <h3 style={{ marginBottom: 6 }}>Exchange A • Mid {midA.toFixed(4)}</h3>
              <OrderBook bids={bidsA} asks={asksA} />
              <div style={{ marginTop: 12 }}>
                <PriceSparkline data={historyA.slice(-20)} color="#00d4aa" />
              </div>
            </div>

            <div style={{ width: 220, textAlign: 'center' }}>
              <h4>Price Difference</h4>
              <PriceDifference priceA={midA} priceB={midB} />
              <div style={{ marginTop: 8 }}>
                <div className="muted" style={{ fontSize: 12 }}>Best A bid / Best B ask</div>
                <div style={{ marginTop: 6 }}><strong>{bestBidA.toFixed(4)} / {bestAskB.toFixed(4)}</strong></div>
              </div>
            </div>

            <div style={{ flex: 1 }}>
              <h3 style={{ marginBottom: 6 }}>Exchange B • Mid {midB.toFixed(4)}</h3>
              <OrderBook bids={bidsB} asks={asksB} />
              <div style={{ marginTop: 12 }}>
                <PriceSparkline data={historyB.slice(-20)} color="#4488ff" />
              </div>
            </div>
          </div>
        </div>

        <div>
          <div className="card">
            <div className="card-header">
              <h2>Flashloan Options</h2>
            </div>
            <FlashloanPanel providers={providers} enabled={enabled} onToggle={toggleProvider} />
            <div style={{ marginTop: 16 }}>
              <div style={{ fontSize: 12, color: 'var(--text-muted)' }}>Estimated cost (simulated / provider API)</div>
              <div style={{ marginTop: 8 }}>
                <Button variant="secondary" size="sm" onClick={refreshEstimates}>Refresh Estimates</Button>
              </div>

              <div style={{ marginTop: 12 }}>
                {Object.keys(estimates).length === 0 && <div className="muted">No estimates yet. Click "Refresh Estimates" or run a simulation.</div>}
                {Object.entries(estimates).map(([p, e]) => (
                  <div key={p} style={{ display: 'flex', justifyContent: 'space-between', marginTop: 6 }}>
                    <div>{p}</div>
                    <div className="mono">{e.error ? '—' : `${(e.feePct*100).toFixed(3)}% (max ${e.maxLiquidity.toLocaleString()})`}</div>
                  </div>
                ))}
              </div>
            </div>

            <div style={{ marginTop: 18 }}>
              <h3>Selected Providers</h3>
              <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
                {Object.keys(enabled).filter((k)=>enabled[k]).map((k)=> <Badge key={k} variant="blue">{k}</Badge>)}
              </div>
            </div>
          </div>

          <div className="card" style={{ marginTop: 16 }}>
            <div className="card-header"><h2>Execution</h2></div>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
              <div>
                <div className="muted">This is a simulated desk — execution buttons will hook into production router in later iterations.</div>
                <div style={{ marginTop: 12 }}>
                  <Button variant="primary" onClick={simulateArbitrage}>Run Simulation</Button>
                  <Button variant="danger" size="sm" style={{ marginLeft: 8 }}>Liquidate</Button>
                </div>
              </div>

              <div style={{ width: 220 }}>
                <div style={{ fontSize: 12, color: 'var(--text-muted)', marginBottom: 6 }}>Connect wallet to sign / route</div>
                <WalletConnect onConnect={(info)=>console.log('Connected', info)} />
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
