const fs = require("fs/promises");
const path = require("path");
const { readStore, updateStore } = require("./store");

function fileTimeToIso(stat) {
  return stat?.mtime ? stat.mtime.toISOString() : new Date().toISOString();
}

function validateEmail(email) {
  return typeof email === "string" && /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email.trim());
}

function normalizeFundingStatus(status) {
  const normalized = String(status || "submitted").toLowerCase();
  const allowed = ["submitted", "review", "approved", "rejected", "funded", "changes_requested"];
  return allowed.includes(normalized) ? normalized : "submitted";
}

function requireString(field, value) {
  if (!value || String(value).trim().length === 0) {
    throw new Error(`Missing required field: ${field}`);
  }
  return String(value).trim();
}

function requirePositiveNumber(field, value) {
  const num = Number(value);
  if (!Number.isFinite(num) || num <= 0) {
    throw new Error(`Invalid positive numeric value for field: ${field}`);
  }
  return num;
}

function countryFlag(countryCode) {
  if (!countryCode || countryCode.length !== 2) return "🌍";
  return countryCode
    .toUpperCase()
    .split("")
    .map((char) => String.fromCodePoint(127397 + char.charCodeAt(0)))
    .join("");
}

function formatMoney(value) {
  return `$${Number(value || 0).toLocaleString("en-US")}`;
}

function relativeTime(isoString) {
  const delta = Date.now() - new Date(isoString).getTime();
  const seconds = Math.max(0, Math.floor(delta / 1000));
  if (seconds < 60) return "just now";
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

function bucketReservations(reservations, hours = 24) {
  const now = Date.now();
  const buckets = Array.from({ length: hours }, () => 0);
  for (const reservation of reservations) {
    const ageHours = Math.floor((now - new Date(reservation.createdAt).getTime()) / 3600000);
    if (ageHours >= 0 && ageHours < hours) {
      buckets[hours - ageHours - 1] += 1;
    }
  }
  return buckets;
}

function tierLabel(tier) {
  const mapping = {
    genesis: "Genesis",
    star: "Star",
    lite: "Lite",
  };
  return mapping[tier] || String(tier || "").replace(/^./, (char) => char.toUpperCase());
}

function regionFromCountry(countryCode) {
  const regions = {
    US: "North America",
    CA: "North America",
    MX: "North America",
    BR: "South America",
    AR: "South America",
    CL: "South America",
    GB: "Europe",
    DE: "Europe",
    FR: "Europe",
    NL: "Europe",
    CH: "Europe",
    SE: "Europe",
    ES: "Europe",
    IT: "Europe",
    SG: "Asia Pacific",
    JP: "Asia Pacific",
    KR: "Asia Pacific",
    HK: "Asia Pacific",
    CN: "Asia Pacific",
    AU: "Asia Pacific",
    NZ: "Asia Pacific",
    IN: "Asia Pacific",
    AE: "Middle East",
    SA: "Middle East",
    NG: "Africa",
    GH: "Africa",
  };
  return regions[String(countryCode || "").toUpperCase()] || "Other";
}

function classifyWhale(type) {
  const mapping = {
    whale: "WHALE",
    institution: "INSTITUTION",
    validator: "VALIDATOR",
  };
  return mapping[type] || String(type || "").toUpperCase();
}

function getGenesisSlotEntries(reservations, totalSlots) {
  const genesisReservations = reservations
    .filter((reservation) => reservation.tier === "genesis")
    .sort((left, right) => new Date(left.createdAt) - new Date(right.createdAt));
  const assigned = genesisReservations.map((reservation, index) => ({
    ...reservation,
    slotNumber: reservation.slotNumber || index + 1,
  }));
  return Array.from({ length: totalSlots }, (_, index) => {
    const slotNumber = index + 1;
    const reservation = assigned.find((entry) => entry.slotNumber === slotNumber);
    return {
      slotNumber,
      reserved: Boolean(reservation),
      reservation: reservation
        ? {
            id: reservation.id,
            name: reservation.name,
            flag: countryFlag(reservation.countryCode),
            location: reservation.location,
            wallet: reservation.wallet,
            createdAt: reservation.createdAt,
            timeAgo: relativeTime(reservation.createdAt),
          }
        : null,
    };
  });
}

function aggregateTopInvestors(reservations, limit = 6) {
  const byWallet = new Map();
  for (const reservation of reservations) {
    const key = reservation.wallet || `${reservation.name}:${reservation.location}`;
    const current = byWallet.get(key) || {
      key,
      wallet: reservation.wallet,
      name: reservation.name,
      countryCode: reservation.countryCode,
      amountUsd: 0,
      tier: reservation.tier,
      count: 0,
      latestAt: reservation.createdAt,
    };
    current.amountUsd += Number(reservation.amountUsd || 0) * Number(reservation.quantity || 1);
    current.count += 1;
    if (new Date(reservation.createdAt) > new Date(current.latestAt)) {
      current.latestAt = reservation.createdAt;
      current.name = reservation.name;
      current.countryCode = reservation.countryCode;
      current.tier = reservation.tier;
    }
    byWallet.set(key, current);
  }
  return Array.from(byWallet.values())
    .sort((left, right) => right.amountUsd - left.amountUsd)
    .slice(0, limit)
    .map((entry, index) => ({
      rank: index + 1,
      name: entry.name,
      flag: countryFlag(entry.countryCode),
      amountUsd: entry.amountUsd,
      badge:
        entry.amountUsd >= 100000
          ? "WHALE"
          : entry.tier === "genesis"
            ? "GENESIS NODE"
            : `${tierLabel(entry.tier).toUpperCase()} NODE`,
    }));
}

function summarizeStatuses(statuses) {
  if (statuses.includes("unavailable")) return "unavailable";
  if (statuses.includes("stale")) return "stale";
  return "live";
}

function envelope(data, options) {
  return {
    data,
    source: options.source,
    lastUpdated: options.lastUpdated || new Date().toISOString(),
    status: options.status || "live",
    ...(options.staleReason ? { staleReason: options.staleReason } : {}),
  };
}

function createSiteServices(options) {
  const rootDir = options.rootDir;
  const storePath =
    options.storePath || path.join(rootDir, "data", "business-store.json");
  const seedPath =
    options.seedPath || path.join(rootDir, "data", "business-store.json");
  const rpcUrl = options.rpcUrl || process.env.X3_RPC_URL || "http://127.0.0.1:9944";
  const gatewayUrl =
    options.gatewayUrl || process.env.X3_GATEWAY_GRAPHQL_URL || "";
  const fetchImpl = options.fetchImpl || fetch;

  const benchmarkFiles = {
    rpc: path.join(rootDir, "..", "infra-structure", "validator", "benchmarks", "rpc_benchmark_report.json"),
    live: path.join(rootDir, "..", "infra-structure", "validator", "benchmarks", "live_benchmark_report.json"),
    tps: path.join(rootDir, "..", "infra-structure", "validator", "benchmarks", "tps_benchmark_results.json"),
    crypto: path.join(rootDir, "..", "tests_core", "p4_benchmarks", "crypto_bench_report.json"),
  };

  async function statOrNull(filePath) {
    try {
      return await fs.stat(filePath);
    } catch {
      return null;
    }
  }

  async function readJsonOrNull(filePath) {
    try {
      const raw = await fs.readFile(filePath, "utf8");
      return JSON.parse(raw);
    } catch {
      return null;
    }
  }

  async function rpcCall(method, params = []) {
    const response = await fetchImpl(rpcUrl, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        jsonrpc: "2.0",
        id: Date.now(),
        method,
        params,
      }),
    });
    if (!response.ok) {
      throw new Error(`RPC ${method} failed with ${response.status}`);
    }
    const payload = await response.json();
    if (payload.error) {
      throw new Error(payload.error.message || `RPC ${method} error`);
    }
    return payload.result;
  }

  async function safeRpcState() {
    try {
      const [blockHex, gasHex, health, chainName, finalizedHead] = await Promise.all([
        rpcCall("eth_blockNumber"),
        rpcCall("eth_gasPrice"),
        rpcCall("system_health"),
        rpcCall("system_name"),
        rpcCall("chain_getFinalizedHead"),
      ]);
      return {
        ok: true,
        lastUpdated: new Date().toISOString(),
        data: {
          blockNumber: Number.parseInt(blockHex, 16),
          gasPriceWei: Number.parseInt(gasHex, 16),
          health,
          chainName,
          finalizedHead,
        },
      };
    } catch (error) {
      return {
        ok: false,
        lastUpdated: new Date().toISOString(),
        error: error.message,
      };
    }
  }

  async function safeRecentBlocks(limit = 5) {
    const state = await safeRpcState();
    if (!state.ok) {
      return { ok: false, error: state.error, lastUpdated: state.lastUpdated, blocks: [] };
    }
    try {
      const latest = state.data.blockNumber;
      const heights = Array.from({ length: limit }, (_, index) => latest - index).filter(
        (value) => value >= 0,
      );
      const blocks = await Promise.all(
        heights.map(async (height) => {
          const hash = await rpcCall("chain_getBlockHash", [height]);
          const block = await rpcCall("chain_getBlock", [hash]);
          return {
            height,
            hash,
            extrinsics: block?.block?.extrinsics || [],
          };
        }),
      );
      return {
        ok: true,
        lastUpdated: new Date().toISOString(),
        blocks,
      };
    } catch (error) {
      return {
        ok: false,
        error: error.message,
        lastUpdated: new Date().toISOString(),
        blocks: [],
      };
    }
  }

  async function safeGatewaySummary() {
    if (!gatewayUrl) {
      return {
        ok: false,
        error: "gateway not configured",
        lastUpdated: new Date().toISOString(),
      };
    }
    const query = `
      query SiteSummary {
        latestBlock {
          number
          hash
          timestamp
        }
        blocks(limit: 3) {
          number
          hash
          timestamp
        }
      }
    `;
    try {
      const response = await fetchImpl(gatewayUrl, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ query }),
      });
      if (!response.ok) {
        throw new Error(`Gateway query failed with ${response.status}`);
      }
      const payload = await response.json();
      if (payload.errors?.length) {
        throw new Error(payload.errors[0].message);
      }
      return {
        ok: true,
        lastUpdated: new Date().toISOString(),
        data: payload.data,
      };
    } catch (error) {
      return {
        ok: false,
        error: error.message,
        lastUpdated: new Date().toISOString(),
      };
    }
  }

  async function getBenchmarksOverview() {
    const [rpcReport, liveReport, tpsReport, cryptoReport] = await Promise.all([
      readJsonOrNull(benchmarkFiles.rpc),
      readJsonOrNull(benchmarkFiles.live),
      readJsonOrNull(benchmarkFiles.tps),
      readJsonOrNull(benchmarkFiles.crypto),
    ]);
    const [rpcStat, liveStat, tpsStat, cryptoStat] = await Promise.all([
      statOrNull(benchmarkFiles.rpc),
      statOrNull(benchmarkFiles.live),
      statOrNull(benchmarkFiles.tps),
      statOrNull(benchmarkFiles.crypto),
    ]);
    return {
      chainbench: {
        sourceFile: benchmarkFiles.rpc,
        lastUpdated: fileTimeToIso(rpcStat),
        summary: rpcReport?.summary || null,
      },
      live: {
        sourceFile: benchmarkFiles.live,
        lastUpdated: fileTimeToIso(liveStat),
        summary: liveReport || null,
      },
      tps: {
        sourceFile: benchmarkFiles.tps,
        lastUpdated: fileTimeToIso(tpsStat),
        summary: tpsReport || null,
      },
      crypto: {
        sourceFile: benchmarkFiles.crypto,
        lastUpdated: fileTimeToIso(cryptoStat),
        summary: cryptoReport || null,
      },
    };
  }

  async function getHealth() {
    const store = await readStore(storePath, seedPath);
    const [rpcState, gatewayState, benchmarks] = await Promise.all([
      safeRpcState(),
      safeGatewaySummary(),
      getBenchmarksOverview(),
    ]);
    const benchmarkStatus = benchmarks.live.summary ? "stale" : "unavailable";
    const statuses = [
      rpcState.ok ? "live" : "unavailable",
      gatewayState.ok ? "live" : "unavailable",
      "live",
      benchmarkStatus,
    ];
    return envelope(
      {
        rpc: {
          status: rpcState.ok ? "live" : "unavailable",
          lastUpdated: rpcState.lastUpdated,
          detail: rpcState.ok ? rpcState.data.chainName : rpcState.error,
        },
        gateway: {
          status: gatewayState.ok ? "live" : "unavailable",
          lastUpdated: gatewayState.lastUpdated,
          detail: gatewayState.ok ? "graphql available" : gatewayState.error,
        },
        businessStore: {
          status: "live",
          lastUpdated: fileTimeToIso(await statOrNull(storePath)),
          detail: `${store.reservations.length} reservations`,
        },
        benchmarks: {
          status: benchmarkStatus,
          lastUpdated: benchmarks.live.lastUpdated,
          detail: benchmarks.live.summary
            ? `${benchmarks.live.summary.combined_tps} combined benchmark TPS`
            : "no benchmark snapshot",
        },
      },
      {
        source: "rpc,gateway,business-store,benchmarks",
        status: summarizeStatuses(statuses),
        lastUpdated: new Date().toISOString(),
        staleReason: rpcState.ok ? undefined : "X3 RPC unavailable; some pages will render unavailable",
      },
    );
  }

  async function getDashboard() {
    const store = await readStore(storePath, seedPath);
    const rpcState = await safeRpcState();
    const benchmarks = await getBenchmarksOverview();
    const blockNumber = rpcState.ok ? rpcState.data.blockNumber : null;
    const gasPriceGwei = rpcState.ok ? Math.floor(rpcState.data.gasPriceWei / 1e9) : null;
    const network = {
      tps: benchmarks.live.summary?.combined_tps || benchmarks.tps.summary?.peak_tps || null,
      validators: rpcState.ok ? rpcState.data.health?.peers || null : null,
      uptime: rpcState.ok ? (rpcState.data.health?.isSyncing ? 0 : 99.9) : null,
      finality: rpcState.ok ? 0.4 : null,
      blockNumber,
    };
    const grants = (store.grantApplications || []).map((grant) => ({
      id: grant.id,
      name:
        grant.projectName ||
        grant.name ||
        grant.teamName ||
        grant.organization ||
        grant.company ||
        grant.title ||
        "Grant",
      amountUsd:
        grant.amountUsd ||
        grant.requestedUsd ||
        grant.requestedAmountUsd ||
        grant.budgetUsd ||
        grant.fundingRequestedUsd ||
        0,
      progressPct: grant.progressPct || grant.progress || grant.completionPct || null,
      status: grant.status || grant.stage || "submitted",
      createdAt: grant.createdAt,
    }));
    return envelope(
      {
        blockNumber,
        gasPriceGwei,
        network,
        token: store.token,
        funding: {
          ...store.presale,
          hardCap: store.presale.hardCapUsd,
          raised: store.presale.raisedUsd,
          activeGrants: store.grantApplications.length,
          investorCount: store.presale.investors,
        },
        grants,
      },
      {
        source: rpcState.ok ? "rpc,business-store,benchmarks" : "business-store,benchmarks",
        status: rpcState.ok ? "stale" : "stale",
        lastUpdated: rpcState.ok ? rpcState.lastUpdated : fileTimeToIso(await statOrNull(storePath)),
        staleReason: "Token and funding metrics are served from the local business store; TPS comes from benchmark artifacts.",
      },
    );
  }

  async function getNetwork() {
    const [rpcState, blocks, store] = await Promise.all([
      safeRpcState(),
      safeRecentBlocks(6),
      readStore(storePath, seedPath),
    ]);
    const transactions = blocks.blocks.flatMap((block) =>
      block.extrinsics.map((extrinsic, index) => ({
        hash: `${block.hash.slice(0, 12)}…${index}`,
        type: extrinsic.slice(0, 2) === "0x" ? "EXTRINSIC" : "CALL",
        detail: `Block #${block.height}`,
        amount: extrinsic.length,
        blockNumber: block.height,
        timestamp: blocks.lastUpdated,
      })),
    );
    const benchmark = await readJsonOrNull(benchmarkFiles.live);
    const validators = (store.networkTelemetry?.validators || []).map((validator) => ({
      id: validator.id,
      name: validator.id,
      operator: validator.name,
      tier: validator.tier,
      location: validator.location,
      countryCode: validator.countryCode,
      flag: countryFlag(validator.countryCode),
      lat: validator.lat,
      lon: validator.lon,
      tps: validator.tps,
      uptimePct: validator.uptimePct,
      latencyMs: validator.latencyMs,
      peers: validator.peers,
      stakeX3S: validator.stakeX3S,
      status: validator.status,
      lastHeartbeat: validator.lastHeartbeat,
    }));
    const regionMap = new Map();
    validators.forEach((validator) => {
      const region = regionFromCountry(validator.countryCode);
      const current = regionMap.get(region) || 0;
      regionMap.set(region, current + 1);
    });
    const regions = Array.from(regionMap.entries())
      .map(([region, count]) => ({
        region,
        count,
        pct: validators.length ? Number(((count / validators.length) * 100).toFixed(1)) : 0,
      }))
      .sort((left, right) => right.count - left.count);
    return envelope(
      {
        tps: benchmark?.combined_tps || null,
        blockNumber: rpcState.ok ? rpcState.data.blockNumber : null,
        totalTx: transactions.length,
        transactions,
        validators,
        regions,
        uptimePct: store.networkTelemetry?.avgUptime || null,
        finalitySeconds: store.networkTelemetry?.avgFinalitySeconds || null,
        avgFeeUsd: store.networkTelemetry?.avgFeeUsd || null,
      },
      {
        source: rpcState.ok ? "rpc,benchmarks,business-store" : "benchmarks,business-store",
        status: rpcState.ok ? "stale" : "stale",
        lastUpdated: rpcState.ok ? rpcState.lastUpdated : new Date().toISOString(),
        staleReason: rpcState.ok
          ? "Transaction feed is live from RPC. Validator map and regional telemetry are served from the indexed site store."
          : "RPC unavailable; validator map remains available from the indexed site store.",
      },
    );
  }

  async function getNodeHealth() {
    const [rpcState, store] = await Promise.all([safeRpcState(), readStore(storePath, seedPath)]);
    const nodes = (store.networkTelemetry?.validators || []).map((validator) => ({
      id: validator.id,
      name: validator.name,
      operatorId: validator.id,
      tier: validator.tier,
      location: validator.location,
      countryCode: validator.countryCode,
      flag: countryFlag(validator.countryCode),
      status: validator.status,
      uptimePct: validator.uptimePct,
      latencyMs: validator.latencyMs,
      peers: validator.peers,
      tps: validator.tps,
      healthScore: validator.healthScore,
      stakeX3S: validator.stakeX3S,
      lastHeartbeat: validator.lastHeartbeat,
      heartbeatAge: relativeTime(validator.lastHeartbeat),
    }));
    return envelope(
      {
        status: rpcState.ok ? "healthy" : "unavailable",
        blockNumber: rpcState.ok ? rpcState.data.blockNumber : null,
        peers: rpcState.ok ? rpcState.data.health?.peers || 0 : 0,
        isSyncing: rpcState.ok ? Boolean(rpcState.data.health?.isSyncing) : null,
        activeValidators: store.presale.tiers[0].reservedSlots,
        warnings: nodes.filter((node) => node.status === "warning").length,
        slashed: 0,
        uptime: store.networkTelemetry?.avgUptime || null,
        tps: null,
        nodes,
      },
      {
        source: rpcState.ok ? "rpc,business-store" : "business-store",
        status: rpcState.ok ? "live" : "stale",
        lastUpdated: rpcState.ok
          ? rpcState.lastUpdated
          : store.networkTelemetry?.generatedAt || fileTimeToIso(await statOrNull(storePath)),
        staleReason: rpcState.ok
          ? undefined
          : "RPC unavailable; per-node cards are served from the latest indexed telemetry snapshot.",
      },
    );
  }

  async function getGovernance() {
    const store = await readStore(storePath, seedPath);
    return envelope(
      {
        proposals: store.governance.proposals,
        proposalsCount: store.governance.proposals.length,
        activeProposals: store.governance.proposals.filter(
          (proposal) => proposal.status === "active",
        ).length,
        voters: store.governance.voters,
        treasury: store.governance.treasuryUsd,
        delegates: store.governance.delegates || [],
        treasuryAllocations: store.governance.treasuryAllocations || [],
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getStaking() {
    const store = await readStore(storePath, seedPath);
    return envelope(
      {
        totalValueLocked: store.staking.totalValueLockedUsd,
        avgApy: store.staking.avgApy,
        totalStakers: store.staking.totalStakers,
        dailyRewards: store.staking.dailyRewardsUsd,
        totalStaked: store.staking.totalStaked,
        pools: store.staking.pools,
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getLedger() {
    const store = await readStore(storePath, seedPath);
    return envelope(
      {
        raisedUsd: store.presale.raisedUsd,
        treasuryUsd: store.ledger.treasuryUsd,
        round3AmountUsd: store.presale.raisedUsd,
        lastVerified: new Date().toISOString(),
        events: store.ledger.events,
        multisigSigners: store.ledger.multisigSigners,
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getProofs() {
    const store = await readStore(storePath, seedPath);
    const reservations = [...store.reservations].sort(
      (left, right) => new Date(right.createdAt) - new Date(left.createdAt),
    );
    const genesisTier = store.presale.tiers.find((tier) => tier.name === "genesis");
    const slotsLeft = genesisTier.totalSlots - genesisTier.reservedSlots;
    const countryCounts = new Map();
    for (const reservation of reservations) {
      const key = String(reservation.countryCode || "??").toUpperCase();
      countryCounts.set(key, (countryCounts.get(key) || 0) + 1);
    }
    const topCountries = Array.from(countryCounts.entries())
      .sort((left, right) => right[1] - left[1])
      .slice(0, 7)
      .map(([code, count]) => ({
        code,
        flag: countryFlag(code),
        count,
      }));
    return envelope(
      {
        totalOperators: store.presale.investors,
        slotsLeft,
        totalSlots: genesisTier.totalSlots,
        reservedSlots: genesisTier.reservedSlots,
        selloutEtaHours: store.presale.daysRemaining * 24,
        tokenPriceUsd: store.presale.tokenPriceUsd,
        pace24h: bucketReservations(reservations),
        topCountries,
        operators: reservations.slice(0, 24).map((reservation) => ({
          name: reservation.name,
          location: reservation.location,
          flag: countryFlag(reservation.countryCode),
          tier: reservation.tier,
          amount: formatMoney(reservation.amountUsd),
          time: relativeTime(reservation.createdAt),
        })),
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getPresale() {
    const store = await readStore(storePath, seedPath);
    const tiers = store.presale.tiers.map((tier) => ({
      ...tier,
      slotsLeft: tier.totalSlots - tier.reservedSlots,
    }));
    return envelope(
      {
        ...store.presale,
        tiers,
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getGrants() {
    const store = await readStore(storePath, seedPath);
    const programs = store.grantPrograms || [];
    const applications = store.grantApplications || [];
    const totalPoolUsd = programs.reduce((sum, program) => sum + Number(program.amountUsd || 0), 0);
    const largestGrantUsd = programs.reduce(
      (max, program) => Math.max(max, Number(program.amountUsd || 0)),
      0,
    );
    const byStatus = programs.reduce((acc, program) => {
      const key = String(program.status || "unknown").toLowerCase();
      acc[key] = (acc[key] || 0) + 1;
      return acc;
    }, {});
    return envelope(
      {
        programs,
        applications,
        summary: {
          totalPoolUsd,
          largestGrantUsd,
          totalPrograms: programs.length,
          statusCounts: byStatus,
        },
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getBounties() {
    const store = await readStore(storePath, seedPath);
    const bounties = store.bounties || [];
    const totalPoolUsd = bounties.reduce((sum, bounty) => sum + Number(bounty.rewardUsd || 0), 0);
    const openCount = bounties.filter((bounty) => bounty.status === "open" || bounty.status === "hot").length;
    const claimedCount = bounties.filter((bounty) => bounty.status === "claimed").length;
    return envelope(
      {
        bounties,
        summary: {
          totalPoolUsd,
          openCount,
          claimedCount,
          totalCount: bounties.length,
        },
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getDeals() {
    const store = await readStore(storePath, seedPath);
    return envelope(
      {
        deals: store.deals || [],
        intakeCount: store.dealIntakes ? store.dealIntakes.length : 0,
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getLeaderboard() {
    const store = await readStore(storePath, seedPath);
    const validatorsRaw = store.networkTelemetry?.validators || [];
    const validators = validatorsRaw
      .map((validator) => {
        const uptime = Number(validator.uptimePct || 0);
        const tps = Number(validator.tps || 0);
        const stake = Number(validator.stakeX3S || 0);
        const score = Math.round(uptime * 60 + tps * 5 + Math.min(100, stake / 10000));
        return {
          id: validator.id,
          name: validator.id,
          flag: countryFlag(validator.countryCode),
          uptimePct: uptime,
          tps,
          stakeX3S: stake,
          tier: validator.tier,
          score,
          status: validator.status,
        };
      })
      .sort((a, b) => b.score - a.score);
    const investors = aggregateTopInvestors(store.reservations || []);
    const delegates = (store.governance.delegates || []).map((delegate) => ({
      name: delegate.name,
      power: delegate.power || delegate.votingPower || 0,
    }));
    return envelope(
      {
        validators,
        investors,
        delegates,
        summary: {
          validatorsCount: validators.length,
          investorsCount: investors.length,
          delegatesCount: delegates.length,
        },
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getReservations() {
    const store = await readStore(storePath, seedPath);
    const reservations = [...store.reservations].sort(
      (left, right) => new Date(right.createdAt) - new Date(left.createdAt),
    );
    const genesisTier = store.presale.tiers.find((tier) => tier.name === "genesis");
    const slotEntries = getGenesisSlotEntries(reservations, genesisTier.totalSlots);
    return envelope(
      {
        count: reservations.length,
        totalRaisedUsd: store.presale.raisedUsd,
        todayRaisedUsd: store.presale.todayUsd,
        activityHeatmap: bucketReservations(reservations, 24),
        topInvestors: aggregateTopInvestors(reservations),
        recentCards: reservations.slice(0, 24).map((reservation) => ({
          id: reservation.id,
          name: reservation.name,
          flag: countryFlag(reservation.countryCode),
          location: reservation.location,
          type:
            reservation.tier === "genesis"
              ? "node"
              : reservation.tier === "star"
                ? "node"
                : "token",
          tier: reservation.tier,
          amountUsd: reservation.amountUsd,
          detail:
            reservation.tier === "lite"
              ? `${Math.round(reservation.amountUsd / store.presale.tokenPriceUsd).toLocaleString("en-US")} X3S`
              : `${tierLabel(reservation.tier)} Node Reserved`,
          timeAgo: relativeTime(reservation.createdAt),
        })),
        slotTracker: {
          totalSlots: genesisTier.totalSlots,
          reservedSlots: genesisTier.reservedSlots,
          availableSlots: genesisTier.totalSlots - genesisTier.reservedSlots,
          slots: slotEntries,
          recentReservations: slotEntries
            .filter((slot) => slot.reserved)
            .slice(-8)
            .reverse(),
        },
        reservations: reservations.map((reservation) => ({
          ...reservation,
          flag: countryFlag(reservation.countryCode),
          timeAgo: relativeTime(reservation.createdAt),
        })),
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function createReservation(payload) {
    const amountUsd = Number(payload.amountUsd);
    const quantity = Number(payload.quantity || 1);
    const createdAt = new Date().toISOString();
    const nextStore = await updateStore(storePath, seedPath, async (store) => {
      store.counters.reservation += 1;
      const id = `RSV-${String(store.counters.resevvation).padStart(5, "0")}`;
      const record = {
        id,
        name: payload.name || "Anonymous Operator",
        location: payload.location || "Unknown",
        countryCode: payload.countryCode || "US",
        wallet: payload.wallet || "unlinked",
        tier: payload.tier || "lite",
        amountUsd,
        quantity,
        ...(payload.tier === "genesis"
          ? {
              slotNumber:
                (store.presale.tiers.find((entry) => entry.name === "genesis")?.reservedSlots || 0) + 1,
            }
          : {}),
        createdAt,
        status: "pending",
      };
      store.reservations.unshift(record);
      store.presale.raisedUsd += amountUsd * quantity;
      store.presale.todayUsd += amountUsd * quantity;
      store.presale.investors += 1;
      const tier = store.presale.tiers.find((entry) => entry.name === record.tier);
      if (tier) {
        tier.reservedSlots += quantity;
      }
      store.ledger.events.unshift({
        id: `ledger-${store.counters.reservation}`,
        kind: "Reservation",
        description: `${record.name} reserved ${record.tier}`,
        amountUsd: amountUsd * quantity,
        timestamp: createdAt,
      });
      return store;
    });
    return nextStore.reservations[0];
  }

  async function listFormRecords(formType) {
    const store = await readStore(storePath, seedPath);
    const mapping = {
      affiliate: store.affiliateApplications,
      grant: store.grantApplications,
      investor: store.investorInquiries,
      kyc: store.kycApplications,
      deal: store.dealIntakes,
    };
    return envelope(
      {
        type: formType,
        items: mapping[formType] || [],
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function createFormRecord(formType, payload) {
    const collectionMap = {
      affiliate: "affiliateApplications",
      grant: "grantApplications",
      investor: "investorInquiries",
      kyc: "kycApplications",
      deal: "dealIntakes",
    };
    const counterMap = {
      affiliate: "affiliate",
      grant: "grant",
      investor: "investorInquiry",
      kyc: "kyc",
      deal: "deal",
    };
    const collectionName = collectionMap[formType];
    const counterName = counterMap[formType];
    if (!collectionName || !counterName) {
      throw new Error(`Unsupported form type: ${formType}`);
    }
    const createdAt = new Date().toISOString();
    const store = await updateStore(storePath, seedPath, async (current) => {
      current.counters[counterName] = (current.counters[counterName] || 0) + 1;
      current[collectionName] = current[collectionName] || [];
      const record = {
        id: `${formType.toUpperCase()}-${String(current.counters[counterName]).padStart(5, "0")}`,
        createdAt,
        status: "submitted",
        ...payload,
      };
      current[collectionName].unshift(record);
      return current;
    });
    return store[collectionName][0];
  }

  async function getFundingPrograms() {
    const store = await readStore(storePath, seedPath);
    const programs = store.grantPrograms || [];
    return envelope(
      { programs },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getFundingApplications() {
    const store = await readStore(storePath, seedPath);
    const applications = store.grantApplications || [];
    return envelope(
      { applications },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function createFundingApplication(payload) {
    const createdAt = new Date().toISOString();
    const record = {
      projectName: requireString("projectName", payload.projectName),
      organization: requireString("organization", payload.organization),
      contactEmail: payload.contactEmail && validateEmail(payload.contactEmail)
        ? payload.contactEmail.trim()
        : (() => { throw new Error("Invalid contactEmail"); })(),
      fundingLane: requireString("fundingLane", payload.fundingLane),
      requestedUsd: requirePositiveNumber("requestedUsd", payload.requestedUsd),
      summary: requireString("summary", payload.summary),
      status: "submitted",
      progressPct: 0,
      reviewerNotes: [],
      proofLinks: [],
      createdAt,
    };
    const store = await updateStore(storePath, seedPath, async (current) => {
      current.counters.grant = (current.counters.grant || 0) + 1;
      current.grantApplications = current.grantApplications || [];
      record.id = `GRANT-${String(current.counters.grant).padStart(5, "0")}`;
      current.grantApplications.unshift(record);
      return current;
    });
    return store.grantApplications[0];
  }

  async function updateFundingApplication(id, payload) {
    if (!id) {
      throw new Error("Funding application id is required");
    }
    const store = await updateStore(storePath, seedPath, async (current) => {
      current.grantApplications = current.grantApplications || [];
      const index = current.grantApplications.findIndex((item) => item.id === id);
      if (index === -1) {
        throw new Error(`Funding application not found: ${id}`);
      }
      const existing = current.grantApplications[index];
      const updates = {
        ...existing,
        ...payload,
      };
      if (payload.status) {
        updates.status = normalizeFundingStatus(payload.status);
      }
      if (payload.requestedUsd !== undefined) {
        updates.requestedUsd = requirePositiveNumber("requestedUsd", payload.requestedUsd);
      }
      if (payload.contactEmail !== undefined) {
        if (!validateEmail(payload.contactEmail)) {
          throw new Error("Invalid contactEmail");
        }
        updates.contactEmail = payload.contactEmail.trim();
      }
      if (payload.reviewerNotes) {
        updates.reviewerNotes = Array.isArray(payload.reviewerNotes)
          ? payload.reviewerNotes
          : [...(existing.reviewerNotes || []), String(payload.reviewerNotes)];
      }
      current.grantApplications[index] = updates;
      return current;
    });
    return store.grantApplications.find((item) => item.id === id);
  }

  async function listFundingIntakes() {
    const store = await readStore(storePath, seedPath);
    const intakes = store.fundingIntakes || [];
    return envelope(
      { intakes },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function createFundingIntake(payload) {
    const createdAt = new Date().toISOString();
    const record = {
      company: requireString("company", payload.company || payload.organization || payload.name),
      contactEmail: payload.contactEmail && validateEmail(payload.contactEmail)
        ? payload.contactEmail.trim()
        : (() => { throw new Error("Invalid contactEmail"); })(),
      fundingLane: requireString("fundingLane", payload.fundingLane),
      summary: requireString("summary", payload.summary),
      status: "submitted",
      reviewerNotes: [],
      proofLinks: [],
      createdAt,
      ...payload,
    };
    const store = await updateStore(storePath, seedPath, async (current) => {
      current.counters.fundingIntake = (current.counters.fundingIntake || 0) + 1;
      current.fundingIntakes = current.fundingIntakes || [];
      record.id = `FIN-${String(current.counters.fundingIntake).padStart(5, "0")}`;
      current.fundingIntakes.unshift(record);
      return current;
    });
    return store.fundingIntakes[0];
  }

  async function updateFundingIntake(id, payload) {
    if (!id) {
      throw new Error("Funding intake id is required");
    }
    const store = await updateStore(storePath, seedPath, async (current) => {
      current.fundingIntakes = current.fundingIntakes || [];
      const index = current.fundingIntakes.findIndex((item) => item.id === id);
      if (index === -1) {
        throw new Error(`Funding intake not found: ${id}`);
      }
      const existing = current.fundingIntakes[index];
      const updates = { ...existing, ...payload };
      if (payload.status) {
        updates.status = normalizeFundingStatus(payload.status);
      }
      if (payload.contactEmail !== undefined) {
        if (!validateEmail(payload.contactEmail)) {
          throw new Error("Invalid contactEmail");
        }
        updates.contactEmail = payload.contactEmail.trim();
      }
      if (payload.reviewerNotes) {
        updates.reviewerNotes = Array.isArray(payload.reviewerNotes)
          ? payload.reviewerNotes
          : [...(existing.reviewerNotes || []), String(payload.reviewerNotes)];
      }
      current.fundingIntakes[index] = updates;
      return current;
    });
    return store.fundingIntakes.find((item) => item.id === id);
  }

  async function getFundingScoreboard() {
    const store = await readStore(storePath, seedPath);
    const programs = store.grantPrograms || [];
    const applications = store.grantApplications || [];
    const intakes = store.fundingIntakes || [];
    const totalRequestedUsd = applications.reduce(
      (sum, application) => sum + Number(application.requestedUsd || 0),
      0,
    );
    const statusCounts = applications.reduce((result, application) => {
      const status = String(application.status || "submitted").toLowerCase();
      result[status] = (result[status] || 0) + 1;
      return result;
    }, {});
    const programStatusCounts = programs.reduce((result, program) => {
      const status = String(program.status || "unknown").toLowerCase();
      result[status] = (result[status] || 0) + 1;
      return result;
    }, {});
    return envelope(
      {
        programs,
        applications,
        intakes,
        summary: {
          totalRequestedUsd,
          totalApplications: applications.length,
          totalPrograms: programs.length,
          totalIntakes: intakes.length,
          applicationStatusCounts: statusCounts,
          programStatusCounts,
        },
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getBenchmark(name) {
    const benchmarks = await getBenchmarksOverview();
    const mapping = {
      chainbench: benchmarks.chainbench,
      "stress-test": benchmarks.live,
      tps: benchmarks.tps,
      crypto: benchmarks.crypto,
      overview: benchmarks,
    };
    const data = mapping[name];
    return envelope(data || null, {
      source: "benchmarks",
      status: data ? "stale" : "unavailable",
      lastUpdated: data?.lastUpdated || new Date().toISOString(),
      staleReason: data ? "Benchmark data comes from the latest saved artifact." : "Benchmark artifact missing.",
    });
  }

  async function getWhales() {
    const store = await readStore(storePath, seedPath);
    const price = Number(store.token.priceUsd || 0);
    const whales = (store.marketWhales?.wallets || []).map((wallet, index) => ({
      rank: index + 1,
      name: wallet.name,
      address: wallet.address,
      classification: wallet.classification,
      badge: classifyWhale(wallet.classification),
      holdingsX3S: wallet.holdingsX3S,
      holdingsDisplay: `${(wallet.holdingsX3S / 1000000).toFixed(1)}M X3S`,
      usdValue: wallet.usdValue,
      usdDisplay: formatMoney(wallet.usdValue),
      changePct24h: wallet.changePct24h,
      activityScore: wallet.activityScore,
      color:
        wallet.classification === "whale"
          ? "#FFD700"
          : wallet.classification === "institution"
            ? "#8833FF"
            : "#00D4FF",
    }));
    const events = (store.marketWhales?.events || []).map((event) => ({
      ...event,
      amountDisplay: `${Number(event.amountX3S).toLocaleString("en-US")} X3S`,
      amountUsdDisplay: formatMoney(event.amountUsd),
      timeAgo: relativeTime(event.timestamp),
    }));
    return envelope(
      {
        buyPct: store.marketWhales?.buyPct || 0,
        sellPct: store.marketWhales?.sellPct || 0,
        accumulationScore: store.marketWhales?.accumulationScore || 0,
        dominantSentiment:
          (store.marketWhales?.buyPct || 0) >= 60
            ? "BULLISH"
            : (store.marketWhales?.sellPct || 0) >= 60
              ? "BEARISH"
              : "NEUTRAL",
        netFlowX3S24h: store.marketWhales?.netFlowX3S24h || 0,
        priceUsd: price,
        priceChange24h: store.token.priceChange24h,
        totalSupplyX3S: store.token.totalSupply,
        circulatingSupplyX3S: store.token.circulatingSupply,
        whales,
        events,
        alerts: events.filter((event) => event.type === "BUY").length,
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: store.marketWhales?.generatedAt || fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  async function getTokenomics() {
    const store = await readStore(storePath, seedPath);
    const totalSupply = Number(store.token.totalSupply || 0);
    const allocations = (store.tokenomics?.allocations || []).map((allocation) => ({
      ...allocation,
      pct: totalSupply ? Number(((allocation.amountX3S / totalSupply) * 100).toFixed(1)) : 0,
    }));
    return envelope(
      {
        priceUsd: store.token.priceUsd,
        priceChange24h: store.token.priceChange24h,
        marketCapUsd: store.token.marketCapUsd,
        fdvUsd: Number(store.token.priceUsd || 0) * totalSupply,
        totalSupplyX3S: totalSupply,
        circulatingSupplyX3S: store.token.circulatingSupply,
        lockRatePct: totalSupply
          ? Number(
              (
                ((store.staking.totalStaked + (store.tokenomics?.allocations?.find((item) => item.name === "Presale Locked")?.amountX3S || 0)) /
                  totalSupply) *
                100
              ).toFixed(1),
            )
          : 0,
        allocations,
        burnedX3S: store.tokenomics?.burnedX3S || 0,
        burnRateHourlyX3S: store.tokenomics?.burnRateHourlyX3S || 0,
        burnDailyX3S: store.tokenomics?.burnDailyX3S || 0,
        dailyEmissionsX3S: store.tokenomics?.dailyEmissionsX3S || 0,
        halvingInDays: store.tokenomics?.halvingInDays || 0,
        unlock30dX3S: store.tokenomics?.unlock30dX3S || 0,
        unlock90dX3S: store.tokenomics?.unlock90dX3S || 0,
        lockedSupplyX3S: store.tokenomics?.lockedSupplyX3S || 0,
        vesting: store.tokenomics?.vesting || [],
        events: (store.marketWhales?.events || []).slice(0, 6),
      },
      {
        source: "business-store",
        status: "live",
        lastUpdated: store.tokenomics?.generatedAt || fileTimeToIso(await statOrNull(storePath)),
      },
    );
  }

  return {
    getHealth,
    getDashboard,
    getNetwork,
    getNodeHealth,
    getGovernance,
    getStaking,
    getLedger,
    getProofs,
    getPresale,
    getReservations,
    getWhales,
    getTokenomics,
    createReservation,
    listFormRecords,
    createFormRecord,
    getFundingPrograms,
    getFundingApplications,
    createFundingApplication,
    updateFundingApplication,
    listFundingIntakes,
    createFundingIntake,
    updateFundingIntake,
    getFundingScoreboard,
    getBenchmark,
  };
}

module.exports = {
  createSiteServices,
};
