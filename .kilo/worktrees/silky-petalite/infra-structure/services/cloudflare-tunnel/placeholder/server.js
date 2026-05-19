#!/usr/bin/env node
// ============================================================================
// x3star.net — Multi-subdomain Placeholder Server
// ============================================================================
// A single Node.js server on port 7000 that serves unique landing pages
// per subdomain based on the Host header from Cloudflare Tunnel.
// ============================================================================
const http = require("http");
const fs = require("fs");
const path = require("path");

const PORT = process.env.PORT || 7000;
const PAGES_DIR = path.join(__dirname, "pages");

// ── Subdomain metadata ──────────────────────────────────────────────────────
const SITES = {
  ai:               { title: "X3 AI",             icon: "🤖", color: "#8b5cf6", tagline: "Decentralized AI Intelligence Platform" },
  chat:             { title: "X3 Chat",            icon: "💬", color: "#06b6d4", tagline: "Encrypted On-Chain Messaging" },
  social:           { title: "X3 Social",          icon: "🌐", color: "#ec4899", tagline: "Decentralized Social Network" },
  trading:          { title: "X3 Trading",         icon: "📈", color: "#10b981", tagline: "Advanced DeFi Trading Terminal" },
  adult:            { title: "X3 Adult",           icon: "🔞", color: "#f43f5e", tagline: "Age-Verified Content Platform" },
  crypto:           { title: "X3 Crypto",          icon: "₿",  color: "#f59e0b", tagline: "Multi-Chain Crypto Hub" },
  blockchain:       { title: "X3 Blockchain",      icon: "⛓️", color: "#3b82f6", tagline: "X3 Chain Chain Dashboard" },
  testnet:          { title: "X3 Testnet",         icon: "🧪", color: "#a855f7", tagline: "X3 Testnet Faucet & Explorer" },
  ide:              { title: "X3 IDE",             icon: "⌨️", color: "#22d3ee", tagline: "Browser-Based Smart Contract IDE" },
  marketplace:      { title: "X3 Marketplace",     icon: "🏪", color: "#f97316", tagline: "Decentralized Digital Marketplace" },
  pool:             { title: "X3 Pool",            icon: "🏊", color: "#14b8a6", tagline: "Liquidity & Staking Pools" },
  swarm:            { title: "X3 Swarm",           icon: "🐝", color: "#eab308", tagline: "AI Agent Swarm Orchestrator" },
  docs:             { title: "X3 Docs",            icon: "📚", color: "#6366f1", tagline: "Developer Documentation & Guides" },
  infura:           { title: "X3 Infura",          icon: "🔌", color: "#7c3aed", tagline: "Cross-Chain RPC Gateway" },
  validator:        { title: "X3 Validator",       icon: "✅", color: "#059669", tagline: "Node Validator Dashboard" },
  "gpu-validator":  { title: "X3 GPU Validator",   icon: "🎮", color: "#dc2626", tagline: "GPU Compute Validator Network" },
  "x-change":       { title: "X-Change",           icon: "🔄", color: "#2563eb", tagline: "Decentralized Exchange" },
  testing:          { title: "X3 Testing",         icon: "🧪", color: "#8b5cf6", tagline: "Test Harness & CI Dashboard" },
  test:             { title: "X3 Test",            icon: "🔬", color: "#a78bfa", tagline: "Staging & Preview Builds" },
  blockexplorer:    { title: "X3 Block Explorer",  icon: "🔍", color: "#0891b2", tagline: "Full Block Explorer" },
  xplorer:          { title: "X3plorer",           icon: "🗺️", color: "#0d9488", tagline: "Lightweight Chain Explorer" },
  xxx:              { title: "X3 XXX",             icon: "🔥", color: "#e11d48", tagline: "Premium Adult Content" },
  cam:              { title: "X3 Cam",             icon: "📹", color: "#be123c", tagline: "Live Streaming Platform" },
  wallet:           { title: "X3 Wallet",          icon: "👛", color: "#7c3aed", tagline: "Multi-Chain Web Wallet" },
  casino:           { title: "X3 Casino",          icon: "🎰", color: "#d97706", tagline: "Provably Fair On-Chain Gaming" },
  blog:             { title: "X3 Blog",            icon: "📝", color: "#4f46e5", tagline: "News, Updates & Insights" },
  xxxchange:        { title: "XXXchange",          icon: "💎", color: "#9f1239", tagline: "Adult Content Marketplace" },
  pics:             { title: "X3 Pics",            icon: "📸", color: "#db2777", tagline: "Decentralized Image Gallery" },
  vibecode:         { title: "X3 Vibecode",        icon: "🎵", color: "#06b6d4", tagline: "Collaborative Code & IDE" },
  explorer:         { title: "X3 Explorer",        icon: "🔭", color: "#0284c7", tagline: "X3 Chain Explorer" },
};

// ── HTML template ───────────────────────────────────────────────────────────
function renderPage(sub) {
  const s = SITES[sub] || { title: `X3 ${sub}`, icon: "⭐", color: "#6366f1", tagline: "Coming Soon" };
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8"/>
  <meta name="viewport" content="width=device-width,initial-scale=1"/>
  <title>${s.title} — x3star.net</title>
  <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>${encodeURIComponent(s.icon)}</text></svg>"/>
  <style>
    *{margin:0;padding:0;box-sizing:border-box}
    :root{--accent:${s.color};--bg:#0a0a0f;--card:rgba(255,255,255,.04)}
    body{font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',system-ui,sans-serif;background:var(--bg);color:#e2e8f0;min-height:100vh;display:flex;align-items:center;justify-content:center;overflow:hidden}
    .bg{position:fixed;inset:0;z-index:0}
    .bg::before{content:'';position:absolute;inset:0;background:radial-gradient(ellipse at 30% 20%,color-mix(in srgb,var(--accent) 15%,transparent),transparent 60%),radial-gradient(ellipse at 70% 80%,color-mix(in srgb,var(--accent) 8%,transparent),transparent 60%)}
    .grid{position:fixed;inset:0;background-image:linear-gradient(rgba(255,255,255,.02) 1px,transparent 1px),linear-gradient(90deg,rgba(255,255,255,.02) 1px,transparent 1px);background-size:60px 60px;z-index:0}
    .container{position:relative;z-index:1;text-align:center;max-width:600px;padding:2rem}
    .icon{font-size:5rem;margin-bottom:1.5rem;filter:drop-shadow(0 0 40px var(--accent));animation:float 3s ease-in-out infinite}
    @keyframes float{0%,100%{transform:translateY(0)}50%{transform:translateY(-12px)}}
    h1{font-size:2.5rem;font-weight:800;background:linear-gradient(135deg,#fff,var(--accent));-webkit-background-clip:text;-webkit-text-fill-color:transparent;margin-bottom:.5rem}
    .tagline{font-size:1.2rem;color:#94a3b8;margin-bottom:2rem}
    .badge{display:inline-block;padding:.4rem 1rem;border-radius:9999px;border:1px solid color-mix(in srgb,var(--accent) 40%,transparent);background:color-mix(in srgb,var(--accent) 10%,transparent);color:var(--accent);font-size:.85rem;font-weight:600;letter-spacing:.05em;text-transform:uppercase;margin-bottom:2rem}
    .card{background:var(--card);backdrop-filter:blur(20px);border:1px solid rgba(255,255,255,.06);border-radius:1rem;padding:2rem;margin-top:1.5rem}
    .card p{color:#64748b;line-height:1.6;font-size:.95rem}
    .card a{color:var(--accent);text-decoration:none;font-weight:500}
    .card a:hover{text-decoration:underline}
    .dots{display:flex;gap:.5rem;justify-content:center;margin-top:1.5rem}
    .dots span{width:8px;height:8px;border-radius:50%;background:var(--accent);opacity:.3;animation:pulse 1.5s ease-in-out infinite}
    .dots span:nth-child(2){animation-delay:.2s}
    .dots span:nth-child(3){animation-delay:.4s}
    @keyframes pulse{0%,100%{opacity:.3;transform:scale(1)}50%{opacity:1;transform:scale(1.2)}}
    .footer{margin-top:2rem;font-size:.8rem;color:#475569}
    .footer a{color:#64748b}
  </style>
</head>
<body>
  <div class="bg"></div>
  <div class="grid"></div>
  <div class="container">
    <div class="icon">${s.icon}</div>
    <div class="badge">Coming Soon</div>
    <h1>${s.title}</h1>
    <p class="tagline">${s.tagline}</p>
    <div class="card">
      <p>This service is being deployed on the <strong>X3 Chain</strong> network. Stay tuned for launch.</p>
      <div class="dots"><span></span><span></span><span></span></div>
    </div>
    <p class="footer">
      <a href="https://x3star.net">x3star.net</a> &middot; ${sub}.x3star.net &middot; Powered by X3 Chain
    </p>
  </div>
</body>
</html>`;
}

// ── Server ──────────────────────────────────────────────────────────────────
const server = http.createServer((req, res) => {
  const host = (req.headers.host || "").toLowerCase();
  const sub = host.replace(/\.x3star\.net(:\d+)?$/, "").replace(/:\d+$/, "");

  // Health check
  if (req.url === "/health") {
    res.writeHead(200, { "Content-Type": "text/plain" });
    return res.end("ok");
  }

  // Serve static page if exists, otherwise render from template
  const staticFile = path.join(PAGES_DIR, `${sub}.html`);
  if (fs.existsSync(staticFile)) {
    res.writeHead(200, { "Content-Type": "text/html; charset=utf-8" });
    return res.end(fs.readFileSync(staticFile, "utf8"));
  }

  const html = renderPage(sub);
  res.writeHead(200, {
    "Content-Type": "text/html; charset=utf-8",
    "Cache-Control": "public, max-age=300",
  });
  res.end(html);
});

server.listen(PORT, "0.0.0.0", () => {
  console.log(`✅ x3star.net placeholder server running on :${PORT}`);
  console.log(`   Serving ${Object.keys(SITES).length} subdomains`);
});
