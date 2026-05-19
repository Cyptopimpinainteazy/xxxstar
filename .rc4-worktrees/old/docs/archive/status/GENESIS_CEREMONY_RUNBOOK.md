# Genesis Ceremony Runbook

**Document Version:** 1.0  
**Date:** April 26, 2026  
**Status:** ⚠️ HISTORICAL / INTERNAL PRE-LAUNCH PLAN — Not current public readiness status  
**Target Audience:** Core team, founding validators, operations lead

---

## ⚠️ CRITICAL PRE-REQUISITES

**DO NOT PROCEED** unless ALL of the following are true:

- ✅ All 9 security blockers (6 S0 + 3 S1) are RESOLVED
- ✅ ProofForge re-run shows 0 S0/S1 findings
- ✅ External security audit(s) complete with PASS rating
- ✅ Bug bounty program ran 4+ weeks with no Critical/High findings
- ✅ Public testnet ran 8+ weeks successfully (50+ validators, no chain halts)
- ✅ Legal compliance review complete
- ✅ All founding validators passed KYC/background checks
- ✅ Incident response team on standby

**Current Status:** ⚠️ MAINNET READINESS BLOCKED / UNDER REVIEW (historical planning document only)  
**Canonical Status:** [docs/CURRENT_MAINNET_STATUS.md](../CURRENT_MAINNET_STATUS.md)

---

## Executive Summary

The genesis ceremony is a **one-time, irreversible event** that launches the X3 ATOMIC STAR mainnet. This runbook provides step-by-step procedures for:

1. **Validator Selection** (Week -4 to -2)
2. **Key Generation Ceremony** (Week -2)
3. **Genesis Spec Creation** (Week -1)
4. **Launch Coordination** (Launch Day)
5. **Post-Launch Verification** (Day 1-7)
6. **Emergency Rollback** (if needed)

**Timeline:** 4 weeks preparation + Launch Day + 1 week verification  
**Team Size:** 8-12 people (3 core devs, 5-10 founding validators, 1 ops lead, 1 comms lead)  
**Budget:** $50k-$80k (validator incentives, infrastructure, coordination)

---

## Phase 1: Validator Selection (Week -4 to -2)

### Objectives
- Select 5-10 founding validators
- Ensure geographic distribution
- Verify technical capability and security practices
- Establish communication channels

### Selection Criteria

**Technical Requirements:**
```yaml
hardware:
  cpu: "8+ cores (AMD EPYC or Intel Xeon recommended)"
  ram: "32 GB minimum, 64 GB recommended"
  disk: "1 TB NVMe SSD minimum"
  network: "1 Gbps symmetric, <50ms latency to peers"
  
datacenter:
  uptime_sla: "99.9%+"
  ddos_protection: "Required"
  backup_power: "Required (UPS + generator)"
  monitoring: "24/7 NOC"

security:
  key_management: "HSM or secure enclave required"
  access_control: "MFA + audit logging"
  incident_response: "Documented procedures"
  background_check: "Completed and verified"
```

**Geographic Distribution:**
- North America: 2-3 validators
- Europe: 2-3 validators
- Asia-Pacific: 1-2 validators
- Other regions: 1-2 validators

**Reputation Requirements:**
- Active in blockchain community (GitHub, forums, social)
- No history of slashing on other networks
- Reference checks from 2+ other networks

### Selection Process

**Week -4:**
1. Publish validator application form:
   ```markdown
   # X3 ATOMIC STAR Founding Validator Application
   
   **Organization:** _________________
   **Primary Contact:** _________________
   **Email:** _________________
   **Telegram/Discord:** _________________
   
   **Technical Infrastructure:**
   - [ ] Dedicated servers (not shared/VPS)
   - [ ] 1 Gbps+ network
   - [ ] 24/7 monitoring
   - [ ] DDoS protection
   - [ ] HSM or secure key storage
   
   **Experience:**
   - [ ] Validator experience on other networks (list):
   - [ ] Substrate/Polkadot ecosystem experience
   - [ ] Slashing history: Y/N (explain if yes)
   
   **Datacenter Location:**
   - City: _________________
   - Country: _________________
   - Provider: _________________
   
   **References:**
   1. Network + Contact: _________________
   2. Network + Contact: _________________
   ```

2. Distribute application via:
   - X3 Discord/Telegram announcement
   - Twitter/X post
   - Direct outreach to known validators
   - Email to testnet participants

3. **Deadline:** Applications due Week -3 Friday 5pm UTC

**Week -3:**
1. Review all applications
2. Technical screening calls (30min each)
3. Reference checks
4. Background verification (if required by legal)

**Week -2:**
1. Select final 5-10 founding validators
2. Send acceptance notifications
3. Send rejection notifications (with waitlist option)
4. Create private coordination channel (Telegram/Discord)

**Validator Acceptance Template:**
```
Subject: X3 ATOMIC STAR Founding Validator - ACCEPTED

Dear [Validator Name],

Congratulations! You have been selected as a founding validator for X3 ATOMIC STAR mainnet.

NEXT STEPS:
1. Join coordination channel: [Telegram/Discord invite]
2. Attend key generation ceremony: [Date] [Time] UTC
3. Review technical runbook: [Link]
4. Test infrastructure: [Testnet endpoint]

GENESIS TIMELINE:
- Week -2: Key generation ceremony (MANDATORY attendance)
- Week -1: Genesis spec review + signing
- Launch Day: [Date] [Time] UTC
- Week +1: Stability monitoring

COMPENSATION:
- Genesis allocation: [Amount] X3 tokens
- First year rewards: Estimated [Amount] X3 tokens
- Monthly infrastructure stipend: $[Amount] (first 6 months)

REQUIREMENTS:
- Attend all coordination meetings
- Maintain 99.9%+ uptime
- Respond to security incidents within 1 hour
- Sign NDA (if applicable)

POINT OF CONTACT:
- Technical Lead: [Name] [Email] [Telegram]
- Operations Lead: [Name] [Email] [Telegram]

Welcome to the founding validator set!

[Core Team Signature]
```

---

## Phase 2: Key Generation Ceremony (Week -2)
 ... [rest of document content preserved exactly as original] ...

*This archived copy preserves the original historical runbook content for reference.*
