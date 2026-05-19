const http = require("http");
const fs = require("fs/promises");
const path = require("path");
const { URL } = require("url");
const { createSiteServices } = require("./server/site-services");

const rootDir = __dirname;
const services = createSiteServices({ rootDir });

const CONTENT_TYPES = {
  ".html": "text/html; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".ico": "image/x-icon",
};

async function readRequestBody(request) {
  const chunks = [];
  for await (const chunk of request) {
    chunks.push(chunk);
  }
  const body = Buffer.concat(chunks).toString("utf8");
  return body ? JSON.parse(body) : {};
}

function json(response, statusCode, payload) {
  response.writeHead(statusCode, {
    "content-type": "application/json; charset=utf-8",
    "cache-control": "no-store",
  });
  response.end(`${JSON.stringify(payload)}\n`);
}

function injectGlobalChrome(html) {
  const headInjection =
    '<link rel="stylesheet" href="/css/x3-site-nav.css">\n<script defer src="/js/x3-site-nav.js"></script>';
  if (!html.includes("/css/x3-site-nav.css")) {
    html = html.includes("</head>") ? html.replace("</head>", `${headInjection}\n</head>`) : `${headInjection}\n${html}`;
  }
  return html;
}

function getSiteType(request, requestUrl) {
  const forwardedHost = request.headers['x-forwarded-host'];
  const hostHeader =
    forwardedHost || request.headers.host || request.headers.Host || requestUrl.host || requestUrl.hostname || '';
  const host = hostHeader.split(':')[0].toLowerCase();
  if (host.endsWith('.net')) {
    return 'legacy';
  }
  if (host.endsWith('.org') || host === 'localhost' || host === '127.0.0.1') {
    return 'react';
  }
  return 'react';
}

async function serveStatic(request, requestUrl, response) {
  const siteType = getSiteType(request, requestUrl);

  const distDir = path.join(rootDir, 'dist');
  const hasDist = await fs.stat(distDir).catch(() => false);

  // Legacy page paths should be served from the original root HTML, even when dist exists.
  if (siteType === 'react' && requestUrl.pathname === '/x3star-landing.html') {
    response.writeHead(301, { Location: '/' });
    response.end();
    return;
  }

  if (siteType === 'react') {
    const filePath = requestUrl.pathname === '/' ? '/index.html' : requestUrl.pathname;
    if (hasDist) {
      const resolvedDist = path.normalize(path.join(distDir, filePath));
      if (resolvedDist.startsWith(distDir)) {
        try {
          const file = await fs.readFile(resolvedDist);
          const ext = path.extname(resolvedDist).toLowerCase();
          response.writeHead(200, {
            'content-type': CONTENT_TYPES[ext] || 'application/octet-stream',
          });
          response.end(file);
          return;
        } catch (e) {
          if (e.code === 'ENOENT') {
            const indexFile = await fs.readFile(path.join(distDir, 'index.html'));
            response.writeHead(200, {
              'content-type': 'text/html; charset=utf-8',
            });
            response.end(indexFile);
            return;
          }
        }
      }
    } else if (requestUrl.pathname === '/') {
      const indexFile = await fs.readFile(path.join(rootDir, 'index.html'));
      response.writeHead(200, {
        'content-type': 'text/html; charset=utf-8',
      });
      response.end(indexFile);
      return;
    }
  }

  // legacy behaviour (serve original html/js/css)
  const pathname =
    siteType === 'react'
      ? requestUrl.pathname === '/'
        ? '/index.html'
        : requestUrl.pathname
      : requestUrl.pathname === '/'
      ? '/x3star-landing.html'
      : requestUrl.pathname;
  let resolved = path.normalize(path.join(rootDir, pathname));
  if (!resolved.startsWith(rootDir)) {
    json(response, 403, { error: 'Forbidden' });
    return;
  }
  let file;
  try {
    file = await fs.readFile(resolved);
  } catch (error) {
    if (error.code === 'ENOENT' && !path.extname(resolved)) {
      const htmlCandidate = path.normalize(path.join(rootDir, pathname + '.html'));
      const indexCandidate = path.normalize(path.join(rootDir, pathname, 'index.html'));
      for (const candidate of [htmlCandidate, indexCandidate]) {
        if (!candidate.startsWith(rootDir)) continue;
        try {
          file = await fs.readFile(candidate);
          resolved = candidate;
          break;
        } catch (innerError) {
          if (innerError.code !== 'ENOENT') throw innerError;
        }
      }
    }
    if (!file) {
      json(response, 404, { error: 'Not found' });
      return;
    }
  }
  const extension = path.extname(resolved).toLowerCase();
  let body = file;
  if (extension === '.html') {
    body = Buffer.from(injectGlobalChrome(file.toString('utf8')), 'utf8');
  }
  response.writeHead(200, {
    'content-type': CONTENT_TYPES[extension] || 'application/octet-stream',
  });
  response.end(body);
}


function sse(response, handler) {
  response.writeHead(200, {
    "content-type": "text/event-stream",
    "cache-control": "no-store",
    connection: "keep-alive",
  });
  response.write(": connected\n\n");
  const interval = setInterval(handler, 10000);
  response.on("close", () => clearInterval(interval));
}

async function routeApi(request, response, requestUrl) {
  const { pathname } = requestUrl;
  try {
    if (request.method === "GET" && pathname === "/api/site/health") {
      json(response, 200, await services.getHealth());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/dashboard") {
      json(response, 200, await services.getDashboard());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/network") {
      json(response, 200, await services.getNetwork());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/node-health") {
      json(response, 200, await services.getNodeHealth());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/governance") {
      json(response, 200, await services.getGovernance());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/staking") {
      json(response, 200, await services.getStaking());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/ledger") {
      json(response, 200, await services.getLedger());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/proofs") {
      json(response, 200, await services.getProofs());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/presale") {
      json(response, 200, await services.getPresale());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/grants") {
      json(response, 200, await services.getGrants());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/bounties") {
      json(response, 200, await services.getBounties());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/deals") {
      json(response, 200, await services.getDeals());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/leaderboard") {
      json(response, 200, await services.getLeaderboard());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/reservations") {
      json(response, 200, await services.getReservations());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/whales") {
      json(response, 200, await services.getWhales());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/tokenomics") {
      json(response, 200, await services.getTokenomics());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/funding/programs") {
      json(response, 200, await services.getFundingPrograms());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/funding/applications") {
      json(response, 200, await services.getFundingApplications());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/funding/scoreboard") {
      json(response, 200, await services.getFundingScoreboard());
      return true;
    }
    if (request.method === "GET" && pathname === "/api/site/funding/intakes") {
      json(response, 200, await services.listFundingIntakes());
      return true;
    }
    if (request.method === "POST" && pathname === "/api/site/funding/applications") {
      const payload = await readRequestBody(request);
      json(response, 201, {
        data: await services.createFundingApplication(payload),
        status: "live",
        source: "business-store",
        lastUpdated: new Date().toISOString(),
      });
      return true;
    }
    if (request.method === "PATCH" && pathname.startsWith("/api/site/funding/applications/")) {
      const id = pathname.split("/").pop();
      const payload = await readRequestBody(request);
      json(response, 200, {
        data: await services.updateFundingApplication(id, payload),
        status: "live",
        source: "business-store",
        lastUpdated: new Date().toISOString(),
      });
      return true;
    }
    if (request.method === "POST" && pathname === "/api/site/funding/intakes") {
      const payload = await readRequestBody(request);
      json(response, 201, {
        data: await services.createFundingIntake(payload),
        status: "live",
        source: "business-store",
        lastUpdated: new Date().toISOString(),
      });
      return true;
    }
    if (request.method === "PATCH" && pathname.startsWith("/api/site/funding/intakes/")) {
      const id = pathname.split("/").pop();
      const payload = await readRequestBody(request);
      json(response, 200, {
        data: await services.updateFundingIntake(id, payload),
        status: "live",
        source: "business-store",
        lastUpdated: new Date().toISOString(),
      });
      return true;
    }
    if (request.method === "POST" && pathname === "/api/site/reservations") {
      const payload = await readRequestBody(request);
      json(response, 201, {
        data: await services.createReservation(payload),
        status: "live",
        source: "business-store",
        lastUpdated: new Date().toISOString(),
      });
      return true;
    }
    if (pathname.startsWith("/api/site/forms/")) {
      const formType = pathname.split("/").pop();
      if (request.method === "GET") {
        json(response, 200, await services.listFormRecords(formType));
        return true;
      }
      if (request.method === "POST") {
        const payload = await readRequestBody(request);
        json(response, 201, {
          data: await services.createFormRecord(formType, payload),
          status: "live",
          source: "business-store",
          lastUpdated: new Date().toISOString(),
        });
        return true;
      }
    }
    if (pathname.startsWith("/api/site/benchmarks/") && request.method === "GET") {
      const name = pathname.split("/").pop();
      json(response, 200, await services.getBenchmark(name));
      return true;
    }
    if (pathname === "/api/site/stream" && request.method === "GET") {
      const topic = requestUrl.searchParams.get("topic") || "health";
      sse(response, async () => {
        const payloadMap = {
          health: services.getHealth,
          reservations: services.getReservations,
          presale: services.getPresale,
          network: services.getNetwork,
          whales: services.getWhales,
          tokenomics: services.getTokenomics,
        };
        const producer = payloadMap[topic] || services.getHealth;
        const payload = await producer();
        response.write(`event: update\n`);
        response.write(`data: ${JSON.stringify({ topic, payload })}\n\n`);
      });
      return true;
    }
  } catch (error) {
    json(response, 500, { error: error.message });
    return true;
  }
  return false;
}

function createServer() {
  return http.createServer(async (request, response) => {
    const requestUrl = new URL(request.url, "http://localhost");
    if (requestUrl.pathname.startsWith("/api/site/")) {
      const handled = await routeApi(request, response, requestUrl);
      if (handled) return;
    }
    await serveStatic(request, requestUrl, response);
  });
}

if (require.main === module) {
  const port = Number(process.env.PORT || 4173);
  const server = createServer();
  server.listen(port, () => {
    process.stdout.write(`x3fronend server listening on http://127.0.0.1:${port}\n`);
  });
}

module.exports = {
  createServer,
};
