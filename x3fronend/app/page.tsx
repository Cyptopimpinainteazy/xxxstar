import registry from "../data/registry.json";
import { links } from "../lib/links";
import MobileNav from "../components/MobileNav";
import ScrollReveal from "../components/ScrollReveal";

function Logomark({ size = 28 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 40 40" fill="none" aria-hidden="true">
      <circle cx="20" cy="20" r="3.2" fill="#7ee8de" />
      <g stroke="#4fd1c5" strokeWidth="1.4" opacity="0.9">
        <ellipse cx="20" cy="20" rx="17" ry="7" />
        <ellipse cx="20" cy="20" rx="17" ry="7" transform="rotate(60 20 20)" />
        <ellipse cx="20" cy="20" rx="17" ry="7" transform="rotate(120 20 20)" />
      </g>
    </svg>
  );
}

const ATOMIC_STEPS = [
  "Parse", "Semantic check", "Lower to typed IR", "Validate IR",
  "Preflight all legs", "Reserve / lock assets", "Execute route",
  "Verify receipts / proofs", "Commit all legs", "Rollback failed legs",
  "Settle final state", "Assert invariants",
];

const WIRING_LEGEND: [string, string][] = [
  ["I", "implemented / unverified"],
  ["P", "partial"],
  ["D", "disconnected"],
  ["V", "verified"],
];

const SYSTEM_ARCHITECTURE = [
  { status: "P", title: "Wallet / SDK", detail: "wire encoders" },
  { status: "I", title: "RPC / pool", detail: "native tx path" },
  { status: "I", title: "Aura + GRANDPA", detail: "node wiring" },
  { status: "P", title: "FRAME runtime", detail: "custom pallets" },
  { status: "D", title: "Gateway / indexer", detail: "main exits" },
  { status: "P", title: "External chains", detail: "proof trust" },
];

const COMPONENT_DEPENDENCIES = [
  { status: "I", title: "node", detail: "sc-service / sc-network" },
  { status: "I", title: "runtime", detail: "FRAME / sp-api" },
  { status: "P", title: "X3 pallets", detail: "kernel / settlement" },
  { status: "P", title: "VM integrations", detail: "mini EVM / SVM / X3" },
  { status: "P", title: "proof router", detail: "vault / RLP / hashes" },
  { status: "D", title: "gateway executable", detail: "source modules omitted" },
];

const EVIDENCE = [
  {
    text: "cargo check --workspace compiles clean",
    note: "Full workspace, no build errors.",
    src: "CURRENT_MAINNET_STATUS.md",
  },
  {
    text: "cargo audit: 0 blocking vulnerabilities",
    note: "33 advisories tracked and explicitly allow-listed, none blocking.",
    src: "CURRENT_MAINNET_STATUS.md, deny.toml",
  },
  {
    text: "181 tests passing in the root pytest suite",
    note: "",
    src: "CURRENT_MAINNET_STATUS.md",
  },
  {
    text: "7-validator GRANDPA network reached full consensus",
    note: "Fresh-key local network: 7/7 identical finalized heads, 2000/2000 remarks finalized at 110.6 finTPS. This was a local/loopback run — not a public deployment.",
    src: "TESTNET_VERIFICATION.md",
  },
  {
    text: "20+ CI gates enforced on every change",
    note: "fmt, clippy, test, audit, deny, secret-scan, binary checks.",
    src: "LAUNCH_SCOPE.md — CI gate matrix",
  },
  {
    text: "x3-lang compiler: 14/14 conformance tests passing",
    note: "Lexer, parser, typechecker, IR emitter, and verifier — roughly 11k lines of Rust, checked against a structured accept/reject conformance manifest.",
    src: "x3-lang/compiler/, x3-lang/tests/conformance/manifest.json",
  },
  {
    text: "An automated proof pipeline signs claim receipts — including its own failures",
    note: "27 hash-signed receipts on record: 22 verified, 2 partial, 1 blocked, 1 unverified, 1 failed (a supply-conservation test that doesn't exist yet). The failure is published next to the passes, not filtered out.",
    src: "proof-forge/, proof/receipts/claims/",
  },
  {
    text: "CodeQL + Semgrep static analysis, SBOM + attestation, Zombienet multi-node tests, try-runtime upgrade rehearsal",
    note: "All CI-gated, not manual/self-reported.",
    src: "release-hardening.yml, zombienet-integration.yml, try-runtime-upgrade.yml",
  },
];

const GATED_OUT = [
  { feature: "External EVM / SVM / Bitcoin bridge gateway", guard: "compile_error! (mainnet-rc1)", phase: "Post-audit" },
  { feature: "Parallel block execution", guard: "compile_error! (mainnet-rc1)", phase: "Post-audit" },
  { feature: "Application zone contract factory", guard: "compile-time guard", phase: "Post-audit" },
  { feature: "Advanced DEX routing", guard: "compile-time guard", phase: "Post-audit" },
  { feature: "AI consensus optimizer", guard: "compile-time guard", phase: "Post-audit" },
  { feature: "GPU-critical validator acceleration", guard: "compile-time guard, CPU-simulated today", phase: "Post-audit" },
  { feature: "Permissionless validator staking", guard: "NOT IMPLEMENTED — operator/root-controlled validator set", phase: "M3" },
  { feature: "Production post-quantum cryptography", guard: "NOT IMPLEMENTED — research/simulated only", phase: "Post-audit" },
];

const AUDIT_ROWS = [
  { area: "Runtime pallets (router, supply-ledger, settlement, kernel)", status: "Not yet externally audited" },
  { area: "EVM contracts", status: "Not yet externally audited" },
  { area: "SVM / Anchor programs", status: "Not yet externally audited" },
  { area: "DevSecOps / Infrastructure", status: "Not yet externally audited" },
  { area: "Public bug bounty", status: "Not yet launched" },
];

const GATE_CHECKLIST = [
  { label: "Scope reconciliation — LAUNCH_SCOPE.md is the sole status source", done: true },
  { label: "Feature-gate enforcement (external-gateway removed from mainnet-rc1, compile_error! guard)", done: true },
  { label: "CI semantic analysis — CodeQL + Semgrep", done: true },
  { label: "CI release hardening — SBOM + provenance attestation", done: true },
  { label: "Multi-owner review model — CODEOWNERS by domain", done: true },
  { label: "Staging testnet setup guide published", done: true },
  { label: "Signed release tag published (v0.4.0-rc.1, hashes, SBOM, attestations)", done: false },
  { label: "FRAME benchmarking in CI (weight generation + validation)", done: false },
  { label: "External audit — runtime pallets", done: false },
  { label: "External audit — EVM + SVM contracts", done: false },
  { label: "External audit — DevSecOps / infrastructure", done: false },
  { label: "try-runtime rehearsal enforced as mandatory promotion gate", done: false },
  { label: "Zombienet multi-node tests enforced as stage-exit criterion", done: false },
  { label: "Public bug bounty live", done: false },
  { label: "5–7 validator staging infrastructure deployed", done: false },
  { label: "Operator runbooks validated (deploy, restore, incident, upgrade, rollback)", done: false },
];

const ECOSYSTEM = [
  { href: "/validators/", icon: "🌐", title: "Validator Globe", desc: "3D visualization of validator nodes with live RPC polling.", tag: "apps/validators" },
  { href: "/dashboard/", icon: "📊", title: "Modular Dashboard", desc: "Extensible panel-based chain dashboard system.", tag: "apps/dashboard" },
  { href: "/inferstructor/", icon: "⚡", title: "Inferstructor", desc: "GPU validator admin dashboard with TPS leaderboard.", tag: "apps/inferstructor-dashboard" },
  { href: "/tps/", icon: "📈", title: "TPS Monitor", desc: "Transactions-per-second monitoring and session history.", tag: "infra-structure/services/blockchain-tps" },
  { href: "/funding/", icon: "🎯", title: "Funding Strategy", desc: "The real target list of grant programs, funds, and partnerships being pursued.", tag: "apps/x3-funding" },
  { href: "/transparency/", icon: "🔍", title: "Transparency Ledger", desc: "Evidence-only treasury and funding record — seeded at zero pending real transactions.", tag: "apps/x3-transparency" },
];

export default function Home() {
  return (
    <>
      <ScrollReveal />
      <nav className="nav">
        <div className="shell nav-inner">
          <div className="nav-brand"><Logomark size={22} /> X3 ATOMIC STAR</div>
          <div className="nav-links">
            <a href="#architecture">Architecture</a>
            <a href="#status">Status</a>
            <a href="#security">Security</a>
            <a href="#funding">Funding &amp; Grants</a>
            <a href="#ecosystem">Ecosystem</a>
          </div>
          <a className="nav-cta" href="#contact">Contact</a>
          <MobileNav />
        </div>
      </nav>

      <header className="hero shell">
        <svg className="hero-orbit" viewBox="0 0 40 40" fill="none" aria-hidden="true">
          <g stroke="#4fd1c5" strokeWidth="0.35">
            <ellipse cx="20" cy="20" rx="17" ry="7" />
            <ellipse cx="20" cy="20" rx="17" ry="7" transform="rotate(60 20 20)" />
            <ellipse cx="20" cy="20" rx="17" ry="7" transform="rotate(120 20 20)" />
          </g>
        </svg>
        <Logomark size={52} />
        <div className="eyebrow" style={{ marginTop: 18 }}>Multi-VM L1 · X3Native + EVM + SVM</div>
        <h1 className="hero-title">Atomic settlement across three execution environments, or none of it happens at all.</h1>
        <p className="hero-sub">
          X3 Atomic Star is a Substrate L1. x3-lang compiles cross-VM intents into a typed
          route that either commits on every leg or rolls back on all of them — no partial
          fills, no stuck state. Everything below is generated from or cited to the files in
          this repository, not written as marketing copy.
        </p>
        <div className="status-badge">
          <strong>v0.4 Internal Testnet Candidate</strong>
          <span className="sep">·</span>
          <span>closed-operator staged network</span>
          <span className="sep">·</span>
          <span>not a public testnet, not mainnet</span>
          <span className="sep">·</span>
          <span>source: LAUNCH_SCOPE.md</span>
        </div>
        <div className="hero-ctas">
          {links.github ? (
            <a className="btn btn-primary" href={links.github}>View the code</a>
          ) : (
            <a className="btn btn-primary" href="#status">See live status</a>
          )}
          <a className="btn btn-secondary" href={`mailto:${links.email}`}>Grant &amp; investor inquiries</a>
        </div>
      </header>

      <section className="block reveal" id="architecture">
        <div className="shell">
          <div className="section-head">
            <div className="section-kicker">Architecture</div>
            <h2 className="section-title">What X3 actually is</h2>
            <p className="section-desc">
              One chain, three execution domains, one settlement guarantee.
            </p>
          </div>
          <div className="grid grid-3">
            <div className="card">
              <h3>Universal Asset Kernel</h3>
              <p>Supply-ledger invariants enforced on every mint, transfer, and swap. Backed by an EconomicHalt guard that blocks new state changes if the canonical supply invariant ever breaks.</p>
              <div className="src">pallets/x3-atomic-kernel</div>
            </div>
            <div className="card">
              <h3>Three domains, one router</h3>
              <p>X3Native, X3Evm, and X3Svm execute under a shared cross-VM router with a 6-route internal matrix, replay protection, and atomic source-debit / destination-credit accounting.</p>
              <div className="src">pallets/x3-cross-vm-router</div>
            </div>
            <div className="card">
              <h3>x3-lang intents</h3>
              <p>Cross-VM intents are parsed, type-checked, and lowered to a typed IR before any leg executes. A real compiler — lexer, parser, typechecker, IR emitter, verifier, ~11k lines of Rust — not just a spec.</p>
              <div className="src">x3-lang/compiler/</div>
            </div>
          </div>

          <div style={{ marginTop: 40 }}>
            <h3 style={{ fontSize: 15, color: "var(--text-muted)", fontWeight: 600, marginBottom: 16 }}>
              The atomic execution model — every route, every time
            </h3>
            <div className="steps">
              {ATOMIC_STEPS.map((s, i) => (
                <div className="step" key={s}>
                  <span className="n">{String(i + 1).padStart(2, "0")}</span>
                  {s}
                </div>
              ))}
            </div>
          </div>

          <div style={{ marginTop: 40 }}>
            <h3 style={{ fontSize: 15, color: "var(--text-muted)", fontWeight: 600, marginBottom: 4 }}>
              How the pieces are actually wired
            </h3>
            <p style={{ fontSize: 13, color: "var(--text-dim)", marginBottom: 18, maxWidth: 620 }}>
              From an internal engineering review (2026-09-05, commit 6a24d8cf) — static wiring only, no live deployment is asserted.
            </p>
            <div className="grid grid-2" style={{ marginBottom: 20 }}>
              <div>
                <div className="wiring-label">System architecture</div>
                <div className="wiring-grid">
                  {SYSTEM_ARCHITECTURE.map((b) => (
                    <div className={`wiring-box status-${b.status}`} key={b.title}>
                      <span className={`wiring-tag tag-${b.status}`}>{b.status}</span>
                      <div className="wiring-title">{b.title}</div>
                      <div className="wiring-detail">{b.detail}</div>
                    </div>
                  ))}
                </div>
              </div>
              <div>
                <div className="wiring-label">Component dependencies</div>
                <div className="wiring-grid">
                  {COMPONENT_DEPENDENCIES.map((b) => (
                    <div className={`wiring-box status-${b.status}`} key={b.title}>
                      <span className={`wiring-tag tag-${b.status}`}>{b.status}</span>
                      <div className="wiring-title">{b.title}</div>
                      <div className="wiring-detail">{b.detail}</div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
            <div className="wiring-legend">
              {WIRING_LEGEND.map(([k, v]) => (
                <span key={k}><span className={`wiring-tag tag-${k}`}>{k}</span> {v}</span>
              ))}
            </div>
          </div>
        </div>
      </section>

      <section className="block reveal">
        <div className="shell">
          <div className="section-head">
            <div className="section-kicker">Evidence</div>
            <h2 className="section-title">Proof, not promises</h2>
            <p className="section-desc">
              Every line below cites the file or CI workflow it comes from. Nothing here is a projection.
            </p>
          </div>
          <div className="evidence">
            {EVIDENCE.map((e) => (
              <div className="evidence-item" key={e.text}>
                <div className="check">✓</div>
                <div className="body">
                  <p>{e.text}</p>
                  {e.note && <div className="note">{e.note}</div>}
                </div>
                <div className="src mono">{e.src}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      <section className="block reveal" id="status">
        <div className="shell">
          <div className="section-head">
            <div className="section-kicker">Live status</div>
            <h2 className="section-title">Feature readiness, generated at build time</h2>
            <p className="section-desc">
              This table is not hand-typed. It's generated directly from FEATURE_REGISTRY.toml —
              the same file scripts/check-readiness-consistency.sh validates in CI — every time
              this site is built.
            </p>
          </div>

          <div className="registry-summary">
            <div className="big">{registry.averageReadiness}%</div>
            <div className="label">
              average readiness across {registry.featureCount} tracked features, computed live from
              FEATURE_REGISTRY.toml as of {registry.generatedAt}.
            </div>
          </div>

          <div style={{ overflowX: "auto" }}>
            <table className="reg-table">
              <thead>
                <tr>
                  <th>Feature</th>
                  <th>Crate / service</th>
                  <th>Mode</th>
                  <th>Readiness</th>
                  <th>Top blocker</th>
                </tr>
              </thead>
              <tbody>
                {registry.features.map((f) => (
                  <tr key={f.id}>
                    <td className="reg-id">{f.id}</td>
                    <td className="reg-crate">{f.crate_or_service}</td>
                    <td><span className={`mode-pill mode-${f.mode}`}>{f.mode.replace("_", " ")}</span></td>
                    <td>
                      <div className="score-bar-wrap">
                        <div className="score-bar"><div className="score-bar-fill" style={{ width: `${f.readiness_score}%` }} /></div>
                        <span className="score-num">{f.readiness_score}%</span>
                      </div>
                    </td>
                    <td className="blocker-text">{f.blockers[0] ?? "—"}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <div className="registry-footnote">generated_from: FEATURE_REGISTRY.toml · generated_at: {registry.generatedAt}</div>
        </div>
      </section>

      <section className="block reveal">
        <div className="shell">
          <div className="section-head">
            <div className="section-kicker">Scope</div>
            <h2 className="section-title">What's explicitly not live yet</h2>
            <p className="section-desc">
              These are intentionally disabled — compile-time gated or simply not built — until an
              audited phase enables them. Code existing for audit review is not the same as code
              running in production, and this site doesn't blur that line.
            </p>
          </div>
          <div style={{ overflowX: "auto" }}>
            <table className="gated-table">
              <thead>
                <tr><th>Feature</th><th>Guard / reality</th><th>Target phase</th></tr>
              </thead>
              <tbody>
                {GATED_OUT.map((g) => (
                  <tr key={g.feature}>
                    <td style={{ color: "var(--text)" }}>{g.feature}</td>
                    <td className="mono" style={{ fontSize: 13 }}>{g.guard}</td>
                    <td><span className="gate-tag">{g.phase}</span></td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      </section>

      <section className="block reveal" id="security">
        <div className="shell">
          <div className="section-head">
            <div className="section-kicker">Security</div>
            <h2 className="section-title">Audit status</h2>
            <p className="section-desc">
              Zero external audits are complete. We're saying that here so a grant committee never
              has to find it out by digging.
            </p>
          </div>
          <div style={{ overflowX: "auto" }}>
            <table className="gated-table">
              <thead><tr><th>Area</th><th>Status</th></tr></thead>
              <tbody>
                {AUDIT_ROWS.map((a) => (
                  <tr key={a.area}>
                    <td style={{ color: "var(--text)" }}>{a.area}</td>
                    <td>{a.status}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          <p style={{ color: "var(--text-dim)", fontSize: 13, marginTop: 18, maxWidth: 660 }}>
            Formal verification is real but partial: one complete, machine-checked Coq proof of
            supply conservation (<span className="mono">formal-proofs/coq/SupplyInvariant.v</span>,
            no <span className="mono">Admitted</span>/<span className="mono">Axiom</span>) — over an
            abstract model, not yet linked to the production pallet — plus 6 TLA+ specifications
            (consensus, asset kernel, GPU, funding, onboarding, cross-VM) and early K-framework
            scaffolding. What's active today: CI-enforced static analysis (CodeQL + Semgrep), an
            SBOM + attestation pipeline wired into the release workflow, and secret scanning on
            every change.
          </p>
          <p style={{ color: "var(--text-dim)", fontSize: 13, marginTop: 14, maxWidth: 660 }}>
            "Zero external audits" isn't the same as "nobody looked." A self-run adversarial audit
            (2026-09-06) scored the codebase 63/100 and published 4 Critical, 5 High, 6 Medium, and
            3 Low findings — including a fabricated wallet RPC and readiness-registry rows citing
            tests that didn't exist. Those findings are what got fixed, not what got hidden.
            <span className="mono" style={{ display: "block", marginTop: 4 }}>audit-artifacts/mainnet-readiness/2026-09-06-fbd4613b-claude/</span>
          </p>
        </div>
      </section>

      <section className="block reveal" id="funding">
        <div className="shell">
          <div className="section-head">
            <div className="section-kicker">Funding &amp; grants</div>
            <h2 className="section-title">What we're asking for, and why</h2>
          </div>
          <div className="fund-stat">
            <div>
              <div className="v">$0</div>
              <div className="l">external capital raised to date — self-funded, pre-seed</div>
            </div>
            <div>
              <div className="v">{registry.averageReadiness}%</div>
              <div className="l">average feature readiness (live, see above)</div>
            </div>
            <div>
              <div className="v">0</div>
              <div className="l">external security audits completed</div>
            </div>
          </div>
          <p className="section-desc" style={{ marginBottom: 24, maxWidth: 680 }}>
            Grant funding goes directly at the gates below — the same gates LAUNCH_SCOPE.md
            defines as required before this network can move from an internal testnet candidate
            to a public staged testnet. This list is the actual roadmap, not a pitch deck version of it.
          </p>
          <ul className="checklist">
            {GATE_CHECKLIST.map((g) => (
              <li key={g.label}>
                <span className={`tag ${g.done ? "tag-done" : "tag-todo"}`}>{g.done ? "DONE" : "TODO"}</span>
                {g.label}
              </li>
            ))}
          </ul>
        </div>
      </section>

      <section className="block reveal">
        <div className="shell">
          <div className="section-head">
            <div className="section-kicker">Team</div>
            <h2 className="section-title">Building pseudonymously</h2>
          </div>
          <p className="team-note">
            X3 Atomic Star is built by a pseudonymous contributor group — common practice in this
            industry, and not a substitute for the evidence above. Team background and identity are
            available under NDA to grant committees and investors conducting real due diligence.
          </p>
        </div>
      </section>

      <section className="block reveal" id="ecosystem">
        <div className="shell">
          <div className="section-head">
            <div className="section-kicker">Ecosystem</div>
            <h2 className="section-title">What's actually running in this monorepo</h2>
            <p className="section-desc">
              Real sub-apps built alongside the chain, at every stage of completion — not mockups, not hidden because they're unfinished.
            </p>
          </div>
          <div className="grid grid-3">
            {ECOSYSTEM.map((e) => (
              <a className="eco-card" href={e.href} key={e.title}>
                <div className="icon">{e.icon}</div>
                <h3>{e.title}</h3>
                <p>{e.desc}</p>
                <span className="tag mono">{e.tag}</span>
              </a>
            ))}
          </div>
          <p style={{ marginTop: 22, fontSize: 13.5 }}>
            <a href="/apps/" style={{ color: "var(--accent)" }}>Browse the full internal app directory →</a>
          </p>
        </div>
      </section>

      <footer className="site-footer" id="contact">
        <div className="shell">
          <div className="contact-grid" style={{ marginBottom: 28 }}>
            <div>
              <h3 style={{ color: "var(--text)", fontSize: 16, marginBottom: 8 }}>Grants &amp; investment</h3>
              <p style={{ marginBottom: 12 }}>
                <a href={`mailto:${links.email}`} style={{ color: "var(--accent)" }}>{links.email}</a>
              </p>
              <div className="pill-row">
                {links.github && <a className="pill" href={links.github}>GitHub</a>}
                {links.twitter && <a className="pill" href={links.twitter}>X / Twitter</a>}
                {links.discord && <a className="pill" href={links.discord}>Discord</a>}
                {links.telegram && <a className="pill" href={links.telegram}>Telegram</a>}
              </div>
            </div>
            <div>
              <h3 style={{ color: "var(--text)", fontSize: 16, marginBottom: 8 }}>Source of truth</h3>
              <p style={{ marginBottom: 4 }}>LAUNCH_SCOPE.md — authoritative status</p>
              <p style={{ marginBottom: 4 }}>FEATURE_REGISTRY.toml — live readiness data</p>
              <p>scripts/check-readiness-consistency.sh — the check that keeps both honest</p>
            </div>
          </div>
          <div className="footer-row">
            <span>X3 Atomic Star — v0.4 Internal Testnet Candidate</span>
            <span>Every status claim on this page traces to a file in this repository.</span>
          </div>
        </div>
      </footer>
    </>
  );
}
