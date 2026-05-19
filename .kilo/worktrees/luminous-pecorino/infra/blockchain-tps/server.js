#!/usr/bin/env node
const express = require('express');
const bodyParser = require('body-parser');
const { spawn } = require('child_process');
const { v4: uuidv4 } = require('uuid');
const path = require('path');
const fs = require('fs');
const { encryptJson, decryptJson } = require('./utils/crypto');
const fetch = globalThis.fetch || require('node-fetch');
const { notifyLead } = require('./utils/mailer');



const http = require('http');
const { Server } = require('socket.io');
const { connectRedis } = require('./utils/redis');

const app = express();
const PORT = process.env.PORT || 3010;
app.use(require('cors')());
app.use(bodyParser.json());

// Serve basic UI
app.use('/', express.static(path.join(__dirname, 'public')));

// In-memory job store
const jobs = new Map();

// Helper: lightweight RPC probe to estimate baseline TPS
async function probeRpc(rpcUrl, durationSeconds = 5, concurrency = 30) {
  if (!rpcUrl) return 0;
  const end = Date.now() + durationSeconds * 1000;
  let total = 0;

  async function worker() {
    while (Date.now() < end) {
      try {
        const resp = await fetch(rpcUrl, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ jsonrpc: '2.0', method: 'eth_blockNumber', params: [], id: 1 }) });
        if (resp.ok) total++;
      } catch (e) {
        // ignore
      }
    }
  }

  const workers = [];
  for (let i = 0; i < concurrency; i++) workers.push(worker());
  await Promise.all(workers);
  return total / durationSeconds; // TPS
}

// ── Company demo cooldown persistence & session logging (file-backed + Redis) ───────────────────
const COOLDOWN_SECONDS = Number(process.env.COMPANY_DEMO_COOLDOWN_SECONDS || 3600); // default: 1 hour
const SESSIONS_FILE = path.join(__dirname, 'data', 'sessions.json.enc');
const DEMOS_FILE = path.join(__dirname, 'data', 'company_demos.json.enc');

let redisClient = null;

function loadJsonEnc(file, defaultVal) {
  if (fs.existsSync(file)) {
    try { return decryptJson(fs.readFileSync(file, 'utf8')); } catch (e) { return defaultVal; }
  }
  return defaultVal;
}

function saveJsonEnc(file, obj) {
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, encryptJson(obj), 'utf8');
}

// Attempt to read last demo timestamp. If Redis is available, prefer it.
async function getLastDemoTs(companyId) {
  if (!companyId) return null;
  if (redisClient) {
    try {
      const v = await redisClient.get(`company_demo_last:${companyId}`);
      return v ? Number(v) : null;
    } catch (e) {
      console.warn('redis getLastDemoTs fail', e.message);
    }
  }
  const m = loadJsonEnc(DEMOS_FILE, {});
  return m[companyId] || null;
}

async function setLastDemoTs(companyId, ts) {
  if (!companyId) return;
  if (redisClient) {
    try {
      await redisClient.set(`company_demo_last:${companyId}`, String(ts), { EX: COOLDOWN_SECONDS });
    } catch (e) {
      console.warn('redis setLastDemoTs fail', e.message);
    }
  }
  const m = loadJsonEnc(DEMOS_FILE, {});
  m[companyId] = ts;
  saveJsonEnc(DEMOS_FILE, m);
}

async function addSessionRecord(rec) {
  // push to Redis list for quick access
  if (redisClient) {
    try {
      await redisClient.lPush('sessions', JSON.stringify(rec));
      // keep a bounded history
      await redisClient.lTrim('sessions', 0, 9999);
    } catch (e) {
      console.warn('redis addSessionRecord fail', e.message);
    }
  }
  const arr = loadJsonEnc(SESSIONS_FILE, []);
  arr.push(rec);
  saveJsonEnc(SESSIONS_FILE, arr);
}

async function updateSessionRecord(jobId, patch) {
  if (redisClient) {
    try {
      const key = `session:${jobId}`;
      const raw = await redisClient.get(key);
      if (raw) {
        const obj = JSON.parse(raw);
        const merged = { ...obj, ...patch };
        await redisClient.set(key, JSON.stringify(merged));
      }
    } catch (e) {
      console.warn('redis updateSessionRecord fail', e.message);
    }
  }

  const arr = loadJsonEnc(SESSIONS_FILE, []);
  for (let i = 0; i < arr.length; i++) {
    if (arr[i].jobId === jobId) {
      arr[i] = { ...arr[i], ...patch };
      break;
    }
  }
  saveJsonEnc(SESSIONS_FILE, arr);
}

// Public list of who is on now (from Redis if available, else jobs Map)
app.get('/api/whoisonline', async (req, res) => {
  try {
    if (redisClient) {
      const keys = await redisClient.keys('online:*');
      const out = [];
      for (const k of keys) {
        try { const v = await redisClient.get(k); if (v) out.push(JSON.parse(v)); } catch (e) { /* ignore */ }
      }
      return res.json(out);
    }
  } catch (e) {
    console.warn('whoisonline redis fail', e.message);
  }

  // fallback to jobs Map
  const out = [];
  for (const [id, j] of jobs.entries()) {
    if (j.status === 'running') {
      out.push({ jobId: id, company_id: j.company_id || null, rpc: j.rpc || null, evm_tps: j.evm_tps || null, started_at: j.started_at || null });
    }
  }
  res.json(out);
});

// Admin sessions viewer
app.get('/api/sessions', async (req, res) => {
  const adminToken = process.env.SALES_ADMIN_TOKEN || '';
  const token = req.query.token || req.headers['x-admin-token'];
  if (!adminToken || token !== adminToken) return res.status(403).json({ error: 'forbidden' });
  try {
    if (redisClient) {
      const items = await redisClient.lRange('sessions', 0, 9999);
      const arr = items.map(i => { try { return JSON.parse(i); } catch (e) { return null; } }).filter(Boolean);
      return res.json(arr.reverse());
    }
  } catch (e) {
    console.warn('sessions redis fail', e.message);
  }

  const arr = loadJsonEnc(SESSIONS_FILE, []);
  res.json(arr.reverse());
});

// Start a benchmark job
app.post('/api/run', async (req, res) => {
  const { rpc = '', evm_tps = 1000, duration = 10, gpu = false, company_id = null } = req.body || {};

  const id = uuidv4();
  const outFile = path.join(__dirname, `reports/${id}.json`);
  fs.mkdirSync(path.dirname(outFile), { recursive: true });

  jobs.set(id, { status: 'running', rpc, evm_tps, duration, gpu });

  // 1) Probe client's RPC to estimate baseline
  let baseline = 0;
  try {
    baseline = await probeRpc(rpc, Math.min(6, Math.max(3, duration)), 40);
  } catch (e) {
    baseline = 0;
  }

  // 2) Run our benchmark (Python)
  const pyCmd = [
    '-u',
    '-c',
    `import json; from cross_chain_gpu_validator.benchmark import run_benchmark; r=run_benchmark(svm_tps=0, evm_tps=${Number(evm_tps)}, duration_seconds=${Number(duration)}); print(json.dumps(r));`
  ];

  // Enforce per-company cooldown (if provided)
  const now = Date.now();
  if (company_id) {
    const last = await getLastDemoTs(company_id);
    if (last && now - last < COOLDOWN_SECONDS * 1000) {
      return res.status(429).json({ error: 'cooldown_active', retry_after: Math.ceil((COOLDOWN_SECONDS * 1000 - (now - last)) / 1000) });
    }
    // set last demo timestamp immediately to prevent race
    await setLastDemoTs(company_id, now);
  }

  const proc = spawn('python3', pyCmd, { env: process.env });
  jobs.set(id, { proc, status: 'running', rpc, evm_tps, duration, gpu, company_id, started_at: now });

  // Log session
  const sessionRec = { jobId: id, company_id: company_id || null, rpc, evm_tps, duration, gpu, start_ts: now, status: 'running' };
  await addSessionRecord(sessionRec);

  // If Redis present, publish an "online" record and session key
  try {
    if (redisClient) {
      const onlineKey = `online:${id}`;
      await redisClient.set(onlineKey, JSON.stringify({ jobId: id, company_id: company_id || null, rpc, evm_tps, started_at: now }), { EX: Math.max(duration + 60, 300) });
      await redisClient.set(`session:${id}`, JSON.stringify(sessionRec));
      if (io) io.emit('whoisonline_update');
      if (io) io.emit('job_update', { id, status: 'running', rpc, evm_tps });
    }
  } catch (e) { console.warn('redis online set fail', e.message); }
  proc.stdout.on('data', (data) => {
    try {
      const txt = data.toString();
      fs.writeFileSync(outFile, txt, 'utf8');
    } catch (err) {
      console.error('write fail', err);
    }
  });

  proc.on('exit', (code) => {
    const job = jobs.get(id) || {};
    job.status = code === 0 ? 'finished' : 'failed';
    jobs.set(id, job);

    // Persist result into encrypted results store
    let report = { status: job.status, baseline_tps: baseline };
    try {
      const content = fs.existsSync(outFile) ? JSON.parse(fs.readFileSync(outFile, 'utf8')) : {};
      report = { ...report, ...content, timestamp: Date.now() };
    } catch (e) {
      // ignore
    }

    const resultRecord = { id, company_id, rpc, evm_tps, duration, gpu, report };
    const dataFile = path.join(__dirname, 'data', 'results.json.enc');
    fs.mkdirSync(path.dirname(dataFile), { recursive: true });

    let arr = [];
    if (fs.existsSync(dataFile)) {
      try { arr = decryptJson(fs.readFileSync(dataFile, 'utf8')); } catch (e) { arr = []; }
    }
    arr.push(resultRecord);
    fs.writeFileSync(dataFile, encryptJson(arr), 'utf8');

    // update session record
    updateSessionRecord(id, { end_ts: Date.now(), status: job.status, report });

    // Push result into Redis and update leaderboard
    try {
      if (redisClient) {
        await redisClient.lPush('results', JSON.stringify(resultRecord));
        await redisClient.lTrim('results', 0, 9999);
        // If evm_tps present, add to leaderboard (sorted by evm_tps)
        if (report && report.evm_tps) {
          await redisClient.zAdd('leaderboard', [{ score: Number(report.evm_tps), value: id }]);
        }
        // remove online marker
        await redisClient.del(`online:${id}`);
        if (io) {
          io.emit('job_update', { id, status: job.status, report });
          io.emit('whoisonline_update');
          io.emit('leaderboard_update');
        }
      }
    } catch (e) { console.warn('redis result push fail', e.message); }
  });

  res.json({ jobId: id });
});

// Stream job status and report
app.get('/api/status/:id', (req, res) => {
  const id = req.params.id;
  const job = jobs.get(id);
  if (!job) return res.status(404).json({ error: 'job not found' });
  const reportFile = path.join(__dirname, `reports/${id}.json`);
  if (fs.existsSync(reportFile)) {
    return res.json({ status: job.status, report: JSON.parse(fs.readFileSync(reportFile, 'utf8')) });
  }
  res.json({ status: job.status });
});

// Simple stop
app.post('/api/stop/:id', (req, res) => {
  const id = req.params.id;
  const job = jobs.get(id);
  if (!job) return res.status(404).json({ error: 'job not found' });
  try {
    job.proc.kill();
    job.status = 'stopped';
    jobs.set(id, job);
    res.json({ stopped: true });
  } catch (err) {
    res.status(500).json({ error: String(err) });
  }
});

// Persist company profile
app.post('/api/company', (req, res) => {
  const dataFile = path.join(__dirname, 'data', 'companies.json.enc');
  fs.mkdirSync(path.dirname(dataFile), { recursive: true });
  const payload = req.body || {};
  // expected: { name, blockchain, role, rpc, contact }
  const id = uuidv4();
  payload.id = id;
  payload.created_at = Date.now();

  let arr = [];
  if (fs.existsSync(dataFile)) {
    try { arr = decryptJson(fs.readFileSync(dataFile, 'utf8')); } catch (e) { arr = []; }
  }
  arr.push(payload);
  fs.writeFileSync(dataFile, encryptJson(arr), 'utf8');
  res.json({ id });
});

// Leaderboard: return top results by % improvement
app.get('/api/leaderboard', (req, res) => {
  const dataFile = path.join(__dirname, 'data', 'results.json.enc');
  let arr = [];
  if (fs.existsSync(dataFile)) {
    try { arr = decryptJson(fs.readFileSync(dataFile, 'utf8')); } catch (e) { arr = []; }
  }

  const records = arr.map(r => {
    const baseline = (r.report && r.report.baseline_tps) || 1;
    const our = (r.report && r.report.combined_tps) || 0;
    const improvement = baseline > 0 ? (our / baseline - 1) * 100 : null;
    return { id: r.id, company_id: r.company_id, rpc: r.rpc, baseline, our, improvement, timestamp: r.report && r.report.timestamp };
  }).filter(x => x.improvement !== null).sort((a,b) => (b.improvement - a.improvement));

  res.json(records.slice(0, 50));
});

// Rate limiter for presale (simple in-memory)
const presaleLimits = new Map(); // ip -> { count, firstTs }
const MAX_PRESALE_PER_HOUR = Number(process.env.MAX_PRESALE_PER_HOUR || 5);

function checkPresaleRate(ip) {
  const now = Date.now();
  const hour = 3600 * 1000;
  let st = presaleLimits.get(ip) || { count: 0, firstTs: now };
  if (now - st.firstTs > hour) {
    st = { count: 0, firstTs: now };
  }
  st.count += 1;
  presaleLimits.set(ip, st);
  return st.count <= MAX_PRESALE_PER_HOUR;
}

// Verify Turnstile token (Cloudflare) if configured
async function verifyTurnstile(token, remoteIp) {
  const secret = process.env.TURNSTILE_SECRET;
  if (!secret) return true; // no captcha configured
  try {
    const r = await fetch('https://challenges.cloudflare.com/turnstile/v0/siteverify', { method: 'POST', headers: { 'Content-Type': 'application/x-www-form-urlencoded' }, body: new URLSearchParams({ secret, response: token, remoteip: remoteIp }) });
    const j = await r.json();
    return !!j.success;
  } catch (e) {
    console.error('turnstile verify failed', e);
    return false;
  }
}

// Config endpoint for client-side features (Turnstile sitekey, calendar link)
app.get('/api/config', (req, res) => {
  res.json({
    turnstile_sitekey: process.env.TURNSTILE_SITE_KEY || null,
    calendar_link: process.env.SALES_CALENDAR_LINK || null,
  });
});

// Presale lead endpoint — capture a lead, store encrypted, notify sales
app.post('/api/lead', async (req, res) => {
  const payload = req.body || {};
  const ip = req.headers['x-forwarded-for'] || req.connection.remoteAddress || 'unknown';

  // Honeypot check
  if (payload.hp && payload.hp.trim() !== '') return res.status(400).json({ error: 'spam detected' });

  // Rate limit check
  if (!checkPresaleRate(ip)) return res.status(429).json({ error: 'rate limit exceeded' });

  // Turnstile verification
  const token = payload['cf-turnstile-response'];
  const ok = await verifyTurnstile(token, ip);
  if (!ok) return res.status(400).json({ error: 'captcha_failed' });

  const dataFile = path.join(__dirname, 'data', 'leads.json.enc');
  fs.mkdirSync(path.dirname(dataFile), { recursive: true });

  let arr = [];
  if (fs.existsSync(dataFile)) {
    try { arr = decryptJson(fs.readFileSync(dataFile, 'utf8')); } catch (e) { arr = []; }
  }

  const id = uuidv4();
  const rec = { id, ...payload, created_at: Date.now(), ip };
  arr.push(rec);
  fs.writeFileSync(dataFile, encryptJson(arr), 'utf8');

  // notify sales (webhook/email/log)
  try {
    await notifyLead(rec);
    await sendConfirmationEmail(rec);
  } catch (e) {
    console.error('notify failed', e);
  }

  // If requested, optionally start a demo benchmark job (with per-company cooldown)
  if (payload.request_demo && payload.rpc) {
    try {
      // Let the run endpoint enforce cooldown; fire and log the response
      const r = await fetch('http://127.0.0.1:3010/api/run', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ rpc: payload.rpc, evm_tps: payload.evm_tps || 1000, duration: payload.duration || 10, gpu: payload.gpu || false, company_id: id }) });
      if (r.status === 429) {
        console.warn('demo request rejected due to cooldown for', id);
      }
    } catch (e) { console.error('demo start failed', e); }
  }

  res.json({ id, status: 'ok' });
});

// Admin leads viewer (requires SALES_ADMIN_TOKEN query or header)
app.get('/api/leads', (req, res) => {
  const adminToken = process.env.SALES_ADMIN_TOKEN || '';
  const token = req.query.token || req.headers['x-admin-token'];
  if (!adminToken || token !== adminToken) return res.status(403).json({ error: 'forbidden' });

  const dataFile = path.join(__dirname, 'data', 'leads.json.enc');
  let arr = [];
  if (fs.existsSync(dataFile)) {
    try { arr = decryptJson(fs.readFileSync(dataFile, 'utf8')); } catch (e) { arr = []; }
  }
  res.json(arr);
});

// List jobs
app.get('/api/jobs', (req, res) => {
  const out = [];
  for (const [id, j] of jobs.entries()) {
    out.push({ id, status: j.status, evm_tps: j.evm_tps, duration: j.duration });
  }
  res.json(out);
});

// System bulletin helpers
const SYSTEM_FILE = path.join(__dirname, 'data', 'system_bulletin.json.enc');

async function writeSystemBulletin(bulletin) {
  fs.mkdirSync(path.dirname(SYSTEM_FILE), { recursive: true });
  fs.writeFileSync(SYSTEM_FILE, encryptJson(bulletin), 'utf8');
  // Try to post to external social webhook if configured
  const hook = process.env.SOCIAL_POST_WEBHOOK;
  if (hook) {
    try {
      await fetch(hook, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(bulletin) });
      console.log('Posted system bulletin to social webhook');
    } catch (e) {
      console.error('Failed to post system bulletin to webhook', e);
    }
  }
}

app.get('/api/system_bulletin', (req, res) => {
  if (!fs.existsSync(SYSTEM_FILE)) return res.json(null);
  try { const b = decryptJson(fs.readFileSync(SYSTEM_FILE, 'utf8')); return res.json(b); } catch (e) { return res.json(null); }
});

// Create HTTP server and Socket.IO
const server = require('http').createServer(app);
const io = new Server(server, { cors: { origin: '*' } });

io.on('connection', (socket) => {
  // Basic auth for admin actions
  const token = socket.handshake.auth?.token || socket.handshake.query?.token;
  const adminToken = process.env.SALES_ADMIN_TOKEN || '';
  if (token && adminToken && token === adminToken) {
    socket.data.isAdmin = true;
  }

  socket.on('admin:refresh_leaderboard', async () => {
    if (!socket.data.isAdmin) return; // secure
    io.emit('leaderboard_update');
  });

  socket.on('disconnect', () => {
    // nothing yet
  });
});

server.listen(PORT, '0.0.0.0', async () => {
  console.log(`✅ blockchain-tps-runner listening on :${PORT}`);
  // Try to connect Redis (optional)
  try {
    redisClient = await connectRedis();
  } catch (e) {
    console.warn('Redis not available, continuing with file-backed persistence');
  }

  // On startup, create or refresh a pinned system bulletin
  const sys = {
    title: 'X3 TPS — Presale & Demo',
    body: 'Presale & Demo: https://blockchain-tps-go.x3star.net/presale.html\nCompany signup: https://blockchain-tps-go.x3star.net/company.html\nLeaderboard: https://blockchain-tps-go.x3star.net/leaderboard.html\nWho\'s online: https://blockchain-tps-go.x3star.net/whoisonline.html',
    pinned: true,
    timestamp: Date.now(),
  };
  try { await writeSystemBulletin(sys); } catch (e) { console.error('system bulletin write fail', e); }
});
