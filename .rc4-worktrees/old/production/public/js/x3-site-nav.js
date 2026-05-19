(function () {
  "use strict";

  var groups = [
    {
      label: "Overview",
      items: [
        { href: "/x3star-landing.html", title: "Landing", description: "Public homepage and live traction." },
        { href: "/x3star-dashboard.html", title: "Dashboard", description: "High-level telemetry and capital view." },
        { href: "/x3star-spine.html", title: "The Spine", description: "Narrative architecture and origin story." },
        { href: "/x3star-ecosystem-heartbeat.html", title: "Heartbeat", description: "Live ecosystem flow snapshot." },
        { href: "/x3star-mission-terminal.html", title: "Mission Terminal", description: "Milestone and launch timeline." },
      ],
    },
    {
      label: "Operations",
      items: [
        { href: "/x3star-network-pulse.html", title: "Network Pulse", description: "Validator map, throughput, and live feed." },
        { href: "/x3star-node-health.html", title: "Node Health", description: "Operator telemetry and heartbeat status." },
        { href: "/x3star-governance.html", title: "Governance", description: "DAO proposals and treasury status." },
        { href: "/x3star-staking.html", title: "Staking", description: "Pools, APY, and locked capital." },
        { href: "/x3star-transparency-ledger.html", title: "Ledger", description: "Treasury and funding transparency." },
        { href: "/x3star-proof-wall.html", title: "Proof Wall", description: "Operator reservation proof feed." },
        { href: "/x3star-operator-war-room.html", title: "War Room", description: "Operator-facing network console." },
      ],
    },
    {
      label: "Capital",
      items: [
        { href: "/x3star-token-presale.html", title: "Token Presale", description: "Round metrics and sale entry point." },
        { href: "/x3star-validator-presale.html", title: "Validator Presale", description: "Genesis, Star, and Lite node tiers." },
        { href: "/x3star-scarcity-clock.html", title: "Scarcity Clock", description: "Close velocity and sellout estimate." },
        { href: "/x3star-slot-tracker.html", title: "Slot Tracker", description: "Genesis slot allocation grid." },
        { href: "/x3star-fundraise-thermometer.html", title: "Thermometer", description: "Round progress and reservation feed." },
        { href: "/x3star-social-proof-wall.html", title: "Social Proof", description: "Capital inflow wall and purchase stream." },
        { href: "/x3star-investor-relations.html", title: "Investor Relations", description: "Institutional funnel and contact form." },
        { href: "/x3star-kyc-onboarding.html", title: "KYC Onboarding", description: "Application intake workflow." },
        { href: "/x3star-roi-calculator.html", title: "ROI Calculator", description: "Allocation and operator outcome math." },
        { href: "/x3star-if-you-invested.html", title: "If You Invested", description: "Retroactive presale outcome explainer." },
        { href: "/x3star-if-you-had.html", title: "If You Had", description: "Historical operator return framing." },
        { href: "/x3star-portfolio.html", title: "Portfolio", description: "Holdings and raise-facing overview." },
      ],
    },
    {
      label: "Market",
      items: [
        { href: "/x3star-whale-tracker.html", title: "Whale Tracker", description: "Large wallet activity and net flow." },
        { href: "/x3star-tokenomics-warroom.html", title: "Tokenomics", description: "Supply, vesting, burns, and unlocks." },
        { href: "/x3star-arbitrage-engine.html", title: "Arbitrage Engine", description: "Kernel story and benchmark-backed throughput." },
        { href: "/x3star-competitor-graveyard.html", title: "Competitor Graveyard", description: "Comparison narrative and failure cases." },
        { href: "/x3star-leaderboard-arena.html", title: "Leaderboard Arena", description: "Ranking and community competition surfaces." },
        { href: "/x3star-hall-of-fame.html", title: "Hall of Fame", description: "Top contributors and operator recognition." },
      ],
    },
    {
      label: "Ecosystem",
      items: [
        { href: "/x3star-affiliate.html", title: "Affiliate", description: "Partner intake and referral applications." },
        { href: "/x3star-grant-hub.html", title: "Grant Hub", description: "Ecosystem grant directory and targets." },
        { href: "/x3star-grant-mission-control.html", title: "Grant Mission Control", description: "Grant submission and status surface." },
        { href: "/x3star-bounty-board.html", title: "Bounty Board", description: "Open work and contributor rewards." },
        { href: "/x3star-barter-exchange.html", title: "Barter Exchange", description: "Deal room for compute and services." },
      ],
    },
    {
      label: "Benchmarks",
      items: [
        { href: "/chainbench-pro.html", title: "RPC Report", description: "Saved RPC validation artifact." },
        { href: "/chainbench-ultimate.html", title: "Overview", description: "Suite overview and live network snapshot." },
        { href: "/chainbench-ultimate(1).html", title: "Overview Alt", description: "Alternate overview variant." },
        { href: "/blockchain-stress-test.html", title: "Stress Artifact", description: "Saved throughput stress artifact." },
        { href: "/blockchain-stress-test(1).html", title: "Stress Artifact Alt", description: "Alternate stress artifact variant." },
      ],
    },
  ];

  function currentPath() {
    return (window.location.pathname || "/x3star-landing.html").toLowerCase();
  }

  function buildChevron() {
    return '<svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2 4.25 6 8l4-3.75" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="1.4"/></svg>';
  }

  function buildLink(item, current) {
    var currentClass = current === item.href.toLowerCase() ? " is-current" : "";
    return (
      '<a class="x3-nav-link' +
      currentClass +
      '" href="' +
      item.href +
      '"><strong>' +
      item.title +
      "</strong><span>" +
      item.description +
      "</span></a>"
    );
  }

  function buildGroup(group, index, current) {
    var groupCurrent = group.items.some(function (item) {
      return item.href.toLowerCase() === current;
    });
    return (
      '<div class="x3-nav-item' +
      (groupCurrent ? " is-current" : "") +
      '" data-nav-index="' +
      index +
      '"><button class="x3-nav-trigger" type="button" aria-expanded="false" aria-controls="x3-nav-menu-' +
      index +
      '">' +
      group.label +
      buildChevron() +
      '</button><div class="x3-nav-menu" id="x3-nav-menu-' +
      index +
      '"><div class="x3-nav-menu-grid">' +
      group.items.map(function (item) {
        return buildLink(item, current);
      }).join("") +
      "</div></div></div>"
    );
  }

  function installNav() {
    if (document.querySelector(".x3-nav-shell")) return;
    var current = currentPath();
    var shell = document.createElement("div");
    shell.className = "x3-nav-shell";
    shell.innerHTML =
      '<nav class="x3-site-nav" aria-label="Global site navigation"><div class="x3-nav-brand"><a href="/x3star-landing.html">X3STAR.org</a><span>Site map</span></div><button class="x3-nav-toggle" type="button" aria-expanded="false">Browse</button><div class="x3-nav-rail"><a class="x3-nav-home' +
      (current === "/x3star-landing.html" ? " is-current" : "") +
      '" href="/x3star-landing.html">Home</a>' +
      groups.map(function (group, index) {
        return buildGroup(group, index, current);
      }).join("") +
      '</div><div class="x3-nav-actions"><a class="x3-nav-shortcut" href="/x3star-token-presale.html">Token Sale</a><a class="x3-nav-shortcut" href="/x3star-network-pulse.html">Live Network</a></div></nav>';
    document.body.insertBefore(shell, document.body.firstChild);

    var nav = shell.querySelector(".x3-site-nav");
    var toggle = shell.querySelector(".x3-nav-toggle");
    var items = Array.from(shell.querySelectorAll(".x3-nav-item"));

    function closeAll(exceptIndex) {
      items.forEach(function (item, index) {
        var shouldOpen = typeof exceptIndex === "number" && exceptIndex === index;
        item.classList.toggle("is-open", shouldOpen);
        var trigger = item.querySelector(".x3-nav-trigger");
        if (trigger) {
          trigger.setAttribute("aria-expanded", shouldOpen ? "true" : "false");
        }
      });
    }

    items.forEach(function (item, index) {
      var trigger = item.querySelector(".x3-nav-trigger");
      trigger.addEventListener("click", function (event) {
        event.preventDefault();
        var isOpen = item.classList.contains("is-open");
        closeAll(isOpen ? null : index);
      });
      item.addEventListener("mouseenter", function () {
        if (window.innerWidth > 1080) {
          closeAll(index);
        }
      });
      item.addEventListener("mouseleave", function () {
        if (window.innerWidth > 1080) {
          closeAll(null);
        }
      });
    });

    toggle.addEventListener("click", function () {
      var isOpen = nav.classList.toggle("is-mobile-open");
      toggle.setAttribute("aria-expanded", isOpen ? "true" : "false");
      if (!isOpen) closeAll(null);
    });

    document.addEventListener("click", function (event) {
      if (!shell.contains(event.target)) {
        closeAll(null);
        nav.classList.remove("is-mobile-open");
        toggle.setAttribute("aria-expanded", "false");
      }
    });

    document.addEventListener("keydown", function (event) {
      if (event.key === "Escape") {
        closeAll(null);
        nav.classList.remove("is-mobile-open");
        toggle.setAttribute("aria-expanded", "false");
      }
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", installNav);
  } else {
    installNav();
  }
})();
