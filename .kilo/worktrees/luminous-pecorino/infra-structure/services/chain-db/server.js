#!/usr/bin/env node
/**
 * X3 Chain — Chain Database API Server
 *
 * High-performance REST API serving 60,000+ blockchain chain data from SQLite.
 * Powers the inferstructor dashboard chain explorer.
 *
 * Endpoints:
 *   GET  /api/chains                  — Paginated chain listing with filters
 *   GET  /api/chains/search?q=        — Full-text search across chains
 *   GET  /api/chains/:chainId         — Single chain detail with RPC + GPU stats
 *   GET  /api/chains/stats/overview   — Aggregate ecosystem statistics
 *   GET  /api/chains/stats/ecosystems — Ecosystem breakdown
 *   GET  /api/rpc/:chainId            — RPC endpoints for a chain
 *   GET  /api/gpu-stats/:chainId      — GPU validation stats for a chain
 *   GET  /health                      — Health check
 */

const express = require('express');
const cors = require('cors');
const Database = require('better-sqlite3');
const path = require('path');
const crypto = require('crypto');

const app = express();
const PORT = process.env.CHAIN_DB_PORT || 7070;
const DB_PATH = process.env.CHAIN_DB_PATH || path.join(__dirname, '..', '..', 'db', 'chains.db');

app.use(cors());
app.use(express.json());

// ── Database connection ──────────────────────────────────────────────────────

let db;   // read-only connection
let dbW;  // read-write connection (for airdrops/faucets/wallets writes)
try {
  db = new Database(DB_PATH);
  db.pragma('journal_mode = WAL');
  db.pragma('cache_size = -64000'); // 64MB cache
  db.pragma('query_only = ON');     // prevent accidental writes via API

  dbW = new Database(DB_PATH);
  dbW.pragma('journal_mode = WAL');
  dbW.pragma('cache_size = -8000'); // 8MB cache for write conn

  const count = db.prepare('SELECT COUNT(*) as cnt FROM chains').get();
  console.log(`✓ Chain DB connected: ${count.cnt.toLocaleString()} chains loaded from ${DB_PATH}`);
  console.log(`✓ Write connection ready for airdrops/faucets/wallets`);

  // Ensure connector table exists (safe for existing DBs)
  dbW.exec(`
    CREATE TABLE IF NOT EXISTS chain_connectors (
      id               INTEGER PRIMARY KEY AUTOINCREMENT,
      chain_id         TEXT NOT NULL,
      rpc_url          TEXT NOT NULL,
      auth_type        TEXT NOT NULL DEFAULT 'none',
      credential_mask  TEXT,
      credential_enc   TEXT,
      notes            TEXT,
      status           TEXT NOT NULL DEFAULT 'active',
      created_at       DATETIME DEFAULT CURRENT_TIMESTAMP,
      updated_at       DATETIME DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS idx_connectors_chain ON chain_connectors(chain_id);
  `);
} catch (err) {
  console.error(`✗ Failed to open chain database at ${DB_PATH}:`, err.message);
  process.exit(1);
}

// ── Credential encryption helpers ────────────────────────────────────────────

const CRED_KEY_RAW = process.env.CHAIN_DB_CRED_KEY || '';

function loadCredKey() {
  if (!CRED_KEY_RAW) return null;
  if (/^[0-9a-fA-F]{64}$/.test(CRED_KEY_RAW)) return Buffer.from(CRED_KEY_RAW, 'hex');
  try {
    const buf = Buffer.from(CRED_KEY_RAW, 'base64');
    if (buf.length === 32) return buf;
  } catch (e) {
    // ignore
  }
  if (Buffer.byteLength(CRED_KEY_RAW) === 32) return Buffer.from(CRED_KEY_RAW, 'utf8');
  throw new Error('CHAIN_DB_CRED_KEY must be 32 bytes (hex or base64)');
}

const CRED_KEY = loadCredKey();

function maskCredential(value) {
  if (!value) return null;
  if (value.length <= 8) return '****';
  return `${value.slice(0, 4)}...${value.slice(-2)}`;
}

function encryptCredential(value) {
  if (!value) return null;
  if (!CRED_KEY) throw new Error('CHAIN_DB_CRED_KEY not set; refusing to store credentials');
  const iv = crypto.randomBytes(12);
  const cipher = crypto.createCipheriv('aes-256-gcm', CRED_KEY, iv);
  const enc = Buffer.concat([cipher.update(value, 'utf8'), cipher.final()]);
  const tag = cipher.getAuthTag();
  return JSON.stringify({
    v: 1,
    iv: iv.toString('base64'),
    tag: tag.toString('base64'),
    data: enc.toString('base64'),
  });
}

// ── Prepared statements (perf optimization) ──────────────────────────────────

const stmts = {
  listChains: db.prepare(`
    SELECT chain_id, chain_name, chain_numeric_id, ecosystem, chain_type,
           consensus, native_token, is_evm, is_svm, is_testnet,
           supports_gpu, status
    FROM chains
    WHERE (:ecosystem IS NULL OR ecosystem = :ecosystem)
      AND (:chain_type IS NULL OR chain_type = :chain_type)
      AND (:status IS NULL OR status = :status)
      AND (:is_evm IS NULL OR is_evm = :is_evm)
      AND (:is_svm IS NULL OR is_svm = :is_svm)
      AND (:is_testnet IS NULL OR is_testnet = :is_testnet)
    ORDER BY chain_name
    LIMIT :limit OFFSET :offset
  `),

  countChains: db.prepare(`
    SELECT COUNT(*) as total
    FROM chains
    WHERE (:ecosystem IS NULL OR ecosystem = :ecosystem)
      AND (:chain_type IS NULL OR chain_type = :chain_type)
      AND (:status IS NULL OR status = :status)
      AND (:is_evm IS NULL OR is_evm = :is_evm)
      AND (:is_svm IS NULL OR is_svm = :is_svm)
      AND (:is_testnet IS NULL OR is_testnet = :is_testnet)
  `),

  searchChains: db.prepare(`
    SELECT c.chain_id, c.chain_name, c.chain_numeric_id, c.ecosystem, c.chain_type,
           c.consensus, c.native_token, c.is_evm, c.is_svm, c.is_testnet,
           c.supports_gpu, c.status
    FROM chains_fts fts
    JOIN chains c ON c.chain_id = fts.chain_id
    WHERE chains_fts MATCH :query
    ORDER BY rank
    LIMIT :limit
  `),

  getChain: db.prepare(`
    SELECT * FROM chains WHERE chain_id = ?
  `),

  getRpcEndpoints: db.prepare(`
    SELECT * FROM rpc_endpoints WHERE chain_id = ? ORDER BY is_primary DESC
  `),

  getGpuStats: db.prepare(`
    SELECT * FROM gpu_validation_stats WHERE chain_id = ?
  `),

  getMetrics: db.prepare(`
    SELECT * FROM chain_metrics WHERE chain_id = ? ORDER BY measured_at DESC LIMIT 1
  `),
  getConnectorsByChain: db.prepare(`
    SELECT cc.*, c.chain_name, c.ecosystem, c.chain_type
    FROM chain_connectors cc
    LEFT JOIN chains c ON c.chain_id = cc.chain_id
    WHERE cc.chain_id = ?
    ORDER BY cc.created_at DESC
  `),

  ecosystemStats: db.prepare(`
    SELECT ecosystem,
           COUNT(*) as chain_count,
           SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END) as active,
           SUM(CASE WHEN is_testnet = 1 THEN 1 ELSE 0 END) as testnets,
           SUM(CASE WHEN supports_gpu = 1 THEN 1 ELSE 0 END) as gpu_enabled
    FROM chains
    GROUP BY ecosystem
    ORDER BY chain_count DESC
  `),

  overviewStats: db.prepare(`
    SELECT
      COUNT(*) as total_chains,
      SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END) as active_chains,
      SUM(CASE WHEN is_evm = 1 THEN 1 ELSE 0 END) as evm_chains,
      SUM(CASE WHEN is_svm = 1 THEN 1 ELSE 0 END) as svm_chains,
      SUM(CASE WHEN is_testnet = 1 THEN 1 ELSE 0 END) as testnets,
      SUM(CASE WHEN supports_gpu = 1 THEN 1 ELSE 0 END) as gpu_enabled,
      COUNT(DISTINCT ecosystem) as ecosystems,
      COUNT(DISTINCT chain_type) as chain_types
    FROM chains
  `),

  chainTypeBreakdown: db.prepare(`
    SELECT chain_type, COUNT(*) as count
    FROM chains
    GROUP BY chain_type
    ORDER BY count DESC
  `),

  consensusBreakdown: db.prepare(`
    SELECT consensus, COUNT(*) as count
    FROM chains
    GROUP BY consensus
    ORDER BY count DESC
  `),

  recentChains: db.prepare(`
    SELECT chain_id, chain_name, ecosystem, chain_type, status
    FROM chains
    ORDER BY created_at DESC
    LIMIT ?
  `),

  listConnectors: db.prepare(`
    SELECT cc.*, c.chain_name, c.ecosystem, c.chain_type
    FROM chain_connectors cc
    LEFT JOIN chains c ON c.chain_id = cc.chain_id
    ORDER BY cc.created_at DESC
  `),

  insertConnector: dbW.prepare(`
    INSERT INTO chain_connectors (chain_id, rpc_url, auth_type, credential_mask, credential_enc, notes, status)
    VALUES (?, ?, ?, ?, ?, ?, ?)
  `),

  countPrimaryRpc: db.prepare(`
    SELECT COUNT(*) as cnt FROM rpc_endpoints WHERE chain_id = ? AND is_primary = 1
  `),

  insertRpcEndpoint: dbW.prepare(`
    INSERT INTO rpc_endpoints (chain_id, url, protocol, provider, tier, is_primary, is_healthy)
    VALUES (?, ?, ?, ?, ?, ?, 1)
  `),
};

// ── Routes ───────────────────────────────────────────────────────────────────

// Health check
app.get('/health', (_req, res) => {
  const count = db.prepare('SELECT COUNT(*) as cnt FROM chains').get();
  res.json({
    status: 'healthy',
    service: 'x3-chain-db',
    chains_loaded: count.cnt,
    db_path: DB_PATH,
  });
});

// GET /api/chains — Paginated listing with filters
app.get('/api/chains', (req, res) => {
  const page = Math.max(1, parseInt(req.query.page) || 1);
  const limit = Math.min(500, Math.max(1, parseInt(req.query.limit) || 50));
  const offset = (page - 1) * limit;

  const filters = {
    ecosystem: req.query.ecosystem || null,
    chain_type: req.query.chain_type || null,
    status: req.query.status || null,
    is_evm: req.query.is_evm != null ? parseInt(req.query.is_evm) : null,
    is_svm: req.query.is_svm != null ? parseInt(req.query.is_svm) : null,
    is_testnet: req.query.is_testnet != null ? parseInt(req.query.is_testnet) : null,
    limit,
    offset,
  };

  const chains = stmts.listChains.all(filters);
  const { total } = stmts.countChains.get(filters);

  res.json({
    chains,
    pagination: {
      page,
      limit,
      total,
      total_pages: Math.ceil(total / limit),
      has_next: page * limit < total,
      has_prev: page > 1,
    },
  });
});

// GET /api/chains/search?q=ethereum
app.get('/api/chains/search', (req, res) => {
  const q = (req.query.q || '').trim();
  if (!q) {
    return res.status(400).json({ error: 'Query parameter "q" is required' });
  }

  const limit = Math.min(200, Math.max(1, parseInt(req.query.limit) || 50));

  // FTS5 query — support both prefix match and quoted exact match
  const ftsQuery = q.includes('"') ? q : `${q}*`;

  try {
    const chains = stmts.searchChains.all({ query: ftsQuery, limit });
    res.json({ query: q, results: chains, count: chains.length });
  } catch (err) {
    // Fallback to LIKE if FTS fails
    const fallback = db.prepare(`
      SELECT chain_id, chain_name, chain_numeric_id, ecosystem, chain_type,
             consensus, native_token, is_evm, is_svm, is_testnet,
             supports_gpu, status
      FROM chains
      WHERE chain_name LIKE ? OR chain_id LIKE ?
      ORDER BY chain_name
      LIMIT ?
    `).all(`%${q}%`, `%${q}%`, limit);
    res.json({ query: q, results: fallback, count: fallback.length, fallback: true });
  }
});

// GET /api/chains/stats/overview
app.get('/api/chains/stats/overview', (_req, res) => {
  const overview = stmts.overviewStats.get();
  const ecosystems = stmts.ecosystemStats.all();
  const chainTypes = stmts.chainTypeBreakdown.all();
  const consensus = stmts.consensusBreakdown.all();

  res.json({ overview, ecosystems, chain_types: chainTypes, consensus });
});

// GET /api/chains/stats/ecosystems
app.get('/api/chains/stats/ecosystems', (_req, res) => {
  const ecosystems = stmts.ecosystemStats.all();
  res.json({ ecosystems });
});

// GET /api/chains/recent?limit=20
app.get('/api/chains/recent', (req, res) => {
  const limit = Math.min(100, Math.max(1, parseInt(req.query.limit) || 20));
  const chains = stmts.recentChains.all(limit);
  res.json({ chains });
});

// GET /api/chains/:chainId — Full chain detail
app.get('/api/chains/:chainId', (req, res) => {
  const chain = stmts.getChain.get(req.params.chainId);
  if (!chain) {
    return res.status(404).json({ error: 'Chain not found', chain_id: req.params.chainId });
  }

  const rpc_endpoints = stmts.getRpcEndpoints.all(req.params.chainId);
  const gpu_stats = stmts.getGpuStats.get(req.params.chainId);
  const latest_metrics = stmts.getMetrics.get(req.params.chainId);
  const connectors = stmts.getConnectorsByChain.all(req.params.chainId);

  res.json({ ...chain, rpc_endpoints, gpu_stats, latest_metrics, connectors });
});

// GET /api/rpc/stats — RPC pool stats with gas savings calculation
// NOTE: this MUST be registered before /api/rpc/:chainId so Express matches it first
app.get('/api/rpc/stats', (_req, res) => {
  try {
    const totalRpcs = db.prepare('SELECT COUNT(*) as cnt FROM rpc_endpoints').get();
    const healthyRpcs = db.prepare('SELECT COUNT(*) as cnt FROM rpc_endpoints WHERE is_healthy = 1').get();
    const chainsWithRpcs = db.prepare('SELECT COUNT(DISTINCT chain_id) as cnt FROM rpc_endpoints WHERE is_healthy = 1').get();
    const totalRps = db.prepare('SELECT COALESCE(SUM(rate_limit_rps), 0) as rps FROM rpc_endpoints WHERE is_healthy = 1').get();
    const avgLatency = db.prepare('SELECT COALESCE(AVG(latency_ms), 0) as avg, COALESCE(MIN(latency_ms), 0) as min FROM rpc_endpoints WHERE is_healthy = 1 AND latency_ms IS NOT NULL AND latency_ms > 0').get();
    const byProvider = db.prepare(`
      SELECT provider, COUNT(*) as count, COALESCE(AVG(latency_ms), 0) as avg_latency,
             SUM(rate_limit_rps) as rps
      FROM rpc_endpoints WHERE is_healthy = 1 AND provider IS NOT NULL
      GROUP BY provider ORDER BY count DESC LIMIT 15
    `).all();
    const byTier = db.prepare(`
      SELECT COALESCE(tier, 'public') as tier, COUNT(*) as count
      FROM rpc_endpoints WHERE is_healthy = 1
      GROUP BY tier ORDER BY count DESC
    `).all();
    const topFastest = db.prepare(`
      SELECT chain_id, url, provider, latency_ms, rate_limit_rps
      FROM rpc_endpoints WHERE is_healthy = 1 AND latency_ms > 0
      ORDER BY latency_ms ASC LIMIT 10
    `).all();

    // Gas savings: calculate equivalent paid plan costs
    const rps = totalRps.rps || 0;
    const gasSavings = {
      infura_growth_equiv: Math.round(rps / 50 * 225),     // $225/mo for 50 rps
      alchemy_growth_equiv: Math.round(rps / 660 * 199),   // $199/mo for 660 rps
      quicknode_build_equiv: Math.round(rps / 300 * 299),  // $299/mo for 300 rps
      moralis_pro_equiv: Math.round(rps / 500 * 299),      // $299/mo for 500 rps
      total_monthly_saved: Math.round((rps / 50 * 225 + rps / 660 * 199 + rps / 300 * 299) / 3),
      your_cost: 0,
    };

    res.json({
      total_endpoints: totalRpcs.cnt,
      healthy_endpoints: healthyRpcs.cnt,
      chains_covered: chainsWithRpcs.cnt,
      combined_rps: rps,
      avg_latency_ms: Math.round(avgLatency.avg),
      min_latency_ms: Math.round(avgLatency.min),
      by_provider: byProvider,
      by_tier: byTier,
      top_fastest: topFastest,
      gas_savings: gasSavings,
    });
  } catch (err) {
    res.status(500).json({ error: 'Failed to compute RPC stats', message: err.message });
  }
});

// GET /api/rpc/:chainId
app.get('/api/rpc/:chainId', (req, res) => {
  const endpoints = stmts.getRpcEndpoints.all(req.params.chainId);
  res.json({ chain_id: req.params.chainId, endpoints, count: endpoints.length });
});

// GET /api/connectors — Onboarded chain connectors (masked)
app.get('/api/connectors', (_req, res) => {
  const items = stmts.listConnectors.all();
  const sanitized = items.map((row) => ({
    id: row.id,
    chain_id: row.chain_id,
    chain_name: row.chain_name || null,
    ecosystem: row.ecosystem || null,
    chain_type: row.chain_type || null,
    rpc_url: row.rpc_url,
    auth_type: row.auth_type,
    credential_mask: row.credential_mask,
    notes: row.notes || null,
    status: row.status,
    created_at: row.created_at,
    updated_at: row.updated_at,
  }));
  res.json({ items: sanitized });
});

// POST /api/connectors — Register a new connector (stores masked + encrypted credential)
app.post('/api/connectors', (req, res) => {
  const adminKey = process.env.CHAIN_DB_ADMIN_KEY || '';
  const reqKey = req.headers['x-admin-key'];
  if (adminKey && reqKey !== adminKey) {
    return res.status(401).json({ error: 'unauthorized' });
  }

  const {
    chain_id,
    rpc_url,
    auth_type = 'none',
    credential = '',
    notes = '',
    status = 'active',
  } = req.body || {};

  if (!chain_id || !rpc_url) {
    return res.status(400).json({ error: 'chain_id and rpc_url are required' });
  }

  try {
    const masked = maskCredential(String(credential || ''));
    const enc = credential ? encryptCredential(String(credential)) : null;

    const result = stmts.insertConnector.run(
      String(chain_id),
      String(rpc_url),
      String(auth_type || 'none'),
      masked,
      enc,
      String(notes || ''),
      String(status || 'active')
    );

    const protocol = String(rpc_url).startsWith('wss') ? 'wss' : (String(rpc_url).startsWith('ws') ? 'ws' : 'https');
    const primaryCount = stmts.countPrimaryRpc.get(String(chain_id)).cnt || 0;
    const isPrimary = primaryCount === 0 ? 1 : 0;
    stmts.insertRpcEndpoint.run(
      String(chain_id),
      String(rpc_url),
      protocol,
      'custom',
      credential ? 'authenticated' : 'public',
      isPrimary
    );

    return res.json({
      ok: true,
      connector: {
        id: result.lastInsertRowid,
        chain_id: String(chain_id),
        rpc_url: String(rpc_url),
        auth_type: String(auth_type || 'none'),
        credential_mask: masked,
        notes: String(notes || ''),
        status: String(status || 'active'),
      },
    });
  } catch (err) {
    return res.status(500).json({ error: 'failed to store connector', message: err.message });
  }
});

// GET /api/gpu-stats/:chainId
app.get('/api/gpu-stats/:chainId', (req, res) => {
  const stats = stmts.getGpuStats.get(req.params.chainId);
  if (!stats) {
    return res.status(404).json({ error: 'No GPU stats for chain', chain_id: req.params.chainId });
  }
  res.json(stats);
});

// ── Airdrops / Faucets / Wallets / Chain Discoveries API ──────────────────────

// GET /api/airdrops — List airdrops with filters
app.get('/api/airdrops', (req, res) => {
  try {
    const status = req.query.status || null;
    const chain = req.query.chain_id || null;
    const type = req.query.type || null;
    const limit = Math.min(200, Math.max(1, parseInt(req.query.limit) || 50));
    const offset = Math.max(0, parseInt(req.query.offset) || 0);

    const airdrops = db.prepare(`
      SELECT * FROM airdrops
      WHERE (:status IS NULL OR status = :status)
        AND (:chain IS NULL OR chain_id = :chain)
        AND (:type IS NULL OR airdrop_type = :type)
      ORDER BY
        CASE WHEN claim_deadline IS NOT NULL AND claim_deadline > datetime('now') THEN 0 ELSE 1 END,
        claim_deadline ASC,
        discovered_at DESC
      LIMIT :limit OFFSET :offset
    `).all({ status, chain, type, limit, offset });

    const total = db.prepare(`
      SELECT COUNT(*) as cnt FROM airdrops
      WHERE (:status IS NULL OR status = :status)
        AND (:chain IS NULL OR chain_id = :chain)
        AND (:type IS NULL OR airdrop_type = :type)
    `).get({ status, chain, type });

    const stats = db.prepare(`
      SELECT
        COUNT(*) as total,
        SUM(CASE WHEN status = 'discovered' THEN 1 ELSE 0 END) as discovered,
        SUM(CASE WHEN status = 'eligible' THEN 1 ELSE 0 END) as eligible,
        SUM(CASE WHEN status = 'claimed' THEN 1 ELSE 0 END) as claimed,
        SUM(CASE WHEN status = 'expired' THEN 1 ELSE 0 END) as expired,
        SUM(CASE WHEN claim_deadline IS NOT NULL AND claim_deadline > datetime('now') THEN 1 ELSE 0 END) as active_deadlines,
        COALESCE(SUM(estimated_value), 0) as total_estimated_value,
        COALESCE(SUM(actual_value), 0) as total_actual_value
      FROM airdrops
    `).get();

    res.json({ airdrops, total: total.cnt, stats });
  } catch (err) {
    res.status(500).json({ error: 'Failed to fetch airdrops', message: err.message });
  }
});

// POST /api/airdrops — Add a new airdrop
app.post('/api/airdrops', (req, res) => {
  try {
    const { chain_id, name, project, token_symbol, token_address, airdrop_type, source, source_url, claim_url, claim_start, claim_deadline, snapshot_date, estimated_value, eligibility_criteria, notes } = req.body;
    if (!chain_id || !name) return res.status(400).json({ error: 'chain_id and name are required' });

    const result = dbW.prepare(`
      INSERT INTO airdrops (chain_id, name, project, token_symbol, token_address, airdrop_type, source, source_url, claim_url, claim_start, claim_deadline, snapshot_date, estimated_value, eligibility_criteria, notes)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(chain_id, name, project || null, token_symbol || null, token_address || null, airdrop_type || 'unknown', source || 'manual', source_url || null, claim_url || null, claim_start || null, claim_deadline || null, snapshot_date || null, estimated_value || 0, eligibility_criteria || null, notes || null);

    res.json({ id: result.lastInsertRowid, message: 'Airdrop added' });
  } catch (err) {
    res.status(500).json({ error: 'Failed to add airdrop', message: err.message });
  }
});

// PATCH /api/airdrops/:id — Update an airdrop
app.patch('/api/airdrops/:id', (req, res) => {
  try {
    const allowed = ['status', 'claim_url', 'claim_start', 'claim_deadline', 'estimated_value', 'actual_value', 'notes', 'eligibility_criteria'];
    const sets = [];
    const vals = {};
    for (const key of allowed) {
      if (req.body[key] !== undefined) {
        sets.push(`${key} = @${key}`);
        vals[key] = req.body[key];
      }
    }
    if (sets.length === 0) return res.status(400).json({ error: 'No valid fields to update' });
    sets.push("updated_at = datetime('now')");
    vals.id = parseInt(req.params.id);

    dbW.prepare(`UPDATE airdrops SET ${sets.join(', ')} WHERE id = @id`).run(vals);
    res.json({ message: 'Airdrop updated' });
  } catch (err) {
    res.status(500).json({ error: 'Failed to update airdrop', message: err.message });
  }
});

// GET /api/faucets — List faucets with filters
app.get('/api/faucets', (req, res) => {
  try {
    const status = req.query.status || null;
    const chain = req.query.chain_id || null;
    const limit = Math.min(200, Math.max(1, parseInt(req.query.limit) || 50));

    const faucets = db.prepare(`
      SELECT * FROM faucets
      WHERE (:status IS NULL OR status = :status)
        AND (:chain IS NULL OR chain_id = :chain)
      ORDER BY
        CASE WHEN status = 'active' THEN 0 ELSE 1 END,
        last_claimed ASC NULLS FIRST,
        discovered_at DESC
      LIMIT :limit
    `).all({ status, chain, limit });

    const stats = db.prepare(`
      SELECT
        COUNT(*) as total,
        SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END) as active,
        SUM(CASE WHEN status = 'dead' THEN 1 ELSE 0 END) as dead,
        SUM(total_claims) as total_claims,
        COUNT(DISTINCT chain_id) as chains_covered
      FROM faucets
    `).get();

    res.json({ faucets, stats });
  } catch (err) {
    res.status(500).json({ error: 'Failed to fetch faucets', message: err.message });
  }
});

// POST /api/faucets — Add a faucet
app.post('/api/faucets', (req, res) => {
  try {
    const { chain_id, name, provider, url, faucet_type, token_symbol, amount_per_claim, cooldown_hours, requires_auth, auth_type, source } = req.body;
    if (!chain_id || !name || !url) return res.status(400).json({ error: 'chain_id, name, and url are required' });

    const result = dbW.prepare(`
      INSERT OR IGNORE INTO faucets (chain_id, name, provider, url, faucet_type, token_symbol, amount_per_claim, cooldown_hours, requires_auth, auth_type, source)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `).run(chain_id, name, provider || null, url, faucet_type || 'web', token_symbol || null, amount_per_claim || null, cooldown_hours || 24, requires_auth ? 1 : 0, auth_type || null, source || 'manual');

    res.json({ id: result.lastInsertRowid, message: result.changes > 0 ? 'Faucet added' : 'Faucet already exists' });
  } catch (err) {
    res.status(500).json({ error: 'Failed to add faucet', message: err.message });
  }
});

// PATCH /api/faucets/:id — Update a faucet
app.patch('/api/faucets/:id', (req, res) => {
  try {
    const allowed = ['status', 'last_checked', 'last_claimed', 'total_claims', 'total_received', 'cooldown_hours'];
    const sets = [];
    const vals = {};
    for (const key of allowed) {
      if (req.body[key] !== undefined) {
        sets.push(`${key} = @${key}`);
        vals[key] = req.body[key];
      }
    }
    if (sets.length === 0) return res.status(400).json({ error: 'No valid fields to update' });
    sets.push("updated_at = datetime('now')");
    vals.id = parseInt(req.params.id);

    dbW.prepare(`UPDATE faucets SET ${sets.join(', ')} WHERE id = @id`).run(vals);
    res.json({ message: 'Faucet updated' });
  } catch (err) {
    res.status(500).json({ error: 'Failed to update faucet', message: err.message });
  }
});

// GET /api/wallets — List wallets
app.get('/api/wallets', (req, res) => {
  try {
    const chain = req.query.chain_id || null;
    const active = req.query.active != null ? parseInt(req.query.active) : null;

    const wallets = db.prepare(`
      SELECT id, chain_id, address, label, ecosystem, is_active, balance, balance_usd, last_balance_check, created_at
      FROM wallets
      WHERE (:chain IS NULL OR chain_id = :chain)
        AND (:active IS NULL OR is_active = :active)
      ORDER BY chain_id, created_at
    `).all({ chain, active });

    const stats = db.prepare(`
      SELECT
        COUNT(*) as total,
        COUNT(DISTINCT chain_id) as chains,
        SUM(CASE WHEN is_active = 1 THEN 1 ELSE 0 END) as active,
        COALESCE(SUM(balance_usd), 0) as total_balance_usd
      FROM wallets
    `).get();

    res.json({ wallets, stats });
  } catch (err) {
    res.status(500).json({ error: 'Failed to fetch wallets', message: err.message });
  }
});

// POST /api/wallets — Add a wallet
app.post('/api/wallets', (req, res) => {
  try {
    const { chain_id, address, label, ecosystem } = req.body;
    if (!chain_id || !address) return res.status(400).json({ error: 'chain_id and address are required' });

    const result = dbW.prepare(`
      INSERT OR IGNORE INTO wallets (chain_id, address, label, ecosystem)
      VALUES (?, ?, ?, ?)
    `).run(chain_id, address, label || 'auto', ecosystem || 'evm');

    res.json({ id: result.lastInsertRowid, message: result.changes > 0 ? 'Wallet added' : 'Wallet already exists' });
  } catch (err) {
    res.status(500).json({ error: 'Failed to add wallet', message: err.message });
  }
});

// GET /api/discoveries — List chain discoveries
app.get('/api/discoveries', (req, res) => {
  try {
    const status = req.query.status || null;
    const limit = Math.min(200, Math.max(1, parseInt(req.query.limit) || 50));

    const discoveries = db.prepare(`
      SELECT * FROM chain_discoveries
      WHERE (:status IS NULL OR status = :status)
      ORDER BY discovered_at DESC
      LIMIT :limit
    `).all({ status, limit });

    const stats = db.prepare(`
      SELECT
        COUNT(*) as total,
        SUM(CASE WHEN status = 'new' THEN 1 ELSE 0 END) as new_chains,
        SUM(CASE WHEN status = 'added' THEN 1 ELSE 0 END) as added,
        SUM(CASE WHEN is_testnet = 1 THEN 1 ELSE 0 END) as testnets
      FROM chain_discoveries
    `).get();

    res.json({ discoveries, stats });
  } catch (err) {
    res.status(500).json({ error: 'Failed to fetch discoveries', message: err.message });
  }
});

// GET /api/airdrop-claims — Claim history
app.get('/api/airdrop-claims', (req, res) => {
  try {
    const claims = db.prepare(`
      SELECT ac.*, a.name as airdrop_name, a.token_symbol, w.address as wallet_address, w.chain_id as wallet_chain
      FROM airdrop_claims ac
      JOIN airdrops a ON a.id = ac.airdrop_id
      JOIN wallets w ON w.id = ac.wallet_id
      ORDER BY ac.created_at DESC
      LIMIT 100
    `).all();
    res.json({ claims });
  } catch (err) {
    res.status(500).json({ error: 'Failed to fetch claims', message: err.message });
  }
});

// GET /api/faucet-claims — Faucet claim history
app.get('/api/faucet-claims', (req, res) => {
  try {
    const claims = db.prepare(`
      SELECT fc.*, f.name as faucet_name, f.token_symbol, w.address as wallet_address, w.chain_id as wallet_chain
      FROM faucet_claims fc
      JOIN faucets f ON f.id = fc.faucet_id
      JOIN wallets w ON w.id = fc.wallet_id
      ORDER BY fc.claimed_at DESC
      LIMIT 100
    `).all();
    res.json({ claims });
  } catch (err) {
    res.status(500).json({ error: 'Failed to fetch faucet claims', message: err.message });
  }
});

// ── TPS Leaderboard API ──────────────────────────────────────────────────────

// GET /api/tps/leaderboard — TPS leaderboard sorted by measured current TPS
app.get('/api/tps/leaderboard', (req, res) => {
  try {
    const category = req.query.category || 'chain'; // chain | validator | ecosystem | provider
    const sortBy = req.query.sort || 'tps_current'; // tps_current | tps_peak | latency | finality
    const order = req.query.order === 'asc' ? 'ASC' : 'DESC';
    const ecosystem = req.query.ecosystem || null;
    const limit = Math.min(500, Math.max(1, parseInt(req.query.limit) || 100));
    const offset = Math.max(0, parseInt(req.query.offset) || 0);

    let rows, total, stats;

    if (category === 'chain') {
      // By chain — join chain_metrics + chains + best RPC latency
      rows = db.prepare(`
        SELECT c.chain_id, c.chain_name, c.ecosystem, c.chain_type, c.native_token, c.is_testnet,
               COALESCE(m.tps_current, 0) as tps_current,
               COALESCE(m.tps_peak, 0) as tps_peak,
               COALESCE(m.tps_theoretical, 0) as tps_theoretical,
               COALESCE(m.total_txns_24h, 0) as total_txns_24h,
               COALESCE(m.finality_seconds, 0) as finality_seconds,
               m.block_height,
               m.measured_at,
               MIN(r.latency_ms) as best_latency_ms,
               COUNT(DISTINCT r.id) as endpoint_count,
               COALESCE(SUM(r.rate_limit_rps), 0) as total_rps
        FROM chains c
        LEFT JOIN chain_metrics m ON m.chain_id = c.chain_id
          AND m.id = (SELECT MAX(id) FROM chain_metrics WHERE chain_id = c.chain_id)
        LEFT JOIN rpc_endpoints r ON r.chain_id = c.chain_id AND r.is_healthy = 1
        WHERE (:ecosystem IS NULL OR c.ecosystem = :ecosystem)
        GROUP BY c.chain_id
        HAVING (tps_current > 0 OR best_latency_ms > 0)
        ORDER BY ${sortBy === 'latency' ? 'best_latency_ms ASC' : sortBy === 'finality' ? 'finality_seconds ASC' : `${sortBy} ${order}`}
        LIMIT :limit OFFSET :offset
      `).all({ ecosystem, limit, offset });

      total = db.prepare(`
        SELECT COUNT(DISTINCT c.chain_id) as cnt
        FROM chains c
        LEFT JOIN chain_metrics m ON m.chain_id = c.chain_id
        LEFT JOIN rpc_endpoints r ON r.chain_id = c.chain_id AND r.is_healthy = 1
        WHERE (:ecosystem IS NULL OR c.ecosystem = :ecosystem)
          AND (m.tps_current > 0 OR (r.latency_ms > 0 AND r.is_healthy = 1))
      `).get({ ecosystem });
    } else if (category === 'ecosystem') {
      rows = db.prepare(`
        SELECT c.ecosystem,
               COUNT(DISTINCT c.chain_id) as chain_count,
               AVG(CASE WHEN m.tps_current > 0 THEN m.tps_current END) as avg_tps,
               MAX(m.tps_current) as max_tps,
               MAX(m.tps_peak) as peak_tps,
               AVG(CASE WHEN r.latency_ms > 0 THEN r.latency_ms END) as avg_latency_ms,
               MIN(r.latency_ms) as best_latency_ms,
               SUM(COALESCE(m.total_txns_24h, 0)) as total_txns_24h,
               COUNT(DISTINCT r.id) as total_endpoints
        FROM chains c
        LEFT JOIN chain_metrics m ON m.chain_id = c.chain_id
          AND m.id = (SELECT MAX(id) FROM chain_metrics WHERE chain_id = c.chain_id)
        LEFT JOIN rpc_endpoints r ON r.chain_id = c.chain_id AND r.is_healthy = 1
        GROUP BY c.ecosystem
        ORDER BY avg_tps ${order}
        LIMIT :limit
      `).all({ limit });
      total = { cnt: rows.length };
    } else if (category === 'provider') {
      rows = db.prepare(`
        SELECT r.provider,
               COUNT(*) as endpoint_count,
               COUNT(DISTINCT r.chain_id) as chains_covered,
               AVG(r.latency_ms) as avg_latency_ms,
               MIN(r.latency_ms) as best_latency_ms,
               SUM(r.rate_limit_rps) as total_rps,
               SUM(r.requests_total) as total_requests,
               AVG(CASE WHEN m.tps_current > 0 THEN m.tps_current END) as avg_chain_tps
        FROM rpc_endpoints r
        LEFT JOIN chain_metrics m ON m.chain_id = r.chain_id
          AND m.id = (SELECT MAX(id) FROM chain_metrics WHERE chain_id = r.chain_id)
        WHERE r.is_healthy = 1 AND r.provider IS NOT NULL
        GROUP BY r.provider
        ORDER BY ${sortBy === 'latency' ? 'avg_latency_ms ASC' : 'total_rps ' + order}
        LIMIT :limit
      `).all({ limit });
      total = { cnt: rows.length };
    } else {
      // validator — use whatever validator data exists
      rows = [];
      total = { cnt: 0 };
    }

    // Aggregate stats
    stats = db.prepare(`
      SELECT
        COUNT(DISTINCT c.chain_id) as total_chains_measured,
        COALESCE(AVG(m.tps_current), 0) as avg_tps_all,
        COALESCE(MAX(m.tps_current), 0) as max_tps_all,
        COALESCE(MAX(m.tps_peak), 0) as peak_tps_all,
        COALESCE(SUM(m.total_txns_24h), 0) as total_txns_24h_all
      FROM chains c
      LEFT JOIN chain_metrics m ON m.chain_id = c.chain_id
        AND m.id = (SELECT MAX(id) FROM chain_metrics WHERE chain_id = c.chain_id)
      WHERE m.tps_current > 0
    `).get();

    res.json({
      leaderboard: rows,
      total: total.cnt,
      stats,
      category,
      sort: sortBy,
      order,
    });
  } catch (err) {
    res.status(500).json({ error: 'Failed to fetch TPS leaderboard', message: err.message });
  }
});

// POST /api/tps/benchmark — Record TPS benchmark results (used by benchmark script)
app.post('/api/tps/benchmark', (req, res) => {
  try {
    const results = req.body.results;
    if (!Array.isArray(results)) {
      return res.status(400).json({ error: 'results must be an array' });
    }

    const upsert = dbW.prepare(`
      INSERT INTO chain_metrics (chain_id, tps_current, tps_peak, tps_theoretical, total_txns_24h, finality_seconds, block_height, measured_at)
      VALUES (:chain_id, :tps_current, :tps_peak, :tps_theoretical, :total_txns_24h, :finality_seconds, :block_height, datetime('now'))
    `);

    const insertMany = dbW.transaction((items) => {
      let inserted = 0;
      for (const item of items) {
        try {
          upsert.run({
            chain_id: item.chain_id,
            tps_current: item.tps_current || 0,
            tps_peak: item.tps_peak || 0,
            tps_theoretical: item.tps_theoretical || 0,
            total_txns_24h: item.total_txns_24h || 0,
            finality_seconds: item.finality_seconds || null,
            block_height: item.block_height || null,
          });
          inserted++;
        } catch (e) {
          // skip invalid
        }
      }
      return inserted;
    });

    const inserted = insertMany(results);
    res.json({ success: true, inserted, total: results.length });
  } catch (err) {
    res.status(500).json({ error: 'Failed to record benchmark', message: err.message });
  }
});

// GET /api/tps/benchmark/status — Check benchmark progress
app.get('/api/tps/benchmark/status', (req, res) => {
  try {
    const measured = db.prepare('SELECT COUNT(DISTINCT chain_id) as cnt FROM chain_metrics').get();
    const total = db.prepare('SELECT COUNT(*) as cnt FROM chains').get();
    const latest = db.prepare('SELECT MAX(measured_at) as ts FROM chain_metrics').get();
    const top5 = db.prepare(`
      SELECT c.chain_name, m.tps_current, m.tps_peak
      FROM chain_metrics m JOIN chains c ON c.chain_id = m.chain_id
      WHERE m.id IN (SELECT MAX(id) FROM chain_metrics GROUP BY chain_id)
      ORDER BY m.tps_current DESC LIMIT 5
    `).all();
    res.json({
      measured: measured.cnt,
      total: total.cnt,
      progress_pct: total.cnt > 0 ? Math.round(measured.cnt / total.cnt * 100) : 0,
      last_updated: latest.ts,
      top5,
    });
  } catch (err) {
    res.status(500).json({ error: 'Failed to fetch benchmark status', message: err.message });
  }
});

// ── Error handler ────────────────────────────────────────────────────────────

app.use((err, _req, res, _next) => {
  console.error('API Error:', err);
  res.status(500).json({ error: 'Internal server error', message: err.message });
});

// ── Start ────────────────────────────────────────────────────────────────────

app.listen(PORT, () => {
  console.log(`\n🔗 X3 Chain DB API running on http://localhost:${PORT}`);
  console.log(`   Health:    http://localhost:${PORT}/health`);
  console.log(`   Chains:    http://localhost:${PORT}/api/chains`);
  console.log(`   Search:    http://localhost:${PORT}/api/chains/search?q=ethereum`);
  console.log(`   Stats:     http://localhost:${PORT}/api/chains/stats/overview`);
  console.log(`   RPC Stats: http://localhost:${PORT}/api/rpc/stats\n`);
});
