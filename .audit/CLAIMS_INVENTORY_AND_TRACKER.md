# X3STAR Claims Inventory & Remediation Tracker
**Purpose:** Track every unverified claim to file, line, and fix status.  
**Last Updated:** May 16, 2026

---

## LANDING PAGE: x3star-landing.html

### Line-by-Line Claims Audit

#### Meta Tags (Lines 1-10)
```html
<meta name="description" content="X3STAR is a high-performance blockchain built for DeFi, grants, and validators. Join the $14.7M prefunding round.">
```
- **Claim:** "$14.7M prefunding round"
- **Status:** ❌ UNVERIFIED
- **Required Fix:** Link to `/funding` proof page or remove
- **Risk:** Meta description visible in search results → SEO liability

---

#### HERO Section (Lines 350-430)

**Line 389:**
```html
Round III Prefunding — $14.7M Raised — 6 Days Remaining
```
- **Claims:**
  - $14.7M Raised
  - Round III (implies earlier rounds exist)
  - 6 Days Remaining (STALE COUNTDOWN)
- **Status:** 🔴 CRITICAL
- **Fix:** Replace with dynamic date or remove countdown
- **Proof Required:** /funding page with cap table

**Line 399:**
```html
The <strong>high-throughput blockchain</strong> built for real-world DeFi. 4,200 TPS. Sub-second finality. $5M+ in developer grants. Join 312 investors already in.
```
- **Claims:**
  - 4,200 TPS
  - Sub-second finality
  - $5M+ developer grants
  - 312 investors
- **Status:** 🟡 PARTIALLY VERIFIABLE
- **Fixes:**
  - 4,200 TPS: Link to `/benchmarks` (must show test date, hardware, workload)
  - $5M grants: Link to `/grants` (must show disbursed vs. allocated)
  - 312 investors: Link to `/funding` or remove

**Lines 409-410:**
```html
<div class="hm-item"><div class="hm-val" id="hm1">$14.7M</div><div class="hm-key">Raised</div></div>
<div class="hm-item"><div class="hm-val">312</div><div class="hm-key">Investors</div></div>
```
- **Claims:** $14.7M, 312 investors (DUPLICATED from line 399)
- **Status:** ❌ UNVERIFIED
- **Action:** Add data-verifiable="false" attribute; link to proof page in tooltip

**Lines 426-427:**
```html
<div class="tr-item"><span class="tr-num" id="tr1">$14.7M</span><div class="tr-label">Funds Raised</div></div>
<div class="tr-item"><span class="tr-num">312</span><div class="tr-label">Global Investors</div></div>
```
- **Status:** ❌ TRIPLE REPEATED (lines 389, 409, 426)
- **Action:** Consolidate; reference single `/funding` source

**Lines 514-520:**
```html
<div class="tc-label">Security Audits</div>
<div class="tc-number" style="color:var(--gold)">3<span class="tc-unit">✓</span></div>
<div style="font-size:13px;color:var(--muted);margin-top:8px;">Certik · Trail of Bits · OpenZeppelin</div>
```
- **Claims:**
  - 3 audits completed
  - Firm names: Certik, Trail of Bits, OpenZeppelin
- **Status:** 🔴 CRITICAL
- **Required Proofs:**
  - Audit report PDFs with dates
  - Scope docs (code version, coverage %)
  - Severity findings summary
- **Fix:** Links: `[Certik Report](/audits/certik-report.pdf)` etc.

---

#### TECH COMPARISON Section (Lines 525-545)

**Line 530:**
```html
<div class="cmp-tps" style="color:var(--gold)">4,200 TPS</div>
<div style="font-size:11px;color:var(--muted);margin-top:8px;">$0.001 fee · 0.4s</div>
```
- **Claims:**
  - 4,200 TPS (contradicts Decrypt quote: "14M TPS potential")
  - $0.001 fee
  - 0.4s finality (contradicts hero "sub-second")
- **Status:** 🟠 AMBIGUOUS (bench vs. target unclear)
- **Fix:** Add context: `4,200 TPS (testnet, x86 hardware) | Target: 14M TPS (mainnet, full network)`

---

#### VALIDATOR ECOSYSTEM (Line ~555)

```html
<div class="ec-text">1,847 validators securing the network. Genesis validators earn 18% APY + protocol fees.</div>
```
- **Claims:**
  - 1,847 validators (testnet or mainnet?)
  - 18% APY
  - Protocol fees (undefined)
- **Status:** 🔴 CRITICAL
- **Risk:** APY claim without backing model = FTC violation
- **Required Fix:**
  - Link to `/validator-economics` page with:
    - APY calculation formula
    - Historical APY data (if any)
    - Fee distribution model
    - Slashing risks

---

#### TEAM SECTION (Lines 595-650)

**Team Card 1: David K.**
```html
<div class="team-name">David K.</div>
<div class="team-role">CEO & Founder</div>
<div class="team-bio">Former Protocol Lead at Ethereum Foundation. 12 years in distributed systems. PhD in Computer Science.</div>
<div class="team-prev"><span class="prev-tag">ETHEREUM</span><span class="prev-tag">MIT</span><span class="prev-tag">Y COMBINATOR</span></div>
```
- **Claims:**
  - Former Protocol Lead at EF
  - 12 years in distributed systems
  - PhD in Computer Science
  - MIT, Y Combinator background
- **Status:** ❌ UNVERIFIED
- **Required Proofs:**
  - GitHub profile link (EF commits)
  - PhD issuing university + year
  - MIT/YC association details
- **Fix:** Add links: `[GitHub](https://github.com/davidk) · [PhD](https://xxx.edu) · [EF Profile](https://xxx)`

**Team Card 2: Sarah L.**
```html
<div class="team-bio">Built Solana's validator client. Former Principal Engineer at Jump Crypto. 15 years systems engineering.</div>
```
- **Claims:**
  - Built Solana's validator client
  - Principal Engineer at Jump Crypto
  - 15 years systems engineering
- **Status:** ❌ UNVERIFIED
- **Required Proofs:**
  - Solana GitHub commits
  - Jump Crypto employment verification
  - LinkedIn or public CV
- **Fix:** Link to GitHub profile + Solana commits

**Team Card 3: Marcus R.**
```html
<div class="team-bio">Ex-Goldman Sachs Digital Assets. Led $2B+ in blockchain deals. Former Head of DeFi at Binance.</div>
```
- **Claims:**
  - Ex-Goldman Sachs Digital Assets
  - Led $2B+ blockchain deals (specific deals?)
  - Former Head of DeFi at Binance
- **Status:** 🔴 CRITICAL ($2B+ claim unsubstantiated)
- **Required Proofs:**
  - Deal list with descriptions
  - Dates of GS employment
  - Binance position verification
- **Fix:** Either provide deal list or soften: "Advised on $2B+ in deals"

**Team Card 4: Aisha J.**
```html
<div class="team-bio">Built Cosmos grant program from 0 to $20M. Former ecosystem lead at Polygon. 200+ projects onboarded.</div>
```
- **Claims:**
  - Built Cosmos grant program (0 → $20M)
  - Former ecosystem lead at Polygon
  - 200+ projects onboarded
- **Status:** ❌ UNVERIFIED
- **Required Proofs:**
  - Cosmos grant program history (verify $20M allocation)
  - Polygon ecosystem lead verification
  - Project list or reference
- **Fix:** Link to public Cosmos grant records + Polygon employment verification

---

#### PRESS SECTION (Lines 650-680)

**All 5 press quotes:**
```html
<div class="press-logo"><div class="pl-name">COINDESK</div><div class="pl-quote">"X3STAR is the most technically impressive new L1 of 2024"</div></div>
<div class="press-logo"><div class="pl-name">THE BLOCK</div><div class="pl-quote">"Series A backed by Apex Ventures in landmark deal"</div></div>
<div class="press-logo"><div class="pl-name">DECRYPT</div><div class="pl-quote">"14M TPS potential positions X3STAR as Ethereum competitor"</div></div>
<div class="press-logo"><div class="pl-name">COINTELEGRAPH</div><div class="pl-quote">"Grant program signals major ecosystem commitment"</div></div>
<div class="press-logo"><div class="pl-name">BLOOMBERG</div><div class="pl-quote">"Institutional-grade blockchain draws VC attention"</div></div>
```

| Publication | Quote | Status | Fix |
|---|---|---|---|
| **COINDESK** | "Most technically impressive new L1 of 2024" | ❌ NO URL | Remove OR link to article URL + date |
| **THE BLOCK** | "Series A backed by Apex Ventures" | 🔴 CONTRADICTS | Funding section calls it "Round III Prefunding"; contradicts $14.7M Series A |
| **DECRYPT** | "14M TPS potential" | 🟠 AMBIGUOUS | Tech card says 4,200 TPS; clarify potential vs. current |
| **COINTELEGRAPH** | "Grant program signals..." | 🟡 GENERIC | Vague; verify actual article exists |
| **BLOOMBERG** | "Institutional-grade blockchain" | 🟡 GENERIC | No date; verify article exists |

**FIX PATTERN:** Each quote must have:
```html
<a href="https://coindesk.com/article/..." target="_blank" class="press-verify">
  <div class="pl-quote">"..."</div>
  <div class="pl-source">CoinDesk · May 2024</div>
</a>
```

---

#### CTA SECTION (Lines 720-750)

**Button 1: "⬡ Buy X3S Tokens"**
```html
<button class="btn btn-gold" ... onclick="location.href='x3star-token-presale.html'">⬡ Buy X3S Tokens</button>
```
- **Risk:** 🔴 CRITICAL — Live purchase button without legal disclaimers
- **Required Fix:**
  - Add modal: "⚠️ This is testnet. Not a live presale. Tokens non-transferable."
  - Link to `/terms`
  - Require checkbox: "I understand the risks"
- **Renamed:** "Request Presale Access" (removes live-sale appearance)

**Button 2: "Own a Validator Node"**
- **Risk:** 🟡 Could be construed as unregistered security offering
- **Required Fix:** Add disclaimer: "Validator ownership does not confer securities rights"

**Button 3: "Investor Deck"**
- **Risk:** 🟡 Implies investment opportunity; needs legal wrapper
- **Required Fix:** Add disclaimer before download: "Restricted to accredited investors"

**Trust Markers Section:**
```html
<div class="ct-item"><span class="ct-check">✓</span>Audited by Certik</div>
<div class="ct-item"><span class="ct-check">✓</span>KYC Verified</div>
<div class="ct-item"><span class="ct-check">✓</span>Smart Contract Locked</div>
<div class="ct-item"><span class="ct-check">✓</span>Multi-sig Treasury</div>
<div class="ct-item"><span class="ct-check">✓</span>No team unlock at TGE</div>
```

| Trust Marker | Status | Fix |
|---|---|---|
| Audited by Certik | ❌ UNVERIFIED (no report link) | Link to audit report |
| KYC Verified | ⚠️ NEEDS CONTEXT | Link to privacy policy; explain KYC provider |
| Smart Contract Locked | ❌ NO PROOF | Link to blockchain explorer address + lock details |
| Multi-sig Treasury | ❌ NO PROOF | Link to on-chain treasury address |
| No team unlock at TGE | ⚠️ NEEDS PROOF | Link to token vesting schedule |

---

## RELATED FILES

### x3star-token-presale.html

**Issues:**
- Purchase flow without risk disclosure
- No legal documents linked
- "6-day countdown" (stale if hardcoded)
- Token price claim: "$0.12 post-round" (unsubstantiated)

**Required Fixes:**
- Add risk modal before purchase
- Link to whitepaper (currently stub)
- Link to terms of service
- Add SEC disclaimer if targeting US investors

---

### x3star-investor-relations.html

**Issues:**
- "Series A" implied without clarity
- Investor deck CTAs without accreditation language
- Metrics displayed without sources

**Required Fixes:**
- Clarify funding round structure (Seed → Series A → public?)
- Add accreditation disclaimer on investor deck download
- Link every metric to source page

---

### x3star-whitepaper.html

**CRITICAL ISSUE:**
```html
<p>This page is being wired to the live X3 documentation pipeline. Until the canonical PDF and data index are published, this placeholder remains to prevent broken navigation.</p>
```
- **Status:** ❌ PLACEHOLDER (4 days old as of May 16, 2026)
- **Fix:** Publish actual whitepaper PDF

---

## REMEDIATION CHECKLIST

### Phase 1: Emergency Risk Reduction ✏️ (Edit Files)

- [ ] **x3star-landing.html Line 7:** Add `[TESTNET - Click for disclaimers]` link to meta description
- [ ] **x3star-landing.html Line 389:** Replace "6 Days Remaining" with "[View offer details →]"
- [ ] **x3star-landing.html Lines 409, 426:** Wrap $14.7M in `<span data-verify="false">` tag
- [ ] **x3star-landing.html Line 519:** Convert "Certik · Trail of Bits · OpenZeppelin" to links:
  ```html
  <a href="/audits">Certik · Trail of Bits · OpenZeppelin</a>
  ```
- [ ] **x3star-landing.html ~720:** Rename button: "Buy X3S Tokens" → "Join Whitelist"
- [ ] **x3star-landing.html CTA section:** Add legal disclaimer modal

### Phase 2: Create Proof Pages 📄 (New Files)

- [ ] Create `/audits.html` — Link all 3 audit reports (or mark as "pending")
- [ ] Create `/funding.html` — Show $14.7M breakdown, investor count, cap table (anonymized)
- [ ] Create `/team-bios.html` — Full bios with GitHub/LinkedIn verification links
- [ ] Create `/validator-economics.html` — 18% APY calculation, historical data, fees
- [ ] Create `/benchmarks.html` — 4,200 TPS test methodology, hardware specs, date
- [ ] Create `/grants.html` — $5M+ allocation breakdown, disbursed vs. available
- [ ] Create `/press.html` — Each quote with publication URL + date OR remove

### Phase 3: Legal Wrapper 🏛️ (New Files)

- [ ] Create `/risk-disclosure.html` — "This is testnet. Not investment advice. Not insured."
- [ ] Create `/terms-of-service.html` — Link from all CTAs
- [ ] Create `/privacy-policy.html` — KYC & data handling disclosure
- [ ] Update footer: Add links to T&S, Privacy, Risk Disclosure

### Phase 4: Technical Cleanup 🛠️ (Edit Files)

- [ ] Standardize TPS claims: "4,200 TPS (testnet) | 14M potential (projected)"
- [ ] Fix countdown: Remove hardcoded "6 Days" or use serverside date
- [ ] Reconcile finality: "sub-second" vs "0.4s" — pick one and document
- [ ] Validator count: Add "(as of May 16, 2026)" timestamp

---

## VERIFICATION WORKFLOW

**For each claim, verify:**

1. **Source Exists?** (URL, on-chain address, GitHub commit, etc.)
2. **Public?** (Can user independently verify?)
3. **Current?** (Timestamp provided? Not stale?)
4. **Quantified?** (Number or percentage provided? Or vague?)
5. **Linked?** (Click-through to proof page?)

**Status Codes:**
- ✅ VERIFIED — Source confirmed, link provided
- 🟡 PARTIAL — Requires caveat ("pending audit", "projected", "as of May 16")
- ❌ UNVERIFIED — No source; requires removal or proof-page link
- 🔴 CRITICAL — Poses legal/regulatory risk if not fixed

---

## PRIORITY SCORING

| Issue | Severity | Fix Time | Start Order |
|-------|----------|----------|---|
| Add "TESTNET" banner | 🔴 CRITICAL | 5 min | 1️⃣ |
| Fix audit firm links | 🔴 CRITICAL | 10 min | 2️⃣ |
| Remove/fix stale countdown | 🟡 HIGH | 5 min | 3️⃣ |
| Rename "Buy" button | 🟡 HIGH | 5 min | 4️⃣ |
| Publish whitepaper PDF | 🟡 HIGH | 2 hours | 5️⃣ |
| Create `/audits` proof page | 🟡 HIGH | 1 hour | 6️⃣ |
| Create `/funding` proof page | 🟡 HIGH | 1.5 hours | 7️⃣ |
| Create `/team-bios` proof page | 🟡 HIGH | 1 hour | 8️⃣ |
| Verify & link all press quotes | 🟡 HIGH | 2 hours | 9️⃣ |
| Add legal footer + T&S | 🟡 HIGH | 3 hours | 🔟 |

**Total Time Estimate:** 10–12 hours (if proof pages have verified data)

---

## NEXT STEPS

1. **Immediate:** Share this audit with legal team
2. **This Week:** Create proof pages (assuming data exists internally)
3. **Next Sprint:** Implement all fixes in priority order
4. **Post-Fix:** Re-audit and publish compliance scorecard

---

**Audit File:** `.audit/UNVERIFIED_CLAIMS_AUDIT.md`  
**Tracking Issue:** Create GitHub issue: `[BUG] Unverified claims audit — 25 findings to fix`  
**Legal Review:** ⚠️ REQUIRED before public launch
