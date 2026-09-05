# X3 Atomic Star — Step-by-Step Game Plan to Mainnet

**Generated:** 2026-09-04
**Based on:** `docs/current/READINESS_GRAPH.md` (commit `c87f25a3`)
**Total credible timeline:** ~4–6 months from today to public mainnet (assuming focused execution + clean audit)

> **Reading order:** §1 shows the big-picture timeline. §2 is the week-by-week operational game plan. §3 lists what I (the agent) can do autonomously vs. what needs you. §4 is the risk register. §5 is the cost envelope.

---

## 1. Big-picture timeline

```
W0     W3        W9               W15                W22               W28      W32+
 │      │         │                │                  │                 │        │
 ▼      ▼         ▼                ▼                  ▼                 ▼        ▼
M1 RC1 ──→ M2 prep+audit ──→ M2 launch ──→ M2 soak ──→ M3 prep ──→ M3 mainnet
internal   (finality proofs    bridge       (4-6wk       (staking,
testnet    + bridge deploy      testnet      public       multisig,
cut        + 3rd-party          goes         bug          genesis
           audit running)       live)        hunt)        ceremony)
```

**Five phases:**
| Phase | Duration | What you get |
|---|---|---|
| **0. Pre-flight** | This week (W0) | Audit RFP out, host topology decided, tokenomics confirmed-or-deferred |
| **1. M1 closure** | W1–W3 | Internal Staged Testnet RC-1 cut — signed binary + benchmarks + property tests + multi-host mesh proof |
| **2. M2 prep + audit** | W4–W13 | Chain-specific finality proofs + bridge adapters deployed; external audit running in parallel |
| **3. M2 launch + soak** | W14–W22 | Audit findings remediated; public bridge testnet live; bug bounty active; 4–6 week public soak |
| **4. M3 prep** | W23–W28 | Permissionless staking, multisig validator admission, genesis ceremony prep, legal package |
| **5. M3 launch** | W29–W32 | Signed genesis ceremony, mainnet live, post-launch monitoring |

**Total: 32 weeks (~7 months) for the full path, 24 weeks (~5.5 months) for a fast-track if everything goes clean.**

---

## 2. Week-by-week operational game plan

### Phase 0 — Pre-flight (this week, W0)

| Day | Owner | Action | Done-when |
|---|---|---|---|
| W0-Mon | **You** | Decide tokenomics path: (a) keep RC-1 authority-set permanently and document "permissionless staking is a separate phased rollout," OR (b) design permissionless staking now | Decision recorded in `docs/current/TOKENOMICS_DECISION.md` |
| W0-Mon | **You** | Engage 3-host hardware for multi-host mesh proof. Options: (a) 3 cheap VPS from different providers (Hetzner + Vultr + DigitalOcean), (b) 3 home/office machines on different ISPs, (c) 3 cloud VMs in different regions | Hosts provisioned with public IPs and SSH access |
| W0-Tue | **You** | Engage audit firm. Recommended firms for Substrate+Solana+EVM scope: Spearbit, Trail of Bits, Runtime Verification, Zellic, OtterSec. Budget range: $80k–$250k for full scope. | NDA signed, scoping call scheduled for W1, kickoff for W4 |
| W0-Wed | **Agent** | Run full `cargo test --workspace` (background, 30–60min). Identify any new flaky tests. | Test report recorded |
| W0-Thu | **Agent** | Generate RC-1 release candidate 0 (`scripts/create-rc1-release.sh` dry-run) to validate the release pipeline before we depend on it | RC dry-run artifact |
| W0-Fri | **You + Agent** | Decide: which missing-FRAME-benchmark pallets are critical-path vs. nice-to-have? | Critical-path list agreed |

**Phase 0 gates:**
- [ ] Tokenomics decision committed
- [ ] 3 hosts provisioned and reachable
- [ ] Audit firm engaged, kickoff date confirmed
- [ ] RC dry-run artifact produced

---

### Phase 1 — M1 Internal Testnet Closure (W1–W3)

**Goal:** Cut a signed RC-1 artifact with full test coverage that runs a 3-validator testnet on real hardware.

**Week 1 — Benchmarks + property tests + audit kickoff**

| Day | Owner | Action |
|---|---|---|
| W1-Mon | Agent | Add FRAME benchmarks for the 5 highest-traffic pallets: `pallet-x3-settlement-engine` (already has) — extend to 12 critical-path pallets. Use `frame_benchmarking::benchmarks!` macro + `frame-support-procedural`. |
| W1-Tue | Agent | Add `proptest!` state-machine tests for the 6-route atomic-swap transition matrix in `crates/x3-atomic-swap/`. Cover: happy paths, partial-fail rollback, timeout paths, double-spend attempts. |
| W1-Wed | **You + Audit firm** | Audit kickoff call. Walk them through `LAUNCH_SCOPE.md`, `FEATURE_REGISTRY.toml`, `FAILURES_AND_TODOS.md`, `docs/current/READINESS_GRAPH.md`. |
| W1-Thu | Agent | Run `cargo bench` on the new benchmarks; record weights in `reports/benchmark-weights-2026-09.md`. |
| W1-Fri | Agent | Complete remaining 5 critical-path pallet benchmarks. Total: 17 of 42 x3-pallets now benchmarked. Document the remaining 25 as "non-critical-path, deferred to M2 audit window." |

**W1 exit gate:** `cargo test --workspace` green, 17 pallets benchmarked, proptest state-machine harness runs.

**Week 2 — Signed release pipeline + SBOM + provenance**

| Day | Owner | Action |
|---|---|---|
| W2-Mon | Agent | Implement signed release pipeline: `scripts/create-rc1-release.sh` produces (a) runtime WASM, (b) chain spec, (c) SBOM (CycloneDX), (d) sha256 manifest, (e) signed manifest via `cosign` or `minisign`. |
| W2-Tue | Agent | Wire `release-provenance.yml` CI to: (a) build on tagged commit, (b) generate SBOM, (c) sign, (d) upload to internal artifact store. |
| W2-Wed | Agent | Write `docs/STAGING_TESTNET_SETUP.md` for the 3-host operator runbook (validator keys, network topology, RPC firewall, monitoring). |
| W2-Thu | Agent | Run `scripts/run-srtool.sh build` on the RC branch; verify deterministic WASM hash matches across 2+ builds. |
| W2-Fri | **You** | Generate the genesis-state artifact using `sc-cli --chain-spec-raw` + your chosen authority keys. Store in `deployment/chain-specs/rc1/` with sha256 + signature. |

**W2 exit gate:** RC-1 signed artifact reproducible; SBOM generated; operator runbook written; genesis spec produced.

**Week 3 — Multi-host mesh proof + RC-1 cut**

| Day | Owner | Action |
|---|---|---|
| W3-Mon | **You** | Deploy `x3-chain-node` on all 3 hosts using the signed RC-1 binary. Open P2P ports (30333) between them. |
| W3-Tue | Agent | Run the multi-host mesh proof: each host produces blocks, observes peers, finalizes via GRANDPA. Record in `.testnet-audit/run2/multi-host/`. |
| W3-Wed | Agent | Verify: 7/7 finalization across 3 hosts, no missed blocks, no fork. Run the existing `scripts/run-mesh.py` against the new topology. |
| W3-Thu | **You** | Cut RC-1 release: tag `v0.4.0-rc1`, sign, publish. Announce internally to operators. |
| W3-Fri | **You + Agent** | Operator onboarding: 3 internal operators run the chain for 7 days soak. Collect their reports. |

**W3 exit gate (M1 COMPLETE):**
- [ ] Signed RC-1 binary released
- [ ] 3-host mesh proof recorded (loopback-only no longer the only evidence)
- [ ] 17 critical-path pallets benchmarked
- [ ] Property tests for atomic-swap state machine
- [ ] 7-day internal soak report

---

### Phase 2 — M2 Bridge Testnet Prep + External Audit (W4–W13)

**Goal:** External audit running in parallel with chain-specific finality proofs and bridge adapter deployment.

**Weeks 4–6 — Chain-specific finality proofs**

| Day | Owner | Action |
|---|---|---|
| W4-Mon | Agent | Implement Ethereum finality proof: header-chain verification, MMR construction, sync committee verification. Use existing `crates/x3-finality-oracle` as the integration point. |
| W4-Wed | Agent | Implement Solana finality proof: bank-hash chain, slot progression, confirmation depth heuristic + (if time) Solana RPC confirmation. |
| W5-Mon | Agent | Fuzz-test the finality proofs: 100k random headers, malformed chains, reorg attacks, deep reorgs. |
| W5-Wed | **Audit firm** | Starts runtime review (week 1 of ~8-week audit). They'll have read access to a private fork. |
| W6-Mon | Agent | Wire the finality proofs into `x3-crosschain-gateway`. Add dispute window logic. |
| W6-Wed | Agent | Add integration tests: submit Ethereum header → wait for challenge window → finalize. |

**W4–W6 exit gate:** Two chain-specific finality proofs implemented and tested; audit firm has working repo access.

**Weeks 7–9 — Bridge adapter production deployment**

| Day | Owner | Action |
|---|---|---|
| W7-Mon | Agent | Build production-grade Ethereum ↔ X3 bridge adapter (deposit + withdrawal flows, gas oracle, retry logic). |
| W7-Wed | Agent | Build production-grade Solana ↔ X3 bridge adapter. |
| W8-Mon | **Audit firm** | Mid-audit checkpoint: deep-dive on the bridge code. They'll produce interim findings. |
| W8-Wed | Agent | Remediate any interim findings the audit firm flags at midpoint. |
| W9-Mon | Agent | Deploy bridge adapters to a private staging environment with real testnet ETH/SOL (use Sepolia + Solana devnet). |
| W9-Wed | Agent | End-to-end test: deposit 0.1 ETH on Sepolia → see X3 mint → wait finality → burn on X3 → see ETH on Sepolia. Repeat for SOL. |

**W7–W9 exit gate:** Two production bridge adapters deployed to staging; end-to-end deposit/withdrawal cycle tested; audit midpoint findings remediated.

**Weeks 10–13 — Audit final report + last code hardening**

| Day | Owner | Action |
|---|---|---|
| W10-Mon | Agent | Address any remaining low/medium findings from audit midpoint. |
| W10-Wed | Agent | Add monitoring/alerting for the bridge: stuck transactions, oracle lag, dispute volume. |
| W11-Mon | **Audit firm** | Delivers draft final report. |
| W11-Wed | Agent | Begin remediation of all critical/high findings immediately. |
| W12-Mon | Agent | Public RPC gateway separation: validators no longer expose RPC publicly; standalone RPC nodes with rate limits, CORS, signed-query auth. |
| W12-Wed | Agent | Try-runtime rehearsal: simulate a runtime upgrade on the staging chain. Verify state migration is correct. |
| W13-Mon | **Audit firm** | Final report delivered (signed, public-ready). |
| W13-Wed | **You** | Decide: publish audit report publicly? (Recommended: yes, builds trust.) |
| W13-Fri | **You + Agent** | Decide: ready for M2 launch? Confirm audit findings all addressed (or accepted-with-reason). |

**W13 exit gate (M2 prep COMPLETE):**
- [ ] Final audit report published
- [ ] All critical/high findings remediated
- [ ] Two production bridge adapters in staging with E2E test pass
- [ ] Try-runtime rehearsal passed
- [ ] Public RPC separation complete
- [ ] Monitoring + alerting live

---

### Phase 3 — M2 Launch + Public Soak (W14–W22)

**Goal:** Public bridge testnet live; bug bounty active; 4–6 weeks of real public exposure before deciding on M3.

**Week 14 — M2 launch**

| Day | Owner | Action |
|---|---|---|
| W14-Mon | **You** | Cut `v0.4.0-m2` release. Tag, sign, publish. Announce to public testnet users. |
| W14-Tue | **You** | Activate bug bounty (Immunefi or self-hosted HackerOne). Initial budget: $50k–$150k. |
| W14-Wed | Agent | Enable public RPC nodes. Rate limit at 100 req/s per IP. CORS allowlist. |
| W14-Thu | Agent | Public explorer deployed (subscan-style). |
| W14-Fri | Agent | Public faucet (rate-limited per IP/account). |

**Weeks 15–22 — Public soak (8 weeks)**

During this period:
- 24/7 monitoring, on-call rotation
- Daily review of bridge transaction volume, dispute count, oracle lag
- Weekly review of bug bounty submissions
- Two mid-soak snapshots: W17 and W20, write internal status reports
- At W22: review soak metrics, decide go/no-go for M3 prep

**Soak success criteria (must hit to proceed to M3):**
- [ ] 30+ days of zero chain halts
- [ ] 0 critical bug bounty submissions unresolved
- [ ] < 5 medium bug bounty submissions open
- [ ] Bridge TVL (test tokens) > $1M equivalent
- [ ] < 0.1% failed bridge transactions
- [ ] Finality oracle lag < 5 minutes (Ethereum) / < 30 seconds (Solana)

**W22 exit gate (M2 COMPLETE):** Soak criteria met OR explicit decision to extend soak.

---

### Phase 4 — M3 Mainnet Prep (W23–W28)

**Goal:** Tokenomics (if not deferred), multisig validator admission, genesis ceremony prep.

**Weeks 23–25 — Tokenomics + multisig (if Phase 0 decided to implement)**

| Day | Owner | Action |
|---|---|---|
| W23-Mon | Agent | Implement permissionless staking pallet: bond, nominate, validate, reward, slash. |
| W23-Wed | Agent | Tokenomics model implementation: inflation, fee distribution, treasury. |
| W24-Mon | Agent | Multisig-controlled validator admission: `pallet-multisig` + `pallet-sudo` removal post-launch. |
| W24-Wed | **External counsel** | Legal package: terms of service, privacy policy, validator agreement. |
| W25-Mon | Agent | Integrate staking + multisig. Run try-runtime rehearsal on staging. |

**Weeks 26–28 — Genesis ceremony prep**

| Day | Owner | Action |
|---|---|---|
| W26-Mon | **You** | Identify 5+ genesis validators. Sign participation agreement. |
| W26-Wed | Agent | Generate genesis spec with initial validators, balances, staking state. |
| W27-Mon | Agent | Run distributed key-generation ceremony: each validator generates their session key independently, contributes to the genesis state. |
| W27-Wed | **You + Validators** | Audit the genesis: verify all balances, all validator keys, no inflation. Multi-party sign-off. |
| W28-Mon | Agent | Build the release binary for mainnet. Run deterministic srtool build 3 times, verify identical hashes. |
| W28-Wed | **You** | Final go/no-go review. |

**W28 exit gate (M3 prep COMPLETE):**
- [ ] Tokenomics + staking implemented (or document deferral)
- [ ] Multisig validator admission in place
- [ ] Genesis validated by all participants
- [ ] Mainnet binary reproducible

---

### Phase 5 — M3 Launch (W29+)

| Day | Owner | Action |
|---|---|---|
| W29-Mon | **You** | Final launch announcement. |
| W29-Tue | **Validators** | All validators start their mainnet nodes simultaneously at the scheduled block. |
| W29-Wed | Agent | Monitor first 1000 blocks. |
| W30+ | **You + Agent** | Post-launch: incident response, bug bounty triage, governance proposals. |

---

## 3. What I (the agent) can do autonomously vs. what needs you

### I can do (code, tests, docs, ops scripts)
- All FRAME benchmarking work (W1)
- Property tests for state machines (W1)
- Signed release pipeline + SBOM + provenance (W2)
- Operator runbooks (W2, W3)
- Chain-specific finality proofs (W4–W6)
- Bridge adapter implementation (W7)
- Monitoring + alerting (W10)
- Public RPC gateway separation (W12)
- Try-runtime rehearsals (W12)
- Permissionless staking + tokenomics (W23–W24)
- Multisig validator admission (W24)
- Genesis spec generation (W26–W27)
- Deterministic mainnet build verification (W28)

### You need to do (operational, financial, legal)
- **Engage audit firm** (W0) — RFP, NDA, kickoff. Budget: $80k–$250k.
- **Provision 3 hosts** for multi-host mesh proof (W0). ~$50–$200/month.
- **Decide tokenomics path** (W0). One-time strategic call.
- **Set up bug bounty program** (W14). Budget: $50k–$150k initial.
- **Identify genesis validators** (W26). Network effects + reputation work.
- **Legal package** (W24). External counsel budget: $10k–$30k.
- **Coordinate genesis ceremony** (W27). Multi-party sign-off.
- **Decide final launch** (W29). You, not me.

### Hard gates (block everything downstream if missed)
| Gate | Blocks | If missed |
|---|---|---|
| Audit firm engaged by W4 | M2 launch | No mainnet possible |
| 3 hosts provisioned by W1 | Multi-host mesh proof | RC-1 stays loopback-only |
| Tokenomics decision by W0 | M3 staking implementation | Either defer (RC-1 stays authority-set) or scramble to design |
| Audit critical findings remediated by W13 | M2 launch | Either extend remediation or delay launch |

---

## 4. Risk register

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| External audit finds critical issue requiring redesign | Medium | High (delays M2 by 4–8 weeks) | Start audit early (W4); keep scope narrow; have a remediation sprint budgeted |
| Multi-host mesh proof reveals consensus bug | Low | Critical (delays M1 by weeks) | Run the proof on staging first; have rollback to single-host for diagnosis |
| Ethereum/Solana finality edge cases | Medium | Medium (requires re-implementation) | Heavy fuzz testing; start with conservative confirmation depths |
| Bridge deposit/withdrawal race conditions | Medium | High (loss of funds in worst case) | Strict challenge window; circuit breakers; gradual TVL ramp |
| Public soak reveals governance attack | Low | Critical | Time-lock all governance actions; emergency-pause capability from day 1 |
| Bug bounty submission reveals critical | Medium | Medium-High | Have a 24-hour emergency response plan pre-written |
| Validator churn during genesis ceremony | Low | Medium | Over-recruit genesis validators (10+); require redundant sign-off |
| API key leak (recurring issue) | High | Medium | Move all secrets to vault; rotate; enable secret scanning in CI pre-commit |

---

## 5. Cost envelope (rough)

| Item | One-time | Recurring |
|---|---:|---:|
| External audit (runtime + contracts + ops) | $80k–$250k | — |
| Bug bounty program | $50k–$150k (initial pool) | $20k–$50k/yr top-up |
| 3-host testnet (VPS) | — | $150–$600/mo |
| Public RPC hosting (3 regions) | — | $300–$1500/mo |
| Legal counsel | $10k–$30k | — |
| Genesis ceremony coordination | $5k | — |
| CI/CD compute | — | $200–$1000/mo |
| **TOTAL** | **~$150k–$430k** | **~$700–$3000/mo** |

---

## 6. If you only have 6 weeks, the minimum viable mainnet path

If you must compress: **drop M2 entirely**. Go from M1 directly to a permissionless mainnet with NO external bridges. This is the conservative, safe path:

| Week | Action |
|---|---|
| W1–W3 | M1 closure as planned |
| W4–W6 | Permissionless staking + multisig + genesis prep |
| W7 | Mainnet launch (no bridges, no external TVL) |

This ships an internal-testnet-as-mainnet. The product surface is: authority-set validators, internal cross-VM, token factory, launchpad, DEX, LP locker — no external bridge risk. You can add bridges in a M4 later.

**Pros:** Fast, cheap, low-risk launch.
**Cons:** "Mainnet" with no bridges is a limited product. But it's a real, honest, audited mainnet.

---

## 7. How to use this plan

1. **Print this and put it on the wall.** (Or pin it to your repo's project board.)
2. **Every Monday**, review the previous week's deliverables and check the exit gate.
3. **If a gate slips**, immediately update the timeline and notify any downstream stakeholders (audit firm, validators, public testnet users).
4. **If you change scope** (add/remove a feature, swap an audit firm, change tokenomics), update §3 (autonomous vs. needs-you) and §5 (cost).
5. **At each Phase exit**, do a written go/no-go review. Don't proceed to the next phase without one.

---

## 8. The honest version

The repo is at **~85–90% for M1** today. The plan above assumes you can allocate:
- **Full-time engineering** for the next 13 weeks (me + you + maybe 1 more dev for the finality-proof work)
- **$150k–$430k one-time spend** for audit + bounty + legal
- **$700–$3000/month recurring** for hosting

If any of those are constrained, the plan lengthens proportionally. There's no shortcut to the external audit, the public soak, or the genesis ceremony — those are the trust anchors that make mainnet mean something.
