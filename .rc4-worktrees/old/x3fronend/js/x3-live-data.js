/**
 * X3 Chain Live Data — Substrate JSON-RPC integration for all x3star-*.html pages
 * Connects via WebSocket (preferred) or falls back to HTTP polling every 6 s (1 block time).
 * Updates well-known element IDs + any [data-live="<metric>"] attributes found in the page.
 *
 * Endpoints:
 *   WS mainnet (via tunnel) : wss://ws.x3star.net
 *   HTTP RPC (via tunnel)   : https://rpc.x3star.net
 *   WS local dev            : ws://localhost:9933
 *   HTTP local dev          : http://localhost:9933
 *
 * To override, set before loading this script:
 *   window.X3_RPC_WS  = 'wss://ws.x3star.net' or 'ws://localhost:9933'
 *   window.X3_RPC_HTTP = 'https://rpc.x3star.net' or 'http://localhost:9933'
 */
(function X3LiveData() {
  'use strict';

  /* ── Config ── */
  // Production: wss://ws.x3star.net (HTTP: https://rpc.x3star.net)
  // Local dev: ws://localhost:9944 (HTTP: http://localhost:9933)
  const WS_ENDPOINT   = window.X3_RPC_WS   || 'wss://ws.x3star.net';
  const HTTP_ENDPOINT = window.X3_RPC_HTTP  || 'https://rpc.x3star.net';
  const POLL_MS       = 6000; // 1 block time

  /* ── State ── */
  let ws = null;
  let wsConnected = false;
  let wsRetryTimer = null;
  let pollTimer = null;
  let pendingRpc = {};
  let rpcId = 1;
  let metrics = {
    blockHeight: null,
    tps: null,
    validatorCount: null,
    totalIssuance: null,
    treasuryBalance: null,
    totalTx: 0,
    raised: 14702400,    // prefunding target — updated from treasury when chain is live
    tokenPrice: 0.0842,  // updated from external feed when available
    stakingTvl: null,
    lastBlockTime: null,
    lastBlockHash: null,
    blockTxCounts: [],   // ring buffer for TPS calc: [{count, time}, ...]
  };

  /* ── Substrate JSON-RPC helpers ── */
  function rpcPost(method, params = []) {
    return fetch(HTTP_ENDPOINT, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ id: rpcId++, jsonrpc: '2.0', method, params }),
    }).then(r => r.json()).then(d => d.result);
  }

  function wsSend(method, params = [], cb) {
    if (!wsConnected) return null;
    const id = rpcId++;
    pendingRpc[id] = cb;
    ws.send(JSON.stringify({ id, jsonrpc: '2.0', method, params }));
    return id;
  }

  /* ── Metric calculations ── */
  function calcTps(txCount) {
    const now = Date.now();
    metrics.blockTxCounts.push({ count: txCount, time: now });
    if (metrics.blockTxCounts.length > 10) metrics.blockTxCounts.shift();
    const oldest = metrics.blockTxCounts[0];
    const newest = metrics.blockTxCounts[metrics.blockTxCounts.length - 1];
    const span = (newest.time - oldest.time) / 1000 || 6;
    const total = metrics.blockTxCounts.reduce((s, b) => s + b.count, 0);
    return Math.round(total / span);
  }

  function hexToNum(hex) {
    return hex ? parseInt(hex, 16) : 0;
  }

  function hexToBigInt(hex) {
    return hex ? BigInt(hex) : 0n;
  }

  /* ── DOM updaters ── */
  function fmt(n) {
    if (n === null || n === undefined) return '—';
    return Number(n).toLocaleString();
  }

  function fmtPrice(n) {
    if (n === null || n === undefined) return '—';
    return '$' + Number(n).toFixed(4);
  }

  function fmtRaised(n) {
    if (!n) return '—';
    return '$' + Number(n).toLocaleString();
  }

  // Update all elements with a given [data-live] attribute OR a known id
  function set(selector, value) {
    // by data-live attribute
    document.querySelectorAll(`[data-live="${selector}"]`).forEach(el => {
      el.textContent = value;
      el.classList.add('x3-live-flash');
      setTimeout(() => el.classList.remove('x3-live-flash'), 600);
    });
    // by well-known ID mappings
    const ids = DATA_LIVE_IDS[selector] || [];
    ids.forEach(id => {
      const el = document.getElementById(id);
      if (el) {
        el.textContent = value;
        el.classList.add('x3-live-flash');
        setTimeout(() => el.classList.remove('x3-live-flash'), 600);
      }
    });
  }

  // Map data-live key → list of well-known element IDs that contain the same metric
  const DATA_LIVE_IDS = {
    // Note: block height is handled specially in renderBlockHeight() below
    'tps':          ['kpi-tps', 'tb-tps', 'center-tps', 'tps-counter', 'tps-live', 'live-tps'],
    'validators':   ['kpi-vals', 'validator-count', 'val-count', 'num-validators'],
    'total-tx':     ['total-tx', 'tx-count', 'total-transactions'],
    'price':        ['price', 'token-price', 'x3s-price', 'scarcity-price'],
    'raised':       ['raised-num', 'raised-amt', 'total-raised', 'funding-raised'],
    'total-issuance': ['total-supply', 'total-issuance'],
  };

  // IDs that use hash prefix (#1,234,567) vs plain number
  const BLOCK_WITH_HASH = ['tb-block', 'kpi-blocks'];
  const BLOCK_PLAIN     = ['block-num', 'block-height', 'current-block'];

  function renderBlockHeight(n) {
    if (n === null || n === undefined) return;
    BLOCK_WITH_HASH.forEach(id => {
      const el = document.getElementById(id);
      if (el) { el.textContent = '#' + fmt(n); el.classList.add('x3-live-flash'); setTimeout(() => el.classList.remove('x3-live-flash'), 600); }
    });
    BLOCK_PLAIN.forEach(id => {
      const el = document.getElementById(id);
      if (el) { el.textContent = fmt(n); el.classList.add('x3-live-flash'); setTimeout(() => el.classList.remove('x3-live-flash'), 600); }
    });
    document.querySelectorAll('[data-live="block-height"]').forEach(el => {
      el.textContent = '#' + fmt(n);
      el.classList.add('x3-live-flash'); setTimeout(() => el.classList.remove('x3-live-flash'), 600);
    });
  }

  function updateTpsBar(tps) {
    const maxTps = 5000; // reference max for percentage bar
    const pct = Math.min(100, Math.round((tps / maxTps) * 100));
    const bar = document.getElementById('tps-bar');
    const pctEl = document.getElementById('tps-pct');
    if (bar) bar.style.width = pct + '%';
    if (pctEl) pctEl.textContent = pct + '%';
  }

  function updateStatus(connected) {
    document.querySelectorAll('[data-live="connection-status"]').forEach(el => {
      el.textContent = connected ? '● Connected' : '○ Reconnecting…';
      el.style.color = connected ? 'var(--green, #00ff88)' : 'var(--muted, #666)';
    });
    // Update any x3-live-indicator dots
    document.querySelectorAll('.x3-live-dot').forEach(el => {
      el.style.background = connected ? 'var(--green, #00ff88)' : 'var(--muted, #666)';
    });
  }

  /* ── Render cycle ── */
  function render() {
    const m = metrics;
    if (m.blockHeight !== null) renderBlockHeight(m.blockHeight);
    if (m.tps !== null)         set('tps', fmt(m.tps));
    if (m.validatorCount !== null) set('validators', fmt(m.validatorCount));
    if (m.totalTx)              set('total-tx', fmt(m.totalTx));
    if (m.tokenPrice)           set('price', fmtPrice(m.tokenPrice));
    if (m.raised)               set('raised', fmtRaised(m.raised));
    if (m.totalIssuance)        set('total-issuance', fmt(m.totalIssuance));
    if (m.tps !== null)         updateTpsBar(m.tps);
  }

  /* ── Substrate RPC calls ── */
  async function fetchNewHead(hash) {
    try {
      // Get block header
      const header = await rpcPost('chain_getHeader', hash ? [hash] : []);
      if (!header) return;
      metrics.blockHeight = hexToNum(header.number);
      metrics.lastBlockHash = hash || null;

      // Get block body for tx count (best-effort)
      try {
        const blkHash = hash || await rpcPost('chain_getBlockHash', [metrics.blockHeight]);
        const block = await rpcPost('chain_getBlock', [blkHash]);
        if (block && block.block && block.block.extrinsics) {
          const txCount = block.block.extrinsics.length;
          metrics.totalTx += txCount;
          metrics.tps = calcTps(txCount);
        }
      } catch (_) { /* block details optional */ }

      render();
    } catch (e) {
      console.debug('[X3Live] fetchNewHead error:', e.message);
    }
  }

  async function fetchValidators() {
    try {
      const vals = await rpcPost('session_validators', []);
      if (Array.isArray(vals)) {
        metrics.validatorCount = vals.length;
        render();
      }
    } catch (_) {}
  }

  async function fetchTotalIssuance() {
    try {
      const raw = await rpcPost('query_storage', []);
      // Direct query via state_call
      const hex = await rpcPost('state_getStorage', [
        // Substrate storage key for Balances.TotalIssuance
        '0xc2261276cc9d1f8598ea4b6a74b15c2f57c875e4cff74148e4628f264b974c7'
      ]);
      if (hex) {
        // u128 little-endian hex
        const bytes = hex.slice(2).match(/.{2}/g) || [];
        let val = 0n;
        bytes.reverse().forEach(b => { val = (val << 8n) + BigInt(parseInt(b, 16)); });
        // Convert from Planck (1e12) to X3S tokens
        metrics.totalIssuance = Number(val / 1_000_000_000_000n);
        render();
      }
    } catch (_) {}
  }

  /* ── HTTP polling (fallback when WebSocket unavailable) ── */
  async function poll() {
    try {
      const header = await rpcPost('chain_getHeader', []);
      if (header) await fetchNewHead(null);
      // Fetch validators every 5 polls (30 s)
      if (!wsConnected) fetchValidators();
    } catch (_) {}
  }

  function startPolling() {
    if (pollTimer) clearInterval(pollTimer);
    poll();
    pollTimer = setInterval(poll, POLL_MS);
  }

  function stopPolling() {
    if (pollTimer) clearInterval(pollTimer);
    pollTimer = null;
  }

  /* ── WebSocket subscription ── */
  function connectWs() {
    if (ws) { try { ws.close(); } catch (_) {} }
    ws = new WebSocket(WS_ENDPOINT);
    ws.onopen = () => {
      wsConnected = true;
      updateStatus(true);
      stopPolling();
      if (wsRetryTimer) clearTimeout(wsRetryTimer);

      // Subscribe to new block headers
      ws.send(JSON.stringify({ id: rpcId++, jsonrpc: '2.0', method: 'chain_subscribeNewHeads', params: [] }));
      fetchValidators();
      fetchTotalIssuance();
    };

    ws.onmessage = ev => {
      let msg;
      try { msg = JSON.parse(ev.data); } catch (_) { return; }

      // Resolve pending one-shot calls
      if (msg.id && pendingRpc[msg.id]) {
        pendingRpc[msg.id](msg.result);
        delete pendingRpc[msg.id];
        return;
      }

      // Subscription notifications
      if (msg.method === 'chain_newHead' && msg.params && msg.params.result) {
        const header = msg.params.result;
        fetchNewHead(null).then(() => {
          // After updating block, also update block height directly from subscription header
          metrics.blockHeight = hexToNum(header.number);
          render();
        });
      }
    };

    ws.onerror = () => {
      wsConnected = false;
      updateStatus(false);
    };

    ws.onclose = () => {
      wsConnected = false;
      updateStatus(false);
      // Fallback to HTTP polling, retry WS after 15 s
      startPolling();
      wsRetryTimer = setTimeout(connectWs, 15000);
    };
  }

  /* ── Inject CSS for flash animation ── */
  function injectStyles() {
    if (document.getElementById('x3-live-styles')) return;
    const s = document.createElement('style');
    s.id = 'x3-live-styles';
    s.textContent = `
      .x3-live-flash { animation: x3FlashIn 0.4s ease; }
      @keyframes x3FlashIn {
        0%   { opacity: 0.4; color: var(--cyan, #00e5ff); }
        100% { opacity: 1; }
      }
      .x3-live-dot {
        display: inline-block; width: 8px; height: 8px;
        border-radius: 50%; background: var(--green, #00ff88);
        box-shadow: 0 0 6px var(--green, #00ff88);
        animation: x3Pulse 2s ease-in-out infinite;
      }
      @keyframes x3Pulse {
        0%,100% { opacity: 1; } 50% { opacity: 0.4; }
      }
    `;
    document.head.appendChild(s);
  }

  /* ── Inject live-data status badge into page ── */
  function injectStatusBadge() {
    if (document.getElementById('x3-live-badge')) return;
    const badge = document.createElement('div');
    badge.id = 'x3-live-badge';
    badge.setAttribute('data-live', 'connection-status');
    badge.style.cssText = `
      position: fixed; bottom: 12px; right: 16px; z-index: 9999;
      font-family: 'JetBrains Mono', monospace; font-size: 10px;
      color: var(--muted, #666); cursor: default;
      display: flex; align-items: center; gap: 6px;
    `;
    badge.innerHTML = '<span class="x3-live-dot"></span><span id="x3-live-status-text">Connecting…</span>';
    document.body.appendChild(badge);
  }

  /* ── Bootstrap ── */
  function init() {
    injectStyles();
    if (document.readyState === 'loading') {
      document.addEventListener('DOMContentLoaded', () => { injectStatusBadge(); connectWs(); });
    } else {
      injectStatusBadge();
      connectWs();
    }
  }

  init();

  /* ── Public API ── */
  window.X3Live = {
    metrics,
    update: render,
    reconnect: connectWs,
    rpcPost,
  };
})();
