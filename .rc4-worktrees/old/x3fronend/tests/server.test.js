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

test("funding application create and list works", async () => {
  await withTempStore(async ({ rootDir, storePath, seedPath }) => {
    const services = createSiteServices({ rootDir, storePath, seedPath });
    const application = await services.createFundingApplication({
      projectName: "Quantum Router Upgrade",
      organization: "Nova Research",
      contactEmail: "hello@x3atomicstar.org",
      fundingLane: "Post-Quantum Security",
      requestedUsd: 250000,
      summary: "Proof-of-concept for post-quantum validator signatures.",
    });
    assert.equal(application.projectName, "Quantum Router Upgrade");
    assert.equal(application.status, "submitted");
    assert.equal(application.requestedUsd, 250000);

    const applications = await services.getFundingApplications();
    assert.equal(applications.data.applications[0].id, application.id);
    assert.equal(applications.data.applications[0].status, "submitted");
  });
});

test("funding intake create and list works", async () => {
  await withTempStore(async ({ rootDir, storePath, seedPath }) => {
    const services = createSiteServices({ rootDir, storePath, seedPath });
    const intake = await services.createFundingIntake({
      company: "Apex Capital",
      contactEmail: "ops@apex.capital",
      fundingLane: "AI Infrastructure",
      summary: "Cloud infrastructure cost and node hosting proposal.",
    });
    assert.equal(intake.company, "Apex Capital");
    assert.equal(intake.status, "submitted");

    const intakes = await services.listFundingIntakes();
    assert.equal(intakes.data.intakes[0].id, intake.id);
    assert.equal(intakes.data.intakes[0].contactEmail, "ops@apex.capital");
  });
});

test("funding intake update patches status and appends reviewer notes", async () => {
  await withTempStore(async ({ rootDir, storePath, seedPath }) => {
    const services = createSiteServices({ rootDir, storePath, seedPath });
    const intake = await services.createFundingIntake({
      company: "Nova Fund",
      contactEmail: "hello@novafund.io",
      fundingLane: "Validator Growth",
      summary: "Strategic validator seat acquisition.",
    });
    assert.equal(intake.status, "submitted");

    const updated = await services.updateFundingIntake(intake.id, {
      status: "approved",
      reviewerNotes: "Strong team, good traction.",
    });
    assert.equal(updated.status, "approved");
    assert.ok(updated.reviewerNotes.includes("Strong team, good traction."));

    const second = await services.updateFundingIntake(intake.id, {
      reviewerNotes: "Follow-up: confirm node count.",
    });
    assert.equal(second.reviewerNotes.length, 2);
    assert.equal(second.reviewerNotes[1], "Follow-up: confirm node count.");
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

    const htmlResponse = await fetch(`http://127.0.0.1:${port}/x3star-landing.html`, {
      headers: { Host: "x3.net", "x-forwarded-host": "x3.net" },
    });
    assert.equal(htmlResponse.status, 200);
    const htmlBody = await htmlResponse.text();
    assert.match(htmlBody, /x3-site-nav\.css/);
    assert.match(htmlBody, /x3-site-nav\.js/);
  } finally {
    await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  }
});

test("http server routes .org to the built React/Tauri app", async () => {
  const server = createServer();
  await new Promise((resolve) => server.listen(0, resolve));
  const port = server.address().port;

  try {
    const response = await fetch(`http://127.0.0.1:${port}/`, {
      headers: { Host: "x3.org", "x-forwarded-host": "x3.org" },
    });
    assert.equal(response.status, 200);
    const body = await response.text();
    assert.match(body, /<title>X3STAR Frontend<\/title>/);
    assert.match(body, /<div id="root"><\/div>/);
  } finally {
    await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  }
});

test("http server routes .net to legacy static HTML site", async () => {
  const server = createServer();
  await new Promise((resolve) => server.listen(0, resolve));
  const port = server.address().port;

  try {
    const response = await fetch(`http://127.0.0.1:${port}/`, {
      headers: { Host: "x3.net", "x-forwarded-host": "x3.net" },
    });
    assert.equal(response.status, 200);
    const body = await response.text();
    assert.match(body, /<title>X3STAR — The Next-Generation Blockchain Infrastructure<\/title>/);
    assert.match(body, /x3-site-nav\.css/);
  } finally {
    await new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve())));
  }
});
