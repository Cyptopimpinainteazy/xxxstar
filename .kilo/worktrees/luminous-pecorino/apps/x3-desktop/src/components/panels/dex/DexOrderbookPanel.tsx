/**
 * DexOrderbookPanel — full orderbook DEX with price ladder, depth chart, and order entry.
 *
 * Features:
 * - Live orderbook with bid/ask sides
 * - Spread indicator and mid-price
 * - Limit & market order entry
 * - Recent fills / trade history
 * - Pair selector
 */
import React, { useEffect, useMemo, useRef, useState } from 'react';
import { createChart, type CandlestickData, type IChartApi, type UTCTimestamp } from 'lightweight-charts';
import x3Chain, { TOKEN_IDS, type PriceObservationPoint } from '@/services/x3ChainService';

/* ── Types ─────────────────────────────────────────── */
interface OrderLevel {
  price: number;
  size: number;
  total: number;
}

interface Fill {
  id: number;
  price: number;
  size: number;
  side: 'buy' | 'sell';
  time: string;
}

interface Pair {
  symbol: string;
  base: string;
  quote: string;
  lastPrice: number;
  change24h: number;
  high24h: number;
  low24h: number;
  vol24h: string;
}

/* ── Mock Data ─────────────────────────────────────── */
const PAIRS: Pair[] = [
  { symbol: 'X3/USDC', base: 'X3', quote: 'USDC', lastPrice: 1.2512, change24h: 5.23, high24h: 1.2890, low24h: 1.1845, vol24h: '4.2M' },
  { symbol: 'ETH/USDC',   base: 'ETH',   quote: 'USDC', lastPrice: 3245.80, change24h: -1.12, high24h: 3302.00, low24h: 3189.50, vol24h: '12.8M' },
  { symbol: 'SOL/USDC',   base: 'SOL',   quote: 'USDC', lastPrice: 178.42, change24h: 3.71, high24h: 182.10, low24h: 171.30, vol24h: '6.1M' },
  { symbol: 'X3/ETH',  base: 'X3', quote: 'ETH',  lastPrice: 0.000385, change24h: 6.14, high24h: 0.000398, low24h: 0.000361, vol24h: '890K' },
  { symbol: 'SOL/ETH',    base: 'SOL',   quote: 'ETH',  lastPrice: 0.05495, change24h: 4.88, high24h: 0.05610, low24h: 0.05220, vol24h: '310K' },
];

const CANDLE_BUCKET_SECONDS = 60;

function buildCandles(observations: PriceObservationPoint[], fallbackPrice: number): CandlestickData[] {
  if (!observations.length) {
    const now = Math.floor(Date.now() / 1000) as UTCTimestamp;
    return [{
      time: now,
      open: fallbackPrice,
      high: fallbackPrice,
      low: fallbackPrice,
      close: fallbackPrice,
    }];
  }

  const buckets = new Map<number, CandlestickData>();

  observations.forEach(obs => {
    const bucket = Math.floor(obs.timestamp / CANDLE_BUCKET_SECONDS);
    const candleTime = (bucket * CANDLE_BUCKET_SECONDS) as UTCTimestamp;
    const existing = buckets.get(bucket);
    if (!existing) {
      buckets.set(bucket, {
        time: candleTime,
        open: obs.price,
        high: obs.price,
        low: obs.price,
        close: obs.price,
      });
    } else {
      existing.high = Math.max(existing.high, obs.price);
      existing.low = Math.min(existing.low, obs.price);
      existing.close = obs.price;
    }
  });

  return Array.from(buckets.values()).sort((a, b) => (a.time as number) - (b.time as number));
}

function generateOrderbook(mid: number): { asks: OrderLevel[]; bids: OrderLevel[] } {
  const asks: OrderLevel[] = [];
  const bids: OrderLevel[] = [];
  let askTotal = 0;
  let bidTotal = 0;
  const step = mid * 0.001;
  for (let i = 0; i < 15; i++) {
    const askSize = Math.round((Math.random() * 5000 + 200));
    askTotal += askSize;
    asks.push({ price: +(mid + step * (i + 1)).toFixed(6), size: askSize, total: askTotal });

    const bidSize = Math.round((Math.random() * 5000 + 200));
    bidTotal += bidSize;
    bids.push({ price: +(mid - step * (i + 1)).toFixed(6), size: bidSize, total: bidTotal });
  }
  return { asks: asks.reverse(), bids }; // asks high→low (reversed for display top-down)
}

const MOCK_FILLS: Fill[] = [
  { id: 1,  price: 1.2512, size: 3200,  side: 'buy',  time: '14:32:01' },
  { id: 2,  price: 1.2508, size: 1800,  side: 'sell', time: '14:31:58' },
  { id: 3,  price: 1.2510, size: 5400,  side: 'buy',  time: '14:31:55' },
  { id: 4,  price: 1.2505, size: 900,   side: 'sell', time: '14:31:52' },
  { id: 5,  price: 1.2515, size: 2200,  side: 'buy',  time: '14:31:48' },
  { id: 6,  price: 1.2503, size: 4100,  side: 'sell', time: '14:31:44' },
  { id: 7,  price: 1.2518, size: 1500,  side: 'buy',  time: '14:31:40' },
  { id: 8,  price: 1.2500, size: 6800,  side: 'sell', time: '14:31:36' },
  { id: 9,  price: 1.2520, size: 980,   side: 'buy',  time: '14:31:30' },
  { id: 10, price: 1.2498, size: 3300,  side: 'sell', time: '14:31:25' },
];

/* ── Component ─────────────────────────────────────── */
const DexOrderbookPanel: React.FC = () => {
  const [selectedPair, setSelectedPair] = useState(PAIRS[0]);
  const [orderType, setOrderType] = useState<'limit' | 'market'>('limit');
  const [orderSide, setOrderSide] = useState<'buy' | 'sell'>('buy');
  const [limitPrice, setLimitPrice] = useState(selectedPair.lastPrice.toString());
  const [orderSize, setOrderSize] = useState('');
  const [showPairList, setShowPairList] = useState(false);
  const [tab, setTab] = useState<'book' | 'fills'>('book');
  const [chartCandles, setChartCandles] = useState<CandlestickData[]>([]);
  const [chartLoading, setChartLoading] = useState(true);
  const [chartError, setChartError] = useState<string | null>(null);
  const [trading, setTrading] = useState(false);
  const [tradeStatus, setTradeStatus] = useState<string | null>(null);
  const [fills, setFills] = useState<Fill[]>(MOCK_FILLS);

  const orderbook = useMemo(() => generateOrderbook(selectedPair.lastPrice), [selectedPair]);

  useEffect(() => {
    const unsub = x3Chain.subscribeTrades((trade) => {
      setFills(prev => [{
        id: Math.random(),
        price: trade.price,
        size: trade.size,
        side: trade.side,
        time: trade.time,
      } as Fill, ...prev].slice(0, 50));
    });
    return () => unsub();
  }, []);

  const maxTotal = Math.max(
    orderbook.asks[0]?.total ?? 0,
    orderbook.bids[orderbook.bids.length - 1]?.total ?? 0,
  );

  const spread = orderbook.bids[0] && orderbook.asks[orderbook.asks.length - 1]
    ? +(orderbook.asks[orderbook.asks.length - 1].price - orderbook.bids[0].price).toFixed(6)
    : 0;

  const estTotal = orderSize
    ? (parseFloat(orderSize) * (orderType === 'limit' ? parseFloat(limitPrice) : selectedPair.lastPrice)).toFixed(4)
    : '0';

  const selectPair = (p: Pair) => {
    setSelectedPair(p);
    setLimitPrice(p.lastPrice.toString());
    setShowPairList(false);
  };

  const handlePlaceOrder = async () => {
    if (!orderSize || parseFloat(orderSize) <= 0) return;
    setTrading(true);
    setTradeStatus('Preparing trade...');
    
    try {
      const isBuy = orderSide === 'buy';
      const tokenInKey = isBuy ? selectedPair.quote : selectedPair.base;
      const tokenOutKey = isBuy ? selectedPair.base : selectedPair.quote;
      
      const tokenIn = TOKEN_IDS[tokenInKey];
      const tokenOut = TOKEN_IDS[tokenOutKey];
      
      if (!tokenIn || !tokenOut) throw new Error('Invalid pair identifiers');
      
      const decIn = x3Chain.getAssetDecimals(tokenIn);
      const decOut = x3Chain.getAssetDecimals(tokenOut);
      
      const amountInBig = x3Chain.toChainUnits(orderSize, decIn);
      const priceVal = orderType === 'limit' ? parseFloat(limitPrice) : selectedPair.lastPrice;
      
      const expectedOut = isBuy 
        ? parseFloat(orderSize) / priceVal
        : parseFloat(orderSize) * priceVal;
      
      const minAmountOutBig = x3Chain.toChainUnits(expectedOut * 0.98, decOut); // 2% tolerance

      await x3Chain.submitSwap(
        '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY', // Alice
        [{
          vmType: 2, // X3
          tokenIn,
          tokenOut,
          amountIn: BigInt(amountInBig),
          minAmountOut: BigInt(minAmountOutBig),
          deadline: Math.floor(Date.now() / 1000) + 3600,
        }],
        200, // 2%
        (status) => setTradeStatus(`${status}...`),
        (event) => {
          if (event.type === 'batch_created') setTradeStatus(`Batch: ${event.batchId.slice(0, 8)}`);
        }
      );
      setTradeStatus('Trade successful!');
    } catch (err: any) {
      setTradeStatus(`Failed: ${err.message?.slice(0, 20)}`);
    } finally {
      setTimeout(() => {
        setTrading(false);
        setTradeStatus(null);
      }, 4000);
    }
  };

  const formatPrice = (n: number) => {
    if (n >= 1) return n.toFixed(4);
    if (n >= 0.001) return n.toFixed(6);
    return n.toFixed(8);
  };

  useEffect(() => {
    let active = true;
    const fetchCandles = async () => {
      setChartLoading(true);
      setChartError(null);
      try {
        const tokenIn = TOKEN_IDS[selectedPair.base] ?? TOKEN_IDS.X3;
        const tokenOut = TOKEN_IDS[selectedPair.quote] ?? TOKEN_IDS.USDC;
        const observations = await x3Chain.getPriceObservations(tokenIn, tokenOut, 240);
        if (!active) return;
        const candles = buildCandles(observations, selectedPair.lastPrice);
        setChartCandles(candles);
      } catch (err: any) {
        if (!active) return;
        setChartError(err?.message ?? 'Unable to load TWAP data');
        setChartCandles(buildCandles([], selectedPair.lastPrice));
      } finally {
        if (!active) return;
        setChartLoading(false);
      }
    };

    fetchCandles();
    const interval = setInterval(() => {
      fetchCandles();
    }, 30_000);

    return () => {
      active = false;
      clearInterval(interval);
    };
  }, [selectedPair.base, selectedPair.lastPrice, selectedPair.quote]);

  /* ── Styles (inline for panel context) ── */
  const s = {
    root: { display: 'flex', flexDirection: 'column' as const, height: '100%', background: '#0a0e17', color: '#e0e0e0', fontFamily: 'monospace', fontSize: '0.78rem', overflow: 'hidden' },
    header: { display: 'flex', alignItems: 'center', gap: 8, padding: '8px 12px', borderBottom: '1px solid #1a1f2e', flexShrink: 0 },
    body: { display: 'flex', flex: 1, overflow: 'hidden' },
    bookCol: { flex: 1, display: 'flex', flexDirection: 'column' as const, overflow: 'hidden' },
    chartSection: { borderBottom: '1px solid #1a1f2e', padding: '10px 12px', flexShrink: 0 },
    chartHeader: { display: 'flex', justifyContent: 'space-between', alignItems: 'baseline', marginBottom: 6, fontSize: '0.72rem', color: '#777' },
    bookContent: { flex: 1, display: 'flex', flexDirection: 'column' as const, overflow: 'hidden' },
    orderCol: { width: 240, borderLeft: '1px solid #1a1f2e', display: 'flex', flexDirection: 'column' as const, overflow: 'auto', padding: '8px 10px' },
    row: { display: 'flex', alignItems: 'center', padding: '1px 10px', position: 'relative' as const, cursor: 'pointer' },
    spreadBar: { textAlign: 'center' as const, padding: '4px 0', fontSize: '0.7rem', color: '#888', borderTop: '1px solid #1a1f2e', borderBottom: '1px solid #1a1f2e', flexShrink: 0 },
  };

  return (
    <div style={s.root}>
      {/* ── Header: Pair selector + stats ── */}
      <div style={s.header}>
        <div style={{ position: 'relative' }}>
          <button
            onClick={() => setShowPairList(!showPairList)}
            style={{ background: '#141824', border: '1px solid #2a2f3e', borderRadius: 6, padding: '4px 10px', color: '#fff', fontWeight: 700, cursor: 'pointer', fontSize: '0.85rem' }}
          >
            {selectedPair.symbol} ▾
          </button>
          {showPairList && (
            <>
              <div style={{ position: 'fixed', inset: 0, zIndex: 40 }} onClick={() => setShowPairList(false)} />
              <div style={{ position: 'absolute', top: 32, left: 0, zIndex: 50, background: '#141824', border: '1px solid #2a2f3e', borderRadius: 8, minWidth: 200, padding: 4 }}>
                {PAIRS.map(p => (
                  <button key={p.symbol} onClick={() => selectPair(p)}
                    style={{ display: 'block', width: '100%', textAlign: 'left', padding: '6px 10px', background: 'transparent', border: 'none', color: p.symbol === selectedPair.symbol ? '#00e5ff' : '#ccc', cursor: 'pointer', fontSize: '0.8rem', borderRadius: 4 }}
                    onMouseOver={e => (e.currentTarget.style.background = '#1e2436')}
                    onMouseOut={e => (e.currentTarget.style.background = 'transparent')}
                  >
                    <span style={{ fontWeight: 600 }}>{p.symbol}</span>
                    <span style={{ float: 'right', color: p.change24h >= 0 ? '#4caf50' : '#ef5350' }}>
                      {p.change24h >= 0 ? '+' : ''}{p.change24h.toFixed(2)}%
                    </span>
                  </button>
                ))}
              </div>
            </>
          )}
        </div>
        <span style={{ fontWeight: 700, fontSize: '1rem', color: selectedPair.change24h >= 0 ? '#4caf50' : '#ef5350' }}>
          {formatPrice(selectedPair.lastPrice)}
        </span>
        <span style={{ color: selectedPair.change24h >= 0 ? '#4caf50' : '#ef5350', fontSize: '0.75rem' }}>
          {selectedPair.change24h >= 0 ? '▲' : '▼'} {Math.abs(selectedPair.change24h).toFixed(2)}%
        </span>
        <div style={{ flex: 1 }} />
        <div style={{ display: 'flex', gap: 12, fontSize: '0.7rem', color: '#777' }}>
          <span>H: {formatPrice(selectedPair.high24h)}</span>
          <span>L: {formatPrice(selectedPair.low24h)}</span>
          <span>Vol: {selectedPair.vol24h}</span>
        </div>
      </div>

      {/* ── Tabs ── */}
      <div style={{ display: 'flex', borderBottom: '1px solid #1a1f2e', flexShrink: 0 }}>
        {(['book', 'fills'] as const).map(t => (
          <button key={t} onClick={() => setTab(t)}
            style={{ flex: 1, padding: '5px 0', background: 'transparent', border: 'none', borderBottom: tab === t ? '2px solid #00e5ff' : '2px solid transparent', color: tab === t ? '#00e5ff' : '#777', cursor: 'pointer', fontWeight: 600, textTransform: 'uppercase', fontSize: '0.7rem' }}>
            {t === 'book' ? '📊 Orderbook' : '🔄 Trades'}
          </button>
        ))}
      </div>

      {/* ── Body ── */}
      <div style={s.body}>
        {/* Left: Book or Fills */}
        <div style={s.bookCol}>
          <div style={s.chartSection}>
            <div style={s.chartHeader}>
              <span style={{ fontWeight: 700, color: '#e0e0e0' }}>TWAP Candlesticks</span>
              <span>{selectedPair.base}/{selectedPair.quote}</span>
            </div>
            <TwapChart data={chartCandles} loading={chartLoading} error={chartError} pairLabel={selectedPair.symbol} />
            <div style={{ marginTop: 6, fontSize: '0.7rem', color: '#777' }}>
              Last price: {formatPrice(selectedPair.lastPrice)} {selectedPair.quote}
            </div>
          </div>

          <div style={s.bookContent}>
            {tab === 'book' ? (
              <div style={{ display: 'flex', flexDirection: 'column', flex: 1, overflow: 'hidden' }}>
                {/* Column headers */}
                <div style={{ display: 'flex', padding: '4px 10px', fontSize: '0.65rem', color: '#555', textTransform: 'uppercase', letterSpacing: 1, flexShrink: 0 }}>
                  <span style={{ flex: 1 }}>Price ({selectedPair.quote})</span>
                  <span style={{ flex: 1, textAlign: 'right' }}>Size ({selectedPair.base})</span>
                  <span style={{ flex: 1, textAlign: 'right' }}>Total</span>
                </div>

                {/* Asks (sells) */}
                <div style={{ flex: 1, overflow: 'auto', display: 'flex', flexDirection: 'column', justifyContent: 'flex-end' }}>
                  {orderbook.asks.map((lvl, i) => (
                    <div key={`a${i}`} style={s.row}
                      onClick={() => { setLimitPrice(lvl.price.toString()); setOrderSide('buy'); }}>
                      <div style={{ position: 'absolute', right: 0, top: 0, bottom: 0, background: 'rgba(239,83,80,0.08)', width: `${(lvl.total / maxTotal) * 100}%` }} />
                      <span style={{ flex: 1, color: '#ef5350', zIndex: 1 }}>{formatPrice(lvl.price)}</span>
                      <span style={{ flex: 1, textAlign: 'right', zIndex: 1 }}>{lvl.size.toLocaleString()}</span>
                      <span style={{ flex: 1, textAlign: 'right', color: '#777', zIndex: 1 }}>{lvl.total.toLocaleString()}</span>
                    </div>
                  ))}
                </div>

                {/* Spread */}
                <div style={s.spreadBar as React.CSSProperties}>
                  Spread: {formatPrice(spread)} ({((spread / selectedPair.lastPrice) * 100).toFixed(3)}%)
                </div>

                {/* Bids (buys) */}
                <div style={{ flex: 1, overflow: 'auto' }}>
                  {orderbook.bids.map((lvl, i) => (
                    <div key={`b${i}`} style={s.row}
                      onClick={() => { setLimitPrice(lvl.price.toString()); setOrderSide('sell'); }}>
                      <div style={{ position: 'absolute', right: 0, top: 0, bottom: 0, background: 'rgba(76,175,80,0.08)', width: `${(lvl.total / maxTotal) * 100}%` }} />
                      <span style={{ flex: 1, color: '#4caf50', zIndex: 1 }}>{formatPrice(lvl.price)}</span>
                      <span style={{ flex: 1, textAlign: 'right', zIndex: 1 }}>{lvl.size.toLocaleString()}</span>
                      <span style={{ flex: 1, textAlign: 'right', color: '#777', zIndex: 1 }}>{lvl.total.toLocaleString()}</span>
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div style={{ flex: 1, display: 'flex', flexDirection: 'column', overflow: 'hidden' }}>
                <div style={{ overflow: 'auto', flex: 1 }}>
                  <div style={{ display: 'flex', padding: '4px 10px', fontSize: '0.65rem', color: '#555', textTransform: 'uppercase', letterSpacing: 1 }}>
                    <span style={{ flex: 1 }}>Price</span>
                    <span style={{ flex: 1, textAlign: 'right' }}>Size</span>
                    <span style={{ flex: 1, textAlign: 'right' }}>Time</span>
                  </div>
                  {fills.map(f => (
                    <div key={f.id} style={{ display: 'flex', padding: '2px 10px' }}>
                      <span style={{ flex: 1, color: f.side === 'buy' ? '#4caf50' : '#ef5350' }}>{formatPrice(f.price)}</span>
                      <span style={{ flex: 1, textAlign: 'right' }}>{f.size.toLocaleString()}</span>
                      <span style={{ flex: 1, textAlign: 'right', color: '#777' }}>{f.time}</span>
                    </div>
                  ))}
                </div>
              </div>
            )}
          </div>
        </div>

        {/* Right: Order Entry */}
        <div style={s.orderCol}>
          <div style={{ fontWeight: 700, marginBottom: 8, fontSize: '0.85rem' }}>Place Order</div>

          {/* Buy / Sell toggle */}
          <div style={{ display: 'flex', gap: 4, marginBottom: 8 }}>
            <button onClick={() => setOrderSide('buy')}
              style={{ flex: 1, padding: '6px 0', borderRadius: 6, border: 'none', fontWeight: 700, cursor: 'pointer', fontSize: '0.8rem',
                background: orderSide === 'buy' ? '#4caf50' : '#1e2436', color: orderSide === 'buy' ? '#fff' : '#777' }}>
              Buy
            </button>
            <button onClick={() => setOrderSide('sell')}
              style={{ flex: 1, padding: '6px 0', borderRadius: 6, border: 'none', fontWeight: 700, cursor: 'pointer', fontSize: '0.8rem',
                background: orderSide === 'sell' ? '#ef5350' : '#1e2436', color: orderSide === 'sell' ? '#fff' : '#777' }}>
              Sell
            </button>
          </div>

          {/* Order type */}
          <div style={{ display: 'flex', gap: 4, marginBottom: 10 }}>
            {(['limit', 'market'] as const).map(t => (
              <button key={t} onClick={() => setOrderType(t)}
                style={{ flex: 1, padding: '4px 0', borderRadius: 4, border: orderType === t ? '1px solid #00e5ff' : '1px solid #2a2f3e', background: 'transparent', color: orderType === t ? '#00e5ff' : '#777', cursor: 'pointer', fontSize: '0.72rem', textTransform: 'capitalize' }}>
                {t}
              </button>
            ))}
          </div>

          {/* Price (limit only) */}
          {orderType === 'limit' && (
            <div style={{ marginBottom: 8 }}>
              <label style={{ fontSize: '0.65rem', color: '#777', textTransform: 'uppercase', letterSpacing: 1 }}>
                Price ({selectedPair.quote})
              </label>
              <input value={limitPrice} onChange={e => setLimitPrice(e.target.value)}
                style={{ width: '100%', padding: '6px 8px', background: '#0d1117', border: '1px solid #2a2f3e', borderRadius: 6, color: '#fff', fontSize: '0.82rem', marginTop: 2, fontFamily: 'monospace' }} />
            </div>
          )}

          {/* Size */}
          <div style={{ marginBottom: 8 }}>
            <label style={{ fontSize: '0.65rem', color: '#777', textTransform: 'uppercase', letterSpacing: 1 }}>
              Amount ({selectedPair.base})
            </label>
            <input value={orderSize} onChange={e => setOrderSize(e.target.value)} placeholder="0.00"
              style={{ width: '100%', padding: '6px 8px', background: '#0d1117', border: '1px solid #2a2f3e', borderRadius: 6, color: '#fff', fontSize: '0.82rem', marginTop: 2, fontFamily: 'monospace' }} />
          </div>

          {/* Quick size buttons */}
          <div style={{ display: 'flex', gap: 4, marginBottom: 10 }}>
            {['25%', '50%', '75%', '100%'].map(pct => (
              <button key={pct}
                style={{ flex: 1, padding: '3px 0', borderRadius: 4, border: '1px solid #2a2f3e', background: 'transparent', color: '#777', cursor: 'pointer', fontSize: '0.65rem' }}
                onClick={() => setOrderSize((parseInt(pct) * 100).toString())}
              >{pct}</button>
            ))}
          </div>

          {/* Total */}
          <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.75rem', marginBottom: 10, color: '#999' }}>
            <span>Total</span>
            <span>{estTotal} {selectedPair.quote}</span>
          </div>

          {/* Submit */}
          <button
            disabled={trading}
            style={{
              width: '100%', padding: '10px 0', borderRadius: 8, border: 'none', fontWeight: 700, fontSize: '0.85rem', cursor: trading ? 'default' : 'pointer',
              background: trading ? '#1e2436' : (orderSide === 'buy' ? '#4caf50' : '#ef5350'), color: trading ? '#777' : '#fff', opacity: trading ? 0.7 : 1,
              transition: 'all 0.2s ease',
            }}
            onClick={handlePlaceOrder}
          >
            {trading ? (
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: 6 }}>
                <span className="animate-spin text-xs">🌀</span>
                {tradeStatus ?? 'Processing...'}
              </div>
            ) : (
              `${orderSide === 'buy' ? 'Buy' : 'Sell'} ${selectedPair.base}`
            )}
          </button>

          {/* Open orders placeholder */}
          <div style={{ marginTop: 16 }}>
            <div style={{ fontSize: '0.7rem', fontWeight: 600, color: '#777', marginBottom: 4 }}>Open Orders</div>
            <div style={{ fontSize: '0.7rem', color: '#555', textAlign: 'center', padding: 12 }}>No open orders</div>
          </div>
        </div>
      </div>
    </div>
  );
};

interface TwapChartProps {
  data: CandlestickData[];
  loading: boolean;
  error: string | null;
  pairLabel: string;
}

const TwapChart: React.FC<TwapChartProps> = ({ data, loading, error, pairLabel }) => {
  const containersRef = useRef<HTMLDivElement | null>(null);
  const chartRef = useRef<IChartApi | null>(null);
  const candlestickSeriesRef = useRef<ReturnType<IChartApi['addCandlestickSeries']> | null>(null);
  const resizeObserverRef = useRef<ResizeObserver | null>(null);

  useEffect(() => {
    if (!containersRef.current) return;
    const chart = createChart(containersRef.current, {
      layout: {
        background: { color: '#0d1117' },
        textColor: '#e0e0e0',
        fontFamily: 'monospace',
      },
      grid: {
        vertLines: { color: '#151b29' },
        horzLines: { color: '#151b29' },
      },
      crosshair: {
        horzLine: { visible: true, color: '#222b3b' },
        vertLine: { visible: true, color: '#222b3b' },
      },
      timeScale: {
        timeVisible: true,
        secondsVisible: true,
        borderColor: '#1a1f2e',
        rightOffset: 3,
      },
      height: 200,
      width: containersRef.current.clientWidth,
    });

    chartRef.current = chart;

    const series = chart.addCandlestickSeries({
      upColor: '#4caf50',
      downColor: '#ef5350',
      borderVisible: false,
      wickColor: '#888',
    });
    candlestickSeriesRef.current = series;
    if (data.length) {
      series.setData(data);
    }

    if (typeof ResizeObserver !== 'undefined') {
      const observer = new ResizeObserver(entries => {
        entries.forEach(entry => {
          chart.applyOptions({ width: entry.contentRect.width });
        });
      });
      observer.observe(containersRef.current);
      resizeObserverRef.current = observer;
    }

    return () => {
      resizeObserverRef.current?.disconnect();
      chart.remove();
      chartRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (!candlestickSeriesRef.current) return;
    if (data.length) {
      candlestickSeriesRef.current.setData(data);
    }
  }, [data]);

  const overlayMessage = error ?? (loading ? 'Loading price history…' : null);

  return (
    <div style={{ position: 'relative', minHeight: 200 }}>
      <div
        ref={containersRef}
        aria-label={`TWAP candlestick chart for ${pairLabel}`}
        role="img"
        style={{ height: 200 }}
      />
      {overlayMessage && (
        <div
          style={{
            position: 'absolute',
            inset: 0,
            background: error ? 'rgba(239,83,80,0.3)' : 'rgba(10,14,23,0.45)',
            color: '#fff',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            fontSize: '0.75rem',
            textAlign: 'center',
            padding: '0 8px',
            pointerEvents: 'none',
          }}
        >
          {overlayMessage}
        </div>
      )}
    </div>
  );
};

export default DexOrderbookPanel;
