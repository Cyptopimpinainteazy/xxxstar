#!/usr/bin/env node
/**
 * generate-site-index.js
 * Reads scripts/app-manifest.json and regenerates site/index.html.
 * Run: node scripts/generate-site-index.js
 */
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO = resolve(__dirname, "..");
const manifest = JSON.parse(readFileSync(resolve(__dirname, "app-manifest.json"), "utf8"));

const CATEGORY_ORDER = [
  "Infrastructure",
  "Trading & DeFi",
  "Validators",
  "Analytics",
  "Site Pages",
  "Tools",
];

const CATEGORY_TAG_CLASS = {
  "Infrastructure": "tag-infra",
  "Trading & DeFi": "tag-trading",
  "Validators":     "tag-validators",
  "Analytics":      "tag-analytics",
  "Site Pages":     "tag-site",
  "Tools":          "tag-tools",
};

function appHref(app) {
  if (app.siteSubdir === ".") return "./";
  return `${app.siteSubdir}/`;
}

function renderCard(app) {
  const href = appHref(app);
  const tagClass = CATEGORY_TAG_CLASS[app.category] ?? "tag-tools";
  return `      <a class="app-card" href="${href}" data-category="${app.category}" data-title="${app.title}" data-desc="${app.description}">
        <div class="app-icon">${app.icon}</div>
        <div class="app-title">${app.title}</div>
        <div class="app-desc">${app.description}</div>
        <span class="app-tag ${tagClass}">${app.category}</span>
      </a>`;
}

function renderSection(category, apps) {
  const escapedCat = category.replace(/&/g, "&amp;");
  const cards = apps.map(renderCard).join("\n");
  return `  <section class="section" data-category="${escapedCat}">
    <div class="section-title">${escapedCat}</div>
    <div class="app-grid">
${cards}
    </div>
  </section>`;
}

const grouped = {};
for (const app of manifest) {
  if (!grouped[app.category]) grouped[app.category] = [];
  grouped[app.category].push(app);
}

const sections = CATEGORY_ORDER
  .filter(cat => grouped[cat])
  .map(cat => renderSection(cat, grouped[cat]))
  .join("\n\n");

// Quick nav links (first 7 non-root apps)
const navApps = manifest.filter(a => a.siteSubdir !== ".").slice(0, 7);
const navLinks = navApps
  .map(a => `    <li><a href="${a.siteSubdir}/">${a.title.split(" ").slice(0, 2).join(" ")}</a></li>`)
  .join("\n");

const totalApps = manifest.length;

const html = `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>X3 Chain — No Frontend Left Behind</title>
  <style>
    *, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }
    :root {
      --bg: #07070d; --bg-card: #0d0d1a; --border: #1a1a2e;
      --accent: #ff6b35; --accent2: #00e5c3; --text: #e0e0e0; --muted: #666;
      --infra: #00bfff; --trading: #00e5c3; --validators: #a78bfa;
      --analytics: #fbbf24; --site: #ff6b35; --tools: #34d399;
    }
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; background: var(--bg); color: var(--text); min-height: 100vh; }
    nav { display: flex; align-items: center; justify-content: space-between; padding: 1rem 2rem; border-bottom: 1px solid var(--border); position: sticky; top: 0; background: rgba(7,7,13,.92); backdrop-filter: blur(12px); z-index: 50; }
    .nav-logo { display: flex; align-items: center; gap: .5rem; font-size: 1.1rem; font-weight: 700; color: var(--text); text-decoration: none; }
    .nav-logo span { color: var(--accent); }
    .nav-links { display: flex; gap: 1.5rem; list-style: none; }
    .nav-links a { font-size: .8rem; color: var(--muted); text-decoration: none; transition: color .15s; }
    .nav-links a:hover { color: var(--text); }
    .hero { text-align: center; padding: 5rem 2rem 3rem; max-width: 600px; margin: 0 auto; }
    .hero h1 { font-size: 2.5rem; font-weight: 800; line-height: 1.15; margin-bottom: 1rem; }
    .hero h1 em { font-style: normal; color: var(--accent); }
    .hero p { color: var(--muted); font-size: 1rem; line-height: 1.6; margin-bottom: 2rem; }
    .hero-badge { display: inline-block; font-size: .7rem; text-transform: uppercase; letter-spacing: .15em; color: var(--accent2); border: 1px solid var(--accent2); border-radius: 100px; padding: .2rem .8rem; margin-bottom: 1.5rem; }
    .search-bar { max-width: 420px; margin: 0 auto 3rem; position: relative; }
    .search-bar input { width: 100%; background: var(--bg-card); border: 1px solid var(--border); border-radius: .75rem; padding: .75rem 1rem .75rem 2.5rem; font-size: .875rem; color: var(--text); outline: none; transition: border-color .15s; }
    .search-bar input::placeholder { color: var(--muted); }
    .search-bar input:focus { border-color: rgba(255,107,53,.5); }
    .search-bar::before { content: "🔍"; position: absolute; left: .75rem; top: 50%; transform: translateY(-50%); font-size: .875rem; }
    .category-filter { display: flex; gap: .5rem; justify-content: center; flex-wrap: wrap; margin-bottom: 2.5rem; }
    .cat-btn { font-size: .7rem; padding: .25rem .75rem; border-radius: 100px; border: 1px solid var(--border); background: none; color: var(--muted); cursor: pointer; transition: all .15s; }
    .cat-btn:hover, .cat-btn.active { background: var(--accent); border-color: var(--accent); color: #fff; }
    main { max-width: 1200px; margin: 0 auto; padding: 0 2rem 4rem; }
    .section { margin-bottom: 3rem; }
    .section-title { font-size: .65rem; font-weight: 600; text-transform: uppercase; letter-spacing: .15em; color: var(--muted); margin-bottom: 1rem; }
    .app-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: .75rem; }
    .app-card { display: block; text-decoration: none; background: var(--bg-card); border: 1px solid var(--border); border-radius: .75rem; padding: 1.25rem; transition: border-color .15s, background .15s; }
    .app-card:hover { border-color: rgba(255,107,53,.4); background: #0f0f20; }
    .app-icon { font-size: 2rem; margin-bottom: .75rem; }
    .app-title { font-size: .875rem; font-weight: 600; color: var(--text); margin-bottom: .25rem; line-height: 1.3; }
    .app-card:hover .app-title { color: var(--accent); }
    .app-desc { font-size: .7rem; color: var(--muted); line-height: 1.5; margin-bottom: .75rem; display: -webkit-box; -webkit-line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
    .app-tag { font-size: .6rem; font-weight: 600; padding: .15rem .5rem; border-radius: 100px; border: 1px solid; display: inline-block; }
    .tag-infra       { color: var(--infra);      border-color: var(--infra);      background: rgba(0,191,255,.08); }
    .tag-trading     { color: var(--trading);    border-color: var(--trading);    background: rgba(0,229,195,.08); }
    .tag-validators  { color: var(--validators); border-color: var(--validators); background: rgba(167,139,250,.08); }
    .tag-analytics   { color: var(--analytics);  border-color: var(--analytics);  background: rgba(251,191,36,.08); }
    .tag-site        { color: var(--site);        border-color: var(--site);        background: rgba(255,107,53,.08); }
    .tag-tools       { color: var(--tools);       border-color: var(--tools);       background: rgba(52,211,153,.08); }
    footer { border-top: 1px solid var(--border); text-align: center; padding: 2rem; font-size: .75rem; color: var(--muted); }
    footer a { color: var(--accent); text-decoration: none; }
    .app-card[data-hidden="true"] { display: none; }
  </style>
</head>
<body>

<nav>
  <a href="/" class="nav-logo">⬡ X3 <span>Chain</span></a>
  <ul class="nav-links">
${navLinks}
  </ul>
</nav>

<div class="hero">
  <div class="hero-badge">No Frontend Left Behind</div>
  <h1>All <em>${totalApps}</em> frontends.<br/>One hub.</h1>
  <p>Every UI surface of X3 Chain — unified in one place.</p>
</div>

<div class="search-bar">
  <input type="text" id="search" placeholder="Search apps…" oninput="filterApps()" />
</div>

<div class="category-filter">
  <button class="cat-btn active" onclick="setCat('All', this)">All</button>
  ${CATEGORY_ORDER.map(c => `<button class="cat-btn" onclick="setCat('${c}', this)">${c.replace(/&/g, "&amp;")}</button>`).join("\n  ")}
</div>

<main>
${sections}
</main>

<footer>
  <p>X3 Chain &mdash; <a href="https://x3-chain.io">x3-chain.io</a> &mdash; No Frontend Left Behind</p>
</footer>

<script>
  let activeCategory = 'All';
  function setCat(cat, btn) {
    activeCategory = cat;
    document.querySelectorAll('.cat-btn').forEach(b => b.classList.remove('active'));
    btn.classList.add('active');
    filterApps();
  }
  function filterApps() {
    const q = document.getElementById('search').value.toLowerCase();
    document.querySelectorAll('.app-card').forEach(card => {
      const cardCat = card.dataset.category;
      const title = (card.dataset.title || '').toLowerCase();
      const desc = (card.dataset.desc || '').toLowerCase();
      const matchesCat = activeCategory === 'All' || cardCat === activeCategory;
      const matchesQ = !q || title.includes(q) || desc.includes(q) || cardCat.toLowerCase().includes(q);
      card.dataset.hidden = String(!(matchesCat && matchesQ));
    });
    document.querySelectorAll('.section').forEach(section => {
      const visible = [...section.querySelectorAll('.app-card')].some(c => c.dataset.hidden !== 'true');
      section.style.display = visible ? '' : 'none';
    });
  }
</script>

</body>
</html>`;

const outPath = resolve(REPO, "site", "index.html");
writeFileSync(outPath, html, "utf8");
console.log(`[generate-site-index] Written → ${outPath}`);
console.log(`[generate-site-index] ${manifest.length} apps across ${CATEGORY_ORDER.filter(c => grouped[c]).length} categories`);
