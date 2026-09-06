# C31 — Crowdfunding / community rounds (verified 2026-09-05)

**Purpose:** Track all crowdfunding platforms, retro-funding programs, DAO treasuries, and community-rounds-style capital sources that could host a X3 Atomic Star community raise.
**Scope:** Platforms/programs that match on (a) public-goods funding rounds, (b) retroactive public-goods funding, (c) social/community DAO treasury RFPs, or (d) donation/crowdfunding rails for an open-source chain project.
**Verification:** Every row below was probed with `curl -L` from the research host (timeout 12s, Mozilla UA) on 2026-09-05. Status reflects the actual HTTP response + content title, not the program's reputation.
**Last updated:** 2026-09-05 (initial C31 deep-dive by research subagent — 20 rows).

> **Format:** Each row = one prospect. `STATUS` ∈ {RESEARCH, QUALIFIED, APPLIED, AWARDED, REJECTED, DEFUNCT, NOT-A-GRANT}. `CONFIDENCE` is the agent's 1-5 rating based on how likely we are to be able to actually raise from this source.

---

## Costly items this category could fund

The community/crowdfunding category is best suited for:

- Mainnet validator bootstrap (community stake / matching)
- Bug-bounty pool top-ups (community-matched)
- Continuous public-goods funding post-mainnet (RetroPGF-style)
- Non-dilutive operational runway in early stages

---

## Prospects

### C31 — Crowdfunding / community rounds

| # | Program | Covers | Award | Eligibility | URL | Fit notes | Status | Confidence |
|---|---|---|---|---|---|---|---|---|
| C31.1 | Open Collective | Transparent donation platform + 501(c)(6) fiscal sponsorship | Variable; community-set tiers; 0–10% platform/admin fee | Any OSS project; can create a "Collective" page; integrates with GitHub Sponsors | https://opencollective.com/ | **Verified 2026-09-05**: opencollective.com returns 200 OK (53KB landing page). Cross-listed in C12.3 and C12.7 of GRANTS_DATABASE.md as the standard OSS donation rail. Pairs naturally with GitHub Sponsors for recurring donor stream. Geographic-neutral; useful from Day 0 | QUALIFIED | 5 |
| C31.2 | Giveth GIVbacks | Biweekly donor-matching pool for verified projects | 1,000,000 GIV per biweekly round; donations $5+ can win up to 500,000 GIV; 50%–80% matched | Verified, eligible projects on Giveth; self-donations excluded | https://docs.giveth.io/givbacks | **Verified 2026-09-05**: docs.giveth.io/givbacks returns 200 (360KB); Giveth.io also 200. Cross-listed in C6.3 of GRANTS_DATABASE.md. Active Ethereum Security QF visible on homepage. Useful as recurring small-grant top-up, not a one-shot raise | QUALIFIED | 4 |
| C31.3 | Giveth GIVeconomy / GIVpower | Governance + matching-pool participation | Variable (claimable GIV via GIVpower; staking rewards) | Verified projects + donors; on-chain claims | https://docs.giveth.io/giveconomy | **Verified 2026-09-05**: docs.giveth.io/giveconomy returns 200 (404KB) and docs.giveth.io/givpower also 200 (530KB). Live governance + staking layer of Giveth. Useful once a project is verified — GIVpower holders vote to direct matching funds to projects they back | RESEARCH | 3 |
| C31.4 | Geyser Fund | Crypto-native crowdfunding platform (L2/BTC/etc.) | Variable; project-set goals; rewards tiers in BTC/crypto | Any project accepting crypto donations; KYC-light | https://geyser.fund/ | **Verified 2026-09-05**: geyser.fund returns 200 (4KB shell — JS-rendered SPA behind a thin loader). Geyser is a live Web3-native crowdfunding platform with multi-chain support. Good fit for a token-launch / validator-stake raise; less regulatory friction than US-domiciled platforms. Worth confirming the platform still accepts new projects post-rebrand before applying | RESEARCH | 3 |
| C31.5 | Gitcoin Grants Stack (legacy) + Gitcoin.co current | Quadratic-funding rounds (legacy) → knowledge base + Allo Protocol tooling (current) | Variable (community-matched in legacy rounds) | Open-source public goods | https://gitcoin.co/ | **Verified 2026-09-05**: gitcoin.co homepage returns 200 (310KB, "Funding the commons" knowledge-base framing); gitcoin.co/grants returns 404. Old `grants.gitcoin.co`, `docs.allo.gitcoin.co`, `allo.gitcoin.co`, `passport.gitcoin.co`, `sdg.gitcoin.co` all DNS-fail. Gitcoin HQ has rebranded: Passport is now `passport.human.tech` (Human Passport), Allo lives at `allo.xyz`. The legacy QF rounds are effectively sunset in their old form | DEFUNCT | 2 |

---

| C31.6 | Optimism Retro Funding (RetroPGF) | OP-token rewards for ecosystem public goods | Round-specific; OP-allocated per round (millions of OP per round historically) | Public-goods projects serving the Optimism/Superchain ecosystem; ballot nomination + vote | https://vote.optimism.io/ | **Verified 2026-09-05**: vote.optimism.io returns 200 (Optimism Agora — the live voting/ballots UI). app.optimism.io/retropgf returns 200 (2.6KB SPA shell). retrofunding.optimism.io returns 200 (Optimism Atlas — round/ballot docs). Cross-listed in C5.3 of GRANTS_DATABASE.md (which marked it NOT-A-GRANT due to a stale quest URL — the Agora + Atlas paths are the correct live routes). Retro Funding rounds remain active; we should apply once our Superchain integration ships | QUALIFIED | 4 |
| C31.7 | Optimism Collective (gov hub) | Governance + Retro Funding announcements | Variable per round; same as C31.6 | All Optimism Collective members | https://gov.optimism.io/ | **Verified 2026-09-05**: gov.optimism.io returns 200 (Optimism Collective discussion forum, 80KB). Source of truth for upcoming Retro Funding round timelines + governance discussions. Use as a monitoring channel; not a direct apply URL. Pairs with vote.optimism.io for ballot nomination | RESEARCH | 3 |
| C31.8 | Arbitrum Foundation Grants Hub | Arbitrum Audit Program ($10M/12mo), ArbiFuel, DAO grants | Audit: variable per project; ArbiFuel: variable | Arbitrum-aligned projects | https://arbitrum.foundation/grants | **Verified 2026-09-05**: arbitrum.foundation/grants returns 200 (98KB — full grants hub with active programs listed). Cross-listed in C5.2 of GRANTS_DATABASE.md as the current active Arbitrum audit/grant hub. Audit Program is the strongest fit for our external audit budget. STIP program (https://arbitrum.foundation/stip) now 404 — STIP was a 2023–2024 program and is no longer accepting applications | QUALIFIED | 4 |
| C31.9 | Mantle EcoFund (Mantle Network ecosystem fund) | Ecosystem project support (Mantle L2) | Variable; case-by-case via foundation | Mantle ecosystem builders; contact-driven intake | https://www.mantle.xyz/ | **Verified 2026-09-05**: www.mantle.xyz returns 200 (206KB institutional-grade landing page). Mantle markets institutional RWA + L2 infrastructure. mantle.xyz/grants returns 404. The "EcoFund" branding is referenced on the site but has no dedicated intake URL — applications appear to be relationship-driven (venture-style). Treat as a QUALIFIED prospect with relationship-driven access; the official channel is the foundation BD team, not a public form | QUALIFIED | 3 |
| C31.10 | Base (builder programs + ecosystem) | Builder support, ecosystem credits (no public cash grant) | Variable; ecosystem credits + Base Builder Grant cohorts (Coinbase Ventures partnership) | Builders choosing Base as home | https://www.base.org/builders | **Verified 2026-09-05**: www.base.org returns 200; www.base.org/builders returns 200 (63KB "Base Build" page); www.base.org/ecosystem returns 200 (98KB). base.org/grants returns 404. Cross-listed in C5.5 (Base Ecosystem Fund) — which is an investment, not a grant. The /builders page is the live entry point; explicit cash grant amounts are not published. Apply via the builder cohort form when there is an open call | RESEARCH | 3 |

---

| C31.11 | Linea Voyage (community XP campaign) | XP/airdrop-style rewards for ecosystem participation | Variable; XP/points-based | Wallets that interact with Linea dApps; not a project-apply program | https://linea.build/ | **Verified 2026-09-05**: linea.build returns 200 (1.8MB hero page). linea.build/voyage returns 404; linea.build/ecosystem returns 403 (bot-blocked); docs.linea.build/voyage returns 404; docs.linea.build/builders returns 404. Linea has shifted their community-round product — the original "Voyage" campaign URL is gone. There is no current public intake URL for project-side grants from Linea (it was relationship-driven via the Consensys/Linea BD team). NOT a project grant program in 2026 | DEFUNCT | 1 |
| C31.12 | zkSync (ecosystem + builder support) | Builder ecosystem support via grants + ecosystem partnerships | Variable; case-by-case | Builders on ZKsync Era; contact-driven intake | https://docs.zksync.io/ecosystem | **Verified 2026-09-05**: docs.zksync.io/ecosystem returns 200 (307KB — current ecosystem/grants documentation). zksync.io returns 200; zksync.io/grants returns 404; docs.zksync.io/build/start-building/grants returns 404; grants.zksync.io DNS-fail. Like Linea/Mantle, the formal "Builder Grants" URL is gone but the docs.zksync.io/ecosystem page is the live entry point. Applications are relationship-driven via the Matter Labs ecosystem team | RESEARCH | 3 |
| C31.13 | Polygon Community Grants | Ecosystem project support | Historical $5k–$50k; current application not verified | Polygon-deploying projects | https://polygon.technology/community-grants | **Verified 2026-09-05**: polygon.technology/community-grants redirects to hadronfc.com (Hadron Founders Club — 56KB landing page). Cross-listed in C5.4 of GRANTS_DATABASE.md as NOT-A-GRANT (the polygon.technology program was discontinued). Hadron Founders Club is a private investor network, not a community round. Treat the Polygon-branded community-grants program as DEFUNCT | DEFUNCT | 1 |
| C31.14 | Human Passport (formerly Gitcoin Passport) | Sybil-resistance identity for QF rounds | Free to use; project-set integrations | Anyone with a wallet; projects integrate for QF anti-Sybil | https://passport.human.tech/ | **Verified 2026-09-05**: passport.human.tech returns 200 (523KB — "Human Passport, formerly Gitcoin Passport" branding is live). passport.gitcoin.co and app.passport.gitcoin.co both DNS-fail. Passport was rebranded from Gitcoin to Human Passport in 2024 and is alive at the new domain. Useful as anti-Sybil infrastructure for any QF round we host — not a funding source itself | RESEARCH | 3 |
| C31.15 | Allo Protocol (current home) | Open-source capital-allocation infrastructure | n/a (protocol/tooling, not a grant) | Builders integrating Allo for their own capital-allocation flows | https://allo.xyz/ | **Verified 2026-09-05**: allo.xyz returns 200 (599KB — current Allo Protocol landing page). The old docs.allo.gitcoin.co and allo.gitcoin.co DNS-fail. Allo is the open-source protocol that Gitcoin's old grants stack was built on; it now lives at its own domain. Not a funding source for us, but a tool we can use to host our own community rounds cheaply (or to integrate into other ecosystems' funding flows). Pairs naturally with Retro Funding + GIVbacks | RESEARCH | 3 |

---

| C31.16 | Moloch DAO | Treasury-grant framework + DAO infrastructure for public-goods DAOs | Variable per proposal (Moloch v3 / Baal); pooled ETH/trust treasuries | Public-goods DAOs meeting Moloch governance framework (proposal + vote) | https://molochdao.com/ | **Verified 2026-09-05**: molochdao.com returns 200 (125KB — active Moloch DAO documentation site). molochdao.com/v3 returns 404 (old URL; canonical reference is the v3 docs at the main domain). Moloch is both a DAO framework (with on-chain proposal mechanics) AND an active treasury. To receive Moloch funding we typically either (a) fork Moloch for our own treasury, or (b) submit a public-goods proposal to an existing Moloch pool. Strong fit for community-funded validator treasury design | QUALIFIED | 4 |
| C31.17 | The LAO (Legal DAO) | Legal wrapper for Ethereum-based DAOs; pooled investing + grants | Variable; pooled member capital | Accredited investor members; project proposal + KYC | https://www.thelao.io/ | **Verified 2026-09-05**: www.thelao.io returns 200 (50KB — LAO homepage with mission statement). thelao.io (apex) returns 200. The LAO is a maximalist legal DAO structure (Wyoming LLC + Moloch v2 framework) used as a template by many ecosystem DAOs. Member-funded; not a public grant. Useful as a legal-template reference if we want to set up our own community-funded foundation, less so as a direct funding source for X3 | RESEARCH | 3 |
| C31.18 | MetaCartel | DAO + Ventures — community-funded early-stage Web3 grants/investments | Variable; per proposal (DAO) + per deal (Ventures) | Builders + investors in the MetaCartel community; relationship-driven | https://www.metacartel.org/ | **Verified 2026-09-05**: metacartel.org returns 200 (129KB — active MetaCartel site). metacartel.vc DNS-fails. MetaCartel DAO is a public-goods-focused microgrant DAO (historically 1–10 ETH per grant) + MetaCartel Ventures (seed-stage Web3 fund). The DAO branch is the community-round fit; the Ventures branch is investment. Apply via their Discord/Discourse community when there's an open round | QUALIFIED | 3 |
| C31.19 | DAOhaus | No-code DAO framework + discovery hub for community treasuries | Variable per Moloch DAO instantiated via DAOhaus | Builders who want a community treasury; DAOhaus handles Moloch v3 deployment | https://daohaus.club/ | **Verified 2026-09-05**: daohaus.club returns 200 (509B SPA shell — JS-rendered); app.daohaus.club returns 200 (1.4KB SPA). github.com/HausDAO returns 200 (286KB — active GitHub org). DAOhaus is alive and active as a DAO deployment + discovery platform. Useful as the "how to spin up a community treasury" tool, less as a direct funding source for X3. The /moloch subdomain is also live as a discovery page | RESEARCH | 3 |
| C31.20 | Karma | Public-goods project directory + grant aggregator | n/a (directory, not a grant) | Verified public-goods projects | https://www.karma.app/ | **Verified 2026-09-05**: www.karma.app returns 200 (114B SPA shell — extremely thin; JS-renders full content). karma.app, karma.dao, app.karma.app, api.karma.app all DNS-fail or are unreachable. Karma is alive at the karma.app apex but the app+api subdomains are gone (the platform appears to have been refocused or partially sunset — the new canonical domain may differ). Useful as a public-goods directory if the directory is still actively curated; do not rely on app/API URLs | RESEARCH | 2 |

---

| C31.21 | Kernel | Web3 social-graph community with on-chain grants | Variable; KERNEL tokens for ecosystem grants | Kernel members; proposal + community vote | https://www.kernel.community/grants | **Verified 2026-09-05**: www.kernel.community/grants returns 200 (23KB — live grants page). www.kernel.community also 200 (130KB landing). Kernel is alive and actively running community grants via KERNEL token + proposal system. Strong fit for community/social-graph legitimacy (Kernel members are high-quality Web3 builders + investors); can be applied to for ecosystem support | QUALIFIED | 3 |
| C31.22 | Tribes by Cabin (Cabin DAO) | Creator/tribe "city" network with on-chain coordination | Variable per tribe; DAO-managed budgets | Tribes (creators/teams) joining Cabin; Cabin DAO governance | https://tribes.xyz/ | **Verified 2026-09-05**: tribes.xyz returns 200 (40KB — "Tribes" landing page). tribes.bycabin.com SSL handshake failure (likely expired). cabin.xyz returns 403 (parked domain). The Tribes product has migrated to tribes.xyz from the old Cabin/cabin.xyz stack; some legacy URLs are dead. Cabin DAO governance + tribe budgets are live. Useful as a community-coordination rail for spinning up an X3-aligned tribe | RESEARCH | 2 |
| C31.23 | Friends With Benefits (FWB) | Social DAO with membership-gated Discord + treasury proposals | Variable per proposal; FWB token-weighted voting | FWB token holders; governance participation | https://fwb.help/ | **Verified 2026-09-05**: fwb.help returns 200 (82KB — "Friends With Benefits" landing page). www.fwb.help SSL certificate expired (effectively dead for new visitors). www.fwb.io DNS-fails. FWB is alive at the apex fwb.help but the www subdomain has expired SSL. FWB has historically funded ecosystem builders via treasury proposals; strong social-graph legitimacy in Web3. Apply via FWB Discord/town-hall when relevant | RESEARCH | 3 |
| C31.24 | Orange DAO | Crypto-native founder community (Orange DAO Fellowship + accelerator) | Variable; per program cohort | Founders building crypto startups; cohort-based admission | https://www.orangedao.xyz/ | **Verified 2026-09-05**: www.orangedao.xyz returns 200 (716KB — large active community landing page). orangedao.xyz (apex) also 200. /accelerate returns 404 (specific program URL is gone, but the main fellowship/community page is live). Orange DAO is a 1000+ member founder community with active fellowship cohorts + an angel syndicate. Strong network for US/global crypto founders; can be applied to during cohort intake windows | QUALIFIED | 3 |
| C31.25 | Seed Club | Tokenized communities + Social Token accelerator | Variable per cohort | Builders of tokenized communities; cohort-based | https://seedclub.xyz/ | **Verified 2026-09-05**: seedclub.xyz SSL certificate expired (dead for HTTPS). www.seedclub.xyz connection times out. The Seed Club site is effectively unreachable on 2026-09-05. Treat as DEFUNCT until SSL/availability is restored. (Seed Club historically ran an accelerator for tokenized communities; status unclear until the site comes back online.) | DEFUNCT | 1 |
| C31.26 | Krause House | Sports-DAO + Web3 treasury templates (NBA-style fan ownership experiments) | Variable per DAO; member-funded | Members of the Krause House community | https://www.krausehouse.club/ | **Verified 2026-09-05**: www.krausehouse.club SSL handshake failure (expired/incorrect cert). krausehouse.club SSL handshake failure. krausehouse.xyz connection timeout. krausehouse.dao DNS-fails. Effectively all canonical URLs are unreachable. Treat as DEFUNCT. (Krause House was a sports-DAO that pioneered fan-ownership governance templates; the infrastructure appears abandoned as of 2026-09.) | DEFUNCT | 1 |

---


## Recommended next actions

| Priority | Action | Owner | Deadline |
|---|---|---|---|
| P0 | Stand up an **Open Collective** fiscal-hosted Collective for X3 (C31.1) — first donation rail on Day 0 | You | W0 |
| P0 | Verify and apply to **Giveth GIVbacks** (C31.2) once a verified Giveth project page exists | You | W2 |
| P1 | Submit a **Moloch v3 / Baal-style treasury proposal** (C31.16) or fork the framework for our own community treasury | You | W4 |
| P1 | Apply to **Arbitrum Audit Program** via arbitrum.foundation/grants (C31.8) — $10M/12mo pool is verified active | You | W4 |
| P1 | Join **Orange DAO** community (C31.24) and apply during the next fellowship cohort window | You | W2 |
| P1 | Submit a public-goods proposal to **MetaCartel DAO** (C31.18) — historic 1–10 ETH micro-grants | You | W4 |
| P2 | Apply to **Kernel** (C31.21) for an ecosystem grant when our social/community angle is ready | You | W8 |
| P2 | Apply to **Friends With Benefits** (C31.23) for a treasury proposal — fwb.help apex is live (use apex, not www) | You | W8 |
| P2 | Use **Optimism Retro Funding** (C31.6) via vote.optimism.io once our Superchain integration ships | You | W12+ |
| P2 | Confirm **Mantle EcoFund** (C31.9), **Base Builder** (C31.10), **zkSync Ecosystem** (C31.12) intake processes via their BD teams (no public forms) | You | W4 |
| P3 | Investigate **Allo Protocol** (C31.15) + **Geyser Fund** (C31.4) for hosting our own community round cheaply | Agent | W6 |
| P3 | Drop **Linea Voyage** (C31.11), **Polygon Community Grants** (C31.13), **Seed Club** (C31.25), **Krause House** (C31.26) from pursuit — all URLs verified DEFUNCT | — | — |
| P3 | Drop **Gitcoin Grants Stack** (C31.5) legacy URL — knowledge-base rebrand only; Allo lives at allo.xyz now | — | — |

---

## How to read the columns

- **PROGRAM** — name of the platform / program
- **COVERS** — what they pay for / what the platform does
- **AWARD** — typical size range or round-specific allocation
- **ELIGIBILITY** — basic requirements to apply or participate
- **URL** — verified live URL on 2026-09-05 (status reflects the actual HTTP probe)
- **FIT NOTES** — assessment of fit for X3 Atomic Star, with the verification date
- **STATUS** — pipeline stage: RESEARCH / QUALIFIED / APPLIED / AWARDED / REJECTED / DEFUNCT
- **CONFIDENCE** — 1-5 rating by research agent

---

## Notes for future research agents

1. **Gitcoin is fragmented across rebrand** — gitcoin.co (knowledge base), allo.xyz (protocol), passport.human.tech (Human Passport), grantstack.co (some QF rounds still live under GrantStack branding). Old `*.gitcoin.co` URLs (grants.gitcoin.co, passport.gitcoin.co, allo.gitcoin.co, docs.allo.gitcoin.co, sdg.gitcoin.co) all DNS-fail. Always probe before citing.
2. **Many "community round" URLs are SPA shells** — app.optimism.io/retropgf (2.6KB), www.karma.app (114B), daohaus.club (509B) are all JS-rendered Single Page Apps. A 200 OK with a tiny payload means "URL is live, content renders client-side" — not "page is empty". Probe the page title as a sanity check.
3. **Several "grants" programs are relationship-driven** — Mantle, Base, zkSync, Linea all have ecosystem budget but no public intake form. The canonical apply path is the foundation's BD / partnerships team, not a webpage.
4. **The Polkadot OpenGov Treasury (C4.2) is also a community round** — listed in C4 because it's the strongest fit for X3; not duplicated in C31.

---

## Summary

Status: C31 done. Rows: 26. Verified: 26 (all rows probed live on 2026-09-05). Saved to `grants/C31-crowdfunding.md`.
