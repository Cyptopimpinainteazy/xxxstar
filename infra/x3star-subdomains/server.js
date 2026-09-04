#!/usr/bin/env node
// ============================================================================
// x3star.net — Subdomain Placeholder Server
// ============================================================================
// Single Node.js process on :7000 that reads the Host header and serves a
// unique branded landing page for each subdomain. Zero dependencies.
// ============================================================================

const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = process.env.PORT || 7000;
const PAGES_DIR = path.join(__dirname, "pages");

// ── Subdomain registry ──────────────────────────────────────────────────────
// Each entry: { title, tagline, icon (emoji), color, description, status }
const sites = {
  ai:              { title: "X3 AI",             tagline: "Artificial Intelligence Platform",   icon: "🤖", color: "#8B5CF6", description: "Neural networks, model training, and AI-powered tools for the X3 ecosystem.", status: "Coming Soon", tauri: "apps/x3-desktop — AI panels & eyeball components" },
  chat:            { title: "X3 Chat",           tagline: "Decentralized Messaging",            icon: "💬", color: "#06B6D4", description: "End-to-end encrypted messaging powered by the X3 blockchain.", status: "Coming Soon", tauri: "apps/x3-desktop/src/pages/social/MessagesPage.tsx" },
  social:          { title: "X3 Social",         tagline: "Web3 Social Network",                icon: "🌐", color: "#EC4899", description: "Connect, share, and earn. Your social graph, on-chain.", status: "Coming Soon", tauri: "apps/x3-desktop/src/pages/social/SocialApp.tsx" },
  trading:         { title: "X3 Trading",        tagline: "Advanced Trading Terminal",          icon: "📈", color: "#10B981", description: "Charts, order books, and cross-chain swaps in one interface.", status: "Coming Soon", tauri: "apps/3ai/dex — DEX trading frontend" },
  adult:           { title: "X3 Adult",          tagline: "18+ Content Platform",               icon: "🔞", color: "#F43F5E", description: "Age-verified, creator-owned adult content on the blockchain.", status: "Coming Soon" },
  crypto:          { title: "X3 Crypto",         tagline: "Cryptocurrency Hub",                 icon: "₿",  color: "#F59E0B", description: "Prices, portfolios, and on-chain analytics for every token.", status: "Coming Soon" },
  blockchain:      { title: "X3 Blockchain",     tagline: "Network Dashboard",                  icon: "⛓️", color: "#3B82F6", description: "Real-time block production, finality, and network health.", status: "Coming Soon", tauri: "apps/x3-desktop — system metrics components" },
  testnet:         { title: "X3 Testnet",        tagline: "Test Network Portal",                icon: "🧪", color: "#A855F7", description: "Faucets, test validators, and sandbox environments.", status: "Coming Soon", tauri: "testnet-config/" },
  ide:             { title: "X3 IDE",            tagline: "Online Development Environment",     icon: "🖥️", color: "#64748B", description: "Write, compile, and deploy X3 smart contracts in your browser.", status: "Coming Soon", tauri: "crates/x3-compiler, crates/x3-lsp" },
  marketplace:     { title: "X3 Marketplace",    tagline: "Decentralized Marketplace",          icon: "🏪", color: "#F97316", description: "Buy, sell, and trade digital assets and NFTs.", status: "Coming Soon" },
  pool:            { title: "X3 Pool",           tagline: "Liquidity Pools & Staking",          icon: "🏊", color: "#0EA5E9", description: "Provide liquidity, stake tokens, and earn yield.", status: "Coming Soon", tauri: "contracts/lending, contracts/treasury" },
  swarm:           { title: "X3 Swarm",          tagline: "GPU & Agent Swarm Network",          icon: "🐝", color: "#EAB308", description: "Distributed GPU compute and autonomous AI agent swarms.", status: "Coming Soon", tauri: "swarm/, crates/gpu-swarm, crates/quantum-swarm" },
  docs:            { title: "X3 Docs",           tagline: "Documentation & Guides",             icon: "📚", color: "#6366F1", description: "Developer documentation, API references, and tutorials.", status: "Coming Soon", tauri: "apps/x3-desktop/src/components/documentation" },
  infura:          { title: "X3 Infura",         tagline: "Cross-Chain RPC Gateway",            icon: "🔗", color: "#7C3AED", description: "Multi-chain RPC routing with automatic failover.", status: "Coming Soon", tauri: "crates/external-chains, infra/mainnet-rpc-endpoints.toml" },
  validator:       { title: "X3 Validator",      tagline: "Validator Node Dashboard",           icon: "✅", color: "#059669", description: "Monitor your validator, view rewards, and manage keys.", status: "Coming Soon", tauri: "apps/validators" },
  "gpu-validator": { title: "X3 GPU Validator",  tagline: "GPU Validation Network",             icon: "🎮", color: "#D946EF", description: "GPU-accelerated block validation and proof generation.", status: "Coming Soon", tauri: "cross-chain-gpu-validator/" },
  "x-change":      { title: "X3 Exchange",       tagline: "Decentralized Exchange",             icon: "💱", color: "#14B8A6", description: "Swap any token across chains with minimal slippage.", status: "Coming Soon", tauri: "apps/3ai/dex, crates/x3-swap-router" },
  testing:         { title: "X3 Testing",        tagline: "Test Dashboard",                     icon: "🧪", color: "#78716C", description: "CI/CD pipeline status, test results, and coverage reports.", status: "Coming Soon" },
  test:            { title: "X3 Test",           tagline: "Staging Environment",                icon: "🔬", color: "#9CA3AF", description: "Preview builds and staging deployments.", status: "Coming Soon" },
  blockexplorer:   { title: "X3 Block Explorer", tagline: "Full Block Explorer",                icon: "🔍", color: "#2563EB", description: "Search blocks, transactions, accounts, and events.", status: "Coming Soon" },
  xplorer:         { title: "X3plorer",          tagline: "Lightweight Explorer",               icon: "🗺️", color: "#0284C7", description: "Fast, minimal block and transaction explorer.", status: "Coming Soon" },
  xxx:             { title: "X3 XXX",            tagline: "Premium Adult Content",              icon: "🔥", color: "#E11D48", description: "Creator-first premium content platform.", status: "Coming Soon" },
  cam:             { title: "X3 Cam",            tagline: "Live Streaming",                     icon: "📷", color: "#DB2777", description: "Live video streaming with on-chain tipping.", status: "Coming Soon" },
  wallet:          { title: "X3 Wallet",         tagline: "Web3 Wallet",                        icon: "👛", color: "#7C3AED", description: "Non-custodial multi-chain wallet in your browser.", status: "Coming Soon", tauri: "apps/x3-desktop — wallet components" },
  casino:          { title: "X3 Casino",         tagline: "On-Chain Gaming",                    icon: "🎰", color: "#DC2626", description: "Provably fair games powered by on-chain randomness.", status: "Coming Soon" },
  blog:            { title: "X3 Blog",           tagline: "News & Updates",                     icon: "📝", color: "#4F46E5", description: "Project updates, technical deep-dives, and community news.", status: "Coming Soon" },
  xxxchange:       { title: "X3 XXXchange",      tagline: "Adult Marketplace",                  icon: "💎", color: "#BE123C", description: "Decentralized marketplace for adult creators.", status: "Coming Soon" },
  pics:            { title: "X3 Pics",           tagline: "Image Gallery & Hosting",            icon: "🖼️", color: "#C026D3", description: "Upload, share, and monetize images on IPFS.", status: "Coming Soon", tauri: "apps/x3-desktop/src/pages/social/PhotosPage.tsx" },
  vibecode:        { title: "X3 Vibecode",       tagline: "Collaborative Code Editor",          icon: "✨", color: "#8B5CF6", description: "Real-time collaborative coding with AI assistance.", status: "Coming Soon", tauri: "crates/vibe-bmad" },
  explorer:        { title: "X3 Explorer",       tagline: "Block Explorer",                     icon: "🔎", color: "#1D4ED8", description: "The original X3 block explorer.", status: "Coming Soon" },
};

// ── HTML template ───────────────────────────────────────────────────────────
function renderPage(sub, site) {
  const { title, tagline, icon, color, description, status } = site;
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${title} — ${tagline}</title>
  <meta name="description" content="${description}">
  <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>${encodeURIComponent(icon)}</text></svg>">
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    :root { --accent: ${color}; --bg: #0a0a0f; --card: #12121a; --text: #e2e8f0; --muted: #94a3b8; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background: var(--bg); color: var(--text);
      min-height: 100vh; display: flex; flex-direction: column;
      align-items: center; justify-content: center;
      overflow: hidden;
    }
    /* Animated gradient bg */
    body::before {
      content: ''; position: fixed; inset: 0; z-index: 0;
      background: radial-gradient(ellipse at 20% 50%, ${color}15 0%, transparent 50%),
                  radial-gradient(ellipse at 80% 20%, ${color}10 0%, transparent 50%),
                  radial-gradient(ellipse at 50% 80%, #3b82f610 0%, transparent 50%);
      animation: pulse 8s ease-in-out infinite alternate;
    }
    @keyframes pulse {
      0% { opacity: 0.6; transform: scale(1); }
      100% { opacity: 1; transform: scale(1.05); }
    }
    .container {
      position: relative; z-index: 1;
      text-align: center; padding: 2rem; max-width: 600px;
    }
    .icon { font-size: 5rem; margin-bottom: 1rem; filter: drop-shadow(0 0 30px ${color}80); }
    .title { font-size: 2.5rem; font-weight: 800; letter-spacing: -0.02em; margin-bottom: 0.25rem; }
    .title span { color: var(--accent); }
    .tagline { font-size: 1.1rem; color: var(--muted); margin-bottom: 2rem; }
    .card {
      background: var(--card); border: 1px solid ${color}30;
      border-radius: 16px; padding: 2rem; margin-bottom: 2rem;
      backdrop-filter: blur(10px);
    }
    .desc { font-size: 1rem; line-height: 1.6; color: var(--muted); margin-bottom: 1.5rem; }
    .badge {
      display: inline-block; padding: 0.4rem 1.2rem;
      background: ${color}20; color: ${color};
      border: 1px solid ${color}40; border-radius: 999px;
      font-size: 0.85rem; font-weight: 600; letter-spacing: 0.05em;
      text-transform: uppercase;
    }
    .domain {
      margin-top: 2rem; font-size: 0.8rem; color: var(--muted);
      opacity: 0.5;
    }
    .domain a { color: var(--accent); text-decoration: none; }
    .nav {
      position: fixed; bottom: 2rem; left: 50%; transform: translateX(-50%);
      z-index: 2; display: flex; gap: 0.5rem; flex-wrap: wrap;
      justify-content: center; max-width: 90vw;
    }
    .nav a {
      padding: 0.3rem 0.7rem; background: var(--card);
      border: 1px solid #ffffff10; border-radius: 8px;
      color: var(--muted); text-decoration: none; font-size: 0.7rem;
      transition: all 0.2s;
    }
    .nav a:hover { border-color: var(--accent); color: var(--accent); }
    .nav a.active { border-color: var(--accent); color: var(--accent); background: ${color}15; }
    /* Floating particles */
    .particles { position: fixed; inset: 0; z-index: 0; pointer-events: none; }
    .dot {
      position: absolute; width: 3px; height: 3px;
      background: ${color}; border-radius: 50%; opacity: 0.3;
      animation: float linear infinite;
    }
    @keyframes float {
      0% { transform: translateY(100vh) scale(0); opacity: 0; }
      10% { opacity: 0.3; }
      90% { opacity: 0.3; }
      100% { transform: translateY(-10vh) scale(1); opacity: 0; }
    }
  </style>
</head>
<body>
  <div class="particles" id="particles"></div>
  <div class="container">
    <div class="icon">${icon}</div>
    <h1 class="title"><span>${title}</span></h1>
    <p class="tagline">${tagline}</p>
    <div class="card">
      <p class="desc">${description}</p>
      <span class="badge">${status}</span>
    </div>
    <p class="domain">Part of <a href="https://x3star.net">x3star.net</a> ecosystem</p>
  </div>
  <nav class="nav">
    ${Object.entries(sites).filter(([k]) => !["test","testing"].includes(k)).slice(0, 20).map(([k, v]) =>
      `<a href="https://${k}.x3star.net" class="${k === sub ? 'active' : ''}" title="${v.tagline}">${v.icon} ${k}</a>`
    ).join("\n    ")}
  </nav>
  <script>
    // Floating particles
    const c = document.getElementById('particles');
    for (let i = 0; i < 20; i++) {
      const d = document.createElement('div');
      d.className = 'dot';
      d.style.left = Math.random() * 100 + '%';
      d.style.animationDuration = (8 + Math.random() * 12) + 's';
      d.style.animationDelay = Math.random() * 10 + 's';
      c.appendChild(d);
    }
  </script>
</body>
</html>`;
}

// ── Server ──────────────────────────────────────────────────────────────────
const server = http.createServer((req, res) => {
  const host = (req.headers.host || "").split(":")[0];
  const sub = host.replace(/\.x3star\.net$/, "");

  // Health check
  if (req.url === "/health") {
    res.writeHead(200, { "Content-Type": "text/plain" });
    return res.end("ok");
  }

  const site = sites[sub];
  if (!site) {
    // Unknown subdomain - show a directory page
    res.writeHead(200, { "Content-Type": "text/html" });
    return res.end(renderDirectoryPage());
  }

  res.writeHead(200, {
    "Content-Type": "text/html; charset=utf-8",
    "Cache-Control": "public, max-age=300",
    "X-Subdomain": sub,
  });
  res.end(renderPage(sub, site));
});

// ── Directory page (lists all subdomains) ───────────────────────────────────
function renderDirectoryPage() {
  const cards = Object.entries(sites).map(([sub, s]) =>
    `<a href="https://${sub}.x3star.net" class="site-card" style="--c:${s.color}">
      <span class="site-icon">${s.icon}</span>
      <span class="site-name">${s.title}</span>
      <span class="site-tag">${s.tagline}</span>
    </a>`
  ).join("\n");

  return `<!DOCTYPE html>
<html lang="en"><head>
  <meta charset="UTF-8"><meta name="viewport" content="width=device-width,initial-scale=1">
  <title>X3 Star Network — All Sites</title>
  <style>
    *{margin:0;padding:0;box-sizing:border-box}
    body{font-family:-apple-system,BlinkMacSystemFont,sans-serif;background:#0a0a0f;color:#e2e8f0;min-height:100vh;padding:3rem 1rem}
    h1{text-align:center;font-size:2rem;margin-bottom:0.5rem}
    .sub{text-align:center;color:#94a3b8;margin-bottom:2rem}
    .grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(220px,1fr));gap:1rem;max-width:1200px;margin:0 auto}
    .site-card{display:flex;flex-direction:column;align-items:center;padding:1.5rem;background:#12121a;
      border:1px solid #ffffff10;border-radius:12px;text-decoration:none;color:#e2e8f0;transition:all .2s}
    .site-card:hover{border-color:var(--c);transform:translateY(-2px);box-shadow:0 8px 30px var(--c)20}
    .site-icon{font-size:2.5rem;margin-bottom:0.5rem}
    .site-name{font-weight:700;font-size:1rem}
    .site-tag{font-size:0.75rem;color:#94a3b8;margin-top:0.25rem;text-align:center}
  </style>
</head><body>
  <h1>⭐ X3 Star Network</h1>
  <p class="sub">x3star.net ecosystem — ${Object.keys(sites).length} services</p>
  <div class="grid">${cards}</div>
</body></html>`;
}

server.listen(PORT, "0.0.0.0", () => {
  console.log(`⭐ x3star.net placeholder server running on :${PORT}`);
  console.log(`   Serving ${Object.keys(sites).length} subdomains`);
});
