(function (global) {
  "use strict";

  var CONFIG = {
    apiBase:
      (typeof window !== "undefined" ? window.location.origin : "") + "/api/site",
    cacheTtlMs: 15000,
  };

  var cache = new Map();
  var lastHealth = null;
  var initialized = false;

  function now() {
    return Date.now();
  }

  function getCacheKey(path, method) {
    return method + ":" + path;
  }

  function getCached(path, method) {
    var item = cache.get(getCacheKey(path, method || "GET"));
    if (!item) return null;
    if (now() > item.expiresAt) {
      cache.delete(getCacheKey(path, method || "GET"));
      return null;
    }
    return item.value;
  }

  function setCached(path, method, value) {
    cache.set(getCacheKey(path, method || "GET"), {
      value: value,
      expiresAt: now() + CONFIG.cacheTtlMs,
    });
  }

  function clearCachedPrefix(prefix) {
    Array.from(cache.keys()).forEach(function (key) {
      if (key.indexOf(prefix) !== -1) {
        cache.delete(key);
      }
    });
  }

  function formatTime(isoString) {
    if (!isoString) return "unknown";
    try {
      return new Date(isoString).toLocaleString();
    } catch {
      return isoString;
    }
  }

  function ensureStatusBanner() {
    if (typeof document === "undefined") return null;
    var existing = document.getElementById("x3-site-status");
    if (existing) return existing;
    var banner = document.createElement("div");
    banner.id = "x3-site-status";
    banner.style.position = "fixed";
    banner.style.top = "0";
    banner.style.left = "0";
    banner.style.right = "0";
    banner.style.zIndex = "9999";
    banner.style.padding = "10px 16px";
    banner.style.fontFamily = "JetBrains Mono, monospace";
    banner.style.fontSize = "11px";
    banner.style.letterSpacing = "0.06em";
    banner.style.textTransform = "uppercase";
    banner.style.backdropFilter = "blur(18px)";
    banner.style.borderBottom = "1px solid rgba(255,255,255,0.12)";
    banner.style.background = "rgba(8, 14, 30, 0.92)";
    banner.style.color = "#dbeafe";
    banner.textContent = "Connecting to X3 site data…";
    document.body.appendChild(banner);
    updateStatusOffset(banner);
    return banner;
  }

  function updateStatusOffset(banner) {
    if (typeof document === "undefined" || !banner) return;
    var height = Math.ceil(banner.getBoundingClientRect().height || 0);
    document.documentElement.style.setProperty("--x3-status-height", height + "px");
  }

  function renderStatusBanner(envelope) {
    var banner = ensureStatusBanner();
    if (!banner || !envelope) return;
    var status = envelope.status || "unavailable";
    var palette = {
      live: {
        background: "rgba(3, 44, 33, 0.92)",
        color: "#b7f7d6",
        border: "rgba(16, 185, 129, 0.32)",
      },
      stale: {
        background: "rgba(59, 36, 5, 0.92)",
        color: "#facc15",
        border: "rgba(234, 179, 8, 0.32)",
      },
      unavailable: {
        background: "rgba(65, 12, 23, 0.92)",
        color: "#fecdd3",
        border: "rgba(244, 63, 94, 0.32)",
      },
    }[status] || {
      background: "rgba(8, 14, 30, 0.92)",
      color: "#dbeafe",
      border: "rgba(255,255,255,0.12)",
    };
    banner.style.background = palette.background;
    banner.style.color = palette.color;
    banner.style.borderBottomColor = palette.border;
    banner.textContent =
      "Site status: " +
      status +
      " • source: " +
      (envelope.source || "unknown") +
      " • updated: " +
      formatTime(envelope.lastUpdated) +
      (envelope.staleReason ? " • " + envelope.staleReason : "");
    updateStatusOffset(banner);
  }

  function renderModuleMeta(target, label, envelope) {
    if (typeof document === "undefined") return;
    var host =
      typeof target === "string" ? document.querySelector(target) : target;
    if (!host || !envelope) return;
    var meta = host.querySelector(".x3-module-meta");
    if (!meta) {
      meta = document.createElement("div");
      meta.className = "x3-module-meta";
      meta.style.marginTop = "8px";
      meta.style.fontSize = "10px";
      meta.style.letterSpacing = "0.08em";
      meta.style.textTransform = "uppercase";
      meta.style.fontFamily = "JetBrains Mono, monospace";
      meta.style.opacity = "0.85";
      host.appendChild(meta);
    }
    var prefix = label ? label + " • " : "";
    meta.textContent =
      prefix +
      envelope.status +
      " • " +
      envelope.source +
      " • " +
      formatTime(envelope.lastUpdated);
  }

  async function requestEnvelope(path, options) {
    var method = (options && options.method) || "GET";
    var cached = method === "GET" ? getCached(path, method) : null;
    if (cached && !(options && options.refresh)) {
      return cached;
    }
    var response = await fetch(CONFIG.apiBase + path, {
      method: method,
      headers: {
        "content-type": "application/json",
      },
      body:
        options && options.body ? JSON.stringify(options.body) : undefined,
    });
    if (!response.ok) {
      throw new Error("API " + path + " failed with " + response.status);
    }
    var payload = await response.json();
    if (method === "GET") {
      setCached(path, method, payload);
    } else {
      clearCachedPrefix("/presale");
      clearCachedPrefix("/reservations");
      clearCachedPrefix("/forms/");
      clearCachedPrefix("/dashboard");
      clearCachedPrefix("/proofs");
      clearCachedPrefix("/ledger");
    }
    return payload;
  }

  async function initialize() {
    if (initialized) return publicApi;
    initialized = true;
    try {
      lastHealth = await requestEnvelope("/health", { refresh: true });
      renderStatusBanner(lastHealth);
    } catch (error) {
      renderStatusBanner({
        status: "unavailable",
        source: "site-client",
        lastUpdated: new Date().toISOString(),
        staleReason: error.message,
      });
    }
    return publicApi;
  }

  function createDataGetter(path) {
    return async function (options) {
      var envelope = await requestEnvelope(path, options);
      renderStatusBanner(envelope);
      return envelope.data;
    };
  }

  function createEnvelopeGetter(path) {
    return async function (options) {
      var envelope = await requestEnvelope(path, options);
      renderStatusBanner(envelope);
      return envelope;
    };
  }

  function subscribe(topic, handler) {
    if (typeof EventSource === "undefined") return null;
    var source = new EventSource(
      CONFIG.apiBase + "/stream?topic=" + encodeURIComponent(topic),
    );
    source.addEventListener("update", function (event) {
      try {
        var payload = JSON.parse(event.data);
        handler(payload.payload);
      } catch (error) {
        console.warn("SSE payload error:", error.message);
      }
    });
    return source;
  }

  var publicApi = {
    init: initialize,
    isConnected: function () {
      return !!lastHealth && lastHealth.status !== "unavailable";
    },
    renderModuleMeta: renderModuleMeta,
    renderStatusBanner: renderStatusBanner,
    subscribe: subscribe,
    getHealthEnvelope: createEnvelopeGetter("/health"),
    getDashboardEnvelope: createEnvelopeGetter("/dashboard"),
    getNetworkEnvelope: createEnvelopeGetter("/network"),
    getNodeHealthEnvelope: createEnvelopeGetter("/node-health"),
    getGovernanceEnvelope: createEnvelopeGetter("/governance"),
    getStakingEnvelope: createEnvelopeGetter("/staking"),
    getLedgerEnvelope: createEnvelopeGetter("/ledger"),
    getProofsEnvelope: createEnvelopeGetter("/proofs"),
    getPresaleEnvelope: createEnvelopeGetter("/presale"),
    getGrantsEnvelope: createEnvelopeGetter("/grants"),
    getBountiesEnvelope: createEnvelopeGetter("/bounties"),
    getDealsEnvelope: createEnvelopeGetter("/deals"),
    getLeaderboardEnvelope: createEnvelopeGetter("/leaderboard"),
    getReservationsEnvelope: createEnvelopeGetter("/reservations"),
    getWhalesEnvelope: createEnvelopeGetter("/whales"),
    getTokenomicsEnvelope: createEnvelopeGetter("/tokenomics"),
    getBenchmarkEnvelope: async function (name, options) {
      var envelope = await requestEnvelope("/benchmarks/" + name, options);
      renderStatusBanner(envelope);
      return envelope;
    },
    getHealth: createDataGetter("/health"),
    getDashboardData: createDataGetter("/dashboard"),
    getNetworkStats: createDataGetter("/network"),
    getNodeHealth: createDataGetter("/node-health"),
    getGovernanceData: createDataGetter("/governance"),
    getStakingStats: createDataGetter("/staking"),
    getLedgerData: createDataGetter("/ledger"),
    getProofsData: createDataGetter("/proofs"),
    getPresaleData: createDataGetter("/presale"),
    getGrantsData: createDataGetter("/grants"),
    getBountiesData: createDataGetter("/bounties"),
    getDealsData: createDataGetter("/deals"),
    getLeaderboardData: createDataGetter("/leaderboard"),
    getReservationsData: createDataGetter("/reservations"),
    getWhalesData: createDataGetter("/whales"),
    getTokenomicsData: createDataGetter("/tokenomics"),
    getBlockNumber: async function (options) {
      var data = await publicApi.getDashboardData(options);
      return data.blockNumber;
    },
    getGasPrice: async function (options) {
      var data = await publicApi.getDashboardData(options);
      return data.gasPriceGwei ? data.gasPriceGwei * 1000000000 : null;
    },
    getTokenData: async function (options) {
      var data = await publicApi.getDashboardData(options);
      return data.token;
    },
    getFundingData: async function (options) {
      var data = await publicApi.getDashboardData(options);
      return data.funding;
    },
    submitReservation: async function (payload) {
      return requestEnvelope("/reservations", {
        method: "POST",
        body: payload,
      });
    },
    submitForm: async function (type, payload) {
      return requestEnvelope("/forms/" + type, {
        method: "POST",
        body: payload,
      });
    },
  };

  if (typeof document !== "undefined") {
    document.addEventListener("DOMContentLoaded", function () {
      initialize().catch(function (error) {
        renderStatusBanner({
          status: "unavailable",
          source: "site-client",
          lastUpdated: new Date().toISOString(),
          staleReason: error.message,
        });
      });
    });
  }

  if (typeof module !== "undefined" && module.exports) {
    module.exports = publicApi;
  } else {
    global.X3API = publicApi;
  }
})(typeof window !== "undefined" ? window : globalThis);
