const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("fs/promises");
const os = require("os");
const path = require("path");

const { createServer } = require("../server");
const { createSiteServices } = require("../server/site-services");

async function withTempStore(fn) {
  const tempDir = await fs.mkdtemp(path.join(os.tmpdir(), "x3fronend-"));
  const rootDir = path.join(__dirname, "..");
  const storePath = path.join(tempDir, "business-store.json");
  const seedPath = path.join(rootDir, "data", "business-store.json");
  await fs.copyFile(seedPath, storePath);
  return fn({ rootDir, storePath, seedPath });
}

test("site services expose contract envelopes", async () => {
  await withTempStore(async ({ rootDir, storePath, seedPath }) => {
    const services = createSiteServices({ rootDir, storePath, seedPath });
    const health = await services.getHealth();
    assert.equal(typeof health.status, "string");
    assert.equal(typeof health.source, "string");
    assert.ok(health.lastUpdated);

    const presale = await services.getPresale();
    assert.ok(presale.data.tiers.length > 0);
    assert.equal(presale.status, "live");

    const whales = await services.getWhales();
    assert.ok(whales.data.whales.length > 0);
    assert.equal(whales.status, "live");

    const tokenomics = await services.getTokenomics();
    assert.ok(tokenomics.data.allocations.length > 0);
    assert.equal(tokenomics.status, "live");

    const chainbench = await services.getBenchmark("chainbench");
    assert.ok(chainbench.data);
    assert.equal(chainbench.status, "stale");
  });
});

test("reservation writes update the authoritative store", async () => {
  await withTempStore(async ({ rootDir, storePath, seedPath }) => {
    const services = createSiteServices({ rootDir, storePath, seedPath });
    const created = await services.createReservation({
      name: "Test Operator",
      location: "Denver, US",
      countryCode: "US",
      wallet: "0xtest",
      tier: "lite",
      amountUsd: 5000,
      quantity: 1,
    });
    assert.equal(created.name, "Test Operator");

    const reservations = await services.getReservations();
    assert.equal(reservations.data.reservations[0].name, "Test Operator");
    assert.ok(reservations.data.slotTracker);
  });
});

test("http server serves api contracts", async () => {
  const server = createServer();
  await new Promise((resolve) => server.listen(0, resolve));
  const port = server.address().port;

  try {
    const response = await fetch(`http://127.0.0.1:${port}/api/site/health`);
    assert.equal(response.status, 200);
    const body = await response.json();
    assert.ok(body.status);
    assert.ok(body.lastUpdated);
    assert.ok(body.data.businessStore);

    const whalesResponse = await fetch(`http://127.0.0.1:${port}/api/site/whales`);
    assert.equal(whalesResponse.status, 200);
    const whalesBody = await whalesResponse.json();
    assert.ok(Array.isArray(whalesBody.data.whales));

    const benchmarkResponse = await fetch(`http://127.0.0.1:${port}/api/site/benchmarks/chainbench`);
    assert.equal(benchmarkResponse.status, 200);
    const benchmarkBody = await benchmarkResponse.json();
    assert.ok(benchmarkBody.data.summary.ok >= 0);

    const htmlResponse = await fetch(`http://127.0.0.1:${port}/x3star-landing.html`);
    assert.equal(htmlResponse.status, 200);
    const htmlBody = await htmlResponse.text();
    assert.match(htmlBody, /x3-site-nav\.css/);
    assert.match(htmlBody, /x3-site-nav\.js/);
  } finally {
    await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  }
});
