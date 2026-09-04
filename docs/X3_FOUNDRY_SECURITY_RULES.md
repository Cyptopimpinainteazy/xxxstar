# X3 Foundry Security Rules

## Security Pipeline Overview

Every dApp generated through X3 Foundry passes through a multi-stage security pipeline before deployment. This ensures all deployed applications are safe, transparent, and compliant.

```
User Prompt
    │
    ▼
┌─────────────────────┐
│  Template Integrity  │── Checks template hasn't been tampered
│  Check              │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Compiler           │── Verifies source compiles without errors
│  Verification       │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Static Analysis    │── Scans for vulnerabilities
│  • Reentrancy       │
│  • Ownership        │
│  • Access Control   │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Fee Sanity Check   │── Verifies fee transparency
│  • Not hidden       │
│  • Not excessive    │
│  • Principal safe   │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Scam Detection     │── Detects rug-pull patterns
│  • Ownership        │
│  • Upgradeability   │
│  • Backdoors        │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  License Compliance │── Checks dependency licenses
│  • MIT/Apache/GPL   │
│  • No restricted    │
│  • Compatible       │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Fuzz Testing       │── Random input testing
│  • Edge cases       │
│  • Overflow         │
│  • Underflow        │
└─────────────────────┘
    │
    ▼
┌─────────────────────┐
│  Dry-Run Deployment │── Simulates deployment
│  • Gas estimation   │
│  • State changes    │
│  • Event emission   │
└─────────────────────┘
    │
    ▼
         DEPLOY or BLOCK
```

## Static Analysis Checks

### Reentrancy Detection
- Scans for external calls before state updates
- Checks for missing reentrancy guards
- Validates checks-effects-interactions pattern
- Flags unprotected withdrawal functions

### Ownership Pattern Analysis
- Verifies owner/admin roles are defined
- Checks for multi-sig or timelock on sensitive functions
- Flags single-owner-with-full-control patterns
- Validates ownership transfer mechanisms

### Access Control Verification
- Checks all external functions have proper modifiers
- Verifies onlyOwner on admin functions
- Flags public functions that modify critical state
- Validates role-based access where applicable

## Fee Transparency Verification

### What We Check
- Platform fee is explicitly declared in the fee config
- Fee percentage is within allowed bounds (0.1% – 10%)
- Fee is charged on revenue, not principal
- Fee schedule is visible to end users
- No hidden fee mechanisms in contract code

### What Gets Blocked
- Hidden fee configurations
- Fees charged on user deposits
- Dynamic fees that can exceed max cap
- Fees that can be changed without notice
- Fees routed to unknown addresses

## Principal Safety Verification

**This is the most critical check.** We verify that user principal (deposits, collateral, escrowed funds) can never be taken by the platform fee mechanism.

### Pass Criteria
- Platform fee only applies to protocol revenue
- User deposits are segregated from fee logic
- Withdrawal functions return full user principal
- No mechanism exists to drain user balances
- Fee calculation never touches user principal

### Fail Criteria (Blocks Deployment)
- Fee is calculated on total user deposits
- Admin can withdraw user funds
- Fee routing can access user balances
- No separation between protocol funds and user funds

## Scam Pattern Detection

### Detected Patterns
| Pattern | Risk | Description |
|---------|------|-------------|
| Rug-pull ownership | Critical | Owner can drain all funds |
| Hidden mint function | Critical | Unlimited token minting by owner |
| Fee change without notice | High | Owner can increase fees to 100% |
| Fake token | High | Contract impersonates known tokens |
| Honeypot | Critical | Users can buy but not sell |
| Flash loan attack vector | High | Unprotected price oracles |
| Self-destruct | Critical | Contract can be destroyed |
| Upgradeable proxy abuse | High | Implementation can be swapped to malicious |

### What Gets Blocked
- Any critical finding → deployment blocked
- Hidden fee → deployment blocked
- Principal can be skimmed → deployment blocked
- Ownership can rug funds → deployment blocked
- Unverified external calls control funds → deployment blocked
- Treasury router can drain user balances → deployment blocked

## License Compliance

### Allowed Licenses
- MIT
- Apache-2.0
- BSD-2-Clause / BSD-3-Clause
- Unlicense
- CC0-1.0
- GPL-3.0 (with restrictions)
- LGPL-3.0

### Blocked Licenses
- Unlicensed / All Rights Reserved
- Custom restrictive licenses
- AGPL-3.0 (for certain use cases)
- Licenses incompatible with Apache-2.0

## Deployment Gates

Deployment proceeds only when ALL gates pass:

```
Gate 1: Template Integrity    [PASS/FAIL]
Gate 2: Compiler Check        [PASS/FAIL]
Gate 3: Static Analysis       [PASS/FAIL]
Gate 4: Fee Sanity            [PASS/FAIL]
Gate 5: Principal Safety      [PASS/FAIL]
Gate 6: Scam Detection        [PASS/FAIL]
Gate 7: License Compliance    [PASS/FAIL]
Gate 8: Fuzz Tests            [PASS/FAIL]
Gate 9: Dry-Run               [PASS/FAIL]
─────────────────────────────────────
Overall:                      [PASS/FAIL]
```

## Security Report Format

```json
{
  "project_id": "uuid",
  "template_id": "nft_marketplace",
  "risk_score": 12,
  "passed": true,
  "warnings": [
    "Contract uses transfer() instead of call() for ETH sends"
  ],
  "critical_findings": [],
  "fee_findings": [],
  "ownership_findings": [],
  "license_findings": [],
  "simulation_receipt": "0x...",
  "auditor_signature": "0x..."
}
```

### Risk Score Calculation
- Base score: 0
- +10 per critical finding
- +5 per high finding
- +2 per medium finding
- +1 per low finding
- +5 for hidden fee
- +10 for principal safety violation
- Score > 20 blocks deployment

## Audit Receipt Verification

Every audit produces a signed receipt that can be verified on-chain:
1. Audit report hash is computed
2. Auditor signs the hash
3. Signature is stored on-chain
4. Anyone can verify the audit receipt
5. Receipt includes simulation results
