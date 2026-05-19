(function (global) {
  "use strict";

  var tokenomicsChart = null;
  var whaleFilter = "all";

  function byId(id) {
    return document.getElementById(id);
  }

  function query(selector) {
    return document.querySelector(selector);
  }

  function queryAll(selector) {
    return Array.from(document.querySelectorAll(selector));
  }

  function setText(selectorOrElement, value) {
    var element =
      typeof selectorOrElement === "string"
        ? document.querySelector(selectorOrElement)
        : selectorOrElement;
    if (!element || value === null || value === undefined) return;
    element.textContent = value;
  }

  function setHtml(selectorOrElement, value) {
    var element =
      typeof selectorOrElement === "string"
        ? document.querySelector(selectorOrElement)
        : selectorOrElement;
    if (!element || value === null || value === undefined) return;
    element.innerHTML = value;
  }

  function getNested(obj, path) {
    if (!obj || !path) return undefined;
    var parts = Array.isArray(path) ? path : String(path).split(".");
    var cur = obj;
    for (var i = 0; i < parts.length; i++) {
      if (cur == null) return undefined;
      cur = cur[parts[i]];
    }
    return cur;
  }

  function pickFirst(obj, keys) {
    for (var i = 0; i < keys.length; i++) {
      var key = keys[i];
      var value = Array.isArray(key) ? getNested(obj, key) : getNested(obj, key);
      if (value !== undefined && value !== null) return value;
    }
    return undefined;
  }

  var debugState = {
    enabled: false,
    entries: [],
    payloads: {},
  };

  function addDebug(section, label, value) {
    if (!debugState.enabled) return;
    debugState.entries.push({
      section: section,
      label: label,
      value: value !== undefined && value !== null ? value : "--",
    });
  }

  function ensureDebugFooter() {
    if (!debugState.enabled) return null;
    if (!document || !document.body) return null;
    var existing = byId("x3-debug-footer");
    if (existing) return existing;
    var footer = document.createElement("div");
    footer.id = "x3-debug-footer";
    footer.setAttribute(
      "style",
      [
        "position:fixed",
        "bottom:12px",
        "right:12px",
        "z-index:9999",
        "background:rgba(8,12,16,0.92)",
        "border:1px solid rgba(255,255,255,0.12)",
        "border-radius:12px",
        "padding:10px 12px",
        "font-family:monospace",
        "font-size:10px",
        "color:#d0d6dc",
        "max-width:320px",
        "max-height:260px",
        "overflow:auto",
        "box-shadow:0 12px 30px rgba(0,0,0,0.35)",
      ].join(";"),
    );
    footer.innerHTML = "<div style=\"font-weight:700;margin-bottom:6px;\">X3 Contract Debug</div>";
    document.body.appendChild(footer);
    return footer;
  }

  function renderDebugFooter() {
    var footer = ensureDebugFooter();
    if (!footer) return;
    var payloadHtml = "";
    var payloadKeys = Object.keys(debugState.payloads || {});
    if (payloadKeys.length) {
      payloadHtml =
        "<div style=\"margin-top:8px;border-top:1px solid rgba(255,255,255,0.08);padding-top:8px;\">" +
        "<div style=\"font-weight:700;margin-bottom:6px;\">Raw Payloads</div>" +
        payloadKeys
          .map(function (key) {
            var payload = debugState.payloads[key];
            var json = safeJson(payload);
            return (
              "<details style=\"margin-bottom:6px;\">" +
              "<summary style=\"cursor:pointer;color:#a7f3d0;\">" +
              escapeHtml(key) +
              "</summary>" +
              "<pre style=\"white-space:pre-wrap;word-break:break-word;margin:6px 0 0;color:#cbd5f5;\">" +
              escapeHtml(json) +
              "</pre>" +
              "</details>"
            );
          })
          .join("") +
        "</div>";
    }
    var content =
      "<div style=\"display:grid;gap:6px;\">" +
      debugState.entries
        .map(function (entry) {
          return (
            "<div>" +
            "<span style=\"color:#7dd3fc;\">[" +
            entry.section +
            "]</span> " +
            entry.label +
            ": <span style=\"color:#fbbf24;\">" +
            entry.value +
            "</span></div>"
          );
        })
        .join("") +
      "</div>";
    footer.innerHTML =
      "<div style=\"font-weight:700;margin-bottom:6px;\">X3 Contract Debug</div>" +
      content +
      payloadHtml;
  }

  function escapeHtml(input) {
    return String(input)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/\"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  function safeJson(value) {
    var json;
    try {
      json = JSON.stringify(value, null, 2);
    } catch (err) {
      json = String(value);
    }
    var max = 6000;
    if (json.length > max) {
      json = json.slice(0, max) + "\n…(truncated)";
    }
    return json;
  }

  function wrapApiForDebug(api) {
    if (!debugState.enabled || !api) return;
    if (api.__x3DebugWrapped) return;
    Object.keys(api).forEach(function (key) {
      if (typeof api[key] !== "function") return;
      if (!/Envelope$/.test(key)) return;
      var original = api[key];
      api[key] = async function () {
        var result = await original.apply(api, arguments);
        debugState.payloads[key] = result;
        renderDebugFooter();
        return result;
      };
    });
    if (typeof api.subscribe === "function") {
      var originalSubscribe = api.subscribe;
      api.subscribe = function (topic, handler) {
        var wrapped = handler;
        if (typeof handler === "function") {
          wrapped = function (payload) {
            debugState.payloads["stream:" + topic] = payload;
            renderDebugFooter();
            return handler(payload);
          };
        }
        return originalSubscribe.call(api, topic, wrapped);
      };
    }
    api.__x3DebugWrapped = true;
  }

  function initDebugMode() {
    if (typeof window === "undefined") return;
    if (window.location && /[?&]x3debug=1/.test(window.location.search)) {
      debugState.enabled = true;
      window.__x3Debug = debugState;
    }
  }

  function normalizeBonus(data) {
    var bonus = pickFirst(data, [
      "bonusPct",
      "bonusPercent",
      "bonus_pct",
      "bonus_percent",
      "bonusRate",
      "bonus_rate",
      "roundBonusPct",
      "round_bonus_pct",
      "bonus.pct",
      "bonus.percent",
      "bonus.rate",
    ]);
    var label = pickFirst(data, [
      "bonusLabel",
      "bonus_label",
      "bonus.label",
      "roundBonusLabel",
      "round_bonus_label",
    ]);
    addDebug("presale", "bonusPct", bonus);
    addDebug("presale", "bonusLabel", label);
    return { pct: bonus, label: label };
  }

  function normalizeFeeShare(data) {
    var pct = pickFirst(data, [
      "feeSharePct",
      "fee_share_pct",
      "protocolFeeSharePct",
      "protocol_fee_share_pct",
      "feeShare.percent",
      "feeShare.pct",
      "fee_share.percent",
      "fee_share.pct",
    ]);
    var label = pickFirst(data, [
      "feeShareLabel",
      "fee_share_label",
      "protocolFeeShareLabel",
      "protocol_fee_share_label",
      "feeShare.label",
      "fee_share.label",
      "feeShare",
      "fee_share",
      "protocolFeeShare",
      "protocol_fee_share",
    ]);
    addDebug("presale", "feeSharePct", pct);
    addDebug("presale", "feeShareLabel", label);
    return { pct: pct, label: label };
  }

  function normalizeVotingPower(data) {
    var total = pickFirst(data, [
      "votingPowerTotal",
      "voting_power_total",
      "votingPower",
      "voting_power",
      "vp.total",
      "vpTotal",
      "vp_total",
      "stats.votingPower",
    ]);
    var staking = pickFirst(data, [
      "votingPowerStaking",
      "voting_power_staking",
      "vp.staking",
    ]);
    var node = pickFirst(data, ["votingPowerNode", "voting_power_node", "vp.node"]);
    var delegated = pickFirst(data, [
      "votingPowerDelegated",
      "voting_power_delegated",
      "vp.delegated",
    ]);
    var rank = pickFirst(data, ["votingPowerRank", "voting_power_rank", "vp.rank"]);
    var unit = pickFirst(data, [
      "votingPowerUnit",
      "voting_power_unit",
      "vp.unit",
      "vpUnit",
      "unit",
    ]);
    addDebug("governance", "votingPowerTotal", total);
    addDebug("governance", "votingPowerUnit", unit);
    addDebug("governance", "votingPowerStaking", staking);
    addDebug("governance", "votingPowerNode", node);
    addDebug("governance", "votingPowerDelegated", delegated);
    addDebug("governance", "votingPowerRank", rank);
    return {
      total: total,
      staking: staking,
      node: node,
      delegated: delegated,
      rank: rank,
      unit: unit || "veX3S",
    };
  }

  function fmtMoney(value) {
    return "$" + Number(value || 0).toLocaleString("en-US");
  }

  function fmtCompactMoney(value) {
    var amount = Number(value || 0);
    if (amount >= 1000000) return "$" + (amount / 1000000).toFixed(1) + "M";
    if (amount >= 1000) return "$" + Math.round(amount / 1000) + "K";
    return fmtMoney(amount);
  }

  function fmtNumber(value) {
    return Number(value || 0).toLocaleString("en-US");
  }

  function fmtPct(value) {
    return Number(value || 0).toFixed(1) + "%";
  }

  function fmtX3S(value) {
    return fmtNumber(value) + " X3S";
  }

  function formatDateLabel() {
    return new Date().toLocaleDateString("en-US", {
      month: "long",
      day: "numeric",
      year: "numeric",
    }).toUpperCase();
  }

  function heartbeatClass(status) {
    if (status === "warning") return "pd-warn";
    if (status === "error") return "pd-err";
    return "pd-ok";
  }

  function cardStatusClass(status) {
    if (status === "warning") return "warn";
    if (status === "error") return "err";
    return "ok";
  }

  function tierClass(tier) {
    if (tier === "genesis") return "nt-genesis";
    if (tier === "star") return "nt-star";
    return "nt-lite";
  }

  function tierAccent(tier) {
    if (tier === "genesis") return "var(--gold)";
    if (tier === "star") return "var(--blue, var(--cyan, #00C8FF))";
    return "rgba(255,255,255,0.5)";
  }

  function governanceVoteTotal(proposal) {
    return Number((proposal && proposal.votesFor) || 0) + Number((proposal && proposal.votesAgainst) || 0);
  }

  function governanceSupportPct(proposal) {
    var total = governanceVoteTotal(proposal);
    return total > 0 ? Math.round((Number(proposal.votesFor || 0) / total) * 100) : 0;
  }

  function mountTopNav() {
    if (typeof document === "undefined") return;
    if (document.getElementById("x3-topnav")) return;

    var style = document.createElement("style");
    style.textContent =
      ".x3-topnav{position:fixed;left:0;right:0;top:var(--x3-status-height,0px);z-index:2000;font-family:JetBrains Mono,monospace;background:rgba(6,10,20,0.92);backdrop-filter:blur(18px);border-bottom:1px solid rgba(255,255,255,0.08);}" +
      ".x3-topnav-inner{display:flex;align-items:center;gap:16px;padding:12px 20px;max-width:1200px;margin:0 auto;}" +
      ".x3-topnav-logo{font-weight:800;letter-spacing:0.12em;font-size:12px;text-decoration:none;color:#fff;}" +
      ".x3-topnav-toggle{display:none;background:rgba(255,255,255,0.08);border:1px solid rgba(255,255,255,0.16);color:#fff;padding:8px 12px;border-radius:8px;font-size:11px;letter-spacing:0.12em;text-transform:uppercase;}" +
      ".x3-topnav-links{display:flex;gap:12px;align-items:center;flex:1;}" +
      ".x3-menu{position:relative;}" +
      ".x3-menu-btn{background:transparent;border:0;color:rgba(255,255,255,0.72);font-size:11px;letter-spacing:0.12em;text-transform:uppercase;padding:10px 12px;border-radius:8px;cursor:pointer;}" +
      ".x3-menu-btn:hover,.x3-menu:hover .x3-menu-btn{color:#fff;background:rgba(255,255,255,0.06);}" +
      ".x3-menu-panel{position:absolute;top:38px;left:0;min-width:220px;background:rgba(8,12,24,0.98);border:1px solid rgba(255,255,255,0.08);border-radius:12px;padding:10px;display:none;flex-direction:column;gap:6px;box-shadow:0 18px 45px rgba(0,0,0,0.45);}" +
      ".x3-menu-panel a{color:rgba(255,255,255,0.7);text-decoration:none;padding:8px 10px;border-radius:8px;font-size:11px;letter-spacing:0.08em;text-transform:uppercase;}" +
      ".x3-menu-panel a:hover,.x3-menu-panel a.active{color:#fff;background:rgba(0,212,255,0.12);}" +
      ".x3-menu:hover .x3-menu-panel,.x3-menu:focus-within .x3-menu-panel{display:flex;}" +
      ".x3-topnav-actions{display:flex;align-items:center;gap:10px;}" +
      ".x3-topnav-cta{background:linear-gradient(135deg,#00D4FF,#4B72FF);color:#06121f;text-decoration:none;padding:10px 16px;border-radius:10px;font-size:11px;font-weight:800;letter-spacing:0.14em;text-transform:uppercase;}" +
      ".x3-topnav-secondary{border:1px solid rgba(255,255,255,0.16);color:#fff;text-decoration:none;padding:9px 14px;border-radius:10px;font-size:11px;letter-spacing:0.12em;text-transform:uppercase;}" +
      "body.x3-has-topnav{padding-top:calc(62px + var(--x3-status-height,0px));}" +
      "body.x3-topnav-open .x3-topnav-links{display:flex;flex-direction:column;align-items:flex-start;padding:12px 0;}" +
      "body.x3-topnav-open .x3-menu-panel{position:static;display:flex;box-shadow:none;border-radius:10px;margin-left:10px;}" +
      "@media (max-width: 980px){.x3-topnav-inner{flex-wrap:wrap;}.x3-topnav-toggle{display:inline-flex;}.x3-topnav-links{display:none;width:100%;}.x3-topnav-actions{margin-left:auto;}}";
    document.head.appendChild(style);

    var nav = document.createElement("nav");
    nav.id = "x3-topnav";
    nav.className = "x3-topnav";
    nav.innerHTML =
      '<div class="x3-topnav-inner">' +
      '<a class="x3-topnav-logo" href="x3star-landing.html">X3STAR</a>' +
      '<button class="x3-topnav-toggle" type="button" aria-expanded="false">Menu</button>' +
      '<div class="x3-topnav-links">' +
      buildTopNavMenus() +
      "</div>" +
      '<div class="x3-topnav-actions">' +
      '<a class="x3-topnav-secondary" href="x3star-dashboard.html">Dashboard</a>' +
      '<a class="x3-topnav-cta" href="x3star-token-presale.html">Buy X3S</a>' +
      "</div>" +
      "</div>";

    document.body.insertBefore(nav, document.body.firstChild);
    document.body.classList.add("x3-has-topnav");
    hideLegacyNavs();
    wireTopNav(nav);
  }

  function buildTopNavMenus() {
    var menus = [
      {
        label: "Network",
        items: [
          { label: "Dashboard", href: "x3star-dashboard.html" },
          { label: "Network Pulse", href: "x3star-network-pulse.html" },
          { label: "Node Health", href: "x3star-node-health.html" },
          { label: "Operator War Room", href: "x3star-operator-war-room.html" },
          { label: "Whale Tracker", href: "x3star-whale-tracker.html" },
          { label: "Tokenomics War Room", href: "x3star-tokenomics-warroom.html" },
          { label: "Ecosystem Heartbeat", href: "x3star-ecosystem-heartbeat.html" },
          { label: "Mission Terminal", href: "x3star-mission-terminal.html" },
        ],
      },
      {
        label: "Token",
        items: [
          { label: "Token Presale", href: "x3star-token-presale.html" },
          { label: "Validator Presale", href: "x3star-validator-presale.html" },
          { label: "Slot Tracker", href: "x3star-slot-tracker.html" },
          { label: "Scarcity Clock", href: "x3star-scarcity-clock.html" },
          { label: "Fundraise Thermometer", href: "x3star-fundraise-thermometer.html" },
          { label: "Staking", href: "x3star-staking.html" },
          { label: "Governance", href: "x3star-governance.html" },
          { label: "ROI Calculator", href: "x3star-roi-calculator.html" },
          { label: "If You Had", href: "x3star-if-you-had.html" },
          { label: "If You Invested", href: "x3star-if-you-invested.html" },
          { label: "Portfolio", href: "x3star-portfolio.html" },
        ],
      },
      {
        label: "Proof",
        items: [
          { label: "Proof Wall", href: "x3star-proof-wall.html" },
          { label: "Transparency Ledger", href: "x3star-transparency-ledger.html" },
          { label: "Social Proof Wall", href: "x3star-social-proof-wall.html" },
          { label: "Hall of Fame", href: "x3star-hall-of-fame.html" },
          { label: "Leaderboard Arena", href: "x3star-leaderboard-arena.html" },
        ],
      },
      {
        label: "Business",
        items: [
          { label: "Investor Relations", href: "x3star-investor-relations.html" },
          { label: "KYC Onboarding", href: "x3star-kyc-onboarding.html" },
          { label: "Affiliate Program", href: "x3star-affiliate.html" },
          { label: "Grant Hub", href: "x3star-grant-hub.html" },
          { label: "Grant Mission Control", href: "x3star-grant-mission-control.html" },
          { label: "Bounty Board", href: "x3star-bounty-board.html" },
          { label: "Barter Exchange", href: "x3star-barter-exchange.html" },
        ],
      },
      {
        label: "Intel",
        items: [
          { label: "Arbitrage Engine", href: "x3star-arbitrage-engine.html" },
          { label: "The Spine", href: "x3star-spine.html" },
          { label: "Architecture Deep Dive", href: "x3star-tech-deep-dive.html" },
          { label: "Compute Marketplace", href: "x3star-compute-marketplace.html" },
          { label: "Benchmark Hub", href: "x3star-benchmark-page.html" },
          { label: "Competitor Annihilation", href: "x3star-competitor-annihilation.html" },
          { label: "Competitor Graveyard", href: "x3star-competitor-graveyard.html" },
          { label: "Whitepaper", href: "x3star-whitepaper.html" },
          { label: "Chainbench Pro", href: "chainbench-pro.html" },
          { label: "Chainbench Ultimate", href: "chainbench-ultimate.html" },
          { label: "Stress Test", href: "blockchain-stress-test.html" },
        ],
      },
    ];

    return menus
      .map(function (menu) {
        return (
          '<div class="x3-menu">' +
          '<button class="x3-menu-btn" type="button">' +
          menu.label +
          "</button>" +
          '<div class="x3-menu-panel">' +
          menu.items
            .map(function (item) {
              return '<a data-page="' + item.href + '" href="' + item.href + '">' + item.label + "</a>";
            })
            .join("") +
          "</div>" +
          "</div>"
        );
      })
      .join("");
  }

  function hideLegacyNavs() {
    var navs = Array.from(document.querySelectorAll("nav"));
    navs.forEach(function (nav) {
      if (nav.id === "x3-topnav") return;
      if (nav.classList.contains("sidebar")) return;
      nav.style.display = "none";
    });
  }

  function wireTopNav(nav) {
    var toggle = nav.querySelector(".x3-topnav-toggle");
    if (toggle) {
      toggle.addEventListener("click", function () {
        var isOpen = document.body.classList.toggle("x3-topnav-open");
        toggle.setAttribute("aria-expanded", isOpen ? "true" : "false");
      });
    }

    var page = (global.location.pathname.split("/").pop() || "x3star-landing.html").toLowerCase();
    var links = nav.querySelectorAll(".x3-menu-panel a");
    links.forEach(function (link) {
      if (link.getAttribute("data-page") === page) {
        link.classList.add("active");
      }
    });
  }

  function countdown(closesAt, ids) {
    function tick() {
      var diff = Math.max(0, new Date(closesAt).getTime() - Date.now());
      var days = Math.floor(diff / 86400000);
      var hours = Math.floor((diff % 86400000) / 3600000);
      var minutes = Math.floor((diff % 3600000) / 60000);
      var seconds = Math.floor((diff % 60000) / 1000);
      setText(ids.days, String(days).padStart(2, "0"));
      setText(ids.hours, String(hours).padStart(2, "0"));
      setText(ids.minutes, String(minutes).padStart(2, "0"));
      setText(ids.seconds, String(seconds).padStart(2, "0"));
    }
    tick();
    return global.setInterval(tick, 1000);
  }

  function renderReservationsHeatmap(target, buckets) {
    if (!target) return;
    target.innerHTML = "";
    var maxBucket = Math.max(1, Math.max.apply(null, buckets || [0]));
    (buckets || []).forEach(function (count) {
      var cell = document.createElement("div");
      cell.className = "hm-cell";
      var intensity = count / maxBucket;
      if (target.classList.contains("heatmap")) {
        cell.style.background =
          intensity > 0
            ? "rgba(200,149,10," + (0.15 + intensity * 0.75).toFixed(2) + ")"
            : "rgba(26,18,8,0.08)";
      }
      target.appendChild(cell);
    });
  }

  function renderReservationsCards(target, cards) {
    if (!target) return;
    target.innerHTML = "";
    cards.forEach(function (cardData) {
      var card = document.createElement("div");
      var typeClass = cardData.type === "node" ? "pct-node" : "pct-token";
      var validatorClass = cardData.type === "node" ? " validator-card" : "";
      card.className = "proof-card" + validatorClass;
      card.innerHTML =
        '<div class="pc-stripe" style="background:' +
        (cardData.type === "node" ? "#1A6B3A" : "#C8950A") +
        '"></div><div class="pc-top"><div class="pc-flag">' +
        cardData.flag +
        '</div><div class="pc-type ' +
        typeClass +
        '">' +
        (cardData.type === "node" ? "VALIDATOR NODE" : "TOKEN") +
        '</div></div><div class="pc-name">' +
        cardData.name +
        '</div><div class="pc-loc">' +
        cardData.location +
        '</div><div class="pc-amount" style="color:' +
        (cardData.type === "node" ? "#3EE83E" : "#C8950A") +
        '">' +
        fmtMoney(cardData.amountUsd) +
        '</div><div class="pc-detail">' +
        cardData.detail +
        '</div><div class="pc-time">' +
        cardData.timeAgo +
        "</div>";
      target.appendChild(card);
    });
  }

  function renderTopInvestors(target, investors) {
    if (!target) return;
    target.innerHTML = "";
    investors.forEach(function (investor) {
      var row = document.createElement("tr");
      row.innerHTML =
        '<td class="it-rank">#' +
        investor.rank +
        '</td><td class="it-flag">' +
        investor.flag +
        '</td><td class="it-name">' +
        investor.name +
        '<div class="it-badge ' +
        (investor.badge === "WHALE" ? "ib-whale" : "ib-node") +
        '">' +
        investor.badge +
        '</div></td><td class="it-amount">' +
        fmtCompactMoney(investor.amountUsd) +
        "</td>";
      target.appendChild(row);
    });
  }

  async function initDashboard(api) {
    async function load() {
      var payloads = await Promise.all([
        api.getDashboardEnvelope({ refresh: true }),
        api.getReservationsEnvelope({ refresh: true }),
        api.getPresaleEnvelope({ refresh: true }),
        api.getTokenomicsEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var reservationsEnvelope = payloads[1];
      var presaleEnvelope = payloads[2];
      var tokenomicsEnvelope = payloads[3];
      var data = envelope.data;
      var presale = presaleEnvelope.data || {};
      var tokenomics = tokenomicsEnvelope.data || {};
      setText("#block-num", data.blockNumber ? fmtNumber(data.blockNumber) : "unavailable");
      setText("#gas", data.gasPriceGwei ? data.gasPriceGwei + " gwei" : "unavailable");
      setText("#kpi-updated", new Date(envelope.lastUpdated).toLocaleString());
      setText("#kpi-raised", fmtCompactMoney(data.funding.raised));
      setText("#kpi-grants", fmtNumber(data.funding.activeGrants));
      setText("#kpi-investors", fmtNumber(data.funding.investorCount));
      setText("#kpi-price", "$" + Number(data.token.priceUsd || 0).toFixed(4));
      setText("#kpi-raised-delta", "—");
      setText("#kpi-grants-delta", "—");
      setText("#kpi-investors-delta", "—");
      var priceDelta = Number(data.token.priceChange24h || 0);
      var priceDeltaEl = byId("kpi-price-delta");
      if (priceDeltaEl) {
        priceDeltaEl.textContent = (priceDelta >= 0 ? "+" : "") + priceDelta.toFixed(1) + "%";
        priceDeltaEl.className = priceDelta >= 0 ? "badge-up" : "badge-dn";
      }
      setText("#donut-total", fmtCompactMoney(data.funding.raised));
      setText("#alloc-lead", "--");
      setText("#alloc-grants", "--");
      setText("#alloc-community", "--");
      setText("#alloc-treasury", "--");
      setText("#token-mcap", fmtCompactMoney(data.token.marketCapUsd));
      setText("#token-circ", (data.token.circulatingSupply / 1000000).toFixed(0) + "M");
      setText("#token-vol", fmtCompactMoney(data.token.volume24hUsd));
      setText("#token-holders", fmtNumber(data.token.holders));
      var feed = byId("activity-feed");
      if (feed) {
        feed.innerHTML = "";
        reservationsEnvelope.data.reservations.slice(0, 4).forEach(function (reservation) {
          var row = document.createElement("div");
          row.style.cssText = "display:flex;align-items:flex-start;gap:8px";
          row.innerHTML =
            '<span style="font-size:13px;flex-shrink:0">✅</span><div style="flex:1"><div style="font-size:11px;line-height:1.4">' +
            reservation.name +
            " reserved <b class=\"text-gold\">" +
            fmtMoney(reservation.amountUsd) +
            '</b></div><div style="font-size:10px;color:var(--muted);margin-top:2px">' +
            reservation.timeAgo +
            "</div></div>";
          feed.appendChild(row);
        });
      }
      var investorList = byId("investor-list");
      if (investorList) {
        investorList.innerHTML = "";
        (reservationsEnvelope.data.topInvestors || []).slice(0, 6).forEach(function (investor) {
          var row = document.createElement("div");
          row.className = "investor-item";
          row.innerHTML =
            '<div class="inv-avatar" style="background:linear-gradient(135deg,#4a90ff,#2040c0)">' +
            String(investor.name || "?").slice(0, 1).toUpperCase() +
            '</div><div><div class="inv-name">' +
            investor.name +
            '</div><div class="inv-type">' +
            investor.badge +
            '</div></div><div class="inv-commit">' +
            fmtCompactMoney(investor.amountUsd) +
            '</div><span class="inv-tier tier-seed">LIVE</span>';
          investorList.appendChild(row);
        });
        if (!investorList.children.length) {
          investorList.innerHTML = '<div style="font-size:11px;color:var(--muted)">Awaiting live investor feed.</div>';
        }
      }

      var grantsBody = byId("grants-body");
      if (grantsBody) {
        grantsBody.innerHTML = "";
        var grants = Array.isArray(data.grants) ? data.grants : [];
        grants.slice(0, 4).forEach(function (grant) {
          var name =
            grant.name ||
            grant.projectName ||
            grant.title ||
            grant.organization ||
            grant.team ||
            "Grant";
          var amount =
            grant.amountUsd ||
            grant.fundingRequestedUsd ||
            grant.requestedUsd ||
            grant.amount ||
            0;
          var progress =
            grant.progressPct ||
            grant.progress ||
            grant.milestonePct ||
            grant.completionPct ||
            null;
          var status = grant.status || grant.state || grant.stage || "pending";
          var pctValue =
            progress === null || progress === undefined
              ? null
              : Math.max(0, Math.min(100, Number(progress)));
          var row = document.createElement("tr");
          row.innerHTML =
            "<td>" +
            escapeHtml(name) +
            "</td><td>" +
            fmtCompactMoney(amount) +
            '</td><td><div class="metric-bar-wrap"><div class="prog-bar" style="height:6px;width:' +
            (pctValue == null ? 0 : pctValue) +
            '%;background:var(--cyan)"></div></div><div style="font-size:9px;color:var(--muted);margin-top:4px">' +
            (pctValue == null ? "--" : pctValue + "%") +
            '</div></td><td><span class="badge-up">' +
            String(status).toUpperCase() +
            "</span></td>";
          grantsBody.appendChild(row);
        });
        if (!grantsBody.children.length) {
          grantsBody.innerHTML =
            '<tr><td colspan="4" style="color:var(--muted);font-size:11px;padding:16px 0;">Awaiting live grant feed.</td></tr>';
        }
      }

      var tickerPrice = byId("ticker");
      var tickerChange = byId("ticker-change");
      if (tickerPrice && tickerChange) {
        tickerPrice.querySelector(".ticker-price").textContent = "$" + Number(data.token.priceUsd || 0).toFixed(4);
        tickerChange.textContent = (priceDelta >= 0 ? "▲ +" : "▼ ") + Math.abs(priceDelta).toFixed(1) + "%";
        tickerChange.className = priceDelta >= 0 ? "ticker-up" : "ticker-dn";
      }

      var alertText = byId("alert-text");
      if (alertText) {
        var remainingUsd = Math.max(0, Number(presale.hardCapUsd || 0) - Number(presale.raisedUsd || 0));
        alertText.innerHTML =
          "<strong>Round " +
          (presale.currentRound || "Prefunding") +
          "</strong> closes in " +
          (presale.daysRemaining || "--") +
          " days — " +
          fmtCompactMoney(remainingUsd) +
          " allocation remaining.";
      }

      var hardCap = Number(presale.hardCapUsd || 0);
      var raised = Number(presale.raisedUsd || 0);
      var softCap = Number(presale.softCapUsd || 0);
      var pctRaised = hardCap ? Math.min(100, (raised / hardCap) * 100) : 0;
      setText("#round-hardcap", fmtCompactMoney(hardCap));
      setText("#round-progress-text", pctRaised.toFixed(1) + "% filled • " + fmtCompactMoney(raised) + " raised");
      if (byId("round-progress")) byId("round-progress").style.width = pctRaised.toFixed(1) + "%";
      var softCapPct = softCap ? Math.min(100, (raised / softCap) * 100) : 0;
      if (byId("round-softcap-bar")) byId("round-softcap-bar").style.width = softCapPct.toFixed(1) + "%";
      setText("#round-softcap", raised >= softCap && softCap > 0 ? "✓ HIT" : fmtCompactMoney(softCap));
      var minTicket = Math.min.apply(null, (presale.tiers || []).map(function (tier) { return tier.priceUsd; }));
      var bonusMeta = normalizeBonus(presale);
      setText("#round-min-ticket", isFinite(minTicket) ? fmtMoney(minTicket) : "--");
      setText("#round-vesting", presale.vesting || presale.vestingSchedule || "--");
      setText("#round-price", presale.tokenPriceUsd ? "$" + Number(presale.tokenPriceUsd).toFixed(3) : "--");
      setText(
        "#round-bonus",
        bonusMeta.pct != null ? "+" + bonusMeta.pct + "%" : bonusMeta.label || "--",
      );

      var network = data.network || {};
      setText("#net-tps", fmtNumber(network.tps || 0));
      setText("#net-vals", fmtNumber(network.validators || 0));
      setText("#net-uptime", network.uptime ? fmtPct(network.uptime) : "--");
      setText("#net-finality", network.finality ? network.finality + "s" : "--");
      if (byId("net-tps-bar")) byId("net-tps-bar").style.width = Math.min(100, ((network.tps || 0) / 5000) * 100) + "%";
      if (byId("net-vals-bar")) byId("net-vals-bar").style.width = Math.min(100, ((network.validators || 0) / 2000) * 100) + "%";
      if (byId("net-uptime-bar")) byId("net-uptime-bar").style.width = Math.min(100, Number(network.uptime || 0)) + "%";
      if (byId("net-finality-bar")) byId("net-finality-bar").style.width = Math.min(100, ((network.finality || 0) / 1) * 100) + "%";

      if (global.__x3FundingChart) {
        global.__x3FundingChart.data.labels = ["Now"];
        global.__x3FundingChart.data.datasets[0].data = [Number((raised / 1000000).toFixed(2))];
        global.__x3FundingChart.data.datasets[1].data = [Number((hardCap / 1000000).toFixed(2))];
        global.__x3FundingChart.update();
      }
      if (global.__x3TokenChart) {
        global.__x3TokenChart.data.labels = ["Now"];
        global.__x3TokenChart.data.datasets[0].data = [Number(data.token.priceUsd || 0)];
        global.__x3TokenChart.update();
      }
      if (global.__x3AllocChart) {
        var allocations = tokenomics.allocations || [];
        var total = allocations.reduce(function (sum, entry) { return sum + Number(entry.amountX3S || 0); }, 0) || 0;
        if (total > 0) {
          var pct = allocations.slice(0, 4).map(function (entry) {
            return Math.round((Number(entry.amountX3S || 0) / total) * 100);
          });
          while (pct.length < 4) pct.push(0);
          global.__x3AllocChart.data.datasets[0].data = pct;
          setText("#alloc-lead", pct[0] + "%");
          setText("#alloc-grants", pct[1] + "%");
          setText("#alloc-community", pct[2] + "%");
          setText("#alloc-treasury", pct[3] + "%");
          global.__x3AllocChart.update();
        }
      }

      api.renderModuleMeta(".page-head", "dashboard", envelope);
    }
    await load();
    global.setInterval(load, 15000);
  }

  async function initLanding(api) {
    async function load() {
      var payloads = await Promise.all([
        api.getDashboardEnvelope({ refresh: true }),
        api.getGovernanceEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var data = payloads[0].data;
      var governance = payloads[1].data;
      setText("#hm1", fmtCompactMoney(data.funding.raised));
      var heroSub = query(".hero-sub");
      if (heroSub) {
        heroSub.innerHTML =
          "Live network snapshot: <strong>" +
          fmtNumber(data.network.tps || 0) +
          " TPS</strong>, " +
          fmtNumber(data.network.validators || 0) +
          " validators, " +
          fmtNumber(data.token.holders || 0) +
          " token holders, and treasury visibility through the X3 site backend.";
      }
      var heroMetrics = queryAll(".hero-metrics .hm-val");
      if (heroMetrics[0]) setText(heroMetrics[0], fmtCompactMoney(data.funding.raised));
      if (heroMetrics[1]) setText(heroMetrics[1], fmtNumber(data.funding.investorCount || 0));
      if (heroMetrics[2]) setText(heroMetrics[2], "$" + Number(data.token.priceUsd || 0).toFixed(4));
      if (heroMetrics[3]) setText(heroMetrics[3], fmtNumber(Math.round(Number(data.token.totalSupply || 0) / 1000000)) + "M");
      if (heroMetrics[4]) setText(heroMetrics[4], fmtNumber(data.funding.activeGrants || 0));
      var traction = queryAll(".tr-num");
      if (traction[0]) setText(traction[0], fmtCompactMoney(data.funding.raised));
      if (traction[1]) setText(traction[1], fmtNumber(data.funding.investorCount || 0));
      if (traction[2]) setText(traction[2], fmtNumber(data.network.validators));
      if (traction[3]) setText(traction[3], fmtNumber(data.token.holders));
      if (traction[4]) setText(traction[4], fmtCompactMoney(governance.treasury));
      if (traction[5]) setText(traction[5], fmtNumber(data.funding.activeGrants || 0));
      var badge = query(".hero-badge");
      if (badge) {
        badge.innerHTML =
          '<div class="hb-pulse"></div>Round III Prefunding — ' +
          fmtCompactMoney(data.funding.raised) +
          " Raised — " +
          data.funding.daysRemaining +
          " Days Remaining";
      }
      var techCards = queryAll(".tech-card");
      if (techCards[0]) {
        setText(techCards[0].querySelector(".tc-number"), fmtNumber(data.network.tps || 0) + "TPS");
      }
      if (techCards[1]) {
        setText(techCards[1].querySelector(".tc-number"), (data.network.finality || 0.4) + "sec");
      }
      if (techCards[2]) {
        setText(techCards[2].querySelector(".tc-number"), (data.network.uptime || 0).toFixed(1) + "%");
        var uptimeDesc = techCards[2].querySelector("div[style*='font-size:13px']");
        if (uptimeDesc) uptimeDesc.textContent = "Across " + fmtNumber(data.network.validators || 0) + " observed peers";
      }
      if (techCards[4]) {
        setText(techCards[4].querySelector(".tc-number"), fmtNumber(data.network.validators || 0));
      }
      api.renderModuleMeta(".hero-badges", "landing", envelope);
    }
    await load();
    global.setInterval(load, 15000);
  }

  async function initGovernance(api) {
    async function load() {
      var envelope = await api.getGovernanceEnvelope({ refresh: true });
      var data = envelope.data;
      setText("#gov-total", data.proposalsCount);
      setText("#gov-passed", data.proposalsCount - data.activeProposals);
      setText("#gov-voters", fmtNumber(data.voters));
      setText("#gov-treasury", fmtCompactMoney(data.treasury));
      setText("#gov-treasury-total", fmtCompactMoney(data.treasury));
      var vp = normalizeVotingPower(data);
      setText("#voting-power", (vp.total != null ? fmtNumber(vp.total) : "--") + " " + vp.unit);
      setText("#gov-vp-total", vp.total != null ? fmtNumber(vp.total) : "--");
      setText("#gov-vp-staking", vp.staking != null ? fmtNumber(vp.staking) : "--");
      setText("#gov-vp-node", vp.node != null ? fmtNumber(vp.node) : "--");
      setText("#gov-vp-delegated", vp.delegated != null ? fmtNumber(vp.delegated) : "--");
      setText("#gov-vp-rank", vp.rank != null ? vp.rank : "--");

      var allocationSource =
        data.treasuryAllocations ||
        data.treasuryAllocation ||
        data.treasuryBreakdown ||
        data.treasury_allocations ||
        null;
      var allocationList = [];
      if (Array.isArray(allocationSource)) {
        allocationList = allocationSource;
      } else if (allocationSource && typeof allocationSource === "object") {
        allocationList = Object.keys(allocationSource).map(function (key) {
          return { label: key, amountUsd: allocationSource[key] };
        });
      }
      var allocationTotal = allocationList.reduce(function (sum, entry) {
        return sum + Number(entry.amountUsd || entry.amount || 0);
      }, 0);
      function findAllocation(names) {
        var entry = allocationList.find(function (item) {
          var label = String(item.label || item.name || "").toLowerCase();
          return names.some(function (name) { return label.indexOf(name) !== -1; });
        });
        if (!entry) return null;
        var amount = Number(entry.amountUsd || entry.amount || 0);
        var pct = allocationTotal ? Math.round((amount / allocationTotal) * 100) : 0;
        return { amount: amount, pct: pct };
      }
      var dev = findAllocation(["dev", "development"]);
      var grants = findAllocation(["grant"]);
      var marketing = findAllocation(["marketing", "growth", "community"]);
      var reserve = findAllocation(["reserve", "treasury", "stability"]);
      if (byId("treasury-dev")) setText("#treasury-dev", dev ? fmtCompactMoney(dev.amount) : "--");
      if (byId("treasury-dev-bar")) byId("treasury-dev-bar").style.width = dev ? dev.pct + "%" : "0%";
      if (byId("treasury-grants")) setText("#treasury-grants", grants ? fmtCompactMoney(grants.amount) : "--");
      if (byId("treasury-grants-bar")) byId("treasury-grants-bar").style.width = grants ? grants.pct + "%" : "0%";
      if (byId("treasury-marketing")) setText("#treasury-marketing", marketing ? fmtCompactMoney(marketing.amount) : "--");
      if (byId("treasury-marketing-bar")) byId("treasury-marketing-bar").style.width = marketing ? marketing.pct + "%" : "0%";
      if (byId("treasury-reserve")) setText("#treasury-reserve", reserve ? fmtCompactMoney(reserve.amount) : "--");
      if (byId("treasury-reserve-bar")) byId("treasury-reserve-bar").style.width = reserve ? reserve.pct + "%" : "0%";

      var delegateList = byId("delegate-list");
      if (delegateList) {
        delegateList.innerHTML = "";
        var delegates =
          data.delegates ||
          data.topDelegates ||
          data.delegateLeaders ||
          data.delegations ||
          [];
        if (!Array.isArray(delegates) || !delegates.length) {
          delegateList.textContent = "Awaiting delegate feed.";
        } else {
          delegates.slice(0, 6).forEach(function (delegate) {
            var row = document.createElement("div");
            row.className = "delegate-item";
            var name = delegate.name || delegate.label || delegate.wallet || delegate.id || "Delegate";
            var power = delegate.power || delegate.votingPower || delegate.votes || delegate.weight || "--";
            var avatarColor = delegate.color || "rgba(180,80,255,0.25)";
            row.innerHTML =
              '<div class="d-avatar" style="background:' +
              avatarColor +
              '">' +
              String(name).slice(0, 2).toUpperCase() +
              '</div><div class="d-name">' +
              escapeHtml(name) +
              '</div><div class="d-power">' +
              (typeof power === "number" ? fmtNumber(power) : power) +
              "</div>";
            delegateList.appendChild(row);
          });
        }
      }

      var list = byId("proposal-list");
      if (list) {
        list.innerHTML = "";
        (data.proposals || []).forEach(function (proposal) {
          var totalVotes = Number(proposal.votesFor || 0) + Number(proposal.votesAgainst || 0);
          var yesPct = totalVotes ? Math.round((proposal.votesFor / totalVotes) * 100) : 0;
          var noPct = totalVotes ? Math.round((proposal.votesAgainst / totalVotes) * 100) : 0;
          var status = String(proposal.status || "").toLowerCase();
          var statusClass =
            status === "active"
              ? "ps-active"
              : status === "executed"
                ? "ps-passed"
                : status === "rejected"
                  ? "ps-failed"
                  : "ps-pending";
          var endsAt = proposal.endsAt ? new Date(proposal.endsAt) : null;
          var daysLeft = endsAt ? Math.max(0, Math.ceil((endsAt - Date.now()) / 86400000)) : null;
          var statusLabel =
            status === "active"
              ? "🟢 ACTIVE — " + (daysLeft !== null ? daysLeft + " days left" : "in progress")
              : status === "executed"
                ? "✓ PASSED — Executed"
                : status === "rejected"
                  ? "✗ FAILED — Rejected"
                  : "⏳ PENDING";
          var card = document.createElement("div");
          card.className = "prop-card reveal";
          var desc = proposal.description || "Proposal details available in the governance feed.";
          card.innerHTML =
            '<div class="prop-header"><div><div class="prop-id">' +
            proposal.id +
            '</div></div><span class="prop-status ' +
            statusClass +
            '">' +
            statusLabel +
            '</span></div><div class="prop-title">' +
            proposal.title +
            '</div><div class="prop-desc">' +
            desc +
            '</div><div class="vote-bars"><div class="vb-row"><span class="vb-label yes">YES</span><div class="vb-bar-wrap"><div class="vb-bar" style="width:' +
            yesPct +
            '%;background:var(--green)"></div></div><span class="vb-pct" style="color:var(--green)">' +
            yesPct +
            '%</span></div><div class="vb-row"><span class="vb-label no">NO</span><div class="vb-bar-wrap"><div class="vb-bar" style="width:' +
            noPct +
            '%;background:var(--red)"></div></div><span class="vb-pct" style="color:var(--red)">' +
            noPct +
            '%</span></div></div><div class="prop-footer"><div class="prop-meta"><span>' +
            fmtNumber(totalVotes) +
            ' votes</span><span>Quorum: ' +
            fmtNumber(proposal.quorum || 0) +
            "</span></div></div>";
          list.appendChild(card);
        });
        if (!list.children.length) {
          list.innerHTML =
            '<div class="prop-card reveal"><div class="prop-header"><div><div class="prop-id">No proposals</div></div><span class="prop-status ps-pending">⏳ Pending</span></div><div class="prop-title">No proposals available yet.</div><div class="prop-desc">Governance proposals will populate once the live feed is connected.</div></div>';
        }
      }
      api.renderModuleMeta(".page-header", "governance", envelope);
    }
    await load();
    global.setInterval(load, 30000);
  }

  async function initNodeHealth(api) {
    async function load() {
      var envelope = await api.getNodeHealthEnvelope({ refresh: true });
      var data = envelope.data;
      var values = queryAll(".hrs-val");
      if (values[0]) setText(values[0], fmtNumber(data.activeValidators));
      if (values[1]) setText(values[1], fmtNumber(data.slashed));
      if (values[2]) setText(values[2], fmtNumber(data.warnings));
      if (values[3]) setText(values[3], data.uptime ? fmtPct(data.uptime) : "unavailable");
      if (values[4]) setText(values[4], fmtNumber(data.peers));
      var statusLight = query(".status-light");
      if (statusLight) {
        statusLight.className = "status-light " + (envelope.status === "live" ? "sl-green" : "sl-green");
        statusLight.innerHTML = '<div class="sl-dot"></div>' + (envelope.status === "live" ? "NETWORK HEALTHY" : "INDEXED SNAPSHOT");
      }
      var grid = byId("health-grid");
      if (grid) {
        grid.innerHTML = "";
        (data.nodes || []).forEach(function (node) {
          var card = document.createElement("div");
          card.className = "node-card " + cardStatusClass(node.status);
          card.innerHTML =
            '<div class="nc-head"><div class="nc-flag">' +
            node.flag +
            '</div><div class="nc-id"><div class="nc-name">' +
            node.operatorId +
            '</div><div class="nc-loc">' +
            node.location +
            '</div></div><div class="nc-tier ' +
            tierClass(node.tier) +
            '">' +
            String(node.tier).toUpperCase() +
            '</div><div class="pulse-dot ' +
            heartbeatClass(node.status) +
            '"></div></div><div class="nc-metrics"><div class="ncm"><div class="ncm-val" style="color:' +
            (node.status === "warning" ? "var(--gold)" : "var(--green)") +
            '">' +
            fmtPct(node.uptimePct) +
            '</div><div class="ncm-key">Uptime</div></div><div class="ncm"><div class="ncm-val" style="color:' +
            (node.latencyMs > 50 ? "var(--gold)" : "var(--green)") +
            '">' +
            node.latencyMs +
            'ms</div><div class="ncm-key">Latency</div></div><div class="ncm"><div class="ncm-val" style="color:var(--gold)">' +
            fmtX3S(node.stakeX3S) +
            '</div><div class="ncm-key">Stake</div></div></div><div class="nc-bars"><div class="ncb-row"><span class="ncb-label">Health</span><div class="ncb-track"><div class="ncb-fill" style="width:' +
            node.healthScore +
            '%;background:' +
            (node.status === "warning" ? "var(--gold)" : "var(--green)") +
            '"></div></div><span class="ncb-val" style="color:' +
            (node.status === "warning" ? "var(--gold)" : "var(--green)") +
            '">' +
            node.healthScore +
            '</span></div><div class="ncb-row"><span class="ncb-label">Peers</span><div class="ncb-track"><div class="ncb-fill" style="width:' +
            Math.min(100, node.peers * 2) +
            '%;background:var(--purple)"></div></div><span class="ncb-val" style="color:var(--purple)">' +
            node.peers +
            '</span></div><div class="ncb-row"><span class="ncb-label">TPS Share</span><div class="ncb-track"><div class="ncb-fill" style="width:' +
            Math.min(100, node.tps / 3) +
            '%;background:var(--blue)"></div></div><span class="ncb-val" style="color:var(--blue)">' +
            node.tps +
            '</span></div></div><div class="nc-footer"><div class="slash-badge ' +
            (node.status === "warning" ? "sb-warn" : "sb-clean") +
            '">' +
            (node.status === "warning" ? "WARNINGS" : "0 SLASHES") +
            '</div><div style="font-family:var(--rhm);font-size:10px;color:var(--blue)">' +
            node.name +
            '</div><div class="nc-time">' +
            node.heartbeatAge +
            "</div></div>";
          grid.appendChild(card);
        });
      }
      var barText = query(".bottom-bar .bb-text");
      if (barText) {
        barText.innerHTML =
          data.activeValidators +
          " indexed validators · " +
          fmtPct(data.uptime || 0) +
          " average uptime · " +
          data.warnings +
          " warnings in current telemetry snapshot";
      }
      api.renderModuleMeta(".header-row", "node health", envelope);
    }
    await load();
    global.setInterval(load, 15000);
  }

  async function initStaking(api) {
    async function load() {
      var payloads = await Promise.all([
        api.getStakingEnvelope({ refresh: true }),
        api.getDashboardEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var data = payloads[0].data;
      var dashboard = payloads[1].data;
      global.__x3StakingPools = data.pools || [];
      global.__x3TokenPriceUsd = dashboard.token.priceUsd || 0;
      var stats = queryAll(".ps-item .ps-val");
      if (stats[0]) setText(stats[0], fmtCompactMoney(data.totalValueLocked));
      if (stats[1]) setText(stats[1], fmtPct(data.avgApy));
      if (stats[2]) setText(stats[2], fmtNumber(data.totalStakers));
      if (stats[3]) setText(stats[3], "$" + Math.round(Number(data.dailyRewards) / 1000) + "K");
      if (stats[4]) setText(stats[4], Math.round(Number(data.totalStaked) / 1000000) + "M");
      var cards = queryAll(".pool-card");
      data.pools.forEach(function (pool, index) {
        var card = cards[index];
        if (!card) return;
        var tvl = card.querySelector(".pstat-val");
        var apy = card.querySelector(".pool-apy-val");
        if (tvl) setText(tvl, fmtCompactMoney(pool.tvlUsd));
        if (apy) setHtml(apy, pool.apy + '% <span style="font-size:14px;opacity:0.5">APY</span>');
      });
      var apyRow = [byId("pool-apy-1"), byId("pool-apy-2"), byId("pool-apy-3")];
      data.pools.forEach(function (pool, index) {
        if (apyRow[index]) apyRow[index].textContent = pool.apy + "% APY";
      });
      var calcSelect = byId("calc-pool");
      if (calcSelect) {
        calcSelect.innerHTML = data.pools
          .map(function (pool) {
            return '<option value="' + pool.id + '">' + pool.name + " — " + pool.apy + "% APY</option>";
          })
          .join("");
      }
      if (global.calcRewards) global.calcRewards();
      api.renderModuleMeta(".hero", "staking", envelope);
    }
    await load();
    global.setInterval(load, 30000);
  }

  async function initNetworkPulse(api) {
    async function load() {
      var payloads = await Promise.all([
        api.getNetworkEnvelope({ refresh: true }),
        api.getDashboardEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var dashboard = payloads[1].data;
      var data = envelope.data;
      var tps = data.tps ? fmtNumber(data.tps) : "unavailable";
      setText("#tb-tps", tps);
      setText("#kpi-tps", tps);
      setText("#center-tps", tps);
      setText("#tb-block", data.blockNumber ? "#" + fmtNumber(data.blockNumber) : "unavailable");
      setText("#kpi-blocks", data.blockNumber ? "#" + fmtNumber(data.blockNumber) : "unavailable");
      setText("#kpi-vals", fmtNumber((data.validators || []).length));
      var kpis = queryAll(".kpi-grid .kpi-val");
      if (kpis[1]) setText(kpis[1], data.uptimePct ? fmtPct(data.uptimePct) : "unavailable");
      if (kpis[2]) setText(kpis[2], data.finalitySeconds ? data.finalitySeconds + "s" : "unavailable");
      if (kpis[4]) setText(kpis[4], data.avgFeeUsd ? "$" + Number(data.avgFeeUsd).toFixed(4) : "unavailable");
      var topbarValues = queryAll(".topbar .tb-val");
      if (topbarValues[3]) setText(topbarValues[3], fmtNumber((data.validators || []).length) + " indexed");
      if (topbarValues[4]) setText(topbarValues[4], "$" + Number(dashboard.token.priceUsd).toFixed(4));
      setText("#total-tx", fmtNumber(data.totalTx));
      var alertBar = byId("alert-bar");
      if (alertBar) {
        alertBar.innerHTML =
          '<div class="alert-dot"></div><strong>Network Notice:</strong> ' +
          (data.validators && data.validators.length
            ? data.validators[0].name +
              " heartbeat confirmed in " +
              data.validators[0].location +
              " · " +
              data.validators.length +
              " indexed validators on map"
            : "Indexed validator telemetry available from the site store") +
          '<span id="alert-close" onclick="document.getElementById(\'alert-bar\').style.display=\'none\'">×</span>';
      }
      var valList = byId("val-list");
      if (valList) {
        valList.innerHTML = "";
        (data.validators || []).slice(0, 12).forEach(function (validator) {
          var item = document.createElement("div");
          item.className = "val-item";
          item.innerHTML =
            '<div class="val-dot" style="background:' +
            tierAccent(validator.tier) +
            ';box-shadow:0 0 6px ' +
            tierAccent(validator.tier) +
            '"></div><div class="val-name">' +
            validator.name +
            '</div><span class="val-status ' +
            (validator.status === "warning" ? "vs-warn" : "vs-active") +
            '">' +
            String(validator.status).toUpperCase() +
            '</span><div class="val-tps" style="color:' +
            tierAccent(validator.tier) +
            '">' +
            validator.tps +
            "</div>";
          valList.appendChild(item);
        });
      }
      var feed = byId("tx-feed");
      if (feed) {
        feed.innerHTML = "";
        if (!data.transactions || data.transactions.length === 0) {
          feed.innerHTML =
            '<div class="tx-item"><div class="tx-body"><div class="tx-detail">No indexed transactions are available from the current RPC source.</div></div></div>';
        } else {
          data.transactions.slice(0, 20).forEach(function (item) {
            var row = document.createElement("div");
            row.className = "tx-item";
            row.innerHTML =
              '<div class="tx-type tt-transfer">' +
              item.type +
              '</div><div class="tx-body"><div class="tx-hash">' +
              item.hash +
              '</div><div class="tx-detail">' +
              item.detail +
              '</div><div class="tx-meta">Updated ' +
              new Date(item.timestamp).toLocaleTimeString() +
              '</div></div><div class="tx-amount" style="color:var(--cyan)">' +
              item.amount +
              "</div>";
            feed.appendChild(row);
          });
        }
      }
      var regionBars = byId("region-bars");
      if (regionBars) {
        regionBars.innerHTML = "";
        (data.regions || []).forEach(function (region) {
          var color =
            region.region === "Asia Pacific"
              ? "var(--cyan)"
              : region.region === "North America"
                ? "var(--gold)"
                : region.region === "Europe"
                  ? "var(--green)"
                  : "var(--purple)";
          var wrap = document.createElement("div");
          wrap.innerHTML =
            '<div class="tm-bar-wrap"><div class="tm-bar" style="width:' +
            region.pct +
            "%;background:" +
            color +
            '"></div></div><div class="tm-label"><span class="tm-name">' +
            region.region +
            '</span><span class="tm-val" style="color:' +
            color +
            '">' +
            region.pct +
            "% · " +
            region.count +
            "</span></div>";
          regionBars.appendChild(wrap);
        });
      }
      api.renderModuleMeta(".topbar", "network", envelope);
    }
    await load();
    global.setInterval(load, 15000);
  }

  async function initLedger(api) {
    var envelope = await api.getLedgerEnvelope({ refresh: true });
    var data = envelope.data;
    setText("#sum-raised", fmtMoney(data.raisedUsd));
    setText("#sum-treasury", fmtMoney(data.treasuryUsd));
    var inflowsTotal = (data.events || []).reduce(function (sum, entry) {
      return sum + Number(entry.amountUsd || 0);
    }, 0);
    var outflowsTotal = (data.outflows || []).reduce(function (sum, entry) {
      return sum + Math.abs(Number(entry.amountUsd || 0));
    }, 0);
    setText("#inflows-total", "+" + fmtMoney(inflowsTotal));
    setText("#outflows-total", "−" + fmtMoney(outflowsTotal));
    var inflowsBody = byId("ledger-inflows");
    if (inflowsBody) {
      inflowsBody.innerHTML = "";
      (data.events || []).forEach(function (entry) {
        var row = document.createElement("tr");
        var date = new Date(entry.timestamp).toISOString().slice(0, 10);
        row.innerHTML =
          '<td class="l-date">' +
          date +
          '</td><td><div class="l-desc">' +
          entry.kind +
          '</div><div class="l-sub">' +
          entry.description +
          '</div></td><td><span class="l-cat lc-in">RAISE</span></td><td><div class="l-chain"><span class="lc-hash">offchain</span></div></td><td class="l-amount l-in">+' +
          fmtMoney(entry.amountUsd) +
          "</td>";
        inflowsBody.appendChild(row);
      });
      if (!(data.events || []).length) {
        inflowsBody.innerHTML =
          '<tr><td class="l-date">--</td><td><div class="l-desc">No inflow events published yet.</div><div class="l-sub">Data will appear once the ledger store is updated.</div></td><td><span class="l-cat lc-in">RAISE</span></td><td><div class="l-chain"><span class="lc-hash">n/a</span></div></td><td class="l-amount l-in">--</td></tr>';
      }
    }
    var outflowsBody = byId("ledger-outflows");
    if (outflowsBody) {
      outflowsBody.innerHTML = "";
      (data.outflows || []).forEach(function (entry) {
        var row = document.createElement("tr");
        var date = new Date(entry.timestamp).toISOString().slice(0, 10);
        row.innerHTML =
          '<td class="l-date">' +
          date +
          '</td><td><div class="l-desc">' +
          entry.kind +
          '</div><div class="l-sub">' +
          entry.description +
          '</div></td><td><span class="l-cat lc-out">OUTFLOW</span></td><td><div class="l-chain"><span class="lc-hash">offchain</span></div></td><td class="l-amount l-out">−' +
          fmtMoney(entry.amountUsd) +
          "</td>";
        outflowsBody.appendChild(row);
      });
      if (!(data.outflows || []).length) {
        outflowsBody.innerHTML =
          '<tr><td class="l-date">--</td><td><div class="l-desc">No outflow ledger is published in the live store yet.</div><div class="l-sub">This table will populate once finance events are available.</div></td><td><span class="l-cat lc-out">OUTFLOW</span></td><td><div class="l-chain"><span class="lc-hash">n/a</span></div></td><td class="l-amount l-out">--</td></tr>';
      }
    }
    var multisigList = byId("multisig-list");
    if (multisigList) {
      multisigList.innerHTML = "";
      (data.multisigSigners || []).forEach(function (signer, index) {
        var row = document.createElement("div");
        row.className = "msig-row";
        row.innerHTML =
          '<div class="ms-sig ' +
          (signer.status === "signed" ? "signed" : "") +
          '">' +
          (signer.status === "signed" ? "✓" : String(index + 1)) +
          '</div><div><div class="ms-name">' +
          signer.name +
          '</div><div class="ms-addr">' +
          signer.address +
          '</div></div><div class="ms-date" style="margin-left:auto">' +
          (signer.status || "active") +
          "</div>";
        multisigList.appendChild(row);
      });
      if (!(data.multisigSigners || []).length) {
        multisigList.innerHTML =
          '<div class="msig-row"><div class="ms-sig">--</div><div><div class="ms-name">No signer list published.</div><div class="ms-addr">n/a</div></div><div class="ms-date" style="margin-left:auto">unverified</div></div>';
      }
    }
    setText(
      "#last-verified",
      new Date(data.lastVerified).toISOString().slice(0, 19).replace("T", " ") + " UTC",
    );
    api.renderModuleMeta(".summary-cards", "ledger", envelope);
  }

  async function initProofWall(api) {
    async function load() {
      var envelope = await api.getProofsEnvelope({ refresh: true });
      var data = envelope.data;
      setText("#join-count", fmtNumber(data.totalOperators));
      setText("#wall-count", "Showing " + fmtNumber(data.operators.length) + " reservations");
      setText("#slots-left", data.slotsLeft);
      setText("#v-slots", data.slotsLeft);
      setText("#slots-total", fmtNumber(data.totalSlots || 0));
      setText("#v-price", data.tokenPriceUsd ? "$" + Number(data.tokenPriceUsd).toFixed(3) : "n/a");
      var reservedSlots = Number(data.reservedSlots || 0);
      var totalSlots = Number(data.totalSlots || 0);
      var slotPct = totalSlots ? Math.round((reservedSlots / totalSlots) * 100) : 0;
      setText("#slot-sub", fmtNumber(reservedSlots) + " reserved · " + slotPct + "% filled");
      var slotFill = byId("slot-fill");
      if (slotFill) slotFill.style.width = slotPct + "%";
      var etaHours = Number(data.selloutEtaHours || 0);
      if (etaHours > 0) {
        var etaH = Math.floor(etaHours);
        var etaM = Math.floor((etaHours - etaH) * 60);
        setText("#sellout-time", etaH + "h " + String(etaM).padStart(2, "0") + "m");
      } else {
        setText("#sellout-time", "n/a");
      }
      var pace = data.pace24h || [];
      var lastHour = pace.length ? pace[pace.length - 1] : 0;
      var last6h = pace.slice(-6).reduce(function (sum, value) { return sum + value; }, 0);
      var last24h = pace.reduce(function (sum, value) { return sum + value; }, 0);
      setText("#v-hour", fmtNumber(lastHour));
      setText("#v-today", fmtNumber(last24h));
      setText("#p-1h", fmtNumber(lastHour) + " reservations");
      setText("#p-6h", fmtNumber(last6h) + " reservations");
      setText("#p-24h", fmtNumber(last24h) + " reservations");
      setText("#p-all", fmtNumber(data.totalOperators) + " operators");
      var grid = byId("wall-grid");
      if (grid) {
        grid.innerHTML = "";
        data.operators.forEach(function (operator) {
          var tierClassName =
            operator.tier === "genesis"
              ? "pt-genesis"
              : operator.tier === "star"
                ? "pt-star"
                : "pt-lite";
          var card = document.createElement("div");
          card.className = "proof-card";
          card.innerHTML =
            '<div class="pc-top"><div class="pc-flag">' +
            operator.flag +
            '</div><div class="pc-identity"><div class="pc-name">' +
            operator.name +
            '</div><div class="pc-location">' +
            operator.location +
            '</div></div><div class="pc-tier ' +
            tierClassName +
            '">' +
            operator.tier.toUpperCase() +
            '</div></div><div class="pc-bottom"><span class="pc-amount">' +
            operator.amount +
            '</span><span class="pc-time">' +
            operator.time +
            '</span><span class="pc-action">✓ Reserved</span></div>';
          grid.appendChild(card);
        });
      }
      var pace = byId("pace-bar");
      if (pace) {
        pace.innerHTML = "";
        (data.pace24h || []).forEach(function (count, index) {
          var segment = document.createElement("div");
          segment.className = "pb-seg" + (index === data.pace24h.length - 1 ? " current" : "");
          segment.style.height = Math.max(6, count * 8) + "%";
          pace.appendChild(segment);
        });
      }
      var countries = byId("country-list");
      if (countries) {
        countries.innerHTML = "";
        var rows = data.topCountries || [];
        var maxCount = rows.reduce(function (max, entry) { return Math.max(max, entry.count || 0); }, 1);
        rows.forEach(function (entry) {
          var row = document.createElement("div");
          row.className = "country-row";
          var pct = maxCount ? Math.round((entry.count / maxCount) * 100) : 0;
          row.innerHTML =
            '<span class="cr-flag">' +
            entry.flag +
            '</span><div class="cr-bar-wrap"><div class="cr-bar" style="width:' +
            pct +
            '%"></div></div><span class="cr-count">' +
            fmtNumber(entry.count) +
            "</span>";
          countries.appendChild(row);
        });
        if (!rows.length) {
          countries.innerHTML =
            '<div class="country-row"><span class="cr-flag">🌍</span><div class="cr-bar-wrap"><div class="cr-bar" style="width:10%"></div></div><span class="cr-count">--</span></div>';
        }
      }
      var ticker = byId("ticker-inner");
      if (ticker) {
        var items = data.operators
          .slice(0, 12)
          .map(function (operator) {
            return (
              '<div class="tick-item"><span class="tick-flag">' +
              operator.flag +
              "</span><span>" +
              operator.name +
              '</span><span class="tick-gold">reserved ' +
              operator.tier +
              ' node</span><span class="tick-green">✓</span></div>'
            );
          })
          .join("");
        ticker.innerHTML = items + items;
      }
      api.renderModuleMeta(".hero", "proof wall", envelope);
    }
    await load();
    global.setInterval(load, 20000);
  }

  async function initPresaleMetrics(api) {
    var payloads = await Promise.all([
      api.getPresaleEnvelope({ refresh: true }),
      api.getDashboardEnvelope({ refresh: true }),
      api.getTokenomicsEnvelope({ refresh: true }),
    ]);
    var envelope = payloads[0];
    var data = payloads[0].data;
    var dashboard = payloads[1].data;
    var tokenomics = payloads[2].data || {};
    global.__x3PresaleQuote = pickFirst(data, [
      "quoteRates",
      "quotes",
      "quote_rates",
      "presaleQuotes",
      "rates",
      "ratesBySymbol",
    ]) || {};
    addDebug("presale", "quoteRates", Object.keys(global.__x3PresaleQuote || {}).length ? "resolved" : "missing");
    var roundLabel = data.currentRoundLabel || data.currentRound || "Presale";
    if (byId("presale-round")) setText("#presale-round", roundLabel);
    if (byId("presale-stage")) setText("#presale-stage", "⬡ " + String(roundLabel).toUpperCase());
    if (byId("presale-status")) {
      setText("#presale-status", data.status ? String(data.status).toUpperCase() : "STATUS");
    }
    countdown(data.closesAt, {
      days: "#cd-d",
      hours: "#cd-h",
      minutes: "#cd-m",
      seconds: "#cd-s",
    });
    setText("#raised-amt", fmtMoney(data.raisedUsd));
    setText("#raise-cap", "Hard Cap: " + fmtMoney(data.hardCapUsd));
    var pct = data.hardCapUsd ? Math.round((data.raisedUsd / data.hardCapUsd) * 1000) / 10 : 0;
    var remaining = Math.max(0, Number(data.hardCapUsd || 0) - Number(data.raisedUsd || 0));
    setText("#raise-pct", pct + "% filled — " + fmtCompactMoney(remaining) + " remaining");
    setText("#pf-price", "$" + Number(data.tokenPriceUsd || 0).toFixed(3));
    var bonusMeta = normalizeBonus(data);
    setText("#pf-bonus", bonusMeta.pct != null ? "+" + bonusMeta.pct + "%" : bonusMeta.label || "--");
    setText("#pf-vesting", data.vesting || data.vestingSchedule || "--");
    var bonusLabel = bonusMeta.pct != null ? "+" + bonusMeta.pct + "% X3S" : bonusMeta.label || "--";
    setText("#bonus-pct", bonusLabel);
    if (byId("bonus-text") && bonusMeta.pct == null && !bonusMeta.label) {
      byId("bonus-text").textContent = "Round III Bonus: pending";
    }
    setText("#stat-raised", fmtCompactMoney(data.raisedUsd));
    setText("#stat-investors", fmtNumber(data.investors));
    setText("#stat-holders", fmtNumber(dashboard.token.holders || 0));
    setText(
      "#stat-supply",
      fmtNumber(Math.round(Number(dashboard.token.totalSupply || 0) / 1000000)) + "M",
    );
    var fill = byId("raise-fill");
    if (fill) {
      fill.style.width = ((data.raisedUsd / data.hardCapUsd) * 100).toFixed(1) + "%";
    }
    var totalSupply = Number(dashboard.token.totalSupply || 0);
    if (byId("tko-total")) {
      byId("tko-total").textContent = totalSupply ? fmtNumber(Math.round(totalSupply / 1000000)) + "M" : "--";
    }
    var allocations = tokenomics.allocations || [];
    var totalAlloc = allocations.reduce(function (sum, entry) {
      return sum + Number(entry.amountX3S || 0);
    }, 0);
    function pctOf(nameList) {
      var subtotal = allocations
        .filter(function (entry) {
          return nameList.some(function (name) {
            return String(entry.name || "").toLowerCase().indexOf(name) !== -1;
          });
        })
        .reduce(function (sum, entry) {
          return sum + Number(entry.amountX3S || 0);
        }, 0);
      return totalAlloc ? Math.round((subtotal / totalAlloc) * 100) : 0;
    }
    var presalePct = pctOf(["presale", "circulating"]);
    var ecosystemPct = pctOf(["ecosystem"]);
    var teamPct = pctOf(["team"]);
    var stakingPct = pctOf(["staking"]);
    setText("#tko-presale", presalePct ? presalePct + "%" : "--");
    setText("#tko-ecosystem", ecosystemPct ? ecosystemPct + "%" : "--");
    setText("#tko-team", teamPct ? teamPct + "%" : "--");
    setText("#tko-staking", stakingPct ? stakingPct + "%" : "--");
    if (byId("tko-bar-presale")) byId("tko-bar-presale").style.width = presalePct + "%";
    if (byId("tko-bar-ecosystem")) byId("tko-bar-ecosystem").style.width = ecosystemPct + "%";
    if (byId("tko-bar-team")) byId("tko-bar-team").style.width = teamPct + "%";
    if (byId("tko-bar-staking")) byId("tko-bar-staking").style.width = stakingPct + "%";
    api.renderModuleMeta(".presale-card", "presale", envelope);
    if (global.calcTokens) global.calcTokens();
  }

  async function initReservationsPages(api) {
    async function load() {
      var [presaleEnvelope, reservationsEnvelope, proofsEnvelope] = await Promise.all([
        api.getPresaleEnvelope({ refresh: true }),
        api.getReservationsEnvelope({ refresh: true }),
        api.getProofsEnvelope({ refresh: true }),
      ]);
      var presale = presaleEnvelope.data;
      var reservations = reservationsEnvelope.data;
      setText("#slots-left", presale.tiers[0].slotsLeft);
      setText("#slots-remain", presale.tiers[0].slotsLeft + " SLOTS LEFT");
      setText("#thermo-raised", fmtCompactMoney(presale.raisedUsd));
      setText("#thermo-pct", ((presale.raisedUsd / presale.hardCapUsd) * 100).toFixed(1) + "%");
      setText("#hdr-raised", fmtMoney(presale.raisedUsd));
      setText("#hdr-inv", fmtNumber(presale.investors));
      setText("#sh-raised", fmtMoney(presale.raisedUsd));
      setText("#sh-inv", fmtNumber(presale.investors));
      setText("#sh-today", fmtMoney(presale.todayUsd));
      setText("#sh-slots", fmtNumber(presale.tiers[0].slotsLeft));
      setText("#today-total", "$" + Math.round(Number(presale.todayUsd || 0) / 1000) + "K");
      setText("#r-investors", fmtNumber(presale.investors));
      setText("#r-filled", ((presale.raisedUsd / presale.hardCapUsd) * 100).toFixed(1) + "%");
      setText("#r-price", "$" + Number(presale.tokenPriceUsd || 0).toFixed(3));
      setText("#r-genesis", fmtNumber(presale.tiers[0].slotsLeft));
      setText("#r-closes", presale.daysRemaining + "d");
      setText("#r-next", presale.nextRoundPriceUsd ? "$" + Number(presale.nextRoundPriceUsd).toFixed(2) : "--");
      setHtml(
        "#wall-count",
        "Showing <strong>" +
          reservations.recentCards.length +
          "</strong> of " +
          fmtNumber(presale.investors) +
          " recent purchases",
      );
      setText("#join-count", fmtNumber(proofsEnvelope.data.totalOperators));
      setText("#v-slots", proofsEnvelope.data.slotsLeft);
      setText("#today-date", formatDateLabel());
      var closesLabel = byId("closes-label");
      if (closesLabel) closesLabel.textContent = "in ~" + presale.daysRemaining + " days";
      var nextRound = byId("next-round");
      if (nextRound) {
        if (presale.nextRoundPriceUsd && presale.tokenPriceUsd) {
          var premium = Math.round(((presale.nextRoundPriceUsd - presale.tokenPriceUsd) / presale.tokenPriceUsd) * 100);
          nextRound.textContent =
            "$" + Number(presale.nextRoundPriceUsd).toFixed(2) + " (" + (premium >= 0 ? "+" : "") + premium + "%)";
        } else {
          nextRound.textContent = "--";
        }
      }
      var ticker = byId("sh-ticker");
      if (ticker) {
        var pctFilled = ((presale.raisedUsd / presale.hardCapUsd) * 100).toFixed(1);
        var premiumPct = presale.nextRoundPriceUsd && presale.tokenPriceUsd
          ? Math.round(((presale.nextRoundPriceUsd - presale.tokenPriceUsd) / presale.tokenPriceUsd) * 100)
          : 0;
        ticker.textContent =
          "ROUND III " +
          pctFilled +
          "% FILLED · " +
          fmtNumber(presale.tiers[0].slotsLeft) +
          " GENESIS SLOTS LEFT · NEXT ROUND: " +
          (presale.nextRoundPriceUsd ? "$" + Number(presale.nextRoundPriceUsd).toFixed(2) : "--") +
          " (" +
          (premiumPct >= 0 ? "+" : "") +
          premiumPct +
          "%) · CLOSES IN ~" +
          presale.daysRemaining +
          " DAYS · " +
          fmtNumber(presale.investors) +
          " INVESTORS · X3S " +
          "$" +
          Number(presale.tokenPriceUsd || 0).toFixed(3);
      }
      renderReservationsCards(byId("card-grid"), reservations.recentCards);
      renderTopInvestors(byId("top-investors"), reservations.topInvestors);
      renderReservationsHeatmap(byId("heatmap"), reservations.activityHeatmap);
      var eventList = byId("event-list");
      if (eventList) {
        eventList.innerHTML = "";
        reservations.recentCards.slice(0, 6).forEach(function (entry) {
          var card = document.createElement("div");
          card.className = "ev-card";
          card.innerHTML =
            '<div class="ev-top"><div class="ev-flag">' +
            entry.flag +
            '</div><div class="ev-name">' +
            entry.name +
            '</div><div class="ev-type ' +
            (entry.type === "node" ? "evt-node" : "evt-token") +
            '">' +
            (entry.type === "node" ? "NODE" : "TOKEN") +
            '</div></div><div class="ev-amount">' +
            fmtMoney(entry.amountUsd) +
            '</div><div class="ev-time">' +
            entry.timeAgo +
            "</div>";
          eventList.appendChild(card);
        });
      }
      var milestones = queryAll(".milestone");
      if (milestones.length) {
        var targets = [1000000, 5000000, 10000000, 15000000, 20000000, 25000000];
        milestones.forEach(function (milestone, index) {
          milestone.classList.remove("unlocked", "active");
          if (presale.raisedUsd >= targets[index]) {
            milestone.classList.add("unlocked");
          } else if (index === targets.findIndex(function (value) { return presale.raisedUsd < value; })) {
            milestone.classList.add("active");
          }
        });
      }
      var liquidRect = byId("liquid-rect");
      if (liquidRect) {
        var pct = presale.raisedUsd / presale.hardCapUsd;
        var fillHeight = Math.round(pct * 310);
        var fillY = 330 - fillHeight;
        liquidRect.setAttribute("y", fillY);
        liquidRect.setAttribute("height", fillHeight + 25);
        if (byId("level-line")) {
          byId("level-line").setAttribute("y1", fillY);
          byId("level-line").setAttribute("y2", fillY);
        }
        if (byId("level-arrow")) {
          byId("level-arrow").setAttribute("points", "65," + fillY + " 72," + (fillY - 4) + " 72," + (fillY + 4));
        }
        if (byId("shimmer")) {
          byId("shimmer").setAttribute("y", fillY);
          byId("shimmer").setAttribute("height", fillHeight);
        }
      }
      api.renderModuleMeta(".hero-stats, .hero-strip, .shell, .sub-header, .top-bar", "reservations", reservationsEnvelope);
    }
    await load();
    global.setInterval(load, 15000);
  }

  async function initValidatorPresale(api) {
    async function load() {
      var envelope = await api.getPresaleEnvelope({ refresh: true });
      var data = envelope.data;
      var tiers = {};
      data.tiers.forEach(function (tier) {
        tiers[tier.name] = tier;
      });
      global.__x3PresaleTiers = tiers;
      var genesis = tiers.genesis || {};
      var star = tiers.star || {};
      var lite = tiers.lite || {};
      setText("#slots-left", genesis.slotsLeft || "0");
      setText("#slots-remain", genesis.slotsLeft ? genesis.slotsLeft + " SLOTS LEFT" : "SOLD OUT");
      setText("#slots-total", genesis.totalSlots || "--");
      setText("#hs-apy", genesis.apy || "--");
      var minStake = Math.min(
        Number(genesis.priceUsd || Infinity),
        Number(star.priceUsd || Infinity),
        Number(lite.priceUsd || Infinity),
      );
      setText("#hs-min-stake", isFinite(minStake) ? "$" + Math.round(minStake / 1000) + "K" : "--");
      setText("#tier-genesis-price", genesis.priceUsd ? "$" + Math.round(genesis.priceUsd / 1000) + "K" : "--");
      setText("#tier-star-price", star.priceUsd ? "$" + Math.round(star.priceUsd / 1000) + "K" : "--");
      setText("#tier-lite-price", lite.priceUsd ? "$" + Math.round(lite.priceUsd / 1000) + "K" : "--");
      setText("#tier-genesis-apy", genesis.apy ? "✓ " + genesis.apy + " APY" : "✓ -- APY");
      setText("#tier-star-apy", star.apy ? "✓ " + star.apy + " APY" : "✓ -- APY");
      setText("#tier-lite-apy", lite.apy ? "✓ " + lite.apy + " APY" : "✓ -- APY");
      var feeShareMeta = normalizeFeeShare(data);
      var feeShare =
        feeShareMeta.pct != null ? feeShareMeta.pct + "%" : feeShareMeta.label || "--";
      var bonusMeta = normalizeBonus(data);
      global.__x3PresaleBonusLabel =
        bonusMeta.pct != null ? "+" + bonusMeta.pct + "% X3S" : bonusMeta.label || "--";
      setText("#benefit-apy", genesis.apy ? genesis.apy + " APY" : "--");
      setText("#benefit-fee-share", feeShare);
      setText(
        "#benefit-bonus",
        bonusMeta.pct != null ? "+" + bonusMeta.pct + "% X3S" : bonusMeta.label || "--",
      );
      setText("#ct-min-genesis", genesis.priceUsd ? "$" + genesis.priceUsd.toLocaleString("en-US") : "--");
      setText("#ct-min-star", star.priceUsd ? "$" + star.priceUsd.toLocaleString("en-US") : "--");
      setText("#ct-min-lite", lite.priceUsd ? "$" + lite.priceUsd.toLocaleString("en-US") : "--");
      setText("#ct-apy-genesis", genesis.apy || "--");
      setText("#ct-apy-star", star.apy || "--");
      setText("#ct-apy-lite", lite.apy || "--");
      setText("#ct-fee-genesis", feeShare);
      setText(
        "#ct-bonus-genesis",
        bonusMeta.pct != null ? "+" + bonusMeta.pct + "% X3S" : bonusMeta.label || "--",
      );
      setText(
        "#ct-bonus-star",
        bonusMeta.pct != null ? "+" + bonusMeta.pct + "% X3S" : bonusMeta.label || "--",
      );
      setText(
        "#ct-bonus-lite",
        bonusMeta.pct != null ? "+" + bonusMeta.pct + "% X3S" : bonusMeta.label || "--",
      );
      setText("#ct-genesis", genesis.slotsLeft != null ? genesis.slotsLeft : "--");
      setText("#ct-star", star.slotsLeft != null ? star.slotsLeft : "--");
      setText("#ct-lite", lite.slotsLeft != null ? lite.slotsLeft : "--");
      if (global.updateValidatorSlots && genesis.totalSlots !== undefined) {
        global.updateValidatorSlots(genesis.totalSlots, genesis.reservedSlots || 0);
      }
      api.renderModuleMeta(".hero-shell", "validator presale", envelope);
    }

    global.handleBuy = async function () {
      var type = byId("os-type");
      var qty = byId("os-qty");
      var selectedLabel = type ? type.textContent.trim() : "GENESIS NODE";
      var quantity = qty ? Number(qty.textContent.replace(/[^\d]/g, "")) || 1 : 1;
      var tierKey =
        Object.keys(global.__x3PresaleTiers || {}).find(function (key) {
          return global.__x3PresaleTiers[key].label === selectedLabel;
        }) || "genesis";
      var tier = global.__x3PresaleTiers[tierKey];
      var name = global.prompt("Operator name for this reservation:", "X3 Operator");
      if (!name) return;
      var wallet = global.prompt("Wallet address or contact handle:", "0x");
      var location = global.prompt("Location (City, Country):", "Denver, US");
      var countryCode = ((location || "US").split(",")[1] || "US").trim().slice(0, 2).toUpperCase();
      await api.submitReservation({
        name: name,
        wallet: wallet,
        location: location,
        countryCode: countryCode || "US",
        tier: tierKey,
        quantity: quantity,
        amountUsd: tier ? tier.priceUsd : 50000,
      });
      await load();
      global.alert("Reservation submitted to the X3 site store.");
    };

    await load();
    global.setInterval(load, 15000);
  }

  async function initKyc(api) {
    global.submitApplication = async function () {
      var payload = {
        fullName: (query('input[placeholder*="Legal Name"]') || {}).value || "",
        email: (query('input[type="email"]') || {}).value || "",
        wallet: (byId("wallet-addr") || {}).value || "",
      };
      var result = await api.submitForm("kyc", payload);
      byId("panel5").classList.remove("active");
      byId("step5").className = "step-item done";
      byId("step5").querySelector(".step-circle").textContent = "✓";
      byId("panel-success").classList.add("active");
      setText("#app-id", result.data.id);
      window.scrollTo({ top: 0, behavior: "smooth" });
    };
    var envelope = await api.getPresaleEnvelope({ refresh: true });
    var policy = envelope.data || {};
    var tiers = pickFirst(policy, [
      "bonusTiers",
      "investorTiers",
      "kycTiers",
      "presaleTiers",
      "tiers",
      "policy.bonusTiers",
      "policy.investorTiers",
      "policy.kycTiers",
      "presale.bonusTiers",
      "presale.investorTiers",
    ]) || [];
    tiers = tiers.filter(function (tier) {
      return (
        tier &&
        (tier.bonusPct != null ||
          tier.bonusLabel ||
          tier.minUsd != null ||
          tier.maxUsd != null ||
          tier.rangeLabel)
      );
    });
    function formatTierAmount(tier) {
      if (!tier) return "--";
      if (tier.rangeLabel) return tier.rangeLabel;
      var min = tier.minUsd != null ? tier.minUsd : tier.min;
      var max = tier.maxUsd != null ? tier.maxUsd : tier.max;
      if (min != null && max != null) {
        return fmtCompactMoney(min) + "–" + fmtCompactMoney(max);
      }
      if (min != null) return fmtCompactMoney(min) + "+";
      if (tier.amountUsd != null) return fmtCompactMoney(tier.amountUsd);
      return "--";
    }
    function formatTierBonus(tier) {
      if (!tier) return "--";
      if (tier.bonusPct != null) return "+" + tier.bonusPct + "% X3S";
      if (tier.bonusLabel) return tier.bonusLabel;
      return "--";
    }
    var tierTargets = [
      { amount: "#kyc-tier-1-amount", bonus: "#kyc-tier-1-bonus" },
      { amount: "#kyc-tier-2-amount", bonus: "#kyc-tier-2-bonus" },
      { amount: "#kyc-tier-3-amount", bonus: "#kyc-tier-3-bonus" },
    ];
    addDebug("presale", "kycTiers", tiers.length ? "resolved" : "missing");
    tierTargets.forEach(function (target, index) {
      var tier = tiers[index] || {};
      setText(target.amount, formatTierAmount(tier));
      setText(target.bonus, formatTierBonus(tier));
    });
  }

  async function initAffiliate(api) {
    global.registerAffiliate = async function () {
      var payload = {
        name: (query('input[placeholder*="Full Name"]') || {}).value || "Affiliate applicant",
        email: (query('input[type="email"]') || {}).value || "",
      };
      var result = await api.submitForm("affiliate", payload);
      global.alert("Affiliate application submitted.\n\nYour record id: " + result.data.id);
    };
  }

  async function initInvestorRelations(api) {
    global.submitForm = async function () {
      var fields = queryAll("input, textarea, select");
      var payload = {
        name: fields[0] ? fields[0].value : "",
        email: fields[1] ? fields[1].value : "",
        organization: fields[2] ? fields[2].value : "",
      };
      var result = await api.submitForm("investor", payload);
      global.alert("Investor inquiry submitted: " + result.data.id);
    };

    var payloads = await Promise.all([
      api.getPresaleEnvelope({ refresh: true }),
      api.getDashboardEnvelope({ refresh: true }),
      api.getNetworkEnvelope({ refresh: true }),
      api.getLedgerEnvelope({ refresh: true }),
      api.getStakingEnvelope({ refresh: true }),
    ]);
    var envelope = payloads[0];
    var presale = payloads[0].data;
    var dashboard = payloads[1].data;
    var network = payloads[2].data;
    var ledger = payloads[3].data;
    var staking = payloads[4].data;
    countdown(presale.closesAt, {
      days: "#cd-d",
      hours: "#cd-h",
      minutes: "#cd-m",
      seconds: "#cd-s",
    });
    var pct = presale.hardCapUsd ? ((presale.raisedUsd / presale.hardCapUsd) * 100).toFixed(1) : "0.0";
    setText("#ir-raised", fmtMoney(presale.raisedUsd));
    setText("#ir-hardcap", fmtCompactMoney(presale.hardCapUsd));
    setText("#ir-filled", pct + "%");
    if (byId("ir-prog")) byId("ir-prog").style.width = pct + "%";
    setText("#ir-price", "$" + Number(presale.tokenPriceUsd || 0).toFixed(3));
    var bonusMeta = normalizeBonus(presale);
    setText(
      "#ir-bonus",
      bonusMeta.pct != null ? "+" + bonusMeta.pct + "%" : bonusMeta.label || "--",
    );
    setText("#ir-next", presale.nextRoundPriceUsd ? "$" + Number(presale.nextRoundPriceUsd).toFixed(2) : "--");
    var minTicket = Math.min.apply(null, presale.tiers.map(function (tier) { return tier.priceUsd; }));
    setText("#ir-min", isFinite(minTicket) ? "$" + Math.round(minTicket / 1000) + "K" : "--");
    setText("#ir-m-raised", fmtCompactMoney(presale.raisedUsd));
    setText("#ir-m-investors", fmtNumber(presale.investors));
    setText("#ir-m-price", "$" + Number(dashboard.token.priceUsd || 0).toFixed(4));
    setText("#ir-m-tvl", fmtCompactMoney(staking.totalValueLocked));
    setText("#ir-m-supply", fmtNumber(dashboard.token.totalSupply || 0));
    setText("#ir-m-validators", fmtNumber((network.validators || []).length));
    setText("#ir-m-holders", fmtNumber(dashboard.token.holders || 0));
    setText("#ir-m-treasury", fmtCompactMoney(ledger.treasuryUsd || 0));
    api.renderModuleMeta(".investor-card", "investor relations", envelope);
  }

  async function initSlotTracker(api) {
    async function load() {
      var payloads = await Promise.all([
        api.getReservationsEnvelope({ refresh: true }),
        api.getPresaleEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var presale = payloads[1].data;
      var tracker = envelope.data.slotTracker;
      setText("#avail-num", tracker.availableSlots);
      setText("#pct-fill", Math.round((tracker.reservedSlots / tracker.totalSlots) * 100) + "%");
      setText("#fill-detail", tracker.reservedSlots + " of " + tracker.totalSlots + " filled");
      setText("#tb-genesis", tracker.reservedSlots);
      var bigSub = query(".big-sub");
      if (bigSub) bigSub.textContent = "Genesis slots remaining";
      var rateLabel = byId("rate-label");
      if (rateLabel) {
        rateLabel.textContent =
          "+" +
          envelope.data.activityHeatmap.slice(-1)[0] +
          " reservations in the latest hourly bucket";
      }
      var tierValues = queryAll(".tier-box .tb-val");
      if (tierValues[1]) setText(tierValues[1], presale.tiers[1].reservedSlots);
      if (tierValues[2]) setText(tierValues[2], presale.tiers[2].reservedSlots);
      if (tierValues[3]) {
        setText(
          tierValues[3],
          presale.tiers.reduce(function (sum, tier) {
            return sum + Number(tier.reservedSlots || 0);
          }, 0),
        );
      }
      setText("#tb-genesis-sub", presale.tiers[0].slotsLeft + " left");
      setText("#tb-star-sub", presale.tiers[1].slotsLeft + " left");
      setText("#tb-lite-sub", presale.tiers[2].slotsLeft + " left");
      setText("#tb-total-sub", "indexed");
      var ringArc = byId("ring-arc");
      if (ringArc) {
        var circ = 201.1;
        var pct = tracker.reservedSlots / tracker.totalSlots;
        ringArc.setAttribute("stroke-dashoffset", (circ - circ * pct).toFixed(1));
      }
      var grid = byId("hex-grid");
      var tooltip = byId("tooltip");
      if (grid) {
        grid.innerHTML = "";
        tracker.slots.forEach(function (slot) {
          var hex = document.createElement("div");
          hex.className = "hex-cell " + (slot.reserved ? "reserved" : "empty");
          hex.id = "hex-" + slot.slotNumber;
          hex.innerHTML = slot.reserved
            ? '<div class="hex-flag">' +
              slot.reservation.flag +
              '</div><div class="hex-num">' +
              String(slot.slotNumber).padStart(3, "0") +
              "</div>"
            : '<div class="hex-num" style="font-size:8px">' + String(slot.slotNumber).padStart(3, "0") + "</div>";
          if (tooltip) {
            hex.addEventListener("mouseenter", function (event) {
              tooltip.style.display = "block";
              setText("#tt-num", "Slot #" + String(slot.slotNumber).padStart(3, "0"));
              if (slot.reserved) {
                setText("#tt-status", "✓ Reserved");
                byId("tt-status").style.color = "var(--green)";
                setText("#tt-op", slot.reservation.name);
                setText("#tt-country", slot.reservation.flag + " " + slot.reservation.location);
                setText("#tt-time", slot.reservation.timeAgo);
              } else {
                setText("#tt-status", "● Available");
                byId("tt-status").style.color = "var(--gold)";
                setText("#tt-op", "—");
                setText("#tt-country", "—");
                setText("#tt-time", "Unreserved");
              }
              tooltip.style.left = event.clientX + 12 + "px";
              tooltip.style.top = event.clientY - 60 + "px";
            });
            hex.addEventListener("mousemove", function (event) {
              tooltip.style.left = event.clientX + 12 + "px";
              tooltip.style.top = event.clientY - 60 + "px";
            });
            hex.addEventListener("mouseleave", function () {
              tooltip.style.display = "none";
            });
          }
          if (!slot.reserved) {
            hex.addEventListener("click", function () {
              global.location.href = "x3star-validator-presale.html";
            });
          }
          grid.appendChild(hex);
        });
      }
      var recList = byId("rec-list");
      if (recList) {
        recList.innerHTML = "";
        tracker.recentReservations.forEach(function (slot) {
          var item = document.createElement("div");
          item.className = "rec-item";
          item.innerHTML =
            '<div class="rec-num">#' +
            String(slot.slotNumber).padStart(3, "0") +
            '</div><div class="rec-flag">' +
            slot.reservation.flag +
            '</div><div class="rec-body"><div class="rec-name">' +
            slot.reservation.name +
            '</div><div class="rec-time">' +
            slot.reservation.timeAgo +
            '</div></div><div class="rec-tier" style="background:rgba(255,215,0,0.1);color:var(--gold);border:1px solid rgba(255,215,0,0.2)">GENESIS</div>';
          recList.appendChild(item);
        });
      }
      api.renderModuleMeta(".ha-header", "slot tracker", envelope);
    }
    await load();
    global.setInterval(load, 15000);
  }

  function whaleEventsByFilter(events) {
    if (whaleFilter === "all") return events;
    return events.filter(function (event) {
      return String(event.type || "").toLowerCase() === whaleFilter;
    });
  }

  async function initWhales(api) {
    global.setFilter = function (element, type) {
      whaleFilter = type;
      queryAll(".filter-chip").forEach(function (chip) {
        chip.classList.remove("active");
      });
      if (element && element.classList) element.classList.add("active");
      load();
    };

    global.showAlert = function () {
      var toast = byId("alert-toast");
      if (!toast) return;
      toast.style.display = "block";
      global.setTimeout(function () {
        toast.style.display = "none";
      }, 4000);
    };

    async function load() {
      var envelope = await api.getWhalesEnvelope({ refresh: true });
      var data = envelope.data;
      setText("#whale-count", data.whales.length + " wallets tracked");
      setText("#stream-count", data.events.length);
      setText("#alert-num", data.alerts);
      setText("#buy-pct", data.buyPct + "%");
      setText("#sell-pct", data.sellPct + "%");
      if (byId("buy-bar")) byId("buy-bar").style.width = data.buyPct + "%";
      if (byId("sell-bar")) byId("sell-bar").style.width = data.sellPct + "%";
      setText("#sent-dominant", data.dominantSentiment);
      setText("#accum-score", Number(data.accumulationScore).toFixed(1));
      setText(
        "#accum-sub",
        "out of 10 · " +
          (data.accumulationScore >= 7
            ? "STRONG ACCUMULATION"
            : data.accumulationScore >= 4
              ? "BALANCED FLOW"
              : "DISTRIBUTION"),
      );
      if (byId("accum-bar")) {
        byId("accum-bar").style.width = Math.min(100, Math.max(0, data.accumulationScore * 10)) + "%";
      }
      var headPrice = query(".page-head [style*='X3S/USD']");
      if (headPrice) {
        headPrice.innerHTML =
          'X3S/USD: <span style="color:var(--gold);font-weight:700;">$' +
          Number(data.priceUsd).toFixed(4) +
          '</span> <span style="color:' +
          (data.priceChange24h >= 0 ? "var(--green)" : "var(--red)") +
          '">' +
          (data.priceChange24h >= 0 ? "▲" : "▼") +
          Number(Math.abs(data.priceChange24h)).toFixed(1) +
          "%</span>";
      }
      var whales = data.whales || [];
      var top1 = whales[0] ? whales[0].holdingsX3S : 0;
      var top10 = whales.slice(0, 10).reduce(function (sum, wallet) { return sum + (wallet.holdingsX3S || 0); }, 0);
      var top50 = whales.slice(0, 50).reduce(function (sum, wallet) { return sum + (wallet.holdingsX3S || 0); }, 0);
      setText("#whale-top1", (top1 / 1000000).toFixed(1) + "M X3S");
      setText("#whale-top10", (top10 / 1000000).toFixed(1) + "M X3S");
      setText("#whale-top50", (top50 / 1000000).toFixed(1) + "M X3S");
      var supply = Number(data.totalSupplyX3S || 0);
      var supplyPct = supply ? ((top50 / supply) * 100).toFixed(1) : "0.0";
      setText("#whale-supply-pct", supplyPct + "%");
      var netFlow = Number(data.netFlowX3S24h || 0);
      var netFlowLabel = (netFlow >= 0 ? "+" : "−") + (Math.abs(netFlow) / 1000000).toFixed(1) + "M X3S";
      setText("#whale-netflow", netFlowLabel);
      var whaleList = byId("whale-list");
      if (whaleList) {
        whaleList.innerHTML = "";
        data.whales.forEach(function (wallet, index) {
          var item = document.createElement("div");
          item.className = "whale-card" + (index === 0 ? " selected" : "");
          item.innerHTML =
            '<div class="wc-top"><div class="wc-rank">#' +
            wallet.rank +
            '</div><div class="wc-avatar" style="background:linear-gradient(135deg,' +
            wallet.color +
            ', rgba(85,0,204,0.9))">' +
            wallet.name[0].toUpperCase() +
            '</div><div class="wc-identity"><div class="wc-name" style="color:' +
            wallet.color +
            '">' +
            wallet.name +
            '</div><div class="wc-addr">' +
            wallet.address +
            '</div></div><div class="wc-badge ' +
            (wallet.classification === "whale"
              ? "wb-whale"
              : wallet.classification === "institution"
                ? "wb-inst"
                : "wb-val") +
            '">' +
            wallet.badge +
            '</div></div><div class="wc-bottom"><div class="wcb-stat"><div class="wcbs-val" style="color:var(--gold)">' +
            wallet.holdingsDisplay +
            '</div><div class="wcbs-key">Balance</div></div><div class="wcb-stat"><div class="wcbs-val">' +
            wallet.usdDisplay +
            '</div><div class="wcbs-key">USD Value</div></div><div class="wcb-stat"><div class="wcbs-val" style="color:' +
            (wallet.changePct24h >= 0 ? "var(--green)" : "var(--red)") +
            '">' +
            (wallet.changePct24h >= 0 ? "+" : "") +
            wallet.changePct24h.toFixed(1) +
            '%</div><div class="wcbs-key">24h Change</div></div></div><div class="wc-activity"><div class="wca-fill" style="width:' +
            wallet.activityScore +
            "%;background:" +
            wallet.color +
            '"></div></div>';
          whaleList.appendChild(item);
        });
      }
      var stream = byId("act-stream");
      if (stream) {
        stream.innerHTML = "";
        whaleEventsByFilter(data.events).forEach(function (event) {
          var typeClass =
            event.type === "BUY"
              ? "ait-buy"
              : event.type === "SELL"
                ? "ait-sell"
                : event.type === "STAKE"
                  ? "ait-stake"
                  : event.type === "MOVE"
                    ? "ait-move"
                    : "ait-vote";
          var color =
            event.type === "BUY"
              ? "var(--green)"
              : event.type === "SELL"
                ? "var(--red)"
                : event.type === "STAKE"
                  ? "var(--cyan)"
                  : event.type === "MOVE"
                    ? "var(--orange)"
                    : "var(--purple)";
          var icon =
            event.type === "BUY"
              ? "💰"
              : event.type === "SELL"
                ? "📤"
                : event.type === "STAKE"
                  ? "🔒"
                  : event.type === "MOVE"
                    ? "↔"
                    : "🗳";
          var row = document.createElement("div");
          row.className = "act-item";
          row.innerHTML =
            '<div class="ai-icon" style="background:rgba(255,255,255,0.05)">' +
            icon +
            '</div><div class="ai-body"><div class="ai-top"><span class="ai-wallet" style="color:' +
            color +
            '">' +
            event.wallet +
            '</span><span class="ai-type ' +
            typeClass +
            '">' +
            event.type +
            '</span></div><div class="ai-action">' +
            event.detail +
            '</div><div class="ai-meta">' +
            event.address +
            " · Block #" +
            event.blockNumber +
            '</div></div><div class="ai-amount"><div class="ai-val" style="color:' +
            color +
            '">' +
            event.amountDisplay +
            '</div><div class="ai-usd">' +
            event.amountUsdDisplay +
            '</div><div class="ai-time">' +
            event.timeAgo +
            "</div></div>";
          stream.appendChild(row);
        });
      }
      var toast = byId("alert-toast");
      if (toast && data.events[0]) {
        toast.innerHTML =
          '<div class="at-title">Large Move Detected</div><div class="at-body"><span class="at-val">' +
          data.events[0].wallet +
          "</span> latest event: <span class=\"at-val\" style=\"color:var(--green)\">" +
          data.events[0].amountDisplay +
          "</span> <span class=\"at-val\" style=\"color:var(--gold)\">(" +
          data.events[0].amountUsdDisplay +
          ')</span><div style="margin-top:8px;font-size:10px;">' +
          data.events[0].timeAgo +
          " · Block #" +
          data.events[0].blockNumber +
          "</div></div>";
      }
      api.renderModuleMeta(".page-head", "whales", envelope);
    }
    await load();
    global.setInterval(load, 15000);
  }

  async function initTokenomics(api) {
    async function load() {
      var envelope = await api.getTokenomicsEnvelope({ refresh: true });
      var data = envelope.data;
      setText("#total-supply", fmtNumber(data.totalSupplyX3S));
      setText("#em-rate", fmtX3S(data.dailyEmissionsX3S));
      setText("#halving-days", fmtNumber(data.halvingInDays));
      setText("#burn-val", fmtNumber(data.burnedX3S));
      setText("#burn-rate", "burning ~" + fmtNumber(data.burnRateHourlyX3S) + " X3S/hr");
      setText("#ctr-supply", Math.round(data.lockedSupplyX3S / 1000000) + "M");
      if (byId("em-bar")) {
        var emPct = data.totalSupplyX3S
          ? (data.dailyEmissionsX3S / data.totalSupplyX3S) * 100
          : 0;
        byId("em-bar").style.width = Math.max(2, Math.min(100, emPct * 100)) + "%";
      }
      setText("#mktcap", fmtCompactMoney(data.marketCapUsd));
      setText("#lock-rate", fmtPct(data.lockRatePct));
      setText("#daily-burn", fmtNumber(data.burnDailyX3S));
      setText("#circ-supply", (data.circulatingSupplyX3S / 1000000).toFixed(1) + "M");
      var fdvCell = queryAll(".bs-cell .bsc-val");
      if (fdvCell[1]) setText(fdvCell[1], fmtCompactMoney(data.fdvUsd));
      var supplyBars = queryAll(".supply-bars .sb-row");
      data.allocations.forEach(function (allocation, index) {
        var row = supplyBars[index];
        if (!row) return;
        setText(row.querySelector(".sb-name"), allocation.name);
        setText(row.querySelector(".sb-pct"), allocation.pct + "%");
        var fill = row.querySelector(".sb-fill");
        if (fill) {
          fill.style.width = allocation.pct + "%";
          fill.style.background = allocation.color;
        }
      });
      var vestList = byId("vest-list");
      if (vestList) {
        vestList.innerHTML = "";
        data.vesting.forEach(function (vesting) {
          var row = document.createElement("div");
          row.className = "vest-row";
          row.innerHTML =
            '<div class="vr-top"><span class="vr-name">' +
            vesting.name +
            '</span><span class="vr-date">' +
            vesting.unlockLabel +
            '</span></div><div class="vr-bar-track"><div class="vr-bar" style="width:' +
            vesting.progressPct +
            "%;background:" +
            vesting.color +
            '"></div></div><div class="vr-bottom"><span class="vr-amt" style="color:' +
            vesting.color +
            '">' +
            Math.round(vesting.amountX3S / 1000000) +
            'M X3S</span><span class="vr-status ' +
            (vesting.status === "locked"
              ? "vs-locked"
              : vesting.status === "cliff"
                ? "vs-cliff"
                : "vs-active") +
            '">' +
            String(vesting.status).toUpperCase() +
            "</span></div>";
          vestList.appendChild(row);
        });
      }
      if (byId("unlock-bar")) byId("unlock-bar").style.width = (data.unlock30dX3S / data.totalSupplyX3S) * 100 + "%";
      if (byId("unlock-bar2")) byId("unlock-bar2").style.width = (data.unlock90dX3S / data.totalSupplyX3S) * 100 + "%";
      var eventFeed = byId("event-feed");
      if (eventFeed) {
        eventFeed.innerHTML = "";
        data.events.forEach(function (event) {
          var row = document.createElement("div");
          row.className = "ev-item";
          row.innerHTML =
            '<div style="display:flex;justify-content:space-between;align-items:center;"><span class="ev-type" style="color:var(--gold)">' +
            event.type +
            '</span><span class="ev-amount" style="color:var(--gold)">' +
            fmtNumber(event.amountX3S) +
            "</span></div><div class=\"ev-desc\">" +
            event.detail +
            " · " +
            new Date(event.timestamp).toLocaleTimeString() +
            "</div>";
          eventFeed.appendChild(row);
        });
      }
      if (global.Chart && byId("main-donut")) {
        var ctx = byId("main-donut").getContext("2d");
        if (tokenomicsChart) tokenomicsChart.destroy();
        tokenomicsChart = new global.Chart(ctx, {
          type: "doughnut",
          data: {
            labels: data.allocations.map(function (allocation) {
              return allocation.name;
            }),
            datasets: [
              {
                data: data.allocations.map(function (allocation) {
                  return allocation.amountX3S / 1000000;
                }),
                backgroundColor: data.allocations.map(function (allocation) {
                  return allocation.color;
                }),
                borderWidth: 1,
              },
            ],
          },
          options: {
            responsive: true,
            maintainAspectRatio: true,
            cutout: "72%",
            plugins: {
              legend: { display: false },
            },
          },
        });
      }
      api.renderModuleMeta("nav", "tokenomics", envelope);
    }
    await load();
    global.setInterval(load, 20000);
  }

  async function initScarcityClock(api) {
    var countdownStarted = false;

    global.shareThis = async function () {
      var text = "X3STAR Round III live status: " + global.location.href;
      if (navigator.share) {
        try {
          await navigator.share({ title: "X3STAR Round III", text: text, url: global.location.href });
          return;
        } catch (error) {}
      }
      if (navigator.clipboard) {
        await navigator.clipboard.writeText(text);
      }
      global.alert("Share link copied to clipboard.");
    };

    async function load() {
      var payloads = await Promise.all([
        api.getPresaleEnvelope({ refresh: true }),
        api.getReservationsEnvelope({ refresh: true }),
        api.getProofsEnvelope({ refresh: true }),
        api.getDashboardEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var presale = payloads[0].data;
      var reservations = payloads[1].data;
      var proofs = payloads[2].data;
      var dashboard = payloads[3].data;
      var remainingUsd = Math.max(0, Number(presale.hardCapUsd || 0) - Number(presale.raisedUsd || 0));
      var selloutHours = Math.max(1, Number(proofs.selloutEtaHours || presale.daysRemaining * 24 || 24));
      var perMinuteUsd = Math.max(1, Math.round(remainingUsd / (selloutHours * 60)));
      var pct = Number(((presale.raisedUsd / presale.hardCapUsd) * 100).toFixed(1));
      if (!countdownStarted) {
        countdown(presale.closesAt, {
          days: "#cd-d",
          hours: "#cd-h",
          minutes: "#cd-m",
          seconds: "#cd-s",
        });
        countdownStarted = true;
      }
      setText("#price", "$" + Number(dashboard.token.priceUsd || 0).toFixed(4));
      if (presale.nextRoundPriceUsd) {
        setText("#next-price", "$" + Number(presale.nextRoundPriceUsd).toFixed(2));
        var premiumPct = dashboard.token.priceUsd
          ? Math.round(((presale.nextRoundPriceUsd - dashboard.token.priceUsd) / dashboard.token.priceUsd) * 100)
          : 0;
        setText("#next-premium", (premiumPct >= 0 ? "+" : "") + premiumPct + "%");
      } else {
        setText("#next-price", "--");
        setText("#next-premium", "n/a");
      }
      setText("#raised-num", fmtMoney(presale.raisedUsd));
      setText("#hard-cap", fmtCompactMoney(presale.hardCapUsd));
      setText("#pct-num", pct + "%");
      setText("#s-investors", fmtNumber(presale.investors));
      setText("#s-remaining", fmtCompactMoney(remainingUsd));
      setText("#s-ph", fmtCompactMoney(perMinuteUsd));
      setText("#s-today", fmtCompactMoney(presale.todayUsd));
      setText("#vel-rate", fmtCompactMoney(perMinuteUsd));
      setText(
        "#vel-est",
        fmtCompactMoney(remainingUsd) + " remaining fills in ~" + Math.round(selloutHours) + " hours",
      );
      setText("#s-wallets", fmtNumber(presale.investors));
      setText("#s-genesis-left", proofs.slotsLeft);
      if (byId("prog-fill")) byId("prog-fill").style.width = pct + "%";
      if (byId("mk-0")) setText("#mk-0", "$0");
      if (byId("mk-25")) setText("#mk-25", fmtCompactMoney(presale.hardCapUsd * 0.25));
      if (byId("mk-50")) setText("#mk-50", fmtCompactMoney(presale.hardCapUsd * 0.5));
      if (byId("mk-100")) setText("#mk-100", fmtCompactMoney(presale.hardCapUsd));
      var ticker = byId("ticker");
      if (ticker) {
        var items = reservations.recentCards
          .slice(0, 8)
          .map(function (entry) {
            return (
              '<span class="tick-item"><span class="ti-flag">' +
              entry.flag +
              '</span><span class="ti-name">' +
              entry.name +
              '</span><span class="ti-amt">+' +
              fmtCompactMoney(entry.amountUsd).replace("$", "$") +
              "</span></span><span class=\"ti-sep\">|</span>"
            );
          })
          .join("");
        ticker.innerHTML = items + items;
      }
      api.renderModuleMeta(".top-strip", "scarcity clock", envelope);
    }

    await load();
    global.setInterval(load, 15000);
  }

  async function initEcosystemHeartbeat(api) {
    var ecgFrame = null;

    function startEcg(bpm) {
      var canvas = byId("ecg-canvas");
      if (!canvas || !canvas.getContext || canvas.dataset.started === "true") return;
      canvas.dataset.started = "true";
      var ctx = canvas.getContext("2d");
      function resize() {
        canvas.width = global.innerWidth;
      }
      resize();
      global.addEventListener("resize", resize);
      function draw() {
        var now = Date.now() / 1000;
        var width = canvas.width;
        var height = 80;
        ctx.clearRect(0, 0, width, height);
        ctx.beginPath();
        for (var x = 0; x < width; x += 2) {
          var beatPeriod = 60 / Math.max(60, bpm);
          var t = now + x / 160;
          var phase = (t % beatPeriod) / beatPeriod;
          var y = 40;
          if (phase > 0.45 && phase < 0.48) y = 12;
          else if (phase >= 0.48 && phase < 0.5) y = 70;
          else if (phase >= 0.5 && phase < 0.53) y = 5;
          else if (phase >= 0.53 && phase < 0.56) y = 42;
          else y = 40 + Math.sin((t + x / 90) * 6) * 1.5;
          if (x === 0) ctx.moveTo(x, y);
          else ctx.lineTo(x, y);
        }
        ctx.strokeStyle = "rgba(0,255,106,0.7)";
        ctx.lineWidth = 1.5;
        ctx.stroke();
        ecgFrame = global.requestAnimationFrame(draw);
      }
      draw();
    }

    global.showEmbed = async function () {
      var code =
        '<iframe src="' +
        global.location.origin +
        '/x3star-ecosystem-heartbeat.html" width="400" height="300" frameborder="0"></iframe>';
      if (navigator.clipboard) {
        await navigator.clipboard.writeText(code);
      }
      global.alert("Embed code copied to clipboard.");
    };

    async function load() {
      var payloads = await Promise.all([
        api.getDashboardEnvelope({ refresh: true }),
        api.getStakingEnvelope({ refresh: true }),
        api.getGovernanceEnvelope({ refresh: true }),
        api.getPresaleEnvelope({ refresh: true }),
        api.getNetworkEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var dashboard = payloads[0].data;
      var staking = payloads[1].data;
      var governance = payloads[2].data;
      var presale = payloads[3].data;
      var network = payloads[4].data;
      var bpm = Math.max(96, Math.min(180, 90 + Math.round(Number(network.tps || 0) / 100)));
      var totalValue =
        Number(dashboard.token.volume24hUsd || 0) +
        Number(presale.raisedUsd || 0) +
        Number(staking.totalValueLocked || 0) +
        Number(governance.treasury || 0);
      setText("#beat-bpm", bpm + " BPM");
      setText("#main-num", fmtMoney(totalValue));
      setText("#bd-tvl", fmtCompactMoney(dashboard.token.volume24hUsd));
      setText("#bd-raised", fmtCompactMoney(presale.raisedUsd));
      setText("#bd-staked", fmtCompactMoney(staking.totalValueLocked));
      setText("#bd-grants", fmtCompactMoney(governance.treasury));
      var footer = query(".main > div:last-child");
      if (footer) {
        footer.textContent =
          fmtNumber((network.validators || []).length) +
          " validators · " +
          fmtNumber(network.tps) +
          " TPS · " +
          fmtNumber(dashboard.token.holders) +
          " holders · " +
          fmtNumber(governance.proposalsCount) +
          " proposals · treasury " +
          fmtCompactMoney(governance.treasury);
      }
      startEcg(bpm);
      api.renderModuleMeta(".main", "ecosystem heartbeat", envelope);
    }

    await load();
    global.setInterval(load, 15000);
  }

  async function initOperatorWarRoom(api) {
    async function load() {
      var payloads = await Promise.all([
        api.getNodeHealthEnvelope({ refresh: true }),
        api.getNetworkEnvelope({ refresh: true }),
        api.getGovernanceEnvelope({ refresh: true }),
        api.getPresaleEnvelope({ refresh: true }),
        api.getDashboardEnvelope({ refresh: true }),
        api.getStakingEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var health = payloads[0].data;
      var network = payloads[1].data;
      var governance = payloads[2].data;
      var presale = payloads[3].data;
      var dashboard = payloads[4].data;
      var staking = payloads[5].data;
      var primary = (health.nodes || [])[0];
      if (!primary) return;
      var annualRewardsUsd =
        (Number(primary.stakeX3S || 0) * Number(dashboard.token.priceUsd || 0) * Number(staking.avgApy || 0)) / 100;
      setText("#bonus-apy", "0.0%");
      setText("#tb-node", primary.name + " · " + primary.tier.toUpperCase());
      setText("#tb-apy", Number(staking.avgApy || 0).toFixed(1) + "%");
      setText("#tb-uptime", fmtPct(primary.uptimePct));
      setText("#uptime-val", fmtPct(primary.uptimePct));
      setText("#earned-val", fmtCompactMoney(annualRewardsUsd));
      setText("#blocks-val", "#" + fmtNumber(network.blockNumber));
      setText("#apy-val", Number(staking.avgApy || 0).toFixed(1) + "%");
      setText("#nh-consensus", primary.status ? String(primary.status).toUpperCase() : "UNKNOWN");
      if (byId("nh-block-fill")) byId("nh-block-fill").style.width = Math.min(100, Math.max(10, primary.uptimePct || 0)) + "%";
      setText("#nh-peers", primary.peers ? fmtNumber(primary.peers) + " peers" : "n/a");
      setText("#nh-latency", primary.latencyMs ? primary.latencyMs + "ms" : "n/a");
      setText("#nh-slashes", "0");
      setText("#nh-version", "n/a");
      setText("#nh-staked", fmtNumber(primary.stakeX3S || 0) + " X3S");
      setText("#pending-val", "+" + fmtNumber(Math.round(primary.stakeX3S * 0.012)) + " X3S");
      setText("#net-tps", fmtNumber(network.tps));
      setText("#net-block", "#" + fmtNumber(network.blockNumber));
      setText("#net-vals", fmtNumber(network.validators.length));
      setText("#net-price", "$" + Number(dashboard.token.priceUsd || 0).toFixed(4));
      var networkCards = queryAll(".panel .nkpi-val");
      if (networkCards[6]) setText(networkCards[6], fmtNumber(network.validators.length));
      if (networkCards[7]) setText(networkCards[7], "$" + Number(dashboard.token.priceUsd || 0).toFixed(4));
      setText("#mult-display", Number(staking.avgApy || 0).toFixed(1) + "%");
      setText("#alert-count", governance.activeProposals + " active");
      var govFeed = byId("gov-feed");
      if (govFeed) {
        govFeed.innerHTML = "";
        governance.proposals.slice(0, 4).forEach(function (proposal) {
          var item = document.createElement("div");
          item.className = "gov-item";
          item.innerHTML =
            '<div class="gi-top"><span class="gi-id">' +
            proposal.id +
            '</span><span class="gi-status ' +
            (proposal.status === "active" ? "gs-active" : "gs-new") +
            '">' +
            String(proposal.status).toUpperCase() +
            "</span></div><div class=\"gi-title\">" +
            proposal.title +
            '</div><div class="gi-progress"><div class="gi-fill" style="width:' +
            governanceSupportPct(proposal) +
            '%"></div></div><div class="gi-meta"><span>' +
            governanceSupportPct(proposal) +
            "% support</span><span>" +
            fmtNumber(governanceVoteTotal(proposal)) +
            " votes</span></div>";
          govFeed.appendChild(item);
        });
      }
      var alerts = [
        {
          badge: "ab-vote",
          label: "GOVERNANCE",
          msg:
            governance.proposals[0].id +
            " is active with " +
            governanceSupportPct(governance.proposals[0]) +
            "% support.",
        },
        {
          badge: "ab-info",
          label: "REWARDS",
          msg: "Projected annual rewards " + fmtCompactMoney(annualRewardsUsd) + " at current APY.",
        },
        {
          badge: "ab-info",
          label: "PRESALE",
          msg: presale.tiers[0].slotsLeft + " Genesis slots remain in Round III.",
        },
      ];
      var alertFeed = byId("alert-feed");
      if (alertFeed) {
        alertFeed.innerHTML = "";
        alerts.forEach(function (alertItem, index) {
          var item = document.createElement("div");
          item.className = "alert-item";
          item.innerHTML =
            '<div class="ai-badge ' +
            alertItem.badge +
            '">' +
            alertItem.label +
            '</div><div class="ai-msg">' +
            alertItem.msg +
            '</div><div class="ai-time">' +
            (index === 0 ? "just now" : index * 2 + " hours ago") +
            "</div>";
          alertFeed.appendChild(item);
        });
      }
      var blockFeed = byId("block-feed");
      if (blockFeed) {
        blockFeed.innerHTML = "";
        (network.transactions || []).slice(0, 8).forEach(function (tx) {
          var item = document.createElement("div");
          item.style.cssText =
            "display:flex;justify-content:space-between;padding:3px 0;border-bottom:1px solid rgba(255,255,255,0.04);";
          item.innerHTML =
            '<span style="color:rgba(0,200,100,0.5)">#' +
            fmtNumber(tx.blockNumber || network.blockNumber) +
            '</span><span style="color:var(--muted)">' +
            tx.type +
            '</span><span style="color:rgba(0,200,100,0.4)">' +
            (tx.amount || "live") +
            '</span><span style="color:var(--gold)">' +
            tx.hash +
            "</span>";
          blockFeed.appendChild(item);
        });
      }
      var refTree = query(".ref-tree");
      if (refTree) {
        refTree.innerHTML =
          '<div class="rt-label">Referral State</div><div style="font-size:10px;color:var(--muted);line-height:1.7;">No referral ledger is available from the authoritative business store yet. This panel remains live but intentionally degraded until referral records are persisted server-side.</div>';
      }
      var earnSummary = query(".earn-summary");
      if (earnSummary) {
        earnSummary.innerHTML =
          '<div class="es-grid"><div class="esg"><div class="esg-val" style="color:var(--gold)">$0</div><div class="esg-key">Ref Bonus/yr</div></div><div class="esg"><div class="esg-val" style="color:var(--grn)">0</div><div class="esg-key">Referred</div></div><div class="esg"><div class="esg-val">' +
          Number(staking.avgApy || 0).toFixed(1) +
          '%</div><div class="esg-key">Base APY</div></div><div class="esg"><div class="esg-val" style="color:var(--gold)">unavailable</div><div class="esg-key">Next level</div></div></div><button class="share-ref-btn" disabled style="opacity:0.5;cursor:not-allowed;">REFERRAL STATE UNAVAILABLE</button>';
      }
      var clock = byId("net-clock");
      if (clock) {
        clock.textContent = new Date().toISOString().slice(11, 19) + " UTC";
      }
      api.renderModuleMeta(".topbar", "operator war room", envelope);
    }

    await load();
    global.setInterval(load, 15000);
  }

  async function initMissionTerminal(api) {
    var latestMissionState = null;
    var countdownStarted = false;

    global.handleCmd = function (event) {
      if (event.key !== "Enter") return;
      var input = byId("terminal");
      if (!input) return;
      var cmd = String(input.value || "").trim().toLowerCase();
      input.value = "";
      if (!cmd) return;
      var feed = byId("mission-log");
      if (!feed) return;
      function appendLine(text, className) {
        var row = document.createElement("div");
        row.className = "log-line";
        row.innerHTML =
          '<span class="ll-time">[' +
          new Date().toISOString().slice(11, 19) +
          ']</span><span class="' +
          className +
          '">' +
          text +
          "</span>";
        feed.insertBefore(row, feed.firstChild);
      }
      appendLine("> " + cmd, "ll-info");
      if (!latestMissionState) return;
      var responses = {
        help: "Available: status, tps, validators, price, block, presale, clear",
        status:
          latestMissionState.health.status.toUpperCase() +
          " · " +
          latestMissionState.network.validators.length +
          " validators · " +
          latestMissionState.network.tps +
          " TPS",
        tps: "Current TPS: " + fmtNumber(latestMissionState.network.tps),
        validators: "Indexed validators: " + fmtNumber(latestMissionState.network.validators.length),
        price: "X3S: $" + Number(latestMissionState.dashboard.token.priceUsd || 0).toFixed(4),
        block: "Current block: #" + fmtNumber(latestMissionState.network.blockNumber),
        presale:
          "Round III: " +
          fmtMoney(latestMissionState.presale.raisedUsd) +
          "/" +
          fmtMoney(latestMissionState.presale.hardCapUsd) +
          " · " +
          latestMissionState.presale.daysRemaining +
          " days left",
      };
      if (cmd === "clear") {
        feed.innerHTML = "";
        return;
      }
      appendLine(responses[cmd] || "Command not found. Type 'help' for supported commands.", responses[cmd] ? "ll-ok" : "ll-warn");
    };

    function appendMissionLines(lines) {
      var feed = byId("mission-log");
      if (!feed) return;
      feed.innerHTML = "";
      lines.forEach(function (entry) {
        var row = document.createElement("div");
        row.className = "log-line";
        row.innerHTML =
          '<span class="ll-time">[' +
          entry.time +
          ']</span><span class="' +
          entry.className +
          '">' +
          entry.message +
          "</span>";
        feed.appendChild(row);
      });
    }

    async function load() {
      var payloads = await Promise.all([
        api.getHealthEnvelope({ refresh: true }),
        api.getDashboardEnvelope({ refresh: true }),
        api.getNetworkEnvelope({ refresh: true }),
        api.getPresaleEnvelope({ refresh: true }),
        api.getGovernanceEnvelope({ refresh: true }),
        api.getWhalesEnvelope({ refresh: true }),
        api.getStakingEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var health = payloads[0];
      var dashboard = payloads[1].data;
      var network = payloads[2].data;
      var presale = payloads[3].data;
      var governance = payloads[4].data;
      var whales = payloads[5].data;
      var staking = payloads[6].data;
      latestMissionState = {
        health: health,
        dashboard: dashboard,
        network: network,
        presale: presale,
      };
      setText("#utc-clock", new Date().toISOString().slice(11, 19) + " UTC");
      setText("#hdr-block", "#" + fmtNumber(network.blockNumber));
      var headerVals = queryAll(".hs-val");
      if (headerVals[1]) setText(headerVals[1], "$" + Number(dashboard.token.priceUsd || 0).toFixed(4));
      if (headerVals[2]) setText(headerVals[2], String(envelope.status).toUpperCase());
      var heroName = query(".lh-name");
      if (heroName) heroName.textContent = "ROUND III CLOSE";
      var heroDesc = query(".lh-desc");
      if (heroDesc) {
        heroDesc.textContent =
          "Presale close checkpoint · " +
          fmtMoney(presale.raisedUsd) +
          " raised · " +
          presale.tiers[0].slotsLeft +
          " Genesis slots left";
      }
      if (!countdownStarted) {
        countdown(presale.closesAt, {
          days: "#cd-d",
          hours: "#cd-h",
          minutes: "#cd-m",
          seconds: "#cd-s",
        });
        countdownStarted = true;
      }
      if (byId("launch-prog")) {
        var pct = ((presale.raisedUsd / presale.hardCapUsd) * 100).toFixed(1);
        byId("launch-prog").style.width = pct + "%";
      }
      if (byId("r3-bar")) {
        var pct2 = ((presale.raisedUsd / presale.hardCapUsd) * 100).toFixed(1);
        byId("r3-bar").style.width = pct2 + "%";
        setText("#r3-pct", pct2 + "%");
      }
      setText("#mainnet-pct", "--");
      if (byId("mainnet-prog")) byId("mainnet-prog").style.width = "0%";
      var sysVals = queryAll(".sys-item .si-val");
      if (sysVals[0]) setText(sysVals[0], "● " + String(health.status).toUpperCase());
      if (sysVals[1]) setText(sysVals[1], "● " + fmtNumber(network.validators.length) + " ONLINE");
      if (sysVals[2]) setText(sysVals[2], "● " + fmtNumber(network.tps) + " TPS");
      if (sysVals[3]) setText(sysVals[3], "● " + network.finalitySeconds + "s AVG");
      if (sysVals[10]) setText(sysVals[10], "● " + Number(staking.avgApy || 0).toFixed(1) + "% APY");
      if (sysVals[11]) setText(sysVals[11], "● " + governance.proposalsCount + " LIVE");
      var utilizationPct = Math.max(1, Math.min(100, Math.round((Number(network.tps || 0) / 10000) * 100)));
      setText("#tps-pct", utilizationPct + "%");
      if (byId("tps-bar")) byId("tps-bar").style.width = utilizationPct + "%";
      appendMissionLines(
        [
          {
            className: "ll-ok",
            message:
              "Current network throughput " +
              fmtNumber(network.tps) +
              " TPS across " +
              fmtNumber(network.validators.length) +
              " indexed validators.",
          },
          {
            className: "ll-info",
            message:
              governance.proposals[0].id +
              " live with " +
              governanceSupportPct(governance.proposals[0]) +
              "% support.",
          },
          {
            className: "ll-ok",
            message:
              "Round III now at " +
              fmtMoney(presale.raisedUsd) +
              " raised with " +
              presale.tiers[0].slotsLeft +
              " Genesis slots left.",
          },
          {
            className: "ll-alert",
            message:
              whales.events[0].wallet +
              " latest movement: " +
              whales.events[0].amountDisplay +
              " at block #" +
              whales.events[0].blockNumber +
              ".",
          },
        ].map(function (entry, index) {
          return {
            className: entry.className,
            message: entry.message,
            time: new Date(Date.now() - index * 60000).toISOString().slice(11, 19),
          };
        }),
      );
      api.renderModuleMeta(".header", "mission terminal", envelope);
    }

    await load();
    global.setInterval(load, 15000);
    global.setInterval(function () {
      setText("#utc-clock", new Date().toISOString().slice(11, 19) + " UTC");
    }, 1000);
  }

  async function initArbitrageEngine(api) {
    async function load() {
      var payloads = await Promise.all([
        api.getBenchmarkEnvelope("overview", { refresh: true }),
        api.getNetworkEnvelope({ refresh: true }),
        api.getDashboardEnvelope({ refresh: true }),
        api.getWhalesEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var overview = payloads[0].data;
      var network = payloads[1].data;
      var dashboard = payloads[2].data;
      var whales = payloads[3].data;
      var liveTps = Number(overview.live.summary.combined_tps || 0);
      setText("#tps-counter", fmtNumber(liveTps));
      var ticker = byId("ticker");
      if (ticker) {
        var items = [
          "LIVE BENCH: " + fmtNumber(liveTps) + " TPS",
          "SCHEDULER: " + fmtNumber(overview.tps.summary.scheduler.schedules_per_sec) + "/sec",
          "NETWORK: " + fmtNumber(network.tps) + " TPS",
          "PRICE: $" + Number(dashboard.token.priceUsd || 0).toFixed(4),
          "FLOW: " + whales.events[0].amountDisplay + " " + whales.events[0].type,
        ];
        var tickerHtml = items
          .map(function (item, index) {
            return (
              '<div class="ti-item"><span class="' +
              (index % 2 === 0 ? "ti-hot" : "ti-green") +
              '">' +
              item +
              '</span><span style="color:rgba(57,255,20,0.1)">|</span></div>'
            );
          })
          .join("");
        ticker.innerHTML = tickerHtml + tickerHtml;
      }
      var benchRows = queryAll(".bench-row");
      var metrics = [
        {
          label: "X3 Live",
          value: liveTps,
          color: "linear-gradient(90deg,var(--hot),var(--gold))",
          note: "artifact",
        },
        {
          label: "Scheduler",
          value: Number(overview.tps.summary.scheduler.schedules_per_sec || 0),
          color: "var(--cyan)",
          note: "sched/sec",
        },
        {
          label: "SHA-256 GPU",
          value: Number(overview.crypto.summary.benchmarks[5].throughput_hashes_per_sec || 0),
          color: "var(--green)",
          note: "hashes/sec",
        },
        {
          label: "PoH GPU",
          value: Number(overview.crypto.summary.benchmarks[6].throughput_hashes_per_sec || 0),
          color: "#A060FF",
          note: "hashes/sec",
        },
        {
          label: "Ed25519 GPU",
          value: Number(overview.crypto.summary.benchmarks[7].throughput_sigs_per_sec || 0),
          color: "#FF6B6B",
          note: "sigs/sec",
        },
        {
          label: "Network TPS",
          value: Number(network.tps || 0),
          color: "rgba(255,255,255,0.3)",
          note: "live",
        },
      ];
      var maxMetric = Math.max.apply(
        null,
        metrics.map(function (metric) {
          return metric.value;
        }),
      );
      benchRows.forEach(function (row, index) {
        var metric = metrics[index];
        if (!metric) return;
        var chain = row.querySelector(".br-chain");
        var bar = row.querySelector(".br-bar");
        var val = row.querySelector(".br-val");
        var note = row.querySelector(".br-note");
        if (chain) setText(chain, metric.label);
        if (bar) {
          bar.style.width = ((metric.value / maxMetric) * 100).toFixed(2) + "%";
          bar.style.background = metric.color;
        }
        if (val) setText(val, fmtNumber(Math.round(metric.value)));
        if (note) setText(note, metric.note);
      });
      var names = queryAll(".arb-flow .ac-name");
      var prices = queryAll(".arb-flow .ac-price");
      if (names[0]) setText(names[0], "NETWORK TPS");
      if (names[1]) setText(names[1], "X3KERNEL");
      if (names[2]) setText(names[2], "BENCH TPS");
      if (prices[0]) setText(prices[0], fmtNumber(network.tps));
      if (prices[1]) setText(prices[1], fmtNumber(liveTps));
      var delta = liveTps - Number(network.tps || 0);
      setText("#spread-display", "Delta: +" + fmtNumber(delta) + " → HEADROOM");
      api.renderModuleMeta("nav", "arbitrage engine", envelope);
    }

    await load();
    global.setInterval(load, 15000);
  }

  function benchmarkPageConfig() {
    var page = (global.location.pathname.split("/").pop() || "").toLowerCase();
    var configs = {
      "blockchain-stress-test.html": {
        kind: "stress-test",
        title: "Stress Test Artifact",
        kicker: "Saved throughput artifact plus live network health",
      },
      "blockchain-stress-test(1).html": {
        kind: "stress-test",
        title: "Stress Test Artifact",
        kicker: "Saved throughput artifact plus live network health",
      },
      "chainbench-pro.html": {
        kind: "chainbench",
        title: "Chainbench RPC Report",
        kicker: "Read-only benchmark console sourced from saved reports",
      },
      "chainbench-ultimate.html": {
        kind: "overview",
        title: "Benchmark Overview",
        kicker: "Artifact-backed suite summary plus live network snapshot",
      },
      "chainbench-ultimate(1).html": {
        kind: "overview",
        title: "Benchmark Overview",
        kicker: "Artifact-backed suite summary plus live network snapshot",
      },
    };
    return configs[page];
  }

  async function initBenchmarkConsole(api) {
    function metricCards(config, specific, overview, network, health) {
      if (config.kind === "stress-test") {
        return [
          { label: "Combined TPS", value: fmtNumber(specific.summary.combined_tps) },
          { label: "Duration", value: specific.summary.duration_seconds + "s" },
          { label: "SVM Processed", value: fmtNumber(specific.summary.svm_processed) },
          { label: "EVM Processed", value: fmtNumber(specific.summary.evm_processed) },
        ];
      }
      if (config.kind === "chainbench") {
        return [
          { label: "Checks Passed", value: fmtNumber(specific.summary.ok) },
          { label: "Total Checks", value: fmtNumber(specific.summary.total) },
          { label: "Skipped", value: fmtNumber(specific.summary.skipped) },
          { label: "Live TPS Artifact", value: fmtNumber(overview.live.summary.combined_tps) },
        ];
      }
      return [
        { label: "Live TPS Artifact", value: fmtNumber(overview.live.summary.combined_tps) },
        { label: "RPC Checks", value: fmtNumber(overview.chainbench.summary.ok) + "/" + fmtNumber(overview.chainbench.summary.total) },
        { label: "Scheduler Rate", value: fmtNumber(overview.tps.summary.scheduler.schedules_per_sec) + "/sec" },
        { label: "Network Status", value: String(health.status).toUpperCase() + " · " + fmtNumber(network.tps) + " TPS" },
      ];
    }

    function renderRows(target, rows) {
      if (!target) return;
      target.innerHTML = rows
        .map(function (row) {
          return (
            '<tr><td class="table-key">' +
            row.key +
            '</td><td class="table-val">' +
            row.value +
            "</td></tr>"
          );
        })
        .join("");
    }

    async function load() {
      var config = benchmarkPageConfig();
      if (!config) return;
      var payloads = await Promise.all([
        api.getBenchmarkEnvelope(config.kind, { refresh: true }),
        api.getBenchmarkEnvelope("overview", { refresh: true }),
        api.getNetworkEnvelope({ refresh: true }),
        api.getHealthEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var specific = payloads[0].data;
      var overview = payloads[1].data;
      var network = payloads[2].data;
      var health = payloads[3];
      setText("#bench-title", config.title);
      setText("#bench-kicker", config.kicker);
      setText("#bench-status", String(envelope.status).toUpperCase());
      setText("#bench-updated", "Updated " + new Date(envelope.lastUpdated).toLocaleString());
      setText("#bench-source", specific && specific.sourceFile ? specific.sourceFile : "artifact unavailable");
      metricCards(config, specific || { summary: {} }, overview, network, health).forEach(function (card, index) {
        setText("#bench-card-" + (index + 1) + "-label", card.label);
        setText("#bench-card-" + (index + 1) + "-value", card.value);
      });
      renderRows(byId("bench-summary"), [
        { key: "Benchmark status", value: String(envelope.status).toUpperCase() },
        { key: "Current network TPS", value: fmtNumber(network.tps) },
        { key: "Current validators", value: fmtNumber(network.validators.length) },
        { key: "Health source", value: health.source },
      ]);
      renderRows(byId("bench-artifacts"), [
        {
          key: "RPC checks",
          value:
            fmtNumber(overview.chainbench.summary.ok) +
            "/" +
            fmtNumber(overview.chainbench.summary.total) +
            " passed",
        },
        { key: "Live benchmark", value: fmtNumber(overview.live.summary.combined_tps) + " combined TPS" },
        { key: "Scheduler", value: fmtNumber(overview.tps.summary.scheduler.schedules_per_sec) + " schedules/sec" },
        {
          key: "Fastest crypto op",
          value: fmtNumber(Math.round(overview.crypto.summary.benchmarks[6].throughput_hashes_per_sec)) + " hashes/sec",
        },
      ]);
      var detailRows = [];
      if (config.kind === "stress-test" && specific && specific.summary) {
        detailRows = Object.keys(specific.summary).map(function (key) {
          return { key: key, value: fmtNumber(specific.summary[key]) };
        });
      } else if (config.kind === "chainbench" && specific && specific.summary) {
        detailRows = Object.keys(specific.summary).map(function (key) {
          return { key: key, value: fmtNumber(specific.summary[key]) };
        });
      } else {
        detailRows = overview.crypto.summary.benchmarks.slice(0, 6).map(function (benchmark) {
          return {
            key: benchmark.operation,
            value: fmtNumber(Math.round(benchmark.ops_per_sec || benchmark.throughput_hashes_per_sec || benchmark.throughput_sigs_per_sec || 0)),
          };
        });
      }
      renderRows(byId("bench-details"), detailRows);
      var feed = byId("bench-feed");
      if (feed) {
        feed.innerHTML = (network.transactions || [])
          .slice(0, 6)
          .map(function (tx) {
            return (
              '<div class="feed-row"><span class="feed-type">' +
              tx.type +
              '</span><span class="feed-detail">' +
              tx.detail +
              '</span><span class="feed-hash">' +
              tx.hash +
              "</span></div>"
            );
          })
          .join("");
      }
      api.renderModuleMeta(".bench-shell", "benchmark console", envelope);
    }

    await load();
    global.setInterval(load, 20000);
  }

  async function initGrantHub(api) {
    async function load() {
      var envelope = await api.getGrantsEnvelope({ refresh: true });
      var data = envelope.data || {};
      var summary = data.summary || {};
      setText("#grant-total-pool", fmtCompactMoney(summary.totalPoolUsd));
      setText("#grant-total-programs", fmtNumber(summary.totalPrograms || 0));
      setText("#grant-largest", fmtCompactMoney(summary.largestGrantUsd));
      setText("#grant-ready", fmtNumber((summary.statusCounts && summary.statusCounts.ready) || 0));
      setText("#grant-preparing", fmtNumber((summary.statusCounts && summary.statusCounts.preparing) || 0));
      setText("#grant-review", fmtNumber((summary.statusCounts && summary.statusCounts.review) || 0));
      setText("#grant-programs-count", fmtNumber(summary.totalPrograms || 0));
      var grid = byId("grant-grid");
      if (grid) {
        grid.innerHTML = "";
        (data.programs || []).forEach(function (program) {
          var card = document.createElement("div");
          card.className = "grant-card";
          var statusClass =
            program.status === "ready"
              ? "sb-open"
              : program.status === "review"
                ? "sb-apply"
                : program.status === "preparing"
                  ? "sb-prep"
                  : "sb-done";
          card.innerHTML =
            '<div class="gc-top"><div class="gc-org-icon">⬡</div><div class="gc-org"><div class="gc-org-name">' +
            escapeHtml(program.organization || "Grant Program") +
            '</div><div class="gc-grant-name">' +
            escapeHtml(program.name || "Grant Track") +
            '</div></div><div class="gc-status"><span class="status-badge ' +
            statusClass +
            '">' +
            String(program.status || "open").toUpperCase() +
            "</span></div></div><div class=\"gc-amount\">" +
            fmtCompactMoney(program.amountUsd) +
            '</div><div class="gc-type">' +
            escapeHtml(program.category || "program") +
            '</div><div class="gc-desc">' +
            escapeHtml(program.summary || "Details available in grant records.") +
            '</div><div class="gc-fit">' +
            (program.tags || [])
              .map(function (tag) {
                return '<span class="fit-tag">' + escapeHtml(tag) + "</span>";
              })
              .join("") +
            '</div><button class="gc-cta">View Program</button>';
          grid.appendChild(card);
        });
        if (!grid.children.length) {
          grid.innerHTML = '<div style="padding:24px;color:var(--ink3);font-size:12px;">No grant programs available.</div>';
        }
      }
      api.renderModuleMeta(".hero", "grant hub", envelope);
    }
    await load();
    global.setInterval(load, 30000);
  }

  async function initGrantMissionControl(api) {
    async function load() {
      var envelope = await api.getGrantsEnvelope({ refresh: true });
      var data = envelope.data || {};
      var apps = data.applications || [];
      var poolUsd = (data.summary && data.summary.totalPoolUsd) || 0;
      var distributedUsd = apps
        .filter(function (app) { return app.status === "approved" || app.status === "complete"; })
        .reduce(function (sum, app) { return sum + Number(app.requestedUsd || app.amountUsd || 0); }, 0);
      var activeCount = apps.filter(function (app) { return app.status !== "complete"; }).length;
      setText("#gm-pool", fmtCompactMoney(poolUsd));
      setText("#gm-active", fmtNumber(activeCount));
      setText("#gm-distributed", fmtCompactMoney(distributedUsd));

      var columns = {
        review: byId("gm-col-review"),
        building: byId("gm-col-building"),
        milestone: byId("gm-col-milestone"),
        payout: byId("gm-col-payout"),
        complete: byId("gm-col-complete"),
      };
      Object.keys(columns).forEach(function (key) {
        if (columns[key]) columns[key].innerHTML = "";
      });
      var counts = { review: 0, building: 0, milestone: 0, payout: 0, complete: 0 };
      apps.forEach(function (app) {
        var status = String(app.status || "review").toLowerCase();
        var bucket =
          status.indexOf("review") !== -1
            ? "review"
            : status.indexOf("build") !== -1
              ? "building"
              : status.indexOf("milestone") !== -1
                ? "milestone"
                : status.indexOf("payout") !== -1
                  ? "payout"
                  : status.indexOf("complete") !== -1
                    ? "complete"
                    : status.indexOf("approved") !== -1
                      ? "payout"
                      : "review";
        counts[bucket] += 1;
        var card = document.createElement("div");
        card.className = "grant-card";
        var progress = Number(app.progressPct || 0);
        card.innerHTML =
          '<div class="gc-top"><span class="gc-icon">⬡</span><span class="gc-cat" style="background:rgba(255,255,255,0.06);color:var(--teal);border:1px solid rgba(255,255,255,0.1)">' +
          escapeHtml(app.category || "Grant") +
          '</span></div><div class="gc-title">' +
          escapeHtml(app.projectName || app.name || "Grant Application") +
          '</div><div class="gc-team">by ' +
          escapeHtml(app.organization || "Applicant") +
          '</div><div class="gc-progress"><div class="gcp-labels"><span>Progress</span><span>' +
          (progress ? progress + "%" : "--") +
          '</span></div><div class="gcp-bar-wrap"><div class="gcp-fill" style="width:' +
          progress +
          '%;background:var(--teal)"></div></div></div><div class="gc-meta">' +
          '<span class="gc-tag">' +
          escapeHtml(app.status || "review") +
          "</span></div><div class=\"gc-amount\" style=\"color:var(--gold)\">" +
          fmtCompactMoney(app.requestedUsd || app.amountUsd || 0) +
          "</div>";
        if (columns[bucket]) columns[bucket].appendChild(card);
      });
      setText("#gm-review", fmtNumber(counts.review));
      setText("#gm-building", fmtNumber(counts.building));
      setText("#gm-milestone", fmtNumber(counts.milestone));
      setText("#gm-payout", fmtNumber(counts.payout));
      setText("#gm-complete", fmtNumber(counts.complete));
      setText("#gm-review-count", fmtNumber(counts.review));
      setText("#gm-building-count", fmtNumber(counts.building));
      setText("#gm-milestone-count", fmtNumber(counts.milestone));
      setText("#gm-payout-count", fmtNumber(counts.payout));
      setText("#gm-complete-count", fmtNumber(counts.complete));
      api.renderModuleMeta(".page-head", "grant mission control", envelope);
    }
    await load();
    global.setInterval(load, 30000);
  }

  async function initBountyBoard(api) {
    async function load() {
      var envelope = await api.getBountiesEnvelope({ refresh: true });
      var data = envelope.data || {};
      var summary = data.summary || {};
      setText("#bounty-pool", fmtCompactMoney(summary.totalPoolUsd) + " in open bounties");
      setText("#bounty-open", fmtNumber(summary.openCount || 0));
      setText("#bounty-total-pool", fmtCompactMoney(summary.totalPoolUsd));
      var claimedTotal = (data.bounties || [])
        .filter(function (bounty) { return bounty.status === "claimed"; })
        .reduce(function (sum, bounty) { return sum + Number(bounty.rewardUsd || 0); }, 0);
      setText("#bounty-claimed", fmtCompactMoney(claimedTotal));
      setText("#bounty-builders", fmtNumber(summary.totalCount || 0));
      setText("#bounty-closing", fmtNumber(summary.openCount || 0));
      var grid = byId("bounty-grid");
      if (grid) {
        grid.innerHTML = "";
        (data.bounties || []).forEach(function (bounty) {
          var statusClass =
            bounty.status === "hot" ? "bs-hot" : bounty.status === "claimed" ? "bs-claimed" : "bs-open";
          var statusLabel =
            bounty.status === "hot" ? "HOT" : bounty.status === "claimed" ? "IN PROGRESS" : "OPEN";
          var card = document.createElement("div");
          card.className = "b-card";
          var rewardUsd = Number(bounty.rewardUsd || 0);
          var rewardX3S = Number(bounty.rewardX3S || 0);
          var progress = bounty.progress || null;
          card.innerHTML =
            '<div class="bc-top"><span class="bc-cat" style="background:rgba(255,183,0,0.06);color:var(--amber);border:1px solid rgba(255,183,0,0.12)">' +
            escapeHtml(bounty.category || "Bounty") +
            '</span><span class="bc-status ' +
            statusClass +
            '">' +
            statusLabel +
            "</span></div><div class=\"bc-title\">" +
            escapeHtml(bounty.title) +
            '</div><div class="bc-desc">' +
            escapeHtml(bounty.description || "") +
            '</div><div class="bc-tags">' +
            (bounty.tags || []).map(function (tag) {
              return '<span class="bc-tag">' + escapeHtml(tag) + "</span>";
            }).join("") +
            "</div>" +
            (progress
              ? '<div class="bc-progress"><div class="bcp-label"><span>' +
                escapeHtml(progress.label || "Progress") +
                "</span><span>" +
                Number(progress.pct || 0) +
                '%</span></div><div class="bcp-bar"><div class="bcp-fill" style="width:' +
                Number(progress.pct || 0) +
                '%;background:var(--amber)"></div></div></div>'
              : "") +
            '<div class="bc-footer"><div><div class="bc-reward">$' +
            fmtNumber(rewardUsd) +
            '</div><div class="bc-reward-sub">' +
            fmtNumber(rewardX3S) +
            ' X3S</div><div class="bc-deadline">' +
            escapeHtml(bounty.deadlineLabel || "Rolling") +
            "</div></div><button class=\"bc-claim-btn " +
            (bounty.status === "claimed" ? "taken" : "") +
            "\">" +
            (bounty.status === "claimed" ? "View Progress" : "Claim Bounty") +
            "</button></div>";
          grid.appendChild(card);
        });
        if (!grid.children.length) {
          grid.innerHTML = '<div style="color:var(--muted);font-size:12px;">No bounties available.</div>';
        }
      }
      api.renderModuleMeta(".page", "bounty board", envelope);
    }
    await load();
    global.setInterval(load, 30000);
  }

  async function initHallOfFame(api) {
    async function load() {
      var envelope = await api.getLeaderboardEnvelope({ refresh: true });
      var data = envelope.data || {};
      var validators = data.validators || [];
      var podium = byId("podium");
      if (podium) {
        podium.innerHTML = "";
        validators.slice(0, 3).forEach(function (validator, index) {
          var slot = document.createElement("div");
          slot.className = "podium-slot slot-" + (index + 1);
          var score = validator.score || 0;
          slot.innerHTML =
            '<div class="ps-card"><div class="ps-rank">#' +
            (index + 1) +
            '</div><div class="ps-avatar">' +
            String(validator.name || "?").slice(0, 2).toUpperCase() +
            '</div><div class="ps-name">' +
            escapeHtml(validator.name || "Validator") +
            '</div><div class="ps-node">' +
            escapeHtml(validator.id || "") +
            '</div><div class="ps-stats"><div class="pss-row"><span class="pss-key">Uptime</span><span class="pss-val">' +
            fmtPct(validator.uptimePct || 0) +
            '</span></div><div class="pss-row"><span class="pss-key">Stake</span><span class="pss-val">' +
            fmtNumber(validator.stakeX3S || 0) +
            '</span></div><div class="pss-row"><span class="pss-key">TPS</span><span class="pss-val">' +
            fmtNumber(validator.tps || 0) +
            '</span></div></div><div class="ps-score-label">Score</div><div class="ps-score">' +
            fmtNumber(score) +
            "</div></div>";
          podium.appendChild(slot);
        });
      }
      var body = byId("lb-body");
      if (body) {
        body.innerHTML = "";
        if (!validators.length) {
          body.innerHTML =
            '<tr class="lb-row"><td colspan="7" style="color:var(--muted);">Awaiting live validator leaderboard feed.</td></tr>';
        } else {
          validators.slice(0, 12).forEach(function (validator, index) {
            var tr = document.createElement("tr");
            tr.className = "lb-row";
            tr.innerHTML =
              '<td><div class="rank-col"><div class="rank-num">' +
              (index + 1) +
              '</div><div class="rank-flag">' +
              escapeHtml(validator.flag || "⬡") +
              '</div><div><div class="rank-name">' +
              escapeHtml(validator.name || "Validator") +
              '</div><div class="rank-node">' +
              escapeHtml(validator.id || "") +
              '</div></div></div></td><td>' +
              escapeHtml(String(validator.tier || "node").toUpperCase()) +
              '</td><td>' +
              fmtPct(validator.uptimePct || 0) +
              '</td><td>' +
              fmtNumber(validator.stakeX3S || 0) +
              '</td><td>' +
              fmtNumber(validator.tps || 0) +
              '</td><td><div class="merit-badges"><span class="merit">ACTIVE</span></div></td><td>' +
              fmtNumber(validator.score || 0) +
              "</td>";
            body.appendChild(tr);
          });
        }
      }
      api.renderModuleMeta(".hero", "hall of fame", envelope);
    }
    await load();
    global.setInterval(load, 30000);
  }

  async function initLeaderboardArena(api) {
    async function load() {
      var payloads = await Promise.all([
        api.getLeaderboardEnvelope({ refresh: true }),
        api.getStakingEnvelope({ refresh: true }),
      ]);
      var envelope = payloads[0];
      var data = payloads[0].data || {};
      var staking = payloads[1].data || {};
      function buildBoard(containerId, rows, color) {
        var cont = byId(containerId);
        if (!cont) return;
        cont.innerHTML = "";
        rows.forEach(function (row, index) {
          var div = document.createElement("div");
          div.className = "lb-row " + (index < 3 ? "rank-" + (index + 1) : "");
          div.innerHTML =
            '<div class="lb-rank ' +
            (index === 0 ? "r1" : index === 1 ? "r2" : index === 2 ? "r3" : "rn") +
            '">' +
            (index < 3 ? ["🥇", "🥈", "🥉"][index] : index + 1) +
            '</div><div class="lb-avatar" style="background:rgba(255,255,255,0.08)">' +
            escapeHtml(row.flag || "⬡") +
            '</div><div style="flex:1;min-width:0;"><div class="lb-name">' +
            escapeHtml(row.name || "Participant") +
            '</div><div class="lb-bar"><div class="lb-bar-fill" style="width:' +
            (row.score || 0) +
            '%;background:' +
            color +
            '"></div></div></div><div style="text-align:right;flex-shrink:0;"><div class="lb-val">' +
            escapeHtml(row.value || "--") +
            "</div></div>";
          cont.appendChild(div);
        });
      }
      var investorRows = (data.investors || []).map(function (entry) {
        return {
          name: entry.name,
          flag: entry.flag,
          value: fmtCompactMoney(entry.amountUsd),
          score: Math.min(100, Math.round((entry.amountUsd / (data.investors[0]?.amountUsd || 1)) * 100)),
        };
      });
      var validatorRows = (data.validators || []).slice(0, 8).map(function (entry) {
        return {
          name: entry.name,
          flag: entry.flag,
          value: fmtPct(entry.uptimePct || 0) + " / " + fmtNumber(entry.tps || 0) + " TPS",
          score: Math.min(100, Math.round(entry.score || 0)),
        };
      });
      var delegateRows = (data.delegates || []).map(function (entry) {
        return {
          name: entry.name,
          flag: "🗳",
          value: fmtNumber(entry.power || 0),
          score: Math.min(100, Math.round((entry.power || 0) / ((data.delegates[0]?.power || 1)) * 100)),
        };
      });
      var stakingRows = (staking.pools || []).map(function (pool) {
        return {
          name: pool.name,
          flag: "⬡",
          value: fmtCompactMoney(pool.tvlUsd),
          score: Math.min(100, Math.round((pool.tvlUsd || 0) / ((staking.pools[0]?.tvlUsd || 1)) * 100)),
        };
      });
      buildBoard("ref-board", investorRows, "var(--neon)");
      buildBoard("stake-board", stakingRows, "var(--green)");
      buildBoard("gov-board", delegateRows, "var(--cyan)");
      buildBoard("val-board", validatorRows, "var(--gold)");
      setText("#arena-participants", fmtNumber((data.summary && data.summary.investorsCount) || 0));
      api.renderModuleMeta(".scoreboard-header", "leaderboard arena", envelope);
    }
    await load();
    global.setInterval(load, 30000);
  }

  async function initDealRoom(api) {
    async function load() {
      var payloads = await Promise.all([
        api.getDealsEnvelope({ refresh: true }),
        api.getPresaleEnvelope({ refresh: true }),
        api.getBenchmarkEnvelope("overview", { refresh: true }),
      ]);
      var envelope = payloads[0];
      var deals = payloads[0].data || {};
      var presale = payloads[1].data || {};
      var overview = payloads[2].data || {};
      if (byId("deal-presale-price")) {
        setText("#deal-presale-price", presale.tokenPriceUsd ? "$" + Number(presale.tokenPriceUsd).toFixed(3) : "--");
      }
      if (byId("deal-benchmark-tps")) {
        var tps = overview.live && overview.live.summary ? overview.live.summary.combined_tps : null;
        setText("#deal-benchmark-tps", tps ? fmtNumber(tps) + " TPS" : "unavailable");
      }
      var grid = byId("deal-grid");
      if (grid) {
        grid.innerHTML = "";
        (deals.deals || []).forEach(function (deal) {
          var card = document.createElement("div");
          card.className = "deal-card";
          card.innerHTML =
            '<div class="dc-cat">' +
            escapeHtml(deal.category || "Deal") +
            '</div><div class="dc-title">' +
            escapeHtml(deal.title || "Deal") +
            '</div><div class="dc-desc">' +
            escapeHtml(deal.description || "") +
            '</div><div class="dc-exchange"><div class="dce-side"><div class="dce-label">You provide</div><div class="dce-val">' +
            escapeHtml(deal.provide || "") +
            '</div></div><div class="dce-arrow">⇄</div><div class="dce-side"><div class="dce-label">You receive</div><div class="dce-val" style="color:var(--gold)">' +
            escapeHtml(deal.receive || "") +
            '</div></div></div><div class="dc-terms">' +
            escapeHtml(deal.terms || "") +
            '</div><button class="dc-cta">Start This Deal →</button>';
          grid.appendChild(card);
        });
      }
      api.renderModuleMeta(".masthead", "deal room", envelope);
    }
    await load();
    global.setInterval(load, 30000);
  }

  async function initBenchmarkHub(api) {
    async function load() {
      var envelope = await api.getBenchmarkEnvelope("overview", { refresh: true });
      var data = envelope.data || {};
      setText("#bench-hub-status", String(envelope.status || "unavailable").toUpperCase());
      setText("#bench-hub-updated", new Date(envelope.lastUpdated).toLocaleString());
      setText("#bench-hub-tps", data.live && data.live.summary ? fmtNumber(data.live.summary.combined_tps) : "--");
      api.renderModuleMeta(".shell", "benchmark hub", envelope);
    }
    await load();
    global.setInterval(load, 30000);
  }

  async function initStaticMeta(api) {
    var envelope = await api.getHealthEnvelope({ refresh: true });
    api.renderModuleMeta(".page-head", "site", envelope);
  }

  async function start() {
    mountTopNav();
    if (!global.X3API) return;
    initDebugMode();
    var api = await global.X3API.init();
    wrapApiForDebug(api);
    var page = (global.location.pathname.split("/").pop() || "x3star-landing.html").toLowerCase();
    var adapters = {
      "x3star-dashboard.html": initDashboard,
      "x3star-landing.html": initLanding,
      "x3star-governance.html": initGovernance,
      "x3star-node-health.html": initNodeHealth,
      "x3star-staking.html": initStaking,
      "x3star-network-pulse.html": initNetworkPulse,
      "x3star-transparency-ledger.html": initLedger,
      "x3star-proof-wall.html": initProofWall,
      "x3star-validator-presale.html": initValidatorPresale,
      "x3star-kyc-onboarding.html": initKyc,
      "x3star-affiliate.html": initAffiliate,
      "x3star-investor-relations.html": initInvestorRelations,
      "x3star-token-presale.html": initPresaleMetrics,
      "x3star-social-proof-wall.html": initReservationsPages,
      "x3star-fundraise-thermometer.html": initReservationsPages,
      "x3star-scarcity-clock.html": initScarcityClock,
      "x3star-ecosystem-heartbeat.html": initEcosystemHeartbeat,
      "x3star-operator-war-room.html": initOperatorWarRoom,
      "x3star-mission-terminal.html": initMissionTerminal,
      "x3star-arbitrage-engine.html": initArbitrageEngine,
      "x3star-slot-tracker.html": initSlotTracker,
      "x3star-whale-tracker.html": initWhales,
      "x3star-tokenomics-warroom.html": initTokenomics,
      "x3star-grant-hub.html": initGrantHub,
      "x3star-grant-mission-control.html": initGrantMissionControl,
      "x3star-bounty-board.html": initBountyBoard,
      "x3star-hall-of-fame.html": initHallOfFame,
      "x3star-leaderboard-arena.html": initLeaderboardArena,
      "x3star-barter-exchange.html": initDealRoom,
      "x3star-benchmark-page.html": initBenchmarkHub,
      "x3star-portfolio.html": initStaticMeta,
      "x3star-roi-calculator.html": initStaticMeta,
      "x3star-if-you-invested.html": initStaticMeta,
      "x3star-if-you-had.html": initStaticMeta,
      "x3star-competitor-annihilation.html": initStaticMeta,
      "x3star-competitor-graveyard.html": initStaticMeta,
      "x3star-compute-marketplace.html": initStaticMeta,
      "x3star-spine.html": initStaticMeta,
      "x3star-tech-deep-dive.html": initStaticMeta,
      "x3star-whitepaper.html": initStaticMeta,
      "blockchain-stress-test.html": initBenchmarkConsole,
      "blockchain-stress-test(1).html": initBenchmarkConsole,
      "chainbench-pro.html": initBenchmarkConsole,
      "chainbench-ultimate.html": initBenchmarkConsole,
      "chainbench-ultimate(1).html": initBenchmarkConsole,
    };
    if (adapters[page]) {
      await adapters[page](api);
    }
    renderDebugFooter();
  }

  global.X3PageAdapters = {
    start: start,
  };
})(typeof window !== "undefined" ? window : globalThis);
